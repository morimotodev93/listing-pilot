use crate::{
    domain::image::{
        ImagePreview, ImportItem, ImportReport, ImportStatus, MAX_IMPORT_FILES, ProductImage,
    },
    error::AppError,
    infrastructure::{
        db::{Database, image_repository as repository, product_repository},
        media_storage::{MediaStorage, filename},
    },
};
use chrono::{SecondsFormat, Utc};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};
use uuid::Uuid;

pub fn initialize(
    database: &Result<rusqlite::Connection, AppError>,
    root: &Path,
) -> Result<MediaStorage, AppError> {
    let connection = database.as_ref().map_err(Clone::clone)?;
    // Preflight only: an unavailable DB must not create or reorganize media.
    // Discard this snapshot because another process may still be importing.
    repository::all(connection)?;
    initialize_after_preflight(connection, root)
}

pub(crate) fn initialize_after_preflight(
    connection: &rusqlite::Connection,
    root: &Path,
) -> Result<MediaStorage, AppError> {
    let media = MediaStorage::open(root)?;
    // Re-query while holding the OS lock. No other app process can import now.
    // A failed query returns without reconciliation; never treat it as empty.
    let images = repository::all(connection)?;
    let count = media.reconcile(&images)?;
    if count > 0 {
        eprintln!("Quarantined {count} incomplete or orphaned media files");
    }
    Ok(media)
}

pub fn list(
    database: &Database,
    media: &MediaStorage,
    product_id: &str,
) -> Result<Vec<ImagePreview>, AppError> {
    let images = database.with(|connection| {
        product_repository::get(connection, product_id)?;
        repository::list(connection, product_id)
    })?;
    Ok(images
        .into_iter()
        .map(|image| ImagePreview {
            preview_path: media.preview(&image),
            image,
        })
        .collect())
}

pub fn reorder(
    database: &Database,
    media: &MediaStorage,
    product_id: &str,
    expected_ids: &[String],
    ordered_ids: &[String],
) -> Result<Vec<ImagePreview>, AppError> {
    // Shared with import (including its dialog) and reconciliation; never wait
    // for a media operation while holding the database mutex.
    let _permit = media.begin_import()?;
    let images = database.with_mut(|connection| {
        let transaction =
            connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        product_repository::get(&transaction, product_id)?;
        let current = repository::list(&transaction, product_id)?;
        let expected_set: HashSet<_> = expected_ids.iter().collect();
        let ordered_set: HashSet<_> = ordered_ids.iter().collect();
        if expected_set.len() != expected_ids.len() || ordered_set.len() != ordered_ids.len() {
            return Err(AppError::InvalidImageOrder);
        }
        if !current
            .iter()
            .map(|image| &image.id)
            .eq(expected_ids.iter())
        {
            return Err(AppError::StaleData);
        }
        if ordered_set != expected_set {
            return Err(AppError::InvalidImageOrder);
        }
        repository::update_order(&transaction, product_id, ordered_ids)?;
        let saved = repository::list(&transaction, product_id)?;
        transaction.commit()?;
        Ok(saved)
    })?;
    // Preview validation performs filesystem access after releasing the DB mutex.
    Ok(images
        .into_iter()
        .map(|image| ImagePreview {
            preview_path: media.preview(&image),
            image,
        })
        .collect())
}

#[cfg(test)]
pub fn import_files(
    database: &Database,
    media: &MediaStorage,
    product_id: &str,
    files: &[PathBuf],
) -> Result<ImportReport, AppError> {
    let _permit = media.begin_import()?;
    import_selected(database, media, product_id, files)
}

// Caller keeps an ImportPermit from before opening the dialog until this finishes.
pub(crate) fn import_selected(
    database: &Database,
    media: &MediaStorage,
    product_id: &str,
    files: &[PathBuf],
) -> Result<ImportReport, AppError> {
    database.with(|connection| product_repository::get(connection, product_id))?;
    if files.len() > MAX_IMPORT_FILES {
        return Err(AppError::TooManyImages);
    }
    let items = files
        .iter()
        .map(|path| {
            let result = import_one(database, media, product_id, path);
            let (status, error) = match result {
                Ok(()) => (ImportStatus::Imported, None),
                Err(AppError::DuplicateImage) => {
                    (ImportStatus::Duplicate, Some(AppError::DuplicateImage))
                }
                Err(error) => (ImportStatus::Failed, Some(error)),
            };
            ImportItem {
                filename: filename(path),
                status,
                error,
            }
        })
        .collect();
    Ok(ImportReport {
        cancelled: false,
        items,
    })
}

fn import_one(
    database: &Database,
    media: &MediaStorage,
    product_id: &str,
    source: &Path,
) -> Result<(), AppError> {
    let staged = media.stage(source)?;
    if database.with(|connection| repository::has_hash(connection, product_id, &staged.sha256))? {
        return Err(AppError::DuplicateImage);
    }
    let id = Uuid::new_v4().to_string();
    let image = ProductImage {
        relative_path: MediaStorage::relative_path(product_id, &id, staged.extension)?,
        id,
        product_id: product_id.to_owned(),
        original_filename: filename(source),
        mime_type: staged.mime_type.into(),
        file_size_bytes: staged.size as i64,
        width: staged.width,
        height: staged.height,
        sha256: staged.sha256.clone(),
        sort_order: 0,
        created_at: Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true),
    };
    media.place(&staged, &image)?;
    let registration = database.with(|connection| repository::create(connection, &image));
    if let Err(error) = registration {
        // If an autocommit result is ambiguous, query actual references first.
        // Unknown DB state leaves the file for recovery, never risks a committed image.
        match database
            .with(|connection| repository::references_path(connection, &image.relative_path))
        {
            Ok(false) => {
                if media.remove_unregistered(&image).is_err() {
                    eprintln!("Unregistered media retained for startup recovery");
                }
            }
            Ok(true) => return Ok(()),
            Err(_) => eprintln!("Media commit state unknown; retained for startup recovery"),
        }
        return Err(error);
    }
    Ok(())
}
