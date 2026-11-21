use crate::{
    domain::{
        listing_preview::{IssueCode, IssueSeverity, ListingStatus},
        product_export::ExportReport,
        shopify_export::*,
    },
    error::AppError,
    infrastructure::{
        db::Database,
        export_storage::{self, Workspace},
        media_storage::MediaStorage,
    },
    services::{
        batch_export_service as selection, listing_preview_service as preview, shopify_csv,
    },
};
use sha2::{Digest, Sha256};
use std::{io::Write, path::Path};
use uuid::Uuid;

// Shared product projection only; each CSV profile owns its tokens and image policy.
pub(crate) fn product_preview(
    snapshot: &preview::Snapshot,
) -> Result<(ShopifyProductPreview, String), AppError> {
    let p = preview::build_for_csv(snapshot.clone());
    let token = p.snapshot_token;
    let issues: Vec<_> = p
        .issues
        .into_iter()
        .filter_map(|mut issue| {
            match issue.code {
                IssueCode::ImagesMissing | IssueCode::ImageUnavailable => return None,
                IssueCode::PriceMissing => {
                    issue.severity = IssueSeverity::Error;
                    issue.message = "価格が未設定のためShopify用CSVを出力できません。";
                }
                _ => (),
            }
            Some(issue)
        })
        .collect();
    let errors = issues.iter().any(|i| i.severity == IssueSeverity::Error);
    let warnings = issues.iter().any(|i| i.severity == IssueSeverity::Warning);
    let product = ShopifyProductPreview {
        handle: format!("lp-{}", p.product_id),
        product_id: p.product_id,
        name: snapshot.product.name.clone(),
        vendor: snapshot.product.brand.clone(),
        sku: p.sku,
        title: p.title,
        description: p.description,
        price_yen: p.price_yen,
        status: if errors {
            ListingStatus::Blocked
        } else if warnings {
            ListingStatus::Warning
        } else {
            ListingStatus::Ready
        },
        issues,
    };
    shopify_csv::validate(&product)?;
    Ok((product, token))
}

pub(crate) fn build(snapshots: &[preview::Snapshot]) -> Result<ShopifyPreview, AppError> {
    let mut hash = Sha256::new();
    hash.update(b"ListingPilot-shopify-csv-v1\0");
    let mut products = Vec::new();
    let mut has_warnings = false;
    let mut blocked = false;
    for snapshot in snapshots {
        let (product, token) = product_preview(snapshot)?;
        hash.update(product.product_id.as_bytes());
        hash.update(token.as_bytes());
        has_warnings |= product
            .issues
            .iter()
            .any(|i| i.severity == IssueSeverity::Warning);
        blocked |= product.status == ListingStatus::Blocked;
        products.push(product);
    }
    // Preview and execution use the exact same renderer, safety and byte limit checks.
    // A blocked price is represented in the DTO, never an empty CSV price.
    if !blocked {
        shopify_csv::generate(&products)?;
    }
    Ok(ShopifyPreview {
        snapshot_token: format!("{:x}", hash.finalize()),
        products,
        has_warnings,
        blocked,
    })
}

pub(crate) fn get(database: &Database, ids: &[String]) -> Result<ShopifyPreview, AppError> {
    build(&selection::read(database, &selection::normalize(ids)?)?)
}

fn prepare(
    database: &Database,
    input: &ShopifyExportInput,
) -> Result<(Vec<preview::Snapshot>, ShopifyPreview, String), AppError> {
    let snapshots = selection::read(database, &selection::normalize(&input.product_ids)?)?;
    let p = build(&snapshots)?;
    if p.blocked {
        return Err(AppError::ShopifyExportBlocked);
    }
    if p.snapshot_token != input.snapshot_token {
        return Err(AppError::StaleData);
    }
    if p.has_warnings && input.warning_token.as_deref() != Some(p.snapshot_token.as_str()) {
        return Err(AppError::ExportWarningRequired);
    }
    let csv = shopify_csv::generate(&p.products)?;
    Ok((snapshots, p, csv))
}

pub(crate) fn preflight(database: &Database, input: &ShopifyExportInput) -> Result<(), AppError> {
    prepare(database, input).map(|_| ())
}

pub(crate) fn export(
    database: &Database,
    media: &MediaStorage,
    input: &ShopifyExportInput,
    parent: &Path,
) -> Result<ExportReport, AppError> {
    export_with(database, media, input, parent, || {})
}

pub(crate) fn export_with(
    database: &Database,
    media: &MediaStorage,
    input: &ShopifyExportInput,
    parent: &Path,
    before_finish: impl FnOnce(),
) -> Result<ExportReport, AppError> {
    let parent = export_storage::destination(parent, media.data_directory()?)?;
    let (snapshots, p, csv) = prepare(database, input)?;
    let target = parent.join(format!(
        "ListingPilot_Shopify_{}_{}",
        chrono::Local::now().format("%Y%m%d"),
        Uuid::new_v4()
    ));
    let mut workspace = Workspace::new(&parent)?;
    let operation = (|| {
        {
            let mut file = workspace.file("shopify-products.csv")?;
            file.write_all(csv.as_bytes())
                .and_then(|_| file.sync_all())
                .map_err(|e| export_storage::write_error("shopify_csv", &e))?;
        }
        before_finish();
        database.with_mut(|db| {
            let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            for snapshot in &snapshots {
                if preview::read(&tx, &snapshot.product.id)? != *snapshot {
                    return Err(AppError::StaleData);
                }
            }
            export_storage::destination(&parent, media.data_directory()?)?;
            workspace.finish(&target)
            // Read-only transaction drops; no fallible COMMIT after publication.
        })
    })();
    if let Err(error) = operation {
        let unknown = error == AppError::ExportUnknown;
        let cleanup_failed = !unknown && workspace.cleanup().is_err();
        if unknown || cleanup_failed {
            return Ok(ExportReport::Failed {
                error,
                retained_folder: if unknown { parent } else { workspace.path }
                    .to_string_lossy()
                    .into_owned(),
                cleanup_failed,
            });
        }
        return Err(error);
    }
    Ok(ExportReport::Success {
        folder: target.to_string_lossy().into_owned(),
        product_count: snapshots.len(),
        attribute_count: 0,
        image_count: 0,
        has_warnings: p.has_warnings,
    })
}
