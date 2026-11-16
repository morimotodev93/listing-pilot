use serde::{Deserialize, Serialize};

pub const DEFAULT_GROUP_ID: &str = "7a8f0c3e-4d5b-4c6a-9e20-1234567890ab";

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttributeGroup {
    pub id: String,
    pub name: String,
    pub archived_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateAttributeGroupInput {
    pub name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateAttributeGroupInput {
    pub id: String,
    pub name: String,
    pub expected_updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetAttributeGroupArchivedInput {
    pub id: String,
    pub archived: bool,
    pub expected_updated_at: String,
}
