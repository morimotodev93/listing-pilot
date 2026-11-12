use crate::{domain::attribute::AttributeDataType, error::AppError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProductAttributeValue {
    pub attribute_id: String,
    pub value: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EditableDefinition {
    pub attribute_id: String,
    pub expected_updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AttributeValuesInput {
    pub editable_definitions: Vec<EditableDefinition>,
    pub values: Vec<ProductAttributeValue>,
}

pub fn normalize(value: &str, data_type: AttributeDataType) -> Result<Option<String>, AppError> {
    // Check before trimming: a newline at either edge must not be hidden.
    if value
        .chars()
        .any(|c| c.is_control() || matches!(c, '\u{2028}' | '\u{2029}'))
    {
        return Err(AppError::InvalidAttributeValue);
    }
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    let valid = match data_type {
        AttributeDataType::Text => value.chars().count() <= 2_000,
        AttributeDataType::Number => {
            let unsigned = value.strip_prefix('-').unwrap_or(value);
            let (integer, fraction) = unsigned
                .split_once('.')
                .map_or((unsigned, None), |(i, f)| (i, Some(f)));
            (1..=15).contains(&integer.len())
                && integer.bytes().all(|c| c.is_ascii_digit())
                && fraction.is_none_or(|f| {
                    (1..=6).contains(&f.len()) && f.bytes().all(|c| c.is_ascii_digit())
                })
        }
    };
    if !valid {
        return Err(AppError::InvalidAttributeValue);
    }
    Ok(Some(value.to_owned()))
}
