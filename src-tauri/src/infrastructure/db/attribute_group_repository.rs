use crate::{domain::attribute_group::AttributeGroup, error::AppError};
use rusqlite::{Connection, OptionalExtension, Row, params};

fn from_row(row: &Row<'_>) -> rusqlite::Result<AttributeGroup> {
    Ok(AttributeGroup {
        id: row.get(0)?,
        name: row.get(1)?,
        archived_at: row.get(2)?,
        created_at: row.get(3)?,
        updated_at: row.get(4)?,
    })
}

pub fn list(connection: &Connection) -> Result<Vec<AttributeGroup>, AppError> {
    Ok(connection.prepare("SELECT id,name,archived_at,created_at,updated_at FROM attribute_groups ORDER BY (archived_at IS NOT NULL),created_at,id")?
        .query_map([],from_row)?.collect::<rusqlite::Result<_>>()?)
}

pub fn get(connection: &Connection, id: &str) -> Result<AttributeGroup, AppError> {
    uuid::Uuid::parse_str(id).map_err(|_| AppError::InvalidAttributeGroup)?;
    connection
        .query_row(
            "SELECT id,name,archived_at,created_at,updated_at FROM attribute_groups WHERE id=?1",
            [id],
            from_row,
        )
        .optional()?
        .ok_or(AppError::AttributeGroupNotFound)
}

pub fn create(connection: &Connection, group: &AttributeGroup) -> Result<(), AppError> {
    connection.execute("INSERT INTO attribute_groups(id,name,archived_at,created_at,updated_at) VALUES(?1,?2,NULL,?3,?4)",
        params![group.id,group.name,group.created_at,group.updated_at])?;
    Ok(())
}

pub fn update(
    connection: &Connection,
    group: &AttributeGroup,
    expected: &str,
) -> Result<(), AppError> {
    if connection.execute("UPDATE attribute_groups SET name=?2,archived_at=?3,updated_at=?4 WHERE id=?1 AND updated_at=?5",
        params![group.id,group.name,group.archived_at,group.updated_at,expected])? != 1 {
        get(connection,&group.id)?;
        return Err(AppError::StaleData);
    }
    Ok(())
}

pub fn has_products(connection: &Connection, id: &str) -> Result<bool, AppError> {
    Ok(connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM products WHERE group_id=?1)",
        [id],
        |row| row.get(0),
    )?)
}
