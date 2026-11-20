use crate::{
    domain::image_url::{GetImageUrlsInput, ImageUrlSnapshot, SaveImageUrlsInput},
    error::AppError,
    infrastructure::db::Database,
    services::image_url_service,
};
use tauri::State;

#[tauri::command]
pub async fn get_shopify_image_urls(
    database: State<'_, Database>,
    input: GetImageUrlsInput,
) -> Result<ImageUrlSnapshot, AppError> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || image_url_service::get(&database, &input))
        .await
        .map_err(|_| AppError::DatabaseOperation)?
}

#[tauri::command]
pub async fn save_shopify_image_urls(
    database: State<'_, Database>,
    media: State<'_, super::image::Media>,
    input: SaveImageUrlsInput,
) -> Result<ImageUrlSnapshot, AppError> {
    let database = database.inner().clone();
    let media = media.0.as_ref().map_err(Clone::clone)?.clone();
    tauri::async_runtime::spawn_blocking(move || image_url_service::save(&database, &media, &input))
        .await
        .map_err(|_| AppError::ImageUrlSaveUnknown)?
}
