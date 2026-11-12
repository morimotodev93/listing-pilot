use super::execute;
use crate::{
    domain::product::{Product, ProductInput},
    error::AppError,
    infrastructure::db::Database,
    services::product_service,
};
use tauri::State;

#[tauri::command]
pub async fn list_products(state: State<'_, Database>) -> Result<Vec<Product>, AppError> {
    execute(state, product_service::list).await
}
#[tauri::command]
pub async fn get_product(state: State<'_, Database>, id: String) -> Result<Product, AppError> {
    execute(state, move |connection| {
        product_service::get(connection, &id)
    })
    .await
}
#[tauri::command]
pub async fn create_product(
    state: State<'_, Database>,
    input: ProductInput,
    attribute_values: Option<crate::domain::attribute_value::AttributeValuesInput>,
) -> Result<Product, AppError> {
    execute(state, move |connection| {
        product_service::create_with_values(connection, input, attribute_values)
    })
    .await
}
#[tauri::command]
pub async fn update_product(
    state: State<'_, Database>,
    id: String,
    input: ProductInput,
    attribute_values: Option<crate::domain::attribute_value::AttributeValuesInput>,
    expected_updated_at: String,
) -> Result<Product, AppError> {
    execute(state, move |connection| {
        product_service::update_with_values(
            connection,
            &id,
            input,
            &expected_updated_at,
            attribute_values,
        )
    })
    .await
}

#[tauri::command]
pub async fn get_product_delete_preview(
    database: State<'_, Database>,
    product_id: String,
) -> Result<crate::domain::product::ProductDeletePreview, AppError> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        crate::services::product_deletion_service::preview(&database, &product_id)
    })
    .await
    .map_err(|_| AppError::DatabaseOperation)?
}

#[tauri::command]
pub async fn delete_product(
    database: State<'_, Database>,
    media: State<'_, super::image::Media>,
    input: crate::domain::product::DeleteProductInput,
) -> Result<crate::domain::image::DeleteImageReport, AppError> {
    let database = database.inner().clone();
    let media = media.0.as_ref().map_err(Clone::clone)?.clone();
    tauri::async_runtime::spawn_blocking(move || {
        crate::services::product_deletion_service::delete(&database, &media, &input)
    })
    .await
    .map_err(|_| AppError::ProductDeletionUnknown)?
}

#[tauri::command]
pub async fn get_product_edit_snapshot(
    state: State<'_, Database>,
    id: String,
) -> Result<crate::domain::product::ProductEditSnapshot, AppError> {
    execute(state, move |connection| {
        product_service::edit_snapshot(connection, &id)
    })
    .await
}
