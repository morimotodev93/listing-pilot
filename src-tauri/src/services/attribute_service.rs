use crate::{
    domain::attribute::{
        AttributeDefinition, CreateAttributeInput, SetAttributeArchivedInput, UpdateAttributeInput,
        validate_label,
    },
    error::AppError,
    infrastructure::db::attribute_repository as repository,
};
use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::Connection;
use uuid::Uuid;

pub fn list(connection: &Connection) -> Result<Vec<AttributeDefinition>, AppError> {
    repository::list(connection)
}

pub fn create(
    connection: &Connection,
    input: CreateAttributeInput,
) -> Result<AttributeDefinition, AppError> {
    let transaction =
        rusqlite::Transaction::new_unchecked(connection, rusqlite::TransactionBehavior::Immediate)?;
    let group_id = input
        .group_id
        .clone()
        .unwrap_or_else(|| crate::domain::attribute_group::DEFAULT_GROUP_ID.to_owned());
    super::attribute_group_service::require_active(&transaction, &group_id)?;
    let (name, data_type, unit) = input.validate()?;
    let now = Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true);
    let attribute = AttributeDefinition {
        id: Uuid::new_v4().to_string(),
        group_id,
        name,
        data_type,
        unit,
        archived_at: None,
        created_at: now.clone(),
        updated_at: now,
    };
    repository::create(&transaction, &attribute)?;
    transaction.commit()?;
    Ok(attribute)
}

pub(crate) fn next_timestamp(previous: &str, now: DateTime<Utc>) -> Result<String, AppError> {
    let previous = DateTime::parse_from_rfc3339(previous)
        .map_err(|_| AppError::DatabaseOperation)?
        .with_timezone(&Utc);
    let next = previous
        .checked_add_signed(chrono::Duration::nanoseconds(1))
        .ok_or(AppError::DatabaseOperation)?;
    Ok(now.max(next).to_rfc3339_opts(SecondsFormat::Nanos, true))
}

pub fn update(
    connection: &Connection,
    input: UpdateAttributeInput,
) -> Result<AttributeDefinition, AppError> {
    let name = validate_label(&input.name, 80)?;
    let transaction =
        rusqlite::Transaction::new_unchecked(connection, rusqlite::TransactionBehavior::Immediate)?;
    let mut attribute = repository::get(&transaction, &input.id)?;
    super::attribute_group_service::require_active(&transaction, &attribute.group_id)?;
    if attribute.updated_at != input.expected_updated_at {
        return Err(AppError::StaleData);
    }
    attribute.name = name;
    attribute.updated_at = next_timestamp(&attribute.updated_at, Utc::now())?;
    repository::rename(&transaction, &attribute, &input.expected_updated_at)?;
    transaction.commit()?;
    Ok(attribute)
}

pub fn set_archived(
    connection: &Connection,
    input: SetAttributeArchivedInput,
) -> Result<AttributeDefinition, AppError> {
    let transaction =
        rusqlite::Transaction::new_unchecked(connection, rusqlite::TransactionBehavior::Immediate)?;
    let mut attribute = repository::get(&transaction, &input.id)?;
    super::attribute_group_service::require_active(&transaction, &attribute.group_id)?;
    if attribute.updated_at != input.expected_updated_at {
        return Err(AppError::StaleData);
    }
    attribute.updated_at = next_timestamp(&attribute.updated_at, Utc::now())?;
    attribute.archived_at = input.archived.then(|| attribute.updated_at.clone());
    repository::set_archived(&transaction, &attribute, &input.expected_updated_at)?;
    transaction.commit()?;
    Ok(attribute)
}
