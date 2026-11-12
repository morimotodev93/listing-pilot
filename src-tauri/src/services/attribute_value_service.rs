use crate::{
    domain::attribute_value::{AttributeValuesInput, normalize},
    error::AppError,
    infrastructure::db::{
        attribute_repository as definitions, attribute_value_repository as values,
    },
};
use rusqlite::Connection;
use std::collections::HashSet;

// Caller owns the IMMEDIATE product transaction. Validate every target before writing.
pub fn save(
    connection: &Connection,
    product_id: &str,
    input: &AttributeValuesInput,
) -> Result<(), AppError> {
    let mut ids = HashSet::new();
    let mut supplied = HashSet::new();
    for value in &input.values {
        if !supplied.insert(&value.attribute_id) {
            return Err(AppError::InvalidAttributeValue);
        }
    }
    let mut changes = Vec::new();
    for target in &input.editable_definitions {
        if !ids.insert(&target.attribute_id) {
            return Err(AppError::InvalidAttributeValue);
        }
        let definition = definitions::get(connection, &target.attribute_id)?;
        if definition.updated_at != target.expected_updated_at || definition.archived_at.is_some() {
            return Err(AppError::StaleData);
        }
        let value = input
            .values
            .iter()
            .find(|v| v.attribute_id == target.attribute_id)
            .map_or("", |v| v.value.as_str());
        changes.push((
            &target.attribute_id,
            normalize(value, definition.data_type)?,
        ));
    }
    if supplied.iter().any(|id| !ids.contains(id)) {
        return Err(AppError::InvalidAttributeValue);
    }
    for (id, value) in changes {
        values::set(connection, product_id, id, value.as_deref())?;
    }
    Ok(())
}
