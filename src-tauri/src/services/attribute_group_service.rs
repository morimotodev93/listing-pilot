use crate::{
    domain::{attribute::validate_label, attribute_group::*},
    error::AppError,
    infrastructure::db::attribute_group_repository as repository,
};
use chrono::{SecondsFormat, Utc};
use rusqlite::{Connection, Transaction, TransactionBehavior};
use uuid::Uuid;

pub fn list(connection: &Connection) -> Result<Vec<AttributeGroup>, AppError> {
    repository::list(connection)
}

pub fn require_active(connection: &Connection, id: &str) -> Result<AttributeGroup, AppError> {
    let group = repository::get(connection, id)?;
    if group.archived_at.is_some() {
        return Err(AppError::AttributeGroupArchived);
    }
    Ok(group)
}

fn name(value: &str) -> Result<String, AppError> {
    validate_label(value, 80).map_err(|_| AppError::InvalidAttributeGroup)
}

pub fn create(
    connection: &Connection,
    input: CreateAttributeGroupInput,
) -> Result<AttributeGroup, AppError> {
    let now = Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true);
    let group = AttributeGroup {
        id: Uuid::new_v4().to_string(),
        name: name(&input.name)?,
        archived_at: None,
        created_at: now.clone(),
        updated_at: now,
    };
    repository::create(connection, &group)?;
    Ok(group)
}

pub fn update(
    connection: &Connection,
    input: UpdateAttributeGroupInput,
) -> Result<AttributeGroup, AppError> {
    let transaction = Transaction::new_unchecked(connection, TransactionBehavior::Immediate)?;
    let mut group = repository::get(&transaction, &input.id)?;
    if group.updated_at != input.expected_updated_at {
        return Err(AppError::StaleData);
    }
    group.name = name(&input.name)?;
    group.updated_at = super::attribute_service::next_timestamp(&group.updated_at, Utc::now())?;
    repository::update(&transaction, &group, &input.expected_updated_at)?;
    transaction.commit()?;
    Ok(group)
}

pub fn set_archived(
    connection: &Connection,
    input: SetAttributeGroupArchivedInput,
) -> Result<AttributeGroup, AppError> {
    let transaction = Transaction::new_unchecked(connection, TransactionBehavior::Immediate)?;
    let mut group = repository::get(&transaction, &input.id)?;
    if group.updated_at != input.expected_updated_at {
        return Err(AppError::StaleData);
    }
    if input.archived {
        if group.id == DEFAULT_GROUP_ID {
            return Err(AppError::DefaultAttributeGroup);
        }
        if repository::has_products(&transaction, &group.id)? {
            return Err(AppError::AttributeGroupInUse);
        }
    }
    group.updated_at = super::attribute_service::next_timestamp(&group.updated_at, Utc::now())?;
    group.archived_at = input.archived.then(|| group.updated_at.clone());
    repository::update(&transaction, &group, &input.expected_updated_at)?;
    transaction.commit()?;
    Ok(group)
}
