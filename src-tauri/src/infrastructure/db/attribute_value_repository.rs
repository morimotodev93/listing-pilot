use crate::{domain::attribute_value::ProductAttributeValue, error::AppError};
use rusqlite::{Connection, params};

pub fn list(
    connection: &Connection,
    product_id: &str,
) -> Result<Vec<ProductAttributeValue>, AppError> {
    let mut statement = connection.prepare("SELECT attribute_id, value FROM product_attribute_values WHERE product_id = ?1 ORDER BY attribute_id")?;
    Ok(statement
        .query_map([product_id], |row| {
            Ok(ProductAttributeValue {
                attribute_id: row.get(0)?,
                value: row.get(1)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?)
}

pub fn set(
    connection: &Connection,
    product_id: &str,
    attribute_id: &str,
    value: Option<&str>,
) -> Result<(), AppError> {
    if let Some(value) = value {
        connection.execute("INSERT INTO product_attribute_values (product_id, attribute_id, value) VALUES (?1, ?2, ?3) ON CONFLICT(product_id, attribute_id) DO UPDATE SET value = excluded.value", params![product_id, attribute_id, value])?;
    } else {
        connection.execute(
            "DELETE FROM product_attribute_values WHERE product_id = ?1 AND attribute_id = ?2",
            params![product_id, attribute_id],
        )?;
    }
    Ok(())
}

pub fn delete_product(
    connection: &Connection,
    product_id: &str,
    expected_count: usize,
) -> Result<(), AppError> {
    if connection.execute(
        "DELETE FROM product_attribute_values WHERE product_id = ?1",
        [product_id],
    )? != expected_count
    {
        return Err(AppError::StaleData);
    }
    Ok(())
}
