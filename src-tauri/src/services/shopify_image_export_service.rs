use crate::{
    domain::{
        image_url::{self, ImageUrlSetting, PLATFORM},
        listing_preview::{IssueSeverity, ListingStatus},
        product_export::ExportReport,
        shopify_image_export::*,
    },
    error::AppError,
    infrastructure::{
        db::{Database, image_url_repository},
        export_storage::{self, Workspace},
        media_storage::MediaStorage,
    },
    services::{
        batch_export_service as selection, export_csv, listing_preview_service as listing,
        shopify_export_service, shopify_image_csv as csv,
    },
};
use rusqlite::{Connection, TransactionBehavior};
use sha2::{Digest, Sha256};
use std::{collections::HashSet, io::Write, path::Path};
use uuid::Uuid;

#[derive(Clone, PartialEq)]
pub(crate) struct Snapshot {
    pub listing: listing::Snapshot,
    pub settings: Vec<ImageUrlSetting>,
}

// The caller owns one read/IMMEDIATE transaction covering BOTH reads.
pub(crate) fn read_one_with(
    db: &Connection,
    id: &str,
    store: &str,
    after_listing: impl FnOnce(),
) -> Result<Snapshot, AppError> {
    let listing = listing::read(db, id)?;
    after_listing();
    let settings = image_url_repository::for_product(db, id)?
        .into_iter()
        .filter(|s| s.platform == PLATFORM && s.store_key == store)
        .collect();
    Ok(Snapshot { listing, settings })
}

fn read(database: &Database, ids: &[String], store: &str) -> Result<Vec<Snapshot>, AppError> {
    database.with_mut(|db| {
        let tx = db.transaction()?;
        ids.iter()
            .map(|id| read_one_with(&tx, id, store, || {}))
            .collect()
    })
}

fn field(hash: &mut Sha256, value: &str) {
    hash.update((value.len() as u64).to_le_bytes());
    hash.update(value.as_bytes());
}

pub(crate) fn build(snapshots: &[Snapshot], store: &str) -> Result<ShopifyImagePreview, AppError> {
    let mut hash = Sha256::new();
    field(&mut hash, "ListingPilot-shopify-image-csv-v1\0");
    field(&mut hash, "13-column");
    field(&mut hash, store);
    let mut products = Vec::new();
    let mut has_warnings = false;
    let mut blocked = false;
    let mut image_count = 0;
    for snapshot in snapshots {
        let (mut product, base_token) = shopify_export_service::product_preview(&snapshot.listing)?;
        field(&mut hash, &product.product_id);
        field(&mut hash, &base_token);
        let mut image_issues = Vec::new();
        let mut issues = |code: ImageExportIssueCode, image_id: Option<&str>, position| {
            image_issues.push(ImageExportIssue {
                code,
                image_id: image_id
                    .filter(|id| image_url::valid_uuid(id))
                    .map(str::to_owned),
                position,
                message: code.message(),
            });
        };
        let source_images = &snapshot.listing.images;
        if source_images.is_empty() {
            issues(ImageExportIssueCode::ImagesMissing, None, None);
        }
        if source_images.len() > csv::MAX_PRODUCT_IMAGES {
            issues(ImageExportIssueCode::ImageLimit, None, None);
        }
        let mut images = Vec::new();
        let mut urls = HashSet::new();
        for (index, image) in source_images.iter().enumerate() {
            let position = index + 1;
            for value in [&image.id, &image.sort_order.to_string(), &image.sha256] {
                field(&mut hash, value);
            }
            let setting = snapshot
                .settings
                .iter()
                .find(|s| s.image_id == image.id && s.platform == PLATFORM && s.store_key == store);
            field(&mut hash, if setting.is_some() { "set" } else { "unset" });
            let mut codes = Vec::new();
            if !image_url::valid_uuid(&image.id) || image.product_id != product.product_id {
                codes.push(ImageExportIssueCode::InvalidImage);
            }
            let mut csv_url = None;
            let mut csv_alt = String::new();
            if let Some(setting) = setting {
                for value in [
                    &setting.image_id,
                    &setting.platform,
                    &setting.store_key,
                    &setting.url,
                    &setting.alt_text,
                    &setting.image_sha256,
                    &setting.updated_at,
                ] {
                    field(&mut hash, value);
                }
                match image_url::image_url(&setting.url) {
                    Ok(Some(url)) => {
                        if !urls.insert(url.clone()) {
                            codes.push(ImageExportIssueCode::DuplicateUrl);
                        }
                        csv_url = Some(url);
                    }
                    Ok(None) => codes.push(ImageExportIssueCode::UrlMissing),
                    Err(_) => codes.push(ImageExportIssueCode::InvalidUrl),
                }
                if setting.image_sha256 != image.sha256
                    || image.sha256.len() != 64
                    || !image
                        .sha256
                        .bytes()
                        .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
                {
                    codes.push(ImageExportIssueCode::ShaMismatch);
                }
                if image_url::alt_text(&setting.alt_text).is_err()
                    || export_csv::text(&setting.alt_text, "Image alt text").is_err()
                    || (setting.url.is_empty() && !setting.alt_text.is_empty())
                {
                    codes.push(ImageExportIssueCode::InvalidAlt);
                }
                csv_alt = setting.alt_text.clone();
            } else {
                codes.push(ImageExportIssueCode::UrlMissing);
            }
            for code in &codes {
                issues(*code, Some(&image.id), Some(position));
            }
            images.push(ShopifyImagePreviewRow {
                image_id: if image_url::valid_uuid(&image.id) {
                    image.id.clone()
                } else {
                    String::new()
                },
                position,
                url_configured: setting.is_some_and(|s| !s.url.is_empty()),
                alt_configured: !csv_alt.is_empty(),
                issues: codes,
                csv_url,
                csv_alt,
            });
        }
        if !image_issues.is_empty() {
            product.status = ListingStatus::Blocked;
        }
        has_warnings |= product
            .issues
            .iter()
            .any(|issue| issue.severity == IssueSeverity::Warning);
        blocked |= product.status == ListingStatus::Blocked;
        image_count += images.len();
        products.push(ShopifyImageProductPreview {
            image_count: images.len(),
            url_configured_count: images.iter().filter(|i| i.url_configured).count(),
            main_image_id: images
                .first()
                .filter(|i| !i.image_id.is_empty())
                .map(|i| i.image_id.clone()),
            product,
            images,
            image_issues,
        });
    }
    if !blocked {
        csv::generate(&products)?;
    }
    Ok(ShopifyImagePreview {
        store_key: store.into(),
        products,
        image_count,
        snapshot_token: format!("{:x}", hash.finalize()),
        has_warnings,
        blocked,
    })
}

