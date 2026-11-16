use crate::{
    commands::image::Media, domain::listing_preview::ListingPreview, error::AppError,
    infrastructure::db::Database, services::listing_preview_service,
};
use tauri::State;

#[tauri::command]
pub async fn get_listing_preview(
    database: State<'_, Database>,
    media: State<'_, Media>,
    product_id: String,
) -> Result<ListingPreview, AppError> {
    let database = database.inner().clone();
    let media = media.0.as_ref().map_err(Clone::clone)?.clone();
    tauri::async_runtime::spawn_blocking(move || {
        listing_preview_service::get(&database, &media, &product_id)
    })
    .await
    .map_err(|_| AppError::DatabaseOperation)?
}
