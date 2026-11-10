use crate::{domain::product::Product, error::AppError};
use rusqlite::{Connection, OptionalExtension, Row, params};

fn from_row(row: &Row<'_>) -> rusqlite::Result<Product> {
    Ok(Product {
        id: row.get(0)?,
        sku: row.get(1)?,
        name: row.get(2)?,
        brand: row.get(3)?,
        price_yen: row.get(4)?,
        description_note: row.get(5)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

const COLUMNS: &str = "id, sku, name, brand, price_yen, description_note, created_at, updated_at";

pub fn delete(connection: &Connection, id: &str) -> Result<(), AppError> {
    if connection.execute("DELETE FROM products WHERE id = ?1", [id])? != 1 {
        return Err(AppError::StaleData);
    }
    Ok(())
}

pub fn list(connection: &Connection) -> Result<Vec<Product>, AppError> {
    let mut statement = connection.prepare(&format!(
        "SELECT {COLUMNS} FROM products ORDER BY updated_at DESC, id ASC"
    ))?;
    Ok(statement
        .query_map([], from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn get(connection: &Connection, id: &str) -> Result<Product, AppError> {
    connection
        .query_row(
            &format!("SELECT {COLUMNS} FROM products WHERE id = ?1"),
            [id],
            from_row,
        )
        .optional()?
        .ok_or(AppError::ProductNotFound)
}

pub fn create(connection: &Connection, product: &Product) -> Result<(), AppError> {
    connection.execute(
        "INSERT INTO products (id, sku, name, brand, price_yen, description_note, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![product.id, product.sku, product.name, product.brand, product.price_yen, product.description_note, product.created_at, product.updated_at],
    )?;
    Ok(())
}

pub fn update(connection: &Connection, product: &Product) -> Result<(), AppError> {
    let changed = connection.execute(
        "UPDATE products SET sku = ?2, name = ?3, brand = ?4, price_yen = ?5, description_note = ?6, updated_at = ?7 WHERE id = ?1",
        params![product.id, product.sku, product.name, product.brand, product.price_yen, product.description_note, product.updated_at],
    )?;
    if changed == 0 {
        return Err(AppError::ProductNotFound);
    }
    Ok(())
}
