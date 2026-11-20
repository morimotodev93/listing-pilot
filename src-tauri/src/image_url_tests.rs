use crate::{
    domain::{
        image::ProductImage,
        image_url::*,
        product::{DeleteProductInput, ProductInput},
    },
    error::AppError,
    infrastructure::{
        db::{
            Database, connection, image_repository as images, image_url_repository as urls,
            migrations,
        },
        media_storage::MediaStorage,
    },
    services::{
        image_service, image_url_service as service, product_deletion_service as deletion,
        product_service,
    },
};
use rusqlite::{Connection, types::Value};
use serde_json::json;
use std::{
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use uuid::Uuid;

const A: &str = "a.myshopify.com";
const B: &str = "b.myshopify.com";
const URL: &str = "https://cdn.shopify.com/s/files/photo.png";

struct Temp(PathBuf);
impl Drop for Temp {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
// Close the OS lock and SQLite before removing this test's dedicated root.
struct Fixture {
    media: MediaStorage,
    db: Database,
    temp: Temp,
    id: String,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("listing-image-url-test-{}", Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        let db = connection::open(&root.join("test.sqlite3")).unwrap();
        let id = product(&db, "SKU");
        let f = Self {
            media: MediaStorage::open(&root.join("media")).unwrap(),
            db: Database(Arc::new(Mutex::new(Ok(db)))),
            temp: Temp(root),
            id,
        };
        f.add_image(1);
        f.add_image(2);
        f
    }
    fn add_image(&self, color: u8) {
        let path = self.temp.0.join(format!("source-{color}.png"));
        image::RgbImage::from_pixel(2, 3, image::Rgb([color, 20, 30]))
            .save(&path)
            .unwrap();
        let report = image_service::import_files(&self.db, &self.media, &self.id, &[path]).unwrap();
        assert_eq!(
            report.items[0].status,
            crate::domain::image::ImportStatus::Imported
        );
    }
    fn get(&self, store: &str) -> ImageUrlSnapshot {
        service::get(
            &self.db,
            &GetImageUrlsInput {
                product_id: self.id.clone(),
                store_key: store.into(),
            },
        )
        .unwrap()
    }
    fn input(&self, store: &str) -> SaveImageUrlsInput {
        let p = self.get(store);
        SaveImageUrlsInput {
            product_id: self.id.clone(),
            store_key: store.into(),
            snapshot_token: p.snapshot_token,
            images: p
                .images
                .iter()
                .enumerate()
                .map(|(i, row)| ImageUrlEdit {
                    image_id: row.image_id.clone(),
                    url: format!("{URL}?image={i}"),
                    alt_text: format!("画像{i}"),
                })
                .collect(),
        }
    }
    fn save(&self, store: &str) -> ImageUrlSnapshot {
        service::save(&self.db, &self.media, &self.input(store)).unwrap()
    }
    fn sql(&self, sql: &str) {
        self.db
            .with(|db| {
                db.execute_batch(sql)?;
                Ok(())
            })
            .unwrap();
    }
    fn all(&self) -> Vec<ImageUrlSetting> {
        self.db.with(|db| urls::for_product(db, &self.id)).unwrap()
    }
    fn ordered(&self) -> Vec<ProductImage> {
        self.db.with(|db| images::list(db, &self.id)).unwrap()
    }
    fn delete_input(&self) -> DeleteProductInput {
        let p = deletion::preview(&self.db, &self.id).unwrap();
        DeleteProductInput {
            product_id: p.product_id,
            expected_updated_at: p.updated_at,
            expected_image_ids: p.image_ids,
            expected_product: p.product,
            expected_attribute_values: p.attribute_values,
        }
    }
    fn bytes(&self) -> Vec<Vec<u8>> {
        self.ordered()
            .iter()
            .map(|image| fs::read(self.media.image_path(image).unwrap()).unwrap())
            .collect()
    }
}
fn product(db: &Connection, sku: &str) -> String {
    let input: ProductInput =
        serde_json::from_value(json!({"sku":sku,"name":"管理名","price_yen":123})).unwrap();
    product_service::create(db, input).unwrap().id
}
fn rows(db: &Connection, sql: &str) -> Vec<Vec<Value>> {
    let mut stmt = db.prepare(sql).unwrap();
    let n = stmt.column_count();
    stmt.query_map([], |r| (0..n).map(|i| r.get(i)).collect())
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap()
}
fn legacy_v5() -> Connection {
    let db = Connection::open_in_memory().unwrap();
    db.pragma_update(None, "foreign_keys", true).unwrap();
    for sql in [
        include_str!("../migrations/0001_create_products.sql"),
        include_str!("../migrations/0002_create_product_images.sql"),
        include_str!("../migrations/0003_create_custom_attributes.sql"),
        include_str!("../migrations/0004_add_listing_content.sql"),
        include_str!("../migrations/0005_add_attribute_groups.sql"),
    ] {
        db.execute_batch(sql).unwrap();
    }
    db.pragma_update(None, "user_version", 5).unwrap();
    let id = product(&db, "legacy");
    db.execute("INSERT INTO product_images VALUES ('image',?1,'photo.png','media/originals/p/i.png','image/png',12,2,3,?2,0,'created')", rusqlite::params![id,"a".repeat(64)]).unwrap();
    db.execute_batch("INSERT INTO attribute_definitions VALUES ('field','素材','text',NULL,NULL,'c','u','7a8f0c3e-4d5b-4c6a-9e20-1234567890ab')").unwrap();
    db.execute(
        "INSERT INTO product_attribute_values VALUES (?1,'field','陶器')",
        [&id],
    )
    .unwrap();
    db
}

#[test]
fn migration_v5_to_v6_preserves_all_old_columns_data_and_schema() {
    let mut db = legacy_v5();
    let tables = [
        "products",
        "product_images",
        "attribute_groups",
        "attribute_definitions",
        "product_attribute_values",
    ];
    let before: Vec<_> = tables
        .iter()
        .map(|t| rows(&db, &format!("SELECT * FROM {t} ORDER BY 1")))
        .collect();
    let schema = rows(&db, "SELECT name,sql FROM sqlite_master ORDER BY name");
    migrations::apply(&mut db).unwrap();
    assert_eq!(
        rows(&db, "PRAGMA user_version"),
        vec![vec![Value::Integer(6)]]
    );
    for (table, old) in tables.iter().zip(before) {
        assert_eq!(rows(&db, &format!("SELECT * FROM {table} ORDER BY 1")), old);
    }
    let after = rows(&db, "SELECT name,sql FROM sqlite_master ORDER BY name");
    assert!(schema.iter().all(|row| after.contains(row)));
    assert!(rows(&db, "PRAGMA foreign_key_check").is_empty());
    assert!(rows(&db, "SELECT * FROM image_url_settings").is_empty());
    migrations::apply(&mut db).unwrap();
    assert_eq!(
        rows(&db, "SELECT name,sql FROM sqlite_master ORDER BY name"),
        after
    );
}

#[test]
fn migration_late_ddl_failure_restores_v5_schema_and_data() {
    let mut db = legacy_v5();
    db.execute_batch("CREATE INDEX idx_image_url_settings_store ON products(id)")
        .unwrap();
    let schema = rows(&db, "SELECT name,sql FROM sqlite_master ORDER BY name");
    let data = rows(&db, "SELECT * FROM products");
    assert!(migrations::apply(&mut db).is_err());
    assert_eq!(
        rows(&db, "PRAGMA user_version"),
        vec![vec![Value::Integer(5)]]
    );
    assert_eq!(
        rows(&db, "SELECT name,sql FROM sqlite_master ORDER BY name"),
        schema
    );
    assert_eq!(rows(&db, "SELECT * FROM products"), data);
    assert!(db.prepare("SELECT * FROM image_url_settings").is_err());
}

#[test]
fn migration_foreign_key_failure_rolls_back_v6_addition() {
    let mut db = legacy_v5();
    db.pragma_update(None, "foreign_keys", false).unwrap();
    db.execute_batch("UPDATE product_images SET product_id='missing'")
        .unwrap();
    db.pragma_update(None, "foreign_keys", true).unwrap();
    let before = rows(&db, "SELECT * FROM product_images");
    assert!(migrations::apply(&mut db).is_err());
    assert_eq!(
        rows(&db, "PRAGMA user_version"),
        vec![vec![Value::Integer(5)]]
    );
    assert_eq!(rows(&db, "SELECT * FROM product_images"), before);
    assert!(db.prepare("SELECT * FROM image_url_settings").is_err());
}

#[test]
fn urls_preserve_queries_and_canonicalize_scheme_host_default_port() {
    for raw in [
        format!("{URL}?v=123&width=1200&name=a%20b"),
        format!("{URL}?x=a%2Fb%3Dc"),
    ] {
        assert_eq!(image_url(&raw).unwrap().unwrap(), raw);
    }
    assert_eq!(
        image_url("HTTPS://CDN.SHOPIFY.COM:443/photo.png")
            .unwrap()
            .unwrap(),
        "https://cdn.shopify.com/photo.png"
    );
    assert_eq!(image_url("").unwrap(), None);
}

#[test]
fn urls_reject_unsafe_authorities_schemes_controls_and_ambiguous_forms() {
    for raw in [
        "http://cdn.shopify.com/a",
        "https://cdn.shopify.com.evil/a",
        "https://evilcdn.shopify.com/a",
        "https://sub.cdn.shopify.com/a",
        "https://cdn.shopify.com./a",
        "https://user@cdn.shopify.com/a",
        "https://@cdn.shopify.com/a",
        "https://cdn.shopify.com:444/a",
        "https://cdn.shopify.com:/a",
        "https://cdn.shopify.com:0443/a",
        "https:cdn.shopify.com/a",
        "https:///cdn.shopify.com/a",
        "//cdn.shopify.com/a",
        "https://cdn%2eshopify.com/a",
        "https://cdn.shopify.com\\a",
        " https://cdn.shopify.com/a",
        "https://cdn.shopify.com/a ",
        "https://cdn.shopify.com/a#x",
        "https://cdn.shopify.com/a\n",
        "https://cdn.shopify.com/a%00",
        "https://cdn.shopify.com/a%0d",
        "https://cdn.shopify.com/a%7f",
        "https://cdn.shopify.com/a%C2%85",
        "https://cdn.shopify.com/a%GG",
        "https://cdn.shopify.com/a%",
        "file:///photo.png",
        "data:image/png;base64,xx",
        "javascript:alert(1)",
    ] {
        assert_eq!(
            image_url(raw).err(),
            Some(AppError::InvalidShopifyImageUrl),
            "unexpected accepted input"
        );
    }
    for c in ['\0', '\t', '\r', '\u{85}', '\u{a0}'] {
        assert!(image_url(&format!("{URL}{c}")).is_err());
    }
}

#[test]
fn url_length_checks_input_and_serialized_utf8_at_4096_bytes() {
    let prefix = "https://cdn.shopify.com/";
    let exact = format!("{prefix}{}", "a".repeat(4096 - prefix.len()));
    assert_eq!(image_url(&exact).unwrap().unwrap().len(), 4096);
    assert!(image_url(&(exact + "a")).is_err());
    let raw = format!("{prefix}{}", "あ".repeat(500));
    assert!(raw.len() < 4096);
    assert!(image_url(&raw).is_err()); // URL encoding expands beyond the stored limit.
}

#[test]
fn stores_normalize_and_enforce_single_ascii_label_boundaries() {
    assert_eq!(
        store_key("  My-Store.MYSHOPIFY.COM  ").unwrap(),
        "my-store.myshopify.com"
    );
    assert!(store_key(&format!("{}.myshopify.com", "a".repeat(63))).is_ok());
    for raw in [
        "myshopify.com",
        ".myshopify.com",
        "-x.myshopify.com",
        "x-.myshopify.com",
        "a.b.myshopify.com",
        "*.myshopify.com",
        "https://a.myshopify.com",
        "a.myshopify.com/",
        "a.myshopify.com:443",
        "user@a.myshopify.com",
        "a.myshopify.com.evil",
        "a_my.myshopify.com",
        "日本.myshopify.com",
        "a.myshopify.com.",
        "a.myshopify.com\n",
    ] {
        assert!(store_key(raw).is_err());
    }
    assert!(store_key(&format!("{}.myshopify.com", "a".repeat(64))).is_err());
}

#[test]
fn alt_empty_125_512_scalars_and_combining_characters_are_preserved() {
    let f = Fixture::new();
    for value in [
        String::new(),
        "a".repeat(125),
        "a".repeat(126),
        "𠮷".repeat(512),
        "e\u{301}".repeat(256),
    ] {
        let mut input = f.input(A);
        input.images[0].alt_text = value.clone();
        let saved = service::save(&f.db, &f.media, &input).unwrap();
        assert_eq!(saved.images[0].setting.as_ref().unwrap().alt_text, value);
    }
    let mut input = f.input(A);
    input.images[0].alt_text = "𠮷".repeat(513);
    assert_eq!(
        service::save(&f.db, &f.media, &input).err(),
        Some(AppError::InvalidImageAltText)
    );
}

#[test]
fn alt_rejects_controls_newlines_and_csv_formulas_without_modification() {
    let f = Fixture::new();
    f.save(A);
    let before = f.all();
    for value in [
        "=secret",
        "  +secret",
        "\u{feff}\u{200b}-secret",
        "\u{2060}@secret",
        "ok\n=secret",
        "ok\rtext",
        "ok\ttext",
        "\0",
        "ok\u{2028}text",
        "ok\u{2029}text",
        "\u{85}",
    ] {
        let mut input = f.input(A);
        input.images[1].alt_text = value.into();
        let error = service::save(&f.db, &f.media, &input).err().unwrap();
        assert_eq!(error, AppError::InvalidImageAltText);
        assert!(!serde_json::to_string(&error).unwrap().contains("secret"));
        assert!(f.all() == before);
    }
}

#[test]
fn save_update_clear_and_restart_keep_hashes_queries_sources_and_product() {
    let f = Fixture::new();
    let before_bytes = f.bytes();
    let product_before = f.db.with(|db| product_service::get(db, &f.id)).unwrap();
    let mut input = f.input(A);
    input.images[0].url = format!("{URL}?v=123&width=600");
    input.images[0].alt_text.clear();
    let saved = service::save(&f.db, &f.media, &input).unwrap();
    let s = saved.images[0].setting.as_ref().unwrap();
    assert_eq!(s.url, input.images[0].url);
    assert_eq!(s.image_sha256, saved.images[0].sha256);
    assert_eq!(s.alt_text, "");
    let mut update = f.input(A);
    update.images[0].url = format!("{URL}?new=1");
    let changed = service::save(&f.db, &f.media, &update).unwrap();
    assert!(changed.images[0].setting.as_ref().unwrap().updated_at > s.updated_at);
    let mut clear = f.input(A);
    clear.images[0].url.clear();
    clear.images[0].alt_text.clear();
    let cleared = service::save(&f.db, &f.media, &clear).unwrap();
    assert!(cleared.images[0].setting.is_none());
    assert!(cleared.images[1].setting.is_some());
    assert_eq!(f.bytes(), before_bytes);
    assert_eq!(
        f.db.with(|db| product_service::get(db, &f.id)).unwrap(),
        product_before
    );
    let original_source = fs::read(f.temp.0.join("source-1.png")).unwrap();
    assert_eq!(original_source, before_bytes[0]);
    let expected = serde_json::to_value(f.get(A)).unwrap();
    let Fixture {
        media,
        db,
        temp,
        id,
    } = f;
    drop(db);
    drop(media);
    let db = Database(Arc::new(Mutex::new(Ok(connection::open(
        &temp.0.join("test.sqlite3"),
    )
    .unwrap()))));
    let reread = service::get(
        &db,
        &GetImageUrlsInput {
            product_id: id,
            store_key: A.into(),
        },
    )
    .unwrap();
    assert_eq!(serde_json::to_value(reread).unwrap(), expected);
    drop(db);
    drop(temp);
}

#[test]
fn duplicate_url_scope_is_product_platform_and_store() {
    let f = Fixture::new();
    let mut input = f.input(A);
    input.images[0].url = format!("{URL}?v=1");
    input.images[1].url = "HTTPS://CDN.SHOPIFY.COM:443/s/files/photo.png?v=1".into();
    assert_eq!(
        service::save(&f.db, &f.media, &input).err(),
        Some(AppError::DuplicateShopifyImageUrl)
    );
    assert!(f.all().is_empty());
    input.images[1].url = format!("{URL}?v=2");
    service::save(&f.db, &f.media, &input).unwrap();
    let mut other_store = input.clone();
    other_store.store_key = B.into();
    other_store.snapshot_token = f.get(B).snapshot_token;
    service::save(&f.db, &f.media, &other_store).unwrap();
    let other_product = f.db.with(|db| Ok(product(db, "other"))).unwrap();
    f.db.with(|db| {
        let mut image = images::list(db, &f.id)?.remove(0);
        image.id = Uuid::new_v4().to_string();
        image.product_id = other_product.clone();
        image.relative_path = MediaStorage::relative_path(&other_product, &image.id, "png")?;
        images::create(db, &image)
    })
    .unwrap();
    let p = service::get(
        &f.db,
        &GetImageUrlsInput {
            product_id: other_product.clone(),
            store_key: A.into(),
        },
    )
    .unwrap();
    service::save(
        &f.db,
        &f.media,
        &SaveImageUrlsInput {
            product_id: other_product,
            store_key: A.into(),
            snapshot_token: p.snapshot_token,
            images: vec![ImageUrlEdit {
                image_id: p.images[0].image_id.clone(),
                url: input.images[0].url.clone(),
                alt_text: String::new(),
            }],
        },
    )
    .unwrap();
    // A future platform can keep the same URL; current IPC always fixes Shopify.
    f.sql("INSERT INTO image_url_settings SELECT image_id,'other',store_key,url,image_sha256,alt_text,updated_at FROM image_url_settings WHERE store_key='a.myshopify.com'");
    let no_op = SaveImageUrlsInput {
        snapshot_token: f.get(A).snapshot_token,
        ..input
    };
    service::save(&f.db, &f.media, &no_op).unwrap();
}

#[test]
fn stores_are_isolated_for_read_update_clear_and_tokens() {
    let f = Fixture::new();
    let a = f.save(A);
    let b_before = f.get(B).snapshot_token;
    assert!(f.get(B).images.iter().all(|row| row.setting.is_none()));
    let mut b = f.input(B);
    b.snapshot_token = a.snapshot_token;
    assert_eq!(
        service::save(&f.db, &f.media, &b).err(),
        Some(AppError::StaleData)
    );
    b.snapshot_token = b_before;
    b.images[0].url = format!("{URL}?store=B");
    service::save(&f.db, &f.media, &b).unwrap();
    let b_saved = serde_json::to_value(f.get(B)).unwrap();
    let mut clear = f.input(A);
    for row in &mut clear.images {
        row.url.clear();
        row.alt_text.clear();
    }
    service::save(&f.db, &f.media, &clear).unwrap();
    assert_eq!(serde_json::to_value(f.get(B)).unwrap(), b_saved);
}

#[test]
fn reorder_keeps_uuid_mapping_and_invalidates_old_save() {
    let f = Fixture::new();
    f.save(A);
    let before = f.all();
    let stale = f.input(A);
    let ids: Vec<_> = f.ordered().iter().map(|image| image.id.clone()).collect();
    let mut reversed = ids.clone();
    reversed.reverse();
    image_service::reorder(&f.db, &f.media, &f.id, &ids, &reversed).unwrap();
    assert!(f.all() == before);
    assert_eq!(f.get(A).images[0].image_id, ids[1]);
    assert_eq!(
        service::save(&f.db, &f.media, &stale).err(),
        Some(AppError::StaleData)
    );
}

#[test]
fn image_add_remove_sha_order_and_existing_url_changes_invalidate_save() {
    for mutation in [
        "add",
        "remove",
        "sha",
        "order",
        "url",
        "alt",
        "mapping_sha",
        "unset",
        "timestamp",
    ] {
        let f = Fixture::new();
        f.save(A);
        let stale = f.input(A);
        match mutation {
            "add" => f.add_image(3),
            "remove" => f.db.with(|db| images::delete(db, &f.id, &stale.images[0].image_id)).unwrap(),
            "sha" => f.sql("UPDATE product_images SET sha256=lower(hex(zeroblob(32))) WHERE rowid=(SELECT min(rowid) FROM product_images)"),
            "order" => f.sql("UPDATE product_images SET sort_order=sort_order+1"),
            "url" => f.sql("UPDATE image_url_settings SET url=url||'&changed=1'"),
            "alt" => f.sql("UPDATE image_url_settings SET alt_text='changed'"),
            "mapping_sha" => f.sql("UPDATE image_url_settings SET image_sha256=lower(hex(zeroblob(32)))"),
            "unset" => f.sql("DELETE FROM image_url_settings"),
            _ => f.sql("UPDATE image_url_settings SET updated_at='2099-01-01T00:00:00Z'"),
        }
        let before = f.all();
        assert_eq!(
            service::save(&f.db, &f.media, &stale).err(),
            Some(AppError::StaleData),
            "mutation {mutation}"
        );
        assert!(f.all() == before);
    }
}

#[test]
fn rejects_incomplete_duplicate_foreign_and_missing_image_sets_atomically() {
    let f = Fixture::new();
    f.save(A);
    let before = f.all();
    for case in 0..4 {
        let mut input = f.input(A);
        match case {
            0 => {
                input.images.pop();
            }
            1 => input.images.push(input.images[0].clone()),
            2 => input.images[0].image_id = Uuid::new_v4().to_string(),
            _ => input.images[0].image_id = "not-uuid".into(),
        }
        assert_eq!(
            service::save(&f.db, &f.media, &input).err(),
            Some(AppError::InvalidImageUrlSelection)
        );
        assert!(f.all() == before);
    }
    let mut input = f.input(A);
    input.product_id = Uuid::new_v4().to_string();
    assert_eq!(
        service::save(&f.db, &f.media, &input).err(),
        Some(AppError::ProductNotFound)
    );
}

#[test]
fn save_trigger_failure_after_first_write_rolls_back_entire_batch() {
    let f = Fixture::new();
    f.save(A);
    f.save(B);
    let before = f.all();
    let bytes = f.bytes();
    let target = f.ordered()[1].id.clone();
    f.sql(&format!("CREATE TRIGGER fail_second BEFORE UPDATE ON image_url_settings WHEN NEW.image_id='{target}' BEGIN SELECT RAISE(ABORT,'injected'); END"));
    let mut input = f.input(A);
    for row in &mut input.images {
        row.url.push_str("&changed=1");
    }
    assert_eq!(
        service::save(&f.db, &f.media, &input).err(),
        Some(AppError::DatabaseOperation)
    );
    assert!(f.all() == before);
    assert_eq!(f.bytes(), bytes);
}

#[test]
fn save_commit_failure_reports_unknown_and_rolls_back_all_settings() {
    let f = Fixture::new();
    f.save(A);
    let before = f.all();
    f.sql("CREATE TABLE deferred_blocker (image_id TEXT REFERENCES product_images(id) DEFERRABLE INITIALLY DEFERRED); CREATE TRIGGER fail_commit AFTER UPDATE ON image_url_settings BEGIN INSERT INTO deferred_blocker VALUES ('missing'); END");
    let mut input = f.input(A);
    input.images[0].url.push_str("&changed=1");
    assert_eq!(
        service::save(&f.db, &f.media, &input).err(),
        Some(AppError::ImageUrlSaveUnknown)
    );
    assert!(f.all() == before);
    assert!(f.db.with(|db| Ok(db.is_autocommit())).unwrap());
}

#[test]
fn image_delete_cascades_all_target_stores_only_and_preserves_sources() {
    let f = Fixture::new();
    f.save(A);
    f.save(B);
    let before = f.ordered();
    let source = fs::read(f.temp.0.join("source-1.png")).unwrap();
    image_service::delete(&f.db, &f.media, &f.id, &before[0].id).unwrap();
    assert_eq!(f.all().len(), 2);
    assert!(f.all().iter().all(|s| s.image_id == before[1].id));
    assert_eq!(fs::read(f.temp.0.join("source-1.png")).unwrap(), source);
    f.db.with(|db| {
        assert!(rows(db, "PRAGMA foreign_key_check").is_empty());
        Ok(())
    })
    .unwrap();
}

#[test]
fn product_delete_cascades_target_urls_and_keeps_other_product() {
    let f = Fixture::new();
    f.save(A);
    f.save(B);
    let other = f.db.with(|db| Ok(product(db, "other"))).unwrap();
    let other_image =
        f.db.with(|db| {
            let mut image = images::list(db, &f.id)?.remove(0);
            image.id = Uuid::new_v4().to_string();
            image.product_id = other.clone();
            image.relative_path = MediaStorage::relative_path(&other, &image.id, "png")?;
            images::create(db, &image)?;
            let mut setting = urls::for_product(db, &f.id)?.remove(0);
            setting.image_id = image.id.clone();
            urls::save(db, &setting)?;
            Ok(image)
        })
        .unwrap();
    let other_path = f.temp.0.join(&other_image.relative_path);
    fs::create_dir_all(other_path.parent().unwrap()).unwrap();
    fs::copy(f.temp.0.join("source-1.png"), &other_path).unwrap();
    let other_bytes = fs::read(&other_path).unwrap();
    let other_urls = f.db.with(|db| urls::for_product(db, &other)).unwrap();
    deletion::delete(&f.db, &f.media, &f.delete_input()).unwrap();
    assert!(f.all().is_empty());
    assert!(f.db.with(|db| urls::for_product(db, &other)).unwrap() == other_urls);
    assert!(f.db.with(|db| product_service::get(db, &other)).is_ok());
    assert_eq!(fs::read(other_path).unwrap(), other_bytes);
}

#[test]
fn image_and_product_delete_commit_failures_restore_urls_images_and_originals() {
    for product_delete in [false, true] {
        let f = Fixture::new();
        f.save(A);
        f.save(B);
        let before = f.all();
        let original_images = f.ordered();
        let bytes = f.bytes();
        f.db.with(|db| {
            db.execute_batch("CREATE TABLE blocker (image_id TEXT REFERENCES product_images(id) DEFERRABLE INITIALLY DEFERRED)")?;
            db.execute("INSERT INTO blocker VALUES (?1)",[&original_images[0].id])?; Ok(())
        }).unwrap();
        let result = if product_delete {
            deletion::delete(&f.db, &f.media, &f.delete_input())
        } else {
            image_service::delete(&f.db, &f.media, &f.id, &original_images[0].id)
        };
        assert_eq!(result.err(), Some(AppError::DatabaseOperation));
        assert!(f.all() == before);
        assert_eq!(f.ordered(), original_images);
        assert_eq!(f.bytes(), bytes);
    }
}

#[test]
fn image_and_product_delete_statement_failures_restore_cascaded_urls() {
    for product_delete in [false, true] {
        let f = Fixture::new();
        f.save(A);
        f.save(B);
        let before = f.all();
        let original_images = f.ordered();
        let bytes = f.bytes();
        if product_delete {
            f.sql("CREATE TRIGGER fail_delete BEFORE DELETE ON products BEGIN SELECT RAISE(ABORT,'injected'); END");
        } else {
            f.sql("CREATE TRIGGER fail_delete BEFORE UPDATE ON product_images BEGIN SELECT RAISE(ABORT,'injected'); END");
        }
        let result = if product_delete {
            deletion::delete(&f.db, &f.media, &f.delete_input())
        } else {
            image_service::delete(&f.db, &f.media, &f.id, &original_images[0].id)
        };
        assert_eq!(result.err(), Some(AppError::DatabaseOperation));
        assert!(f.all() == before);
        assert_eq!(f.ordered(), original_images);
        assert_eq!(f.bytes(), bytes);
    }
}

#[test]
fn commit_resolution_detects_url_only_inconsistency_and_unknown_database() {
    let f = Fixture::new();
    f.save(A);
    let before = f.ordered();
    let checked = f.all();
    let checked_image = f.db.with(|db| urls::for_image(db, &before[0].id)).unwrap();
    let resolve_image = || {
        image_service::resolve_delete_commit(
            &f.db,
            &before[0],
            &checked_image,
            Err(AppError::DatabaseOperation),
        )
    };
    let resolve_product = || {
        deletion::resolve_commit(
            &f.db,
            &f.id,
            &before,
            &[],
            &checked,
            Err(AppError::DatabaseOperation),
        )
    };
    assert_eq!(resolve_image(), Err(AppError::DatabaseOperation));
    assert_eq!(resolve_product(), Err(AppError::DatabaseOperation));
    f.sql("DELETE FROM image_url_settings");
    assert_eq!(resolve_image(), Err(AppError::ImageDeletionUnknown));
    assert_eq!(resolve_product(), Err(AppError::ProductDeletionUnknown));
    assert_eq!(f.bytes().len(), 2);
    f.sql("BEGIN IMMEDIATE");
    assert_eq!(resolve_image(), Err(AppError::ImageDeletionUnknown));
    assert_eq!(resolve_product(), Err(AppError::ProductDeletionUnknown));
    f.sql("ROLLBACK; DROP TABLE image_url_settings");
    assert_eq!(resolve_image(), Err(AppError::ImageDeletionUnknown));
    assert_eq!(resolve_product(), Err(AppError::ProductDeletionUnknown));
    assert_eq!(f.bytes().len(), 2);
}

#[test]
fn orphan_url_references_prevent_false_success_during_commit_resolution() {
    let f = Fixture::new();
    f.save(A);
    let before = f.ordered();
    let checked = f.all();
    let checked_image = f.db.with(|db| urls::for_image(db, &before[0].id)).unwrap();
    f.sql("PRAGMA foreign_keys=OFF; DELETE FROM product_images; DELETE FROM products; PRAGMA foreign_keys=ON");
    assert!(
        image_service::resolve_delete_commit(
            &f.db,
            &before[0],
            &checked_image,
            Err(AppError::DatabaseOperation)
        )
        .is_err()
    );
    assert_eq!(
        deletion::resolve_commit(
            &f.db,
            &f.id,
            &before,
            &[],
            &checked,
            Err(AppError::DatabaseOperation)
        ),
        Err(AppError::ProductDeletionUnknown)
    );
    assert!(fs::read(f.temp.0.join(&before[0].relative_path)).is_ok());
}

#[test]
fn input_dtos_reject_unknown_fields_and_output_is_camel_case_without_paths() {
    assert!(
        serde_json::from_value::<GetImageUrlsInput>(
            json!({"productId":"p","storeKey":A,"path":"secret"})
        )
        .is_err()
    );
    assert!(
        serde_json::from_value::<ImageUrlEdit>(
            json!({"imageId":"i","url":URL,"altText":"","sha256":"secret"})
        )
        .is_err()
    );
    assert!(
        serde_json::from_value::<SaveImageUrlsInput>(
            json!({"productId":"p","storeKey":A,"snapshotToken":"x","images":[],"platform":"other"})
        )
        .is_err()
    );
    let f = Fixture::new();
    let output = serde_json::to_value(f.save(A)).unwrap();
    assert!(output.get("snapshotToken").is_some());
    assert!(output.get("storeKey").is_some());
    let text = output.to_string();
    assert!(!text.contains("relative_path"));
    assert!(!text.contains("original_filename"));
    assert!(!text.contains("media/"));
}

#[test]
fn saving_respects_image_operation_guard_without_waiting_for_db_mutex() {
    let f = Fixture::new();
    let input = f.input(A);
    let _permit = f.media.begin_import().unwrap();
    let _db = f.db.0.lock().unwrap();
    assert_eq!(
        service::save(&f.db, &f.media, &input).err(),
        Some(AppError::MediaBusy)
    );
}

#[test]
fn schema_constraints_reject_orphans_duplicate_keys_and_invalid_lengths() {
    let f = Fixture::new();
    f.save(A);
    f.db.with(|db| {
        let original = urls::for_product(db, &f.id)?.remove(0);
        let mut invalid = original.clone();
        invalid.image_id = Uuid::new_v4().to_string();
        assert!(urls::save(db, &invalid).is_err());
        for case in 0..5 {
            let mut invalid = original.clone();
            match case {
                0 => invalid.url = format!("{URL}{}", "x".repeat(4096)),
                1 => invalid.url = "http://cdn.shopify.com/a".into(),
                2 => invalid.image_sha256 = "g".repeat(64),
                3 => invalid.alt_text = "a".repeat(513),
                _ => invalid.alt_text = "ok\0hidden".into(),
            }
            assert!(urls::save(db, &invalid).is_err());
        }
        assert!(
            db.execute_batch("INSERT INTO image_url_settings SELECT * FROM image_url_settings")
                .is_err()
        );
        assert!(
            db.execute("DELETE FROM products WHERE id=?1", [&f.id])
                .is_err()
        );
        assert!(rows(db, "PRAGMA foreign_key_check").is_empty());
        assert_eq!(
            rows(db, "PRAGMA foreign_key_list(image_url_settings)")[0][6],
            Value::Text("CASCADE".into())
        );
        assert_eq!(
            rows(db, "PRAGMA foreign_key_list(product_images)")[0][6],
            Value::Text("RESTRICT".into())
        );
        Ok(())
    })
    .unwrap();
}

#[test]
fn empty_products_have_distinct_tokens_and_stale_product_tokens_are_rejected() {
    let f = Fixture::new();
    let other = f.db.with(|db| Ok(product(db, "empty"))).unwrap();
    let query = GetImageUrlsInput {
        product_id: other.clone(),
        store_key: A.into(),
    };
    let p = service::get(&f.db, &query).unwrap();
    assert!(p.images.is_empty());
    assert_ne!(p.snapshot_token, f.get(A).snapshot_token);
    assert_eq!(
        service::save(
            &f.db,
            &f.media,
            &SaveImageUrlsInput {
                product_id: other.clone(),
                store_key: A.into(),
                snapshot_token: f.get(A).snapshot_token,
                images: vec![],
            }
        )
        .err(),
        Some(AppError::StaleData)
    );
    let saved = service::save(
        &f.db,
        &f.media,
        &SaveImageUrlsInput {
            product_id: other,
            store_key: " A.MYSHOPIFY.COM ".into(),
            snapshot_token: p.snapshot_token.clone(),
            images: vec![],
        },
    )
    .unwrap();
    assert_eq!(saved.snapshot_token, p.snapshot_token);
    // A no-op does not change a row's timestamp or unnecessarily invalidate editors.
    let saved = f.save(A);
    let again = f.save(A);
    assert_eq!(saved.snapshot_token, again.snapshot_token);
}

#[test]
fn save_rechecks_ignored_writes_and_triggered_metadata_changes() {
    for trigger in [
        "CREATE TRIGGER ignore_write BEFORE UPDATE ON image_url_settings BEGIN SELECT RAISE(IGNORE); END",
        "CREATE TRIGGER ignore_write AFTER UPDATE ON image_url_settings BEGIN UPDATE product_images SET sort_order=sort_order+1; END",
    ] {
        let f = Fixture::new();
        f.save(A);
        let before = f.all();
        let before_images = f.ordered();
        f.sql(trigger);
        let mut input = f.input(A);
        input.images[0].url.push_str("&changed=1");
        assert_eq!(
            service::save(&f.db, &f.media, &input).err(),
            Some(AppError::StaleData)
        );
        assert!(f.all() == before);
        assert_eq!(f.ordered(), before_images);
    }
}

#[test]
fn invalid_url_and_unset_url_with_alt_leave_all_stores_unchanged() {
    let f = Fixture::new();
    f.save(A);
    f.save(B);
    let before = f.all();
    for url in ["https://evil.example/a", ""] {
        let mut input = f.input(A);
        input.images[1].url = url.into();
        let error = service::save(&f.db, &f.media, &input).err().unwrap();
        assert_eq!(
            error,
            if url.is_empty() {
                AppError::InvalidImageAltText
            } else {
                AppError::InvalidShopifyImageUrl
            }
        );
        assert!(f.all() == before);
    }
}

#[test]
fn commit_resolution_accepts_confirmed_deletion_with_cascaded_urls() {
    let f = Fixture::new();
    f.save(A);
    f.save(B);
    let before = f.ordered();
    let checked = f.all();
    let checked_image = f.db.with(|db| urls::for_image(db, &before[0].id)).unwrap();
    deletion::delete(&f.db, &f.media, &f.delete_input()).unwrap();
    assert_eq!(
        image_service::resolve_delete_commit(
            &f.db,
            &before[0],
            &checked_image,
            Err(AppError::DatabaseOperation)
        ),
        Ok(())
    );
    assert_eq!(
        deletion::resolve_commit(
            &f.db,
            &f.id,
            &before,
            &[],
            &checked,
            Err(AppError::DatabaseOperation)
        ),
        Ok(())
    );
}
