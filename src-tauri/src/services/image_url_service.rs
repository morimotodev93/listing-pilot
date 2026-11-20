use crate::{
    domain::image_url::{self, *},
    error::AppError,
    infrastructure::{
        db::{Database, image_repository, image_url_repository as urls, product_repository},
        media_storage::MediaStorage,
    },
    services::export_csv,
};
use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{Connection, TransactionBehavior};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};

fn read(db: &Connection, product_id: &str, store_key: &str) -> Result<ImageUrlSnapshot, AppError> {
    product_repository::get(db, product_id)?;
    let settings = urls::for_product(db, product_id)?;
    let images = image_repository::list(db, product_id)?
        .into_iter()
        .map(|image| {
            let setting = settings
                .iter()
                .find(|s| {
                    s.image_id == image.id && s.platform == PLATFORM && s.store_key == store_key
                })
                .cloned();
            ImageUrlRow {
                image_id: image.id,
                sort_order: image.sort_order,
                sha256: image.sha256,
                setting,
            }
        })
        .collect();
    let mut snapshot = ImageUrlSnapshot {
        product_id: product_id.into(),
        platform: PLATFORM,
        store_key: store_key.into(),
        snapshot_token: String::new(),
        images,
    };
    // Length-prefixed fields avoid ambiguous concatenation and do not log values.
    let mut hash = Sha256::new();
    hash.update(b"ListingPilot-image-url-settings-v1\0");
    let mut field = |value: &str| {
        hash.update((value.len() as u64).to_le_bytes());
        hash.update(value.as_bytes());
    };
    for value in [product_id, PLATFORM, store_key] {
        field(value);
    }
    for row in &snapshot.images {
        for value in [&row.image_id, &row.sort_order.to_string(), &row.sha256] {
            field(value);
        }
        field(if row.setting.is_some() {
            "set"
        } else {
            "unset"
        });
        if let Some(s) = &row.setting {
            for value in [&s.url, &s.image_sha256, &s.alt_text, &s.updated_at] {
                field(value);
            }
        }
    }
    snapshot.snapshot_token = format!("{:x}", hash.finalize());
    Ok(snapshot)
}

pub fn get(database: &Database, input: &GetImageUrlsInput) -> Result<ImageUrlSnapshot, AppError> {
    if !valid_uuid(&input.product_id) {
        return Err(AppError::ProductNotFound);
    }
    let store_key = image_url::store_key(&input.store_key)?;
    database.with_mut(|db| {
        let tx = db.transaction()?;
        read(&tx, &input.product_id, &store_key)
    })
}

pub fn save(
    database: &Database,
    media: &MediaStorage,
    input: &SaveImageUrlsInput,
) -> Result<ImageUrlSnapshot, AppError> {
    // Shares the image mutation guard; never wait for it while holding the DB mutex.
    let _permit = media.begin_import()?;
    if !valid_uuid(&input.product_id) {
        return Err(AppError::ProductNotFound);
    }
    let store_key = image_url::store_key(&input.store_key)?;
    let mut edits = HashMap::new();
    let mut unique_urls = HashSet::new();
    for edit in &input.images {
        if !valid_uuid(&edit.image_id) || edits.contains_key(&edit.image_id) {
            return Err(AppError::InvalidImageUrlSelection);
        }
        image_url::alt_text(&edit.alt_text)?;
        export_csv::text(&edit.alt_text, "Image alt text")
            .map_err(|_| AppError::InvalidImageAltText)?;
        let url = image_url::image_url(&edit.url)?;
        if let Some(url) = &url {
            if !unique_urls.insert(url.clone()) {
                return Err(AppError::DuplicateShopifyImageUrl);
            }
        } else if !edit.alt_text.is_empty() {
            // Unset means no mapping row. Do not silently discard an entered alt.
            return Err(AppError::InvalidImageAltText);
        }
        edits.insert(&edit.image_id, (url, &edit.alt_text));
    }
    database.with_mut(|db| {
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut current = read(&tx, &input.product_id, &store_key)?;
        if current.snapshot_token != input.snapshot_token {
            return Err(AppError::StaleData);
        }
        if current.images.len() != edits.len()
            || current
                .images
                .iter()
                .any(|row| !edits.contains_key(&row.image_id))
        {
            return Err(AppError::InvalidImageUrlSelection);
        }
        for row in &mut current.images {
            let (url, alt) = &edits[&row.image_id];
            let Some(url) = url else {
                urls::clear(&tx, &row.image_id, PLATFORM, &store_key)?;
                row.setting = None;
                continue;
            };
            if row.setting.as_ref().is_some_and(|old| {
                old.url == *url && old.alt_text == **alt && old.image_sha256 == row.sha256
            }) {
                continue;
            }
            let now = Utc::now();
            let timestamp = match row.setting.as_ref() {
                Some(old) => {
                    let previous = DateTime::parse_from_rfc3339(&old.updated_at)
                        .map_err(|_| AppError::DatabaseOperation)?
                        .with_timezone(&Utc);
                    now.max(
                        previous
                            .checked_add_signed(chrono::Duration::nanoseconds(1))
                            .ok_or(AppError::DatabaseOperation)?,
                    )
                }
                None => now,
            }
            .to_rfc3339_opts(SecondsFormat::Nanos, true);
            let setting = ImageUrlSetting {
                image_id: row.image_id.clone(),
                platform: PLATFORM.into(),
                store_key: store_key.clone(),
                url: url.clone(),
                image_sha256: row.sha256.clone(),
                alt_text: (*alt).clone(),
                updated_at: timestamp,
            };
            urls::save(&tx, &setting)?;
            row.setting = Some(setting);
        }
        let saved = read(&tx, &input.product_id, &store_key)?;
        // Detect ignored writes and unexpected trigger effects before COMMIT.
        if saved.images != current.images {
            return Err(AppError::StaleData);
        }
        tx.commit().map_err(|_| AppError::ImageUrlSaveUnknown)?;
        Ok(saved)
    })
}
