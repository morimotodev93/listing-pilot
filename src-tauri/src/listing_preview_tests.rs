use crate::{
    domain::{
        listing_preview::*,
        product::{Product, ProductInput},
    },
    error::AppError,
    infrastructure::{
        db::{Database, connection, image_repository, product_repository},
        media_storage::MediaStorage,
    },
    services::{image_service, listing_preview_service as preview, product_service},
};
use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use uuid::Uuid;

struct Temp(PathBuf);
impl Drop for Temp {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
struct Fixture {
    media: MediaStorage,
    db: Database,
    product: Product,
    temp: Temp,
}
impl Fixture {
    fn new() -> Self {
        let temp = Temp(std::env::temp_dir().join(format!("listing-preview-{}", Uuid::new_v4())));
        fs::create_dir(&temp.0).unwrap();
        let db = connection::open(&temp.0.join("test.sqlite3")).unwrap();
        let input: ProductInput = serde_json::from_value(
            json!({"sku":"SKU", "name":"管理名", "description_note":"秘密の内部メモ"}),
        )
        .unwrap();
        let product = product_service::create(&db, input).unwrap();
        let db = Database(Arc::new(Mutex::new(Ok(db))));
        let media = MediaStorage::open(&temp.0.join("media")).unwrap();
        Self {
            media,
            db,
            product,
            temp,
        }
    }
    fn get(&self) -> ListingPreview {
        preview::get(&self.db, &self.media, &self.product.id).unwrap()
    }
    fn complete(&self) {
        self.db
            .with(|db| {
                db.execute(
                    "UPDATE products SET listing_title='公開名', listing_description='一行目
二行目', price_yen=0",
                    [],
                )?;
                Ok(())
            })
            .unwrap();
    }
    fn import(&self) {
        for color in 0..2 {
            let path = self.temp.0.join(format!("source-{color}.png"));
            image::RgbImage::from_pixel(2, 3, image::Rgb([color, 100, 200]))
                .save(&path)
                .unwrap();
            let report =
                image_service::import_files(&self.db, &self.media, &self.product.id, &[path])
                    .unwrap();
            assert_eq!(
                report.items[0].status,
                crate::domain::image::ImportStatus::Imported
            );
        }
    }
    fn dump(&self) -> Vec<Vec<Vec<rusqlite::types::Value>>> {
        self.db
            .with(|db| {
                [
                    "products",
                    "product_images",
                    "attribute_definitions",
                    "product_attribute_values",
                ]
                .iter()
                .map(|table| {
                    let mut statement =
                        db.prepare(&format!("SELECT * FROM {table} ORDER BY 1, 2"))?;
                    let count = statement.column_count();
                    Ok(statement
                        .query_map([], |row| (0..count).map(|i| row.get(i)).collect())?
                        .collect::<rusqlite::Result<Vec<_>>>()?)
                })
                .collect()
            })
            .unwrap()
    }
}

#[test]
fn fallback_warning_order_and_read_only_contract() {
    let f = Fixture::new();
    let before = f.dump();
    for _ in 0..2 {
        let p = f.get();
        assert_eq!(p.title, "管理名");
        assert_eq!(p.title_source, TitleSource::ProductName);
        assert_eq!(p.status, ListingStatus::Warning);
        assert_eq!(
            p.issues.iter().map(|i| &i.code).collect::<Vec<_>>(),
            vec![
                &IssueCode::TitleFallback,
                &IssueCode::DescriptionMissing,
                &IssueCode::PriceMissing,
                &IssueCode::ImagesMissing
            ]
        );
        assert!(
            p.issues
                .iter()
                .all(|i| i.severity == IssueSeverity::Warning)
        );
        let json = serde_json::to_string(&p).unwrap();
        assert!(!json.contains("秘密"));
        assert!(!json.contains("description_note"));
    }
    assert_eq!(f.dump(), before);
    f.db.with(|db| {
        assert_eq!(
            db.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))?,
            4
        );
        Ok(())
    })
    .unwrap();
}

