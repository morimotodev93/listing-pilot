use crate::domain::listing_preview::{ListingIssue, ListingStatus};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ShopifyPreviewInput {
    pub product_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ShopifyExportInput {
    pub product_ids: Vec<String>,
    pub snapshot_token: String,
    pub warning_token: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShopifyProductPreview {
    pub product_id: String,
    pub name: String,
    pub sku: String,
    pub title: String,
    pub description: Option<String>,
    pub vendor: Option<String>,
    pub price_yen: Option<i64>,
    pub handle: String,
    pub status: ListingStatus,
    pub issues: Vec<ListingIssue>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShopifyPreview {
    pub snapshot_token: String,
    pub products: Vec<ShopifyProductPreview>,
    pub has_warnings: bool,
    pub blocked: bool,
}
