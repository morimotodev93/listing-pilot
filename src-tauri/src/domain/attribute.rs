use crate::error::AppError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AttributeDataType {
    Text,
    Number,
}

impl AttributeDataType {
    pub fn parse(value: &str) -> Result<Self, AppError> {
        match value {
            "text" => Ok(Self::Text),
            "number" => Ok(Self::Number),
            _ => Err(AppError::InvalidAttributeDefinition),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Number => "number",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttributeDefinition {
    pub id: String,
    pub name: String,
    pub data_type: AttributeDataType,
    pub unit: Option<String>,
    pub archived_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateAttributeInput {
    pub name: String,
    pub data_type: String,
    pub unit: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateAttributeInput {
    pub id: String,
    pub name: String,
    pub expected_updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetAttributeArchivedInput {
    pub id: String,
    pub archived: bool,
    pub expected_updated_at: String,
}

// Reject controls before trimming so a leading/trailing newline is not hidden.
pub fn validate_label(value: &str, max: usize) -> Result<String, AppError> {
    if value
        .chars()
        .any(|c| c.is_control() || matches!(c, '\u{2028}' | '\u{2029}'))
    {
        return Err(AppError::InvalidAttributeDefinition);
    }
    let value = value.trim();
    if value.is_empty() || value.chars().count() > max {
        return Err(AppError::InvalidAttributeDefinition);
    }
    Ok(value.to_owned())
}

impl CreateAttributeInput {
    pub fn validate(self) -> Result<(String, AttributeDataType, Option<String>), AppError> {
        let name = validate_label(&self.name, 80)?;
        let data_type = AttributeDataType::parse(&self.data_type)?;
        if data_type == AttributeDataType::Text && self.unit.is_some() {
            return Err(AppError::InvalidAttributeDefinition);
        }
        let unit = self
            .unit
            .map(|unit| {
                // Empty units become NULL, but control characters remain invalid.
                if !unit
                    .chars()
                    .any(|c| c.is_control() || matches!(c, '\u{2028}' | '\u{2029}'))
                    && unit.trim().is_empty()
                {
                    Ok(None)
                } else {
                    validate_label(&unit, 20).map(Some)
                }
            })
            .transpose()?
            .flatten();
        Ok((name, data_type, unit))
    }
}
