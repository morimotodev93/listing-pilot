use crate::{
    commands::image::Media,
    domain::product_export::{ExportInput, ExportReport},
    error::AppError,
    infrastructure::db::Database,
    services::product_export_service,
};
use tauri::{State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;

#[tauri::command]
pub async fn get_shopify_export_preview(
    database: State<'_, Database>,
    media: State<'_, Media>,
    input: crate::domain::shopify_export::ShopifyPreviewInput,
) -> Result<crate::domain::shopify_export::ShopifyPreview, AppError> {
    let database = database.inner().clone();
    let media = media.0.as_ref().map_err(Clone::clone)?.clone();
    let permit = media.begin_import()?;
    tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        crate::services::shopify_export_service::get(&database, &input.product_ids)
    })
    .await
    .map_err(|_| AppError::ExportUnknown)?
}

#[tauri::command]
pub async fn export_shopify_products(
    window: WebviewWindow,
    database: State<'_, Database>,
    media: State<'_, Media>,
    input: crate::domain::shopify_export::ShopifyExportInput,
) -> Result<ExportReport, AppError> {
    let database = database.inner().clone();
    let media = media.0.as_ref().map_err(Clone::clone)?.clone();
    let permit = media.begin_import()?;
    tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        crate::services::shopify_export_service::preflight(&database, &input)?;
        let Some(selected) = window
            .dialog()
            .file()
            .set_parent(&window)
            .set_title("Shopify用CSVの出力先の親フォルダーを選択")
            .blocking_pick_folder()
        else {
            return Ok(ExportReport::Cancelled);
        };
        let parent = selected
            .into_path()
            .map_err(|_| AppError::ExportDestination)?;
        crate::services::shopify_export_service::export(&database, &media, &input, &parent)
    })
    .await
    .map_err(|_| AppError::ExportUnknown)?
}

#[tauri::command]
pub async fn get_export_batch_preview(
    database: State<'_, Database>,
    media: State<'_, Media>,
    input: crate::domain::batch_export::BatchPreviewInput,
) -> Result<crate::domain::batch_export::BatchPreview, AppError> {
    let database = database.inner().clone();
    let media = media.0.as_ref().map_err(Clone::clone)?.clone();
    let permit = media.begin_import()?;
    tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        crate::services::batch_export_service::get(&database, &media, &input.product_ids)
    })
    .await
    .map_err(|_| AppError::ExportUnknown)?
}

#[tauri::command]
pub async fn export_products(
    window: WebviewWindow,
    database: State<'_, Database>,
    media: State<'_, Media>,
    input: crate::domain::batch_export::BatchExportInput,
) -> Result<ExportReport, AppError> {
    let database = database.inner().clone();
    let media = media.0.as_ref().map_err(Clone::clone)?.clone();
    let permit = media.begin_import()?;
    tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        crate::services::batch_export_service::preflight(&database, &media, &input)?;
        let Some(selected) = window
            .dialog()
            .file()
            .set_parent(&window)
            .set_title("エクスポート先の親フォルダーを選択")
            .blocking_pick_folder()
        else {
            return Ok(ExportReport::Cancelled);
        };
        let parent = selected
            .into_path()
            .map_err(|_| AppError::ExportDestination)?;
        crate::services::batch_export_service::export(&database, &media, &input, &parent)
    })
    .await
    .map_err(|_| AppError::ExportUnknown)?
}

#[tauri::command]
pub async fn export_product(
    window: WebviewWindow,
    database: State<'_, Database>,
    media: State<'_, Media>,
    input: ExportInput,
) -> Result<ExportReport, AppError> {
    let database = database.inner().clone();
    let media = media.0.as_ref().map_err(Clone::clone)?.clone();
    let permit = media.begin_import()?;
    tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        // Validate before opening the dialog, and again after the selection.
        product_export_service::preflight(&database, &media, &input)?;
        let selected = window
            .dialog()
            .file()
            .set_parent(&window)
            .set_title("エクスポート先の親フォルダーを選択")
            .blocking_pick_folder();
        let Some(selected) = selected else {
            return Ok(ExportReport::Cancelled);
        };
        let parent = selected
            .into_path()
            .map_err(|_| AppError::ExportDestination)?;
        product_export_service::export(&database, &media, &input, &parent)
    })
    .await
    .map_err(|_| AppError::ExportUnknown)?
}
