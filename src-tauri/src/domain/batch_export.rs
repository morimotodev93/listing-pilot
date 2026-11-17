use crate::domain::listing_preview::ListingPreview;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BatchPreviewInput {
    pub product_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BatchExportInput {
    pub product_ids: Vec<String>,
    pub snapshot_token: String,
    pub warning_token: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchProductPreview {
    pub name: String,
    pub group_id: String,
    pub group_name: String,
    pub preview: ListingPreview,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchPreview {
    pub snapshot_token: String,
    pub products: Vec<BatchProductPreview>,
    pub image_count: usize,
    pub has_warnings: bool,
    pub blocked: bool,
}
