use crate::{
    error::AppError,
    infrastructure::{
        db::{connection, image_repository},
        media_storage::MediaStorage,
    },
};
use rusqlite::{Connection, OpenFlags};
use std::path::Path;

// Publish neither resource unless the complete dataset is ready. In particular,
// do not open a writable DB (including migrations) before acquiring its media lock.
pub fn initialize(directory: &Path) -> Result<(Connection, MediaStorage), AppError> {
    let path = directory.join("listing-pilot.sqlite3");
    match std::fs::symlink_metadata(&path) {
        Ok(_) => {
            let probe = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY)
                .map_err(|_| AppError::DatabaseInitialization)?;
            // Check DB access without requiring the newest schema or changing it.
            probe
                .query_row("SELECT count(*) FROM sqlite_schema", [], |row| {
                    row.get::<_, i64>(0)
                })
                .map_err(|_| AppError::DatabaseInitialization)?;
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
        Err(_) => return Err(AppError::DatabaseInitialization),
    }
    let media = MediaStorage::open(&directory.join("media"))?;
    let database = connection::open(&path)?;
    // The reference set is read only AFTER acquiring the OS lock and migrating.
    let images = image_repository::all(&database)?;
    let recovered = media.reconcile(&images)?;
    if recovered > 0 {
        eprintln!("Quarantined {recovered} incomplete or orphaned media files");
    }
    Ok((database, media))
}
