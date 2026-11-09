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
}

impl From<rusqlite::Error> for AppError {
    fn from(error: rusqlite::Error) -> Self {
        if let rusqlite::Error::SqliteFailure(ref code, _) = error
            && code.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE
        {
            return Self::DuplicateSku;
        }
        eprintln!("Database operation failed: {error}");
        Self::DatabaseOperation
    }
}
