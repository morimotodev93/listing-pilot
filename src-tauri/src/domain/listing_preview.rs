use serde::Serialize;

#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TitleSource {
    ListingTitle,
    ProductName,
}

#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ListingStatus {
    Ready,
    Warning,
    Blocked,
}

#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueSeverity {
    Warning,
    Error,
}

#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueCode {
    TitleFallback,
    DescriptionMissing,
    PriceMissing,
    ImagesMissing,
    ImageUnavailable,
}

#[derive(Debug, Serialize)]
pub struct ListingIssue {
    pub code: IssueCode,
    pub severity: IssueSeverity,
    pub message: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListingPreviewImage {
    pub image_id: String,
    pub sort_order: i64,
    pub preview_path: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListingPreviewAttribute {
    pub attribute_id: String,
    pub name: String,
    pub value: String,
    pub unit: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListingPreview {
    pub product_id: String,
    pub sku: String,
    pub title: String,
    pub title_source: TitleSource,
    pub description: Option<String>,
    pub price_yen: Option<i64>,
    pub attributes: Vec<ListingPreviewAttribute>,
    pub images: Vec<ListingPreviewImage>,
    pub status: ListingStatus,
    pub issues: Vec<ListingIssue>,
}
