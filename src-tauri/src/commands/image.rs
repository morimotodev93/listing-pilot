use crate::{
    domain::image::{DeleteImageReport, ImagePreview, ImportReport},
    error::AppError,
    infrastructure::{
        db::{Database, product_repository},
        media_storage::MediaStorage,
    },
    services::image_service,
};
use std::sync::Arc;
use tauri::{State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;

pub struct Media(pub Result<Arc<MediaStorage>, AppError>);

#[tauri::command]
pub async fn delete_product_image(
    database: State<'_, Database>,
    media: State<'_, Media>,
    product_id: String,
    image_id: String,
) -> Result<DeleteImageReport, AppError> {
    let database = database.inner().clone();
    let media = media.0.as_ref().map_err(Clone::clone)?.clone();
    tauri::async_runtime::spawn_blocking(move || {
        image_service::delete(&database, &media, &product_id, &image_id)
    })
    .await
    .map_err(|_| AppError::ImageDeletionUnknown)?
}

#[tauri::command]
pub async fn reorder_product_images(
    database: State<'_, Database>,
    media: State<'_, Media>,
    product_id: String,
    expected_image_ids: Vec<String>,
    ordered_image_ids: Vec<String>,
) -> Result<Vec<ImagePreview>, AppError> {
    let database = database.inner().clone();
    let media = media.0.as_ref().map_err(Clone::clone)?.clone();
    tauri::async_runtime::spawn_blocking(move || {
        image_service::reorder(
            &database,
            &media,
            &product_id,
            &expected_image_ids,
            &ordered_image_ids,
        )
    })
    .await
    .map_err(|_| AppError::DatabaseOperation)?
}

#[tauri::command]
pub async fn list_product_images(
    database: State<'_, Database>,
    media: State<'_, Media>,
    product_id: String,
) -> Result<Vec<ImagePreview>, AppError> {
    let database = database.inner().clone();
    let media = media.0.as_ref().map_err(Clone::clone)?.clone();
    tauri::async_runtime::spawn_blocking(move || {
        image_service::list(&database, &media, &product_id)
    })
    .await
    .map_err(|_| AppError::DatabaseOperation)?
}

#[tauri::command]
pub async fn import_product_images(
    window: WebviewWindow,
    database: State<'_, Database>,
    media: State<'_, Media>,
    product_id: String,
) -> Result<ImportReport, AppError> {
    let database = database.inner().clone();
    let media = media.0.as_ref().map_err(Clone::clone)?.clone();
    let permit = media.begin_import()?;
    tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        database.with(|connection| product_repository::get(connection, &product_id))?;
        let files = window
            .dialog()
            .file()
            .set_parent(&window)
            .set_title("商品画像を選択（1回20枚まで）")
            .add_filter("画像（JPEG / PNG / WebP）", &["jpg", "jpeg", "png", "webp"])
            .blocking_pick_files();
        let Some(files) = files else {
            return Ok(ImportReport::cancelled());
        };
        let paths = files
            .into_iter()
            .map(|file| file.into_path().map_err(|_| AppError::FileRead))
            .collect::<Result<Vec<_>, _>>()?;
        image_service::import_selected(&database, &media, &product_id, &paths)
    })
    .await
    .map_err(|_| AppError::FileRead)?
}
