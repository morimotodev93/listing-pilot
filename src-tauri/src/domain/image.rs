use crate::error::AppError;
use serde::Serialize;

pub const MAX_FILE_BYTES: u64 = 20 * 1024 * 1024;
pub const MAX_PIXELS: u64 = 40_000_000;
pub const MAX_IMPORT_FILES: usize = 20;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ProductImage {
    pub id: String,
    pub product_id: String,
    pub original_filename: String,
    pub relative_path: String,
    pub mime_type: String,
    pub file_size_bytes: i64,
    pub width: u32,
    pub height: u32,
    pub sha256: String,
    pub sort_order: i64,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct ImagePreview {
    #[serde(flatten)]
    pub image: ProductImage,
    // None means missing, unsafe or unreadable; never expose unvalidated DB paths.
    pub preview_path: Option<String>,
}

#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportStatus {
    Imported,
    Duplicate,
    Failed,
}

#[derive(Debug, Serialize)]
pub struct ImportItem {
    pub filename: String,
    pub status: ImportStatus,
    pub error: Option<AppError>,
}

#[derive(Debug, Serialize)]
pub struct ImportReport {
    pub cancelled: bool,
    pub items: Vec<ImportItem>,
}

impl ImportReport {
    pub fn cancelled() -> Self {
        Self {
            cancelled: true,
            items: vec![],
        }
    }
}
