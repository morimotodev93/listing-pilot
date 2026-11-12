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

#[cfg(test)]
pub fn create(connection: &Connection, input: ProductInput) -> Result<Product, AppError> {
    create_with_values(connection, input, None)
}

pub fn create_with_values(
    connection: &Connection,
    input: ProductInput,
    attribute_values: Option<crate::domain::attribute_value::AttributeValuesInput>,
) -> Result<Product, AppError> {
    let transaction =
        rusqlite::Transaction::new_unchecked(connection, rusqlite::TransactionBehavior::Immediate)?;
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
    repository::create(&transaction, &product)?;
    if let Some(values) = attribute_values {
        super::attribute_value_service::save(&transaction, &product.id, &values)?;
    }
    transaction.commit()?;
    Ok(product)
}

#[cfg(test)]
pub fn update(
    connection: &Connection,
    id: &str,
    input: ProductInput,
    expected_updated_at: &str,
) -> Result<Product, AppError> {
    update_with_values(connection, id, input, expected_updated_at, None)
}

pub fn update_with_values(
    connection: &Connection,
    id: &str,
    input: ProductInput,
    expected_updated_at: &str,
    attribute_values: Option<crate::domain::attribute_value::AttributeValuesInput>,
) -> Result<Product, AppError> {
    let transaction =
        rusqlite::Transaction::new_unchecked(connection, rusqlite::TransactionBehavior::Immediate)?;
    let input = input.validate()?;
    let previous = repository::get(&transaction, id)?;
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
    repository::update(&transaction, &product, expected_updated_at)?;
    if let Some(values) = attribute_values {
        super::attribute_value_service::save(&transaction, &product.id, &values)?;
    }
    transaction.commit()?;
    Ok(product)
}

pub fn edit_snapshot(
    connection: &Connection,
    id: &str,
) -> Result<crate::domain::product::ProductEditSnapshot, AppError> {
    let transaction = connection.unchecked_transaction()?;
    let snapshot = crate::domain::product::ProductEditSnapshot {
        product: repository::get(&transaction, id)?,
        definitions: crate::infrastructure::db::attribute_repository::list(&transaction)?,
        values: crate::infrastructure::db::attribute_value_repository::list(&transaction, id)?,
    };
    transaction.commit()?;
    Ok(snapshot)
}
