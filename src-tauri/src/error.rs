use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum AppError {
    RequiredField,
    DuplicateSku,
    InvalidPrice,
    ProductNotFound,
    DatabaseOperation,
    DatabaseInitialization,
    InvalidImage,
    UnsupportedImage,
    ImageTooLarge,
    TooManyImages,
    DuplicateImage,
    InvalidImageOrder,
    ImageNotFound,
    ImageDeletionUnknown,
    StaleData,
    FileRead,
    FileSave,
    UnsafeMediaPath,
    MediaBusy,
    MediaInitialization,
}

impl From<rusqlite::Error> for AppError {
    fn from(error: rusqlite::Error) -> Self {
        if let rusqlite::Error::SqliteFailure(ref code, Some(ref message)) = error
            && code.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE
            && message == "UNIQUE constraint failed: products.sku"
        {
            return Self::DuplicateSku;
        }
        eprintln!("Database operation failed: {error}");
        Self::DatabaseOperation
    }
}
