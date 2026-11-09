#[tauri::command]
pub fn greet() -> String {
    "Listing Pilot の準備ができました。Rustとの接続に成功しました。".to_owned()
}
pub mod image;
pub mod product;

use crate::{error::AppError, infrastructure::db::Database};
use rusqlite::Connection;
use tauri::State;

async fn execute<T: Send + 'static>(
    state: State<'_, Database>,
    operation: impl FnOnce(&Connection) -> Result<T, AppError> + Send + 'static,
) -> Result<T, AppError> {
    let database = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || database.with(operation))
        .await
        .map_err(|_| AppError::DatabaseOperation)?
}