#[test]
fn ready_preserves_content_order_and_files_then_reload_detects_missing_and_unsafe_images() {
    let f = Fixture::new();
    f.complete();
    f.import();
    let images =
        f.db.with(|db| image_repository::list(db, &f.product.id))
            .unwrap();
    let bytes: Vec<_> = images
        .iter()
        .map(|i| fs::read(f.media.image_path(i).unwrap()).unwrap())
        .collect();
    let before = f.dump();
    let p = f.get();
    assert_eq!(p.status, ListingStatus::Ready);
    assert!(p.issues.is_empty());
    assert_eq!(p.title, "公開名");
    assert_eq!(p.title_source, TitleSource::ListingTitle);
    assert_eq!(p.description.as_deref(), Some("一行目\n二行目"));
    assert_eq!(p.price_yen, Some(0));
    for (actual, image) in p.images.iter().zip(&images) {
        assert_eq!(actual.image_id, image.id);
        assert_eq!(actual.preview_path, f.media.preview(image));
    }
    assert_eq!(f.dump(), before);
    for (image, bytes) in images.iter().zip(bytes) {
        assert_eq!(fs::read(f.media.image_path(image).unwrap()).unwrap(), bytes);
    }
    let ids: Vec<_> = images.iter().map(|i| i.id.clone()).collect();
    let reversed: Vec<_> = ids.iter().rev().cloned().collect();
    image_service::reorder(&f.db, &f.media, &f.product.id, &ids, &reversed).unwrap();
    assert_eq!(f.get().images[0].image_id, reversed[0]);
    assert_eq!(
        f.db.with(|db| product_repository::get(db, &f.product.id))
            .unwrap()
            .updated_at,
        f.product.updated_at
    );
    fs::remove_file(f.media.image_path(&images[0]).unwrap()).unwrap();
    f.db.with(|db| {
        db.execute(
            "UPDATE product_images SET relative_path='../unsafe.png' WHERE id=?1",
            [&images[1].id],
        )?;
        Ok(())
    })
    .unwrap();
    let p = f.get();
    assert_eq!(p.status, ListingStatus::Blocked);
    assert!(p.images.iter().all(|i| i.preview_path.is_none()));
    assert_eq!(p.issues.len(), 1);
    assert_eq!(p.issues[0].code, IssueCode::ImageUnavailable);
}

#[test]
fn active_values_preserve_decimal_and_current_definition_order_and_restore() {
    let f = Fixture::new();
    f.db.with(|db| {
        db.execute_batch("INSERT INTO attribute_definitions VALUES ('b','寸法','number','cm',NULL,'1','1'), ('a','素材','text',NULL,NULL,'2','2'), ('c','未設定','text',NULL,NULL,'3','3'), ('d','非表示','text',NULL,'4','4','4');")?;
        for (id, value) in [("b","001.230000"), ("a","陶器"), ("d","保持する値")] {
            db.execute("INSERT INTO product_attribute_values VALUES (?1,?2,?3)", rusqlite::params![f.product.id, id, value])?;
        } Ok(())
    }).unwrap();
    let before = f.dump();
    let p = f.get();
    assert_eq!(p.attributes.len(), 2);
    assert_eq!(p.attributes[0].attribute_id, "b");
    assert_eq!(p.attributes[0].value, "001.230000");
    assert_eq!(p.attributes[0].unit.as_deref(), Some("cm"));
    assert_eq!(f.dump(), before);
    f.db.with(|db| { db.execute_batch("UPDATE attribute_definitions SET name='新素材' WHERE id='a'; UPDATE attribute_definitions SET archived_at=NULL WHERE id='d';")?; Ok(()) }).unwrap();
    let p = f.get();
    assert_eq!(p.attributes[1].name, "新素材");
    assert_eq!(p.attributes[2].value, "保持する値");
}

#[test]
fn missing_product_database_failure_and_mutation_guard_fail_the_request() {
    let f = Fixture::new();
    assert!(matches!(
        preview::get(&f.db, &f.media, "missing"),
        Err(AppError::ProductNotFound)
    ));
    let permit = f.media.begin_import().unwrap();
    assert!(preview::get(&f.db, &f.media, &f.product.id).is_err());
    drop(permit);
    assert_eq!(f.get().status, ListingStatus::Warning);
    f.db.with(|db| {
        db.execute_batch("DROP TABLE product_attribute_values;")?;
        Ok(())
    })
    .unwrap();
    assert!(matches!(
        preview::get(&f.db, &f.media, &f.product.id),
        Err(AppError::DatabaseOperation)
    ));
    assert!(f.db.with(|db| Ok(db.is_autocommit())).unwrap());
}

#[test]
fn serialized_candidate_excludes_storage_metadata_and_internal_note() {
    let f = Fixture::new();
    f.import();
    let value = serde_json::to_value(f.get()).unwrap();
    assert_eq!(value["titleSource"], "product_name");
    let image = value["images"][0].as_object().unwrap();
    assert_eq!(image.len(), 3);
    assert!(!image.contains_key("relative_path"));
    assert_eq!(value["description"], Value::Null);
}
