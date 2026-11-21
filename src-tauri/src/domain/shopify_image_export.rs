use crate::domain::shopify_export::ShopifyProductPreview;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ShopifyImagePreviewInput {
    pub product_ids: Vec<String>,
    pub store_key: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ShopifyImageExportInput {
    pub product_ids: Vec<String>,
    pub store_key: String,
    pub snapshot_token: String,
    pub warning_token: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageExportIssueCode {
    ImagesMissing,
    ImageLimit,
    InvalidImage,
    UrlMissing,
    InvalidUrl,
    DuplicateUrl,
    ShaMismatch,
    InvalidAlt,
}

impl ImageExportIssueCode {
    pub fn message(self) -> &'static str {
        match self {
            Self::ImagesMissing => "画像付きCSVには商品ごとに1枚以上の管理画像が必要です。",
            Self::ImageLimit => "画像付きCSVは1商品250画像までです。",
            Self::InvalidImage => "画像UUIDまたは商品への所属が不正です。",
            Self::UrlMissing => "選択ストア向けの画像URLが未設定です。",
            Self::InvalidUrl => "保存済み画像URLの形式または長さが不正です。",
            Self::DuplicateUrl => "同じ商品・ストアの画像URLが重複しています。",
            Self::ShaMismatch => {
                "設定時と現在の画像SHA情報が一致しません。画像とURLの対応を再確認してください。"
            }
            Self::InvalidAlt => {
                "altの文字数・制御文字・数式リスクまたはURLとの組み合わせが不正です。"
            }
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageExportIssue {
    pub code: ImageExportIssueCode,
    pub image_id: Option<String>,
    pub position: Option<usize>,
    pub message: &'static str,
}

// No Debug and no serialized URL/alt payloads: the preview reveals state, not cell contents.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShopifyImagePreviewRow {
    pub image_id: String,
    pub position: usize,
    pub url_configured: bool,
    pub alt_configured: bool,
    pub issues: Vec<ImageExportIssueCode>,
    #[serde(skip)]
    pub(crate) csv_url: Option<String>,
    #[serde(skip)]
    pub(crate) csv_alt: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShopifyImageProductPreview {
    #[serde(flatten)]
    pub product: ShopifyProductPreview,
    pub image_count: usize,
    pub url_configured_count: usize,
    pub main_image_id: Option<String>,
    pub images: Vec<ShopifyImagePreviewRow>,
    pub image_issues: Vec<ImageExportIssue>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShopifyImagePreview {
    pub store_key: String,
    pub products: Vec<ShopifyImageProductPreview>,
    pub image_count: usize,
    pub snapshot_token: String,
    pub has_warnings: bool,
    pub blocked: bool,
}
