use crate::error::AppError;
use serde::{Deserialize, Serialize};

// JavaScript can represent integer yen exactly throughout this range.
pub const MAX_PRICE_YEN: i64 = 9_007_199_254_740_991;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Product {
    pub id: String,
    pub group_id: String,
    pub sku: String,
    pub name: String,
    pub brand: Option<String>,
    pub price_yen: Option<i64>,
    pub description_note: Option<String>,
    pub listing_title: Option<String>,
    pub listing_description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProductInput {
    pub group_id: Option<String>,
    pub sku: String,
    pub name: String,
    pub brand: Option<String>,
    pub price_yen: Option<i64>,
    pub description_note: Option<String>,
    #[serde(default)]
    pub listing_title: ListingTextInput,
    #[serde(default)]
    pub listing_description: ListingTextInput,
}

// A missing field preserves existing content; explicit null clears it.
#[derive(Debug, Clone, Default, PartialEq)]
pub enum ListingTextInput {
    #[default]
    Omitted,
    Provided(Option<String>),
}

impl<'de> Deserialize<'de> for ListingTextInput {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Option::<String>::deserialize(deserializer).map(Self::Provided)
    }
}

impl ListingTextInput {
    pub fn resolve(self, previous: Option<String>) -> Option<String> {
        match self {
            Self::Omitted => previous,
            Self::Provided(value) => value,
        }
    }

    fn validate(self, multiline: bool) -> Result<Self, AppError> {
        let Self::Provided(Some(value)) = self else {
            return Ok(self);
        };
        let value = if multiline {
            value.replace("\r\n", "\n")
        } else {
            value
        };
        // Inspect before trimming so controls at either end are also rejected.
        // Descriptions allow LF and tab verbatim; lone CR is invalid.
        if value.chars().any(|c| {
            (c.is_control() && !(multiline && matches!(c, '\n' | '\t')))
                || (!multiline && matches!(c, '\u{2028}' | '\u{2029}'))
        }) || value.trim().chars().count() > if multiline { 10_000 } else { 200 }
        {
            return Err(if multiline {
                AppError::InvalidListingDescription
            } else {
                AppError::InvalidListingTitle
            });
        }
        Ok(Self::Provided(nonempty(value)))
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ProductDeletePreview {
    pub product_id: String,
    pub name: String,
    pub sku: String,
    pub updated_at: String,
    pub image_count: usize,
    pub image_ids: Vec<String>,
    pub product: Product,
    pub attribute_values: Vec<super::attribute_value::ProductAttributeValue>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteProductInput {
    pub product_id: String,
    pub expected_updated_at: String,
    pub expected_image_ids: Vec<String>,
    pub expected_product: Product,
    pub expected_attribute_values: Vec<super::attribute_value::ProductAttributeValue>,
}

impl ProductInput {
    pub fn validate(mut self) -> Result<Self, AppError> {
        self.sku = self.sku.trim().to_owned();
        self.name = self.name.trim().to_owned();
        if self.sku.is_empty() || self.name.is_empty() {
            return Err(AppError::RequiredField);
        }
        if self
            .price_yen
            .is_some_and(|price| !(0..=MAX_PRICE_YEN).contains(&price))
        {
            return Err(AppError::InvalidPrice);
        }
        self.brand = self.brand.and_then(nonempty);
        self.description_note = self.description_note.and_then(nonempty);
        self.listing_title = self.listing_title.validate(false)?;
        self.listing_description = self.listing_description.validate(true)?;
        Ok(self)
    }
}

fn nonempty(value: String) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductEditSnapshot {
    pub product: Product,
    pub definitions: Vec<super::attribute::AttributeDefinition>,
    pub values: Vec<super::attribute_value::ProductAttributeValue>,
}
