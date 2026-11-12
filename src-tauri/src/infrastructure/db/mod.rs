pub mod connection;
pub mod image_repository;
pub mod migrations;
pub mod product_repository;

use crate::error::AppError;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Database(pub Arc<Mutex<Result<Connection, AppError>>>);

impl Database {
    pub fn with_mut<T>(
        &self,
        operation: impl FnOnce(&mut Connection) -> Result<T, AppError>,
    ) -> Result<T, AppError> {
        let mut guard = self.0.lock().map_err(|_| AppError::DatabaseOperation)?;
        operation(guard.as_mut().map_err(|error| error.clone())?)
    }

    pub fn with<T>(
        &self,
        operation: impl FnOnce(&Connection) -> Result<T, AppError>,
    ) -> Result<T, AppError> {
        let guard = self.0.lock().map_err(|_| AppError::DatabaseOperation)?;
        operation(guard.as_ref().map_err(Clone::clone)?)
    }
}
pub mod attribute_repository;
