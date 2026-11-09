mod commands;
mod domain;
mod error;
mod infrastructure;
mod services;

#[cfg(test)]
mod tests;

use std::sync::{Arc, Mutex};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let database = app
                .path()
                .app_local_data_dir()
                .map_err(|_| error::AppError::DatabaseInitialization)
                .and_then(|directory| {
                    infrastructure::db::connection::open(&directory.join("listing-pilot.sqlite3"))
                });
            app.manage(commands::product::Database(Arc::new(Mutex::new(database))));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::greet,
            commands::product::list_products,
            commands::product::get_product,
            commands::product::create_product,
            commands::product::update_product
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
