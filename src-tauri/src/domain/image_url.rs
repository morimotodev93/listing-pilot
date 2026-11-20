use crate::error::AppError;
use serde::{Deserialize, Serialize};

pub const PLATFORM: &str = "shopify";
pub const MAX_URL_BYTES: usize = 4096;

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetImageUrlsInput {
    pub product_id: String,
    pub store_key: String,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImageUrlEdit {
    pub image_id: String,
    pub url: String,
    #[serde(default)]
    pub alt_text: String,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveImageUrlsInput {
    pub product_id: String,
    pub store_key: String,
    pub snapshot_token: String,
    pub images: Vec<ImageUrlEdit>,
}

// Deliberately no Debug: URL queries and alt text must not enter diagnostic logs.
#[derive(Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageUrlSetting {
    pub image_id: String,
    pub platform: String,
    pub store_key: String,
    pub url: String,
    pub image_sha256: String,
    pub alt_text: String,
    pub updated_at: String,
}

#[derive(PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageUrlRow {
    pub image_id: String,
    pub sort_order: i64,
    pub sha256: String,
    pub setting: Option<ImageUrlSetting>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageUrlSnapshot {
    pub product_id: String,
    pub platform: &'static str,
    pub store_key: String,
    pub snapshot_token: String,
    pub images: Vec<ImageUrlRow>,
}

pub fn valid_uuid(value: &str) -> bool {
    uuid::Uuid::parse_str(value).is_ok_and(|id| id.to_string() == value)
}

pub fn store_key(value: &str) -> Result<String, AppError> {
    if value.chars().any(char::is_control) {
        return Err(AppError::InvalidShopifyStore);
    }
    let key = value.trim().to_ascii_lowercase();
    let label = key
        .strip_suffix(".myshopify.com")
        .ok_or(AppError::InvalidShopifyStore)?;
    if !(1..=63).contains(&label.len())
        || !label
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-')
        || !label.as_bytes()[0].is_ascii_alphanumeric()
        || !label.as_bytes()[label.len() - 1].is_ascii_alphanumeric()
    {
        return Err(AppError::InvalidShopifyStore);
    }
    Ok(key)
}

pub fn alt_text(value: &str) -> Result<(), AppError> {
    if value.chars().count() > 512
        || value
            .chars()
            .any(|c| c.is_control() || matches!(c, '\u{2028}' | '\u{2029}'))
    {
        return Err(AppError::InvalidImageAltText);
    }
    Ok(())
}

pub fn image_url(value: &str) -> Result<Option<String>, AppError> {
    if value.is_empty() {
        return Ok(None);
    }
    if value.len() > MAX_URL_BYTES
        || value.chars().any(|c| c.is_control() || c.is_whitespace())
        || value.contains(['\\', '#'])
    {
        return Err(AppError::InvalidShopifyImageUrl);
    }
    // Validate the raw authority before WHATWG parsing can repair slashes,
    // decode host escapes, strip an empty userinfo, or normalize a trailing dot.
    let (scheme, rest) = value
        .split_once("://")
        .ok_or(AppError::InvalidShopifyImageUrl)?;
    let end = rest.find(['/', '?']).unwrap_or(rest.len());
    let authority = &rest[..end];
    if !scheme.eq_ignore_ascii_case("https")
        || !(authority.eq_ignore_ascii_case("cdn.shopify.com")
            || authority.eq_ignore_ascii_case("cdn.shopify.com:443"))
    {
        return Err(AppError::InvalidShopifyImageUrl);
    }
    // Reject malformed escapes, encoded C0/DEL and UTF-8 encoded C1 controls.
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = bytes
                .get(i + 1..i + 3)
                .ok_or(AppError::InvalidShopifyImageUrl)?;
            let high = (hex[0] as char)
                .to_digit(16)
                .ok_or(AppError::InvalidShopifyImageUrl)?;
            let low = (hex[1] as char)
                .to_digit(16)
                .ok_or(AppError::InvalidShopifyImageUrl)?;
            decoded.push((high * 16 + low) as u8);
            i += 3;
        } else {
            decoded.push(bytes[i]);
            i += 1;
        }
    }
    if decoded.iter().any(|c| c.is_ascii_control())
        || String::from_utf8_lossy(&decoded)
            .chars()
            .any(char::is_control)
    {
        return Err(AppError::InvalidShopifyImageUrl);
    }
    // Tauri already re-exports url::Url; no new parser dependency is needed.
    let parsed = tauri::Url::parse(value).map_err(|_| AppError::InvalidShopifyImageUrl)?;
    if parsed.scheme() != "https"
        || parsed.host_str() != Some("cdn.shopify.com")
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.port_or_known_default() != Some(443)
        || parsed.fragment().is_some()
        || parsed.as_str().len() > MAX_URL_BYTES
    {
        return Err(AppError::InvalidShopifyImageUrl);
    }
    Ok(Some(parsed.into()))
}
