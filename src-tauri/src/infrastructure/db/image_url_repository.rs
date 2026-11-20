use crate::{domain::image_url::ImageUrlSetting, error::AppError};
use rusqlite::{Connection, Row, params};

fn from_row(row: &Row<'_>) -> rusqlite::Result<ImageUrlSetting> {
    Ok(ImageUrlSetting {
        image_id: row.get(0)?,
        platform: row.get(1)?,
        store_key: row.get(2)?,
        url: row.get(3)?,
        image_sha256: row.get(4)?,
        alt_text: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

pub fn for_image(db: &Connection, image_id: &str) -> Result<Vec<ImageUrlSetting>, AppError> {
    let mut statement = db.prepare("SELECT image_id,platform,store_key,url,image_sha256,alt_text,updated_at FROM image_url_settings WHERE image_id=?1 ORDER BY platform,store_key")?;
    Ok(statement
        .query_map([image_id], from_row)?
        .collect::<rusqlite::Result<_>>()?)
}

pub fn for_product(db: &Connection, product_id: &str) -> Result<Vec<ImageUrlSetting>, AppError> {
    let mut statement = db.prepare("SELECT u.image_id,u.platform,u.store_key,u.url,u.image_sha256,u.alt_text,u.updated_at FROM image_url_settings u JOIN product_images i ON i.id=u.image_id WHERE i.product_id=?1 ORDER BY u.image_id,u.platform,u.store_key")?;
    Ok(statement
        .query_map([product_id], from_row)?
        .collect::<rusqlite::Result<_>>()?)
}

pub fn references_image(db: &Connection, image_id: &str) -> Result<bool, AppError> {
    Ok(db.query_row(
        "SELECT EXISTS(SELECT 1 FROM image_url_settings WHERE image_id=?1)",
        [image_id],
        |row| row.get(0),
    )?)
}

// Only called after complete validation under the service's IMMEDIATE transaction.
pub fn save(db: &Connection, setting: &ImageUrlSetting) -> Result<(), AppError> {
    db.execute("INSERT INTO image_url_settings (image_id,platform,store_key,url,image_sha256,alt_text,updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7) ON CONFLICT(image_id,platform,store_key) DO UPDATE SET url=excluded.url,image_sha256=excluded.image_sha256,alt_text=excluded.alt_text,updated_at=excluded.updated_at",
        params![setting.image_id,setting.platform,setting.store_key,setting.url,setting.image_sha256,setting.alt_text,setting.updated_at])?;
    Ok(())
}

pub fn clear(
    db: &Connection,
    image_id: &str,
    platform: &str,
    store_key: &str,
) -> Result<(), AppError> {
    db.execute(
        "DELETE FROM image_url_settings WHERE image_id=?1 AND platform=?2 AND store_key=?3",
        params![image_id, platform, store_key],
    )?;
    Ok(())
}
