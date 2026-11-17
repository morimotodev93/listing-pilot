use crate::{
    domain::{
        listing_preview::ListingStatus,
        product_export::{ExportInput, ExportReport},
    },
    error::AppError,
    infrastructure::{
        db::Database,
        export_storage::{self, Workspace},
        media_storage::MediaStorage,
    },
    services::{export_csv, listing_preview_service as preview},
};
use std::{io::Write, path::Path};

pub(crate) fn preflight(
    database: &Database,
    media: &MediaStorage,
    input: &ExportInput,
) -> Result<(), AppError> {
    let snapshot = preview::snapshot(database, &input.product_id)?;
    validate(&preview::build(snapshot, media), input)
}

pub(crate) fn validate(
    p: &crate::domain::listing_preview::ListingPreview,
    input: &ExportInput,
) -> Result<(), AppError> {
    if p.status == ListingStatus::Blocked {
        return Err(AppError::ExportBlocked);
    }
    if p.product_id != input.product_id || p.snapshot_token != input.snapshot_token {
        return Err(AppError::StaleData);
    }
    if p.status == ListingStatus::Warning
        && input.warning_token.as_deref() != Some(p.snapshot_token.as_str())
    {
        return Err(AppError::ExportWarningRequired);
    }
    Ok(())
}

// Caller owns the media permit, including the native dialog lifetime.
pub(crate) fn export(
    database: &Database,
    media: &MediaStorage,
    input: &ExportInput,
    parent: &Path,
) -> Result<ExportReport, AppError> {
    export_with(database, media, input, parent, || {})
}

pub(crate) fn export_with(
    database: &Database,
    media: &MediaStorage,
    input: &ExportInput,
    parent: &Path,
    before_finish: impl FnOnce(),
) -> Result<ExportReport, AppError> {
    let parent = export_storage::destination(parent, media.data_directory()?)?;
    let snapshot = preview::snapshot(database, &input.product_id)?;
    let p = preview::build(snapshot.clone(), media);
    validate(&p, input)?;
    let prepared = prepare(&snapshot, &p, media)?;
    // UUID avoids all user-controlled Windows filename hazards. Stable per day
    // makes repeated exports an explicit collision, never an overwrite.
    let target = parent.join(format!(
        "ListingPilot_{}_{}",
        p.product_id,
        chrono::Local::now().format("%Y%m%d")
    ));
    let mut workspace = Workspace::new(&parent)?;
    let operation = (|| {
        workspace.images()?;
        write_product(&mut workspace, None, &snapshot, &prepared, media)?;
        before_finish();
        // A short IMMEDIATE transaction prevents other SQLite connections from
        // writing even in WAL mode. No image I/O occurs under this mutex.
        database.with_mut(|db| {
            let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            if preview::read(&tx, &input.product_id)? != snapshot {
                return Err(AppError::StaleData);
            }
            export_storage::destination(&parent, media.data_directory()?)?;
            workspace.finish(&target)
            // Read-only transaction rolls back on drop. No fallible COMMIT after publication.
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
        product_count: 1,
        attribute_count: p.attributes.len(),
        image_count: prepared.files.len(),
        has_warnings: p.status == ListingStatus::Warning,
    })
}

// Shared by single and batch output: identical CSV generation and original validation.
pub(crate) struct PreparedProduct {
    pub files: Vec<String>,
    pub csv: [String; 3],
}

pub(crate) fn prepare(
    snapshot: &preview::Snapshot,
    p: &crate::domain::listing_preview::ListingPreview,
    media: &MediaStorage,
) -> Result<PreparedProduct, AppError> {
    export_csv::uuid(&p.product_id)?;
    let files = snapshot
        .images
        .iter()
        .enumerate()
        .map(|(i, image)| {
            let path = media.image_path(image)?;
            let ext = path
                .extension()
                .and_then(|v| v.to_str())
                .ok_or(AppError::ExportImageInvalid)?;
            Ok(format!("images/{:04}-{}.{ext}", i + 1, image.id))
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    let csv = export_csv::generate(p, snapshot, &files)?;
    Ok(PreparedProduct { files, csv })
}

pub(crate) fn write_product(
    workspace: &mut Workspace,
    product_id: Option<&str>,
    snapshot: &preview::Snapshot,
    prepared: &PreparedProduct,
    media: &MediaStorage,
) -> Result<(), AppError> {
    let relative = |name: &str| match product_id {
        Some(id) => format!("{id}/{name}"),
        None => name.to_owned(),
    };
    for (name, content) in ["product.csv", "attributes.csv", "images.csv"]
        .iter()
        .zip(&prepared.csv)
    {
        let mut file = workspace.file(&relative(name))?;
        file.write_all(content.as_bytes())
            .map_err(|e| export_storage::write_error("csv_write", &e))?;
        file.sync_all()
            .map_err(|e| export_storage::write_error("csv_sync", &e))?;
    }
    for (image, name) in snapshot.images.iter().zip(&prepared.files) {
        media.copy_original(image, &mut workspace.file(&relative(name))?)?;
    }
    Ok(())
}
