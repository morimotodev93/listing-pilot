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
) -> Result<Product, AppError> {
    execute(state, move |connection| {
        product_service::create(connection, input)
    })
    .await
}
#[tauri::command]
pub async fn update_product(
    state: State<'_, Database>,
    id: String,
    input: ProductInput,
) -> Result<Product, AppError> {
    execute(state, move |connection| {
        product_service::update(connection, &id, input)
    })
    .await
}
