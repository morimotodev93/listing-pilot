use crate::{
    domain::attribute::{AttributeDataType, AttributeDefinition},
    error::AppError,
};
use rusqlite::{Connection, OptionalExtension, Row, params};

const COLUMNS: &str = "id, name, data_type, unit, archived_at, created_at, updated_at";

fn from_row(row: &Row<'_>) -> rusqlite::Result<AttributeDefinition> {
    let data_type: String = row.get(2)?;
    Ok(AttributeDefinition {
        id: row.get(0)?,
        name: row.get(1)?,
        data_type: AttributeDataType::parse(&data_type)
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
        unit: row.get(3)?,
        archived_at: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

pub fn list(connection: &Connection) -> Result<Vec<AttributeDefinition>, AppError> {
    let mut statement = connection.prepare(&format!(
        "SELECT {COLUMNS} FROM attribute_definitions ORDER BY (archived_at IS NOT NULL), created_at, id"
    ))?;
    Ok(statement
        .query_map([], from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn get(connection: &Connection, id: &str) -> Result<AttributeDefinition, AppError> {
    connection
        .query_row(
            &format!("SELECT {COLUMNS} FROM attribute_definitions WHERE id = ?1"),
            [id],
            from_row,
        )
        .optional()?
        .ok_or(AppError::AttributeNotFound)
}

pub fn create(connection: &Connection, attribute: &AttributeDefinition) -> Result<(), AppError> {
    connection.execute(
        "INSERT INTO attribute_definitions (id, name, data_type, unit, archived_at, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?6)",
        params![attribute.id, attribute.name, attribute.data_type.as_str(), attribute.unit, attribute.created_at, attribute.updated_at],
    )?;
    Ok(())
}

pub fn rename(
    connection: &Connection,
    attribute: &AttributeDefinition,
    expected: &str,
) -> Result<(), AppError> {
    let changed = connection.execute(
        "UPDATE attribute_definitions SET name = ?2, updated_at = ?3 WHERE id = ?1 AND updated_at = ?4",
        params![attribute.id, attribute.name, attribute.updated_at, expected],
    )?;
    check_changed(connection, &attribute.id, changed)
}

pub fn set_archived(
    connection: &Connection,
    attribute: &AttributeDefinition,
    expected: &str,
) -> Result<(), AppError> {
    let changed = connection.execute(
        "UPDATE attribute_definitions SET archived_at = ?2, updated_at = ?3 WHERE id = ?1 AND updated_at = ?4",
        params![attribute.id, attribute.archived_at, attribute.updated_at, expected],
    )?;
    check_changed(connection, &attribute.id, changed)
}

fn check_changed(connection: &Connection, id: &str, changed: usize) -> Result<(), AppError> {
    if changed == 0 {
        get(connection, id)?;
        return Err(AppError::StaleData);
    }
    Ok(())
}
