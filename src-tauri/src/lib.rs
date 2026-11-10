mod commands;
mod domain;
mod error;
mod infrastructure;
mod services;

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
            let database = directory
                .as_ref()
                .map_err(Clone::clone)
                .and_then(|directory| {
                    infrastructure::db::connection::open(&directory.join("listing-pilot.sqlite3"))
                });
            // Never touch media when opening/migrating the DB failed. Reconciliation
            // finishes before Commands can import, under the process-wide media lock.
            let media = directory
                .as_ref()
                .map_err(Clone::clone)
                .and_then(|directory| {
                    services::image_service::initialize(&database, &directory.join("media"))
                })
                .map(Arc::new);
            if let Err(error) = &media {
                eprintln!("Media initialization failed: {error:?}");
            }
            app.manage(commands::image::Media(media));
            app.manage(infrastructure::db::Database(Arc::new(Mutex::new(database))));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::greet,
            commands::product::list_products,
            commands::product::get_product,
            commands::product::create_product,
            commands::product::update_product,
            commands::image::list_product_images,
            commands::image::import_product_images,
            commands::image::delete_product_image,
            commands::image::reorder_product_images
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
