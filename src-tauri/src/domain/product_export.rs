use crate::error::AppError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportInput {
    pub product_id: String,
    pub snapshot_token: String,
    pub warning_token: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ExportReport {
    Cancelled,
    Success {
        folder: String,
        product_count: usize,
        attribute_count: usize,
        image_count: usize,
        has_warnings: bool,
    },
    Failed {
        error: AppError,
        // Only the selected export location; never a managed media/database path.
        retained_folder: String,
        cleanup_failed: bool,
    },
}
