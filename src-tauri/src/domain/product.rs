use crate::error::AppError;
use serde::{Deserialize, Serialize};

// JavaScript can represent integer yen exactly throughout this range.
pub const MAX_PRICE_YEN: i64 = 9_007_199_254_740_991;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Product {
    pub id: String,
    pub sku: String,
    pub name: String,
    pub brand: Option<String>,
    pub price_yen: Option<i64>,
    pub description_note: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProductInput {
    pub sku: String,
    pub name: String,
    pub brand: Option<String>,
    pub price_yen: Option<i64>,
    pub description_note: Option<String>,
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
