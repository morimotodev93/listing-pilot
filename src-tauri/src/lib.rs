mod commands;
mod domain;
mod error;
mod infrastructure;
mod services;

#[cfg(test)]
mod attribute_tests;
#[cfg(test)]
mod image_tests;
#[cfg(test)]
mod tests;

use std::sync::{Arc, Mutex};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let directory = app
                .path()
                .app_local_data_dir()
                .map_err(|_| error::AppError::DatabaseInitialization);
            let dataset = directory
                .as_ref()
                .map_err(Clone::clone)
                .and_then(|directory| services::startup_service::initialize(directory));
            let (database, media) = match dataset {
                Ok((database, media)) => (Ok(database), Ok(Arc::new(media))),
                Err(error) => {
                    eprintln!("Dataset initialization failed: {error:?}");
                    (Err(error.clone()), Err(error))
                }
            };
            app.manage(commands::image::Media(media));
            app.manage(infrastructure::db::Database(Arc::new(Mutex::new(database))));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::greet,
            commands::attribute::list_attribute_definitions,
            commands::attribute::create_attribute_definition,
            commands::attribute::update_attribute_definition,
            commands::attribute::set_attribute_archived,
            commands::product::list_products,
            commands::product::get_product,
            commands::product::create_product,
            commands::product::update_product,
            commands::product::get_product_delete_preview,
            commands::product::delete_product,
            commands::image::list_product_images,
            commands::image::import_product_images,
            commands::image::delete_product_image,
            commands::image::reorder_product_images
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
