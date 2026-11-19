use crate::{
    domain::{batch_export::*, listing_preview::ListingStatus, product_export::ExportReport},
    error::AppError,
    infrastructure::{
        db::Database,
        export_storage::{self, Workspace},
        media_storage::MediaStorage,
    },
    services::{listing_preview_service as preview, product_export_service as single},
};
use sha2::{Digest, Sha256};
use std::path::Path;
use uuid::Uuid;

pub(crate) const MAX_BATCH_PRODUCTS: usize = 20;
pub(crate) const MAX_BATCH_IMAGES: usize = 200;
pub(crate) const MAX_BATCH_BYTES: u64 = 1024 * 1024 * 1024;

pub(crate) fn normalize(ids: &[String]) -> Result<Vec<String>, AppError> {
    if ids.is_empty() {
        return Err(AppError::InvalidExportSelection);
    }
    if ids.len() > MAX_BATCH_PRODUCTS {
        return Err(AppError::ExportProductLimit);
    }
    let mut ids = ids.to_vec();
    for id in &ids {
        if !Uuid::parse_str(id).is_ok_and(|uuid| uuid.to_string() == *id) {
            return Err(AppError::InvalidExportSelection);
        }
    }
    ids.sort();
    if ids.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(AppError::InvalidExportSelection);
    }
    Ok(ids)
}

pub(crate) fn read(
    database: &Database,
    ids: &[String],
) -> Result<Vec<preview::Snapshot>, AppError> {
    database.with_mut(|db| {
        let tx = db.transaction()?;
        let snapshots = ids
            .iter()
            .map(|id| preview::read(&tx, id))
            .collect::<Result<Vec<_>, _>>()?;
        tx.commit()?;
        Ok(snapshots)
    })
}

pub(crate) fn limits(snapshots: &[preview::Snapshot]) -> Result<usize, AppError> {
    let mut images = 0usize;
    let mut bytes = 0u64;
    for snapshot in snapshots {
        images = images
            .checked_add(snapshot.images.len())
            .ok_or(AppError::ExportImageLimit)?;
        if images > MAX_BATCH_IMAGES {
            return Err(AppError::ExportImageLimit);
        }
        for image in &snapshot.images {
            let size = u64::try_from(image.file_size_bytes)
                .ok()
                .filter(|size| *size > 0)
                .ok_or(AppError::ExportImageInvalid)?;
            bytes = bytes.checked_add(size).ok_or(AppError::ExportSizeLimit)?;
            if bytes > MAX_BATCH_BYTES {
                return Err(AppError::ExportSizeLimit);
            }
        }
    }
    Ok(images)
}

fn build(snapshots: &[preview::Snapshot], media: &MediaStorage) -> Result<BatchPreview, AppError> {
    let image_count = limits(snapshots)?;
    let mut hash = Sha256::new();
    hash.update(b"ListingPilot-batch-v1\0");
    let mut has_warnings = false;
    let mut blocked = false;
    let products = snapshots
        .iter()
        .map(|snapshot| {
            let p = preview::build(snapshot.clone(), media);
            // Both fields have fixed lengths (canonical UUID and SHA-256 hex).
            hash.update(p.product_id.as_bytes());
            hash.update(p.snapshot_token.as_bytes());
            has_warnings |= p.status == ListingStatus::Warning;
            blocked |= p.status == ListingStatus::Blocked;
            BatchProductPreview {
                name: snapshot.product.name.clone(),
                group_id: snapshot.group.id.clone(),
                group_name: snapshot.group.name.clone(),
                preview: p,
            }
        })
        .collect();
    Ok(BatchPreview {
        snapshot_token: format!("{:x}", hash.finalize()),
        products,
        image_count,
        has_warnings,
        blocked,
    })
}

// All callers own the media permit before entering DB or filesystem operations.
pub(crate) fn get(
    database: &Database,
    media: &MediaStorage,
    ids: &[String],
) -> Result<BatchPreview, AppError> {
    build(&read(database, &normalize(ids)?)?, media)
}

fn validate(p: &BatchPreview, input: &BatchExportInput) -> Result<(), AppError> {
    if p.blocked {
        return Err(AppError::ExportBlocked);
    }
    if p.snapshot_token != input.snapshot_token {
        return Err(AppError::StaleData);
    }
    if p.has_warnings && input.warning_token.as_deref() != Some(p.snapshot_token.as_str()) {
        return Err(AppError::ExportWarningRequired);
    }
    Ok(())
}

struct PreparedBatch {
    snapshots: Vec<preview::Snapshot>,
    preview: BatchPreview,
    products: Vec<single::PreparedProduct>,
}

fn prepare(
    database: &Database,
    media: &MediaStorage,
    input: &BatchExportInput,
) -> Result<PreparedBatch, AppError> {
    let snapshots = read(database, &normalize(&input.product_ids)?)?;
    let p = build(&snapshots, media)?;
    validate(&p, input)?;
    let prepared = snapshots
        .iter()
        .zip(&p.products)
        .map(|(snapshot, product)| {
            single::prepare(snapshot, &product.preview, media).map_err(|error| match error {
                AppError::CsvFormulaRisk { column } => AppError::BatchCsvFormulaRisk {
                    product_id: snapshot.product.id.clone(),
                    column,
                },
                other => other,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(PreparedBatch {
        snapshots,
        preview: p,
        products: prepared,
    })
}

pub(crate) fn preflight(
    database: &Database,
    media: &MediaStorage,
    input: &BatchExportInput,
) -> Result<(), AppError> {
    prepare(database, media, input).map(|_| ())
}

pub(crate) fn export(
    database: &Database,
    media: &MediaStorage,
    input: &BatchExportInput,
    parent: &Path,
) -> Result<ExportReport, AppError> {
    export_with(database, media, input, parent, || {})
}

pub(crate) fn export_with(
    database: &Database,
    media: &MediaStorage,
    input: &BatchExportInput,
    parent: &Path,
    before_finish: impl FnOnce(),
) -> Result<ExportReport, AppError> {
    let parent = export_storage::destination(parent, media.data_directory()?)?;
    let PreparedBatch {
        snapshots,
        preview: p,
        products: prepared,
    } = prepare(database, media, input)?;
    let target = parent.join(format!(
        "ListingPilot_Batch_{}_{}",
        chrono::Local::now().format("%Y%m%d"),
        Uuid::new_v4()
    ));
    let mut workspace = Workspace::new(&parent)?;
    let operation = (|| {
        for (snapshot, prepared) in snapshots.iter().zip(&prepared) {
            workspace.product(&snapshot.product.id)?;
            single::write_product(
                &mut workspace,
                Some(&snapshot.product.id),
                snapshot,
                prepared,
                media,
            )?;
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
            // Read-only transaction rolls back on drop; no fallible COMMIT after publication.
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
        attribute_count: p.products.iter().map(|p| p.preview.attributes.len()).sum(),
        image_count: p.image_count,
        has_warnings: p.has_warnings,
    })
}
