use crate::{
    domain::product::{Product, ProductInput},
    error::AppError,
    infrastructure::db::product_repository as repository,
};
use chrono::{SecondsFormat, Utc};
use rusqlite::Connection;
use uuid::Uuid;

pub fn list(connection: &Connection) -> Result<Vec<Product>, AppError> {
    repository::list(connection)
}
pub fn get(connection: &Connection, id: &str) -> Result<Product, AppError> {
    repository::get(connection, id)
}

pub fn create(connection: &Connection, input: ProductInput) -> Result<Product, AppError> {
    let input = input.validate()?;
    let now = Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true);
    let product = Product {
        id: Uuid::new_v4().to_string(),
        sku: input.sku,
        name: input.name,
        brand: input.brand,
        price_yen: input.price_yen,
        description_note: input.description_note,
        created_at: now.clone(),
        updated_at: now,
    };
    repository::create(connection, &product)?;
    Ok(product)
}

pub fn update(
    connection: &Connection,
    id: &str,
    input: ProductInput,
    expected_updated_at: &str,
) -> Result<Product, AppError> {
    let input = input.validate()?;
    let previous = repository::get(connection, id)?;
    // Advance the token even if the system clock repeats or moves backwards.
    let previous_time = chrono::DateTime::parse_from_rfc3339(&previous.updated_at)
        .map_err(|_| AppError::DatabaseOperation)?
        .with_timezone(&Utc);
    let next = previous_time
        .checked_add_signed(chrono::Duration::nanoseconds(1))
        .ok_or(AppError::DatabaseOperation)?;
    let product = Product {
        id: previous.id,
        sku: input.sku,
        name: input.name,
        brand: input.brand,
        price_yen: input.price_yen,
        description_note: input.description_note,
        created_at: previous.created_at,
        updated_at: Utc::now()
            .max(next)
            .to_rfc3339_opts(SecondsFormat::Nanos, true),
    };
    repository::update(connection, &product, expected_updated_at)?;
    Ok(product)
}