pub(crate) fn get(
    database: &Database,
    input: &ShopifyImagePreviewInput,
) -> Result<ShopifyImagePreview, AppError> {
    let store = image_url::store_key(&input.store_key)?;
    build(
        &read(database, &selection::normalize(&input.product_ids)?, &store)?,
        &store,
    )
}

fn prepare(
    database: &Database,
    input: &ShopifyImageExportInput,
) -> Result<(Vec<Snapshot>, ShopifyImagePreview, String), AppError> {
    let store = image_url::store_key(&input.store_key)?;
    let snapshots = read(database, &selection::normalize(&input.product_ids)?, &store)?;
    let p = build(&snapshots, &store)?;
    if p.snapshot_token != input.snapshot_token {
        return Err(AppError::StaleData);
    }
    for product in &p.products {
        if product.image_count > csv::MAX_PRODUCT_IMAGES {
            return Err(AppError::ShopifyProductImageLimit {
                product_id: product.product.product_id.clone(),
            });
        }
        if let Some(issue) = product.image_issues.first() {
            return Err(csv::blocked(
                &product.product.product_id,
                issue.image_id.as_deref(),
                issue.position,
                issue.code,
            ));
        }
    }
    if p.blocked {
        return Err(AppError::ShopifyExportBlocked);
    }
    if p.has_warnings && input.warning_token.as_deref() != Some(p.snapshot_token.as_str()) {
        return Err(AppError::ExportWarningRequired);
    }
    let rendered = csv::generate(&p.products)?;
    Ok((snapshots, p, rendered))
}

pub(crate) fn preflight(
    database: &Database,
    input: &ShopifyImageExportInput,
) -> Result<(), AppError> {
    prepare(database, input).map(|_| ())
}

pub(crate) fn export(
    database: &Database,
    media: &MediaStorage,
    input: &ShopifyImageExportInput,
    parent: &Path,
) -> Result<ExportReport, AppError> {
    export_with(database, media, input, parent, |_| {})
}

pub(crate) fn export_with(
    database: &Database,
    media: &MediaStorage,
    input: &ShopifyImageExportInput,
    parent: &Path,
    before_finish: impl FnOnce(&Path),
) -> Result<ExportReport, AppError> {
    let parent = export_storage::destination(parent, media.data_directory()?)?;
    let (snapshots, p, rendered) = prepare(database, input)?;
    let target = parent.join(format!(
        "ListingPilot_ShopifyImages_{}_{}",
        chrono::Local::now().format("%Y%m%d"),
        Uuid::new_v4()
    ));
    let mut workspace = Workspace::new(&parent)?;
    let operation = (|| {
        {
            let mut file = workspace.file("shopify-products.csv")?;
            file.write_all(rendered.as_bytes())
                .and_then(|_| file.sync_all())
                .map_err(|e| export_storage::write_error("shopify_image_csv", &e))?;
        }
        before_finish(&target);
        database.with_mut(|db| {
            let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
            for snapshot in &snapshots {
                if read_one_with(&tx, &snapshot.listing.product.id, &p.store_key, || {})?
                    != *snapshot
                {
                    return Err(AppError::StaleData);
                }
            }
            export_storage::destination(&parent, media.data_directory()?)?;
            workspace.finish(&target)
            // Read-only transaction drops. No fallible COMMIT after publication.
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
        image_count: p.image_count,
        attribute_count: 0,
        has_warnings: p.has_warnings,
    })
}
