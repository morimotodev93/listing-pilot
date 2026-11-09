use crate::{
    domain::product::{Product, ProductInput},
    error::AppError,
    services::product_service,
};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use tauri::State;

pub struct Database(pub Arc<Mutex<Result<Connection, AppError>>>);

async fn execute<T: Send + 'static>(
    state: State<'_, Database>,
    operation: impl FnOnce(&Connection) -> Result<T, AppError> + Send + 'static,
) -> Result<T, AppError> {
    let database = Arc::clone(&state.0);
    tauri::async_runtime::spawn_blocking(move || {
        let guard = database.lock().map_err(|_| AppError::DatabaseOperation)?;
        let connection = guard.as_ref().map_err(Clone::clone)?;
        operation(connection)
    })
    .await
    .map_err(|_| AppError::DatabaseOperation)?
}

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
