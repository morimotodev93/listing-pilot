use crate::{domain::image::ProductImage, error::AppError};
use rusqlite::{Connection, Row, params};

fn from_row(row: &Row<'_>) -> rusqlite::Result<ProductImage> {
    Ok(ProductImage {
        id: row.get(0)?,
        product_id: row.get(1)?,
        original_filename: row.get(2)?,
        relative_path: row.get(3)?,
        mime_type: row.get(4)?,
        file_size_bytes: row.get(5)?,
        width: row.get(6)?,
        height: row.get(7)?,
        sha256: row.get(8)?,
        sort_order: row.get(9)?,
        created_at: row.get(10)?,
    })
}
const COLUMNS: &str = "id, product_id, original_filename, relative_path, mime_type, file_size_bytes, width, height, sha256, sort_order, created_at";

pub fn list(connection: &Connection, product_id: &str) -> Result<Vec<ProductImage>, AppError> {
    let mut statement = connection.prepare(&format!("SELECT {COLUMNS} FROM product_images WHERE product_id = ?1 ORDER BY sort_order, created_at, id"))?;
    Ok(statement
        .query_map([product_id], from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn all(connection: &Connection) -> Result<Vec<ProductImage>, AppError> {
    let mut statement = connection.prepare(&format!("SELECT {COLUMNS} FROM product_images"))?;
    Ok(statement
        .query_map([], from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?)
}

// Caller validates the complete image set and holds an IMMEDIATE transaction.
pub fn update_order(
    connection: &Connection,
    product_id: &str,
    ordered_ids: &[String],
) -> Result<(), AppError> {
    let mut statement = connection
        .prepare("UPDATE product_images SET sort_order = ?1 WHERE product_id = ?2 AND id = ?3")?;
    for (position, id) in ordered_ids.iter().enumerate() {
        let position = i64::try_from(position).map_err(|_| AppError::InvalidImageOrder)?;
        if statement.execute(params![position, product_id, id])? != 1 {
            return Err(AppError::InvalidImageOrder);
        }
    }
    Ok(())
}

pub fn has_hash(connection: &Connection, product_id: &str, sha256: &str) -> Result<bool, AppError> {
    Ok(connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM product_images WHERE product_id = ?1 AND sha256 = ?2)",
        params![product_id, sha256],
        |row| row.get(0),
    )?)
}

pub fn references_path(connection: &Connection, path: &str) -> Result<bool, AppError> {
    Ok(connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM product_images WHERE relative_path = ?1)",
        [path],
        |row| row.get(0),
    )?)
}

pub fn create(connection: &Connection, image: &ProductImage) -> Result<(), AppError> {
    // One autocommit statement: ordering allocation and insertion are atomic.
    let result = connection.execute(
        "INSERT INTO product_images (id, product_id, original_filename, relative_path, mime_type, file_size_bytes, width, height, sha256, sort_order, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, (SELECT COALESCE(MAX(sort_order) + 1, 0) FROM product_images WHERE product_id = ?2), ?10)",
        params![image.id, image.product_id, image.original_filename, image.relative_path, image.mime_type, image.file_size_bytes, image.width, image.height, image.sha256, image.created_at],
    );
    match result {
        Ok(_) => Ok(()),
        Err(rusqlite::Error::SqliteFailure(code, Some(message)))
            if code.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE
                && message
                    == "UNIQUE constraint failed: product_images.product_id, product_images.sha256" =>
        {
            Err(AppError::DuplicateImage)
        }
        Err(error) => Err(error.into()),
    }
}

pub fn get(
    connection: &Connection,
    product_id: &str,
    image_id: &str,
) -> Result<ProductImage, AppError> {
    use rusqlite::OptionalExtension;
    connection
        .query_row(
            &format!("SELECT {COLUMNS} FROM product_images WHERE product_id = ?1 AND id = ?2"),
            params![product_id, image_id],
            from_row,
        )
        .optional()?
        .ok_or(AppError::ImageNotFound)
}

pub fn delete(connection: &Connection, product_id: &str, image_id: &str) -> Result<(), AppError> {
    if connection.execute(
        "DELETE FROM product_images WHERE product_id = ?1 AND id = ?2",
        params![product_id, image_id],
    )? != 1
    {
        return Err(AppError::ImageNotFound);
    }
    Ok(())
}

pub fn references_image_or_path(
    connection: &Connection,
    image: &ProductImage,
) -> Result<bool, AppError> {
    Ok(connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM product_images WHERE id = ?1 OR relative_path = ?2)",
        params![image.id, image.relative_path],
        |row| row.get(0),
    )?)
}
