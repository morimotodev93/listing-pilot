use crate::{
    domain::{
        product::ProductInput,
        product_export::{ExportInput, ExportReport},
    },
    error::AppError,
    infrastructure::{
        db::{Database, connection, image_repository},
        export_storage::{self, Workspace},
        media_storage::MediaStorage,
    },
    services::{
        export_csv, image_service, listing_preview_service as preview,
        product_export_service as export, product_service,
    },
};
use serde_json::json;
use std::{
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use uuid::Uuid;

struct Fixture {
    media: MediaStorage,
    db: Database,
    id: String,
    root: PathBuf,
    out: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("listing-export-test-{}", Uuid::new_v4()));
        let data = root.join("data");
        let out = root.join("out");
        fs::create_dir_all(&data).unwrap();
        fs::create_dir(&out).unwrap();
        let db = connection::open(&data.join("test.sqlite3")).unwrap();
        let input: ProductInput = serde_json::from_value(json!({"sku":"日本語SKU", "name":"管理名", "description_note":"極秘メモ", "listing_title":"公開,\"タイトル\"", "listing_description":"日本語\n二行目", "price_yen":0})).unwrap();
        let id = product_service::create(&db, input).unwrap().id;
        Self {
            media: MediaStorage::open(&data.join("media")).unwrap(),
            db: Database(Arc::new(Mutex::new(Ok(db)))),
            id,
            root,
            out,
        }
    }
    fn sql(&self, sql: &str) {
        self.db
            .with(|db| {
                db.execute_batch(sql)?;
                Ok(())
            })
            .unwrap();
    }
    fn input(&self) -> ExportInput {
        let p = preview::get(&self.db, &self.media, &self.id).unwrap();
        ExportInput {
            product_id: self.id.clone(),
            snapshot_token: p.snapshot_token.clone(),
            warning_token: Some(p.snapshot_token),
        }
    }
    fn run(&self, input: &ExportInput) -> Result<ExportReport, AppError> {
        let _permit = self.media.begin_import()?;
        export::export(&self.db, &self.media, input, &self.out)
    }
    fn image(&self, extension: &str, color: u8) {
        let source = self.root.join(format!("source-{color}.{extension}"));
        image::RgbImage::from_pixel(2, 3, image::Rgb([color, 100, 200]))
            .save(&source)
            .unwrap();
        let report =
            image_service::import_files(&self.db, &self.media, &self.id, &[source]).unwrap();
        assert_eq!(
            report.items[0].status,
            crate::domain::image::ImportStatus::Imported
        );
    }
    fn dump(&self) -> String {
        self.db
            .with(|db| {
                let mut result = String::new();
                for table in [
                    "products",
                    "product_images",
                    "attribute_definitions",
                    "product_attribute_values",
                ] {
                    let mut stmt = db.prepare(&format!("SELECT * FROM {table} ORDER BY 1,2"))?;
                    let count = stmt.column_count();
                    let rows = stmt
                        .query_map([], |r| {
                            (0..count)
                                .map(|i| r.get::<_, rusqlite::types::Value>(i))
                                .collect::<rusqlite::Result<Vec<_>>>()
                        })?
                        .collect::<rusqlite::Result<Vec<_>>>()?;
                    result.push_str(&format!("{rows:?}"));
                }
                assert_eq!(
                    db.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))?,
                    4
                );
                Ok(result)
            })
            .unwrap()
    }
    fn empty(&self) {
        assert_eq!(fs::read_dir(&self.out).unwrap().count(), 0);
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        // Release handles before deleting only this test's unique temporary root.
        let _ = self
            .db
            .0
            .lock()
            .map(|mut db| *db = Err(AppError::DatabaseInitialization));
        // Media lock is still open; Windows allows ordinary file deletion here.
        let _ = fs::remove_dir_all(&self.root);
    }
}
fn folder(report: ExportReport) -> PathBuf {
    match report {
        ExportReport::Success {
            folder,
            product_count: 1,
            ..
        } => folder.into(),
        other => panic!("{other:?}"),
    }
}

#[test]
fn csv_unicode_quotes_bom_crlf_and_private_note_exclusion() {
    let f = Fixture::new();
    let before = f.dump();
    let path = folder(f.run(&f.input()).unwrap());
    let csv = fs::read(path.join("product.csv")).unwrap();
    assert!(csv.starts_with(&[0xef, 0xbb, 0xbf]));
    let csv = String::from_utf8(csv).unwrap();
    assert!(csv.contains("\"公開,\"\"タイトル\"\"\""));
    assert!(csv.contains("\"listing_title\",\"日本語\n二行目\",\"0\""));
    assert!(!csv.contains("極秘"));
    assert!(csv.ends_with("\r\n"));
    assert_eq!(
        fs::read_to_string(path.join("attributes.csv")).unwrap(),
        "\u{feff}product_id,attribute_id,name,value,unit\r\n"
    );
    assert_eq!(
        fs::read_to_string(path.join("images.csv")).unwrap(),
        "\u{feff}product_id,image_id,sort_order,relative_file,sha256\r\n"
    );
    assert_eq!(fs::read_dir(path.join("images")).unwrap().count(), 0);
    assert_eq!(before, f.dump());
}

#[test]
fn null_price_and_fallback_are_shared_preview_candidates() {
    let f = Fixture::new();
    f.sql("UPDATE products SET listing_title=NULL, price_yen=NULL");
    let path = folder(f.run(&f.input()).unwrap());
    let csv = fs::read_to_string(path.join("product.csv")).unwrap();
    assert!(csv.contains("\"管理名\",\"product_name\""));
    assert!(csv.contains("\"\",\"\",\"0\"\r\n"));
}

#[test]
fn formula_prefixes_whitespace_controls_and_embedded_lines_are_rejected() {
    for value in [
        "=1",
        "+1",
        "-text",
        "@cmd",
        " \t=1",
        "\0\u{feff}=1",
        "\r\n\t+1",
        "hello\n=1",
        "\u{200b}@x",
    ] {
        assert_eq!(
            export_csv::text(value, "product.title"),
            Err(AppError::CsvFormulaRisk {
                column: "product.title".into()
            })
        );
    }
    for value in ["日本語", "a,b\"c", "一行\n二行", "12-34"] {
        export_csv::text(value, "x").unwrap();
    }
}

#[test]
fn formula_risk_aborts_entire_output_without_modifying_data() {
    for column in ["sku", "listing_title", "listing_description"] {
        let f = Fixture::new();
        f.sql(&format!("UPDATE products SET {column}='=1+1'"));
        let before = f.dump();
        assert!(matches!(
            f.run(&f.input()),
            Err(AppError::CsvFormulaRisk { .. })
        ));
        f.empty();
        assert_eq!(before, f.dump());
    }
}

#[test]
fn numeric_precision_negative_values_and_archived_attributes() {
    let f = Fixture::new();
    let active = Uuid::new_v4().to_string();
    let archived = Uuid::new_v4().to_string();
    f.sql(&format!("INSERT INTO attribute_definitions(id,name,data_type,unit,created_at,updated_at) VALUES('{active}','長さ','number','cm','now','now'); INSERT INTO attribute_definitions(id,name,data_type,archived_at,created_at,updated_at) VALUES('{archived}','非表示','text','now','now','now'); INSERT INTO product_attribute_values(product_id,attribute_id,value) VALUES('{}','{active}','-000123.450000'),('{}','{archived}','=danger');",f.id,f.id));
    let before = f.dump();
    let path = folder(f.run(&f.input()).unwrap());
    let csv = fs::read_to_string(path.join("attributes.csv")).unwrap();
    assert!(csv.contains("\"長さ\",\"-000123.450000\",\"cm\""));
    assert!(!csv.contains("非表示"));
    assert_eq!(before, f.dump());
}

#[test]
fn attribute_formula_columns_are_identified() {
    for (name, value, unit, kind, column) in [
        ("=name", "ok", None, "text", "attributes.name"),
        ("name", "@value", None, "text", "attributes.value"),
        ("name", "-1", Some("+unit"), "number", "attributes.unit"),
    ] {
        let f = Fixture::new();
        let id = Uuid::new_v4();
        f.sql(&format!("INSERT INTO attribute_definitions(id,name,data_type,unit,created_at,updated_at) VALUES('{id}','{name}','{kind}',{},'now','now'); INSERT INTO product_attribute_values(product_id,attribute_id,value) VALUES('{}','{id}','{value}');",unit.map(|s|format!("'{s}'")).unwrap_or("NULL".into()),f.id));
        assert_eq!(
            f.run(&f.input()).unwrap_err(),
            AppError::CsvFormulaRisk {
                column: column.into()
            }
        );
        f.empty();
    }
}

#[test]
fn ready_export_preserves_jpeg_png_webp_bytes_and_normalizes_csv_order_only() {
    let f = Fixture::new();
    for (i, ext) in ["jpg", "png", "webp"].iter().enumerate() {
        f.image(ext, i as u8);
    }
    f.sql("UPDATE product_images SET sort_order=sort_order*5+2");
    let before = f.dump();
    let images = f.db.with(|db| image_repository::list(db, &f.id)).unwrap();
    let p = preview::get(&f.db, &f.media, &f.id).unwrap();
    assert_eq!(
        p.status,
        crate::domain::listing_preview::ListingStatus::Ready
    );
    let path = folder(f.run(&f.input()).unwrap());
    let csv = fs::read_to_string(path.join("images.csv")).unwrap();
    for (i, image) in images.iter().enumerate() {
        let original = f.media.image_path(image).unwrap();
        let relative = format!(
            "images/{:04}-{}.{}",
            i + 1,
            image.id,
            original.extension().unwrap().to_str().unwrap()
        );
        assert_eq!(
            fs::read(original).unwrap(),
            fs::read(path.join(&relative)).unwrap()
        );
        assert!(csv.contains(&format!(
            "\"{}\",\"{i}\",\"{relative}\",\"{}\"",
            image.id, image.sha256
        )));
    }
    assert_eq!(before, f.dump());
}

#[test]
fn warning_requires_confirmation_for_exact_snapshot() {
    let f = Fixture::new();
    let mut input = f.input();
    input.warning_token = None;
    assert_eq!(f.run(&input).unwrap_err(), AppError::ExportWarningRequired);
    input.warning_token = Some("old-token".into());
    assert_eq!(f.run(&input).unwrap_err(), AppError::ExportWarningRequired);
    f.empty();
}

#[test]
fn changed_product_or_definition_or_value_invalidates_confirmation() {
    for sql in [
        "UPDATE products SET listing_title='changed'",
        "UPDATE products SET sku='changed'",
        "UPDATE products SET price_yen=2",
        "UPDATE products SET listing_description='changed'",
        "UPDATE attribute_definitions SET name='changed'",
        "UPDATE attribute_definitions SET archived_at='now'",
        "UPDATE product_attribute_values SET value='changed'",
    ] {
        let f = Fixture::new();
        let id = Uuid::new_v4();
        f.sql(&format!("INSERT INTO attribute_definitions(id,name,data_type,created_at,updated_at) VALUES('{id}','項目','text','now','now'); INSERT INTO product_attribute_values(product_id,attribute_id,value) VALUES('{}','{id}','value')",f.id));
        let input = f.input();
        f.sql(sql);
        assert_eq!(f.run(&input).unwrap_err(), AppError::StaleData);
        f.empty();
    }
}

#[test]
fn changed_images_invalidate_snapshot_even_without_product_timestamp_change() {
    for mutation in [
        "UPDATE product_images SET sort_order=1-sort_order",
        "DELETE FROM product_images WHERE sort_order=0",
    ] {
        let f = Fixture::new();
        f.image("png", 0);
        f.image("png", 1);
        let input = f.input();
        f.sql(mutation);
        assert_eq!(f.run(&input).unwrap_err(), AppError::StaleData);
        f.empty();
    }
    let f = Fixture::new();
    let input = f.input();
    f.image("png", 0);
    assert_eq!(f.run(&input).unwrap_err(), AppError::StaleData);
    f.empty();
}

#[test]
fn missing_image_blocks_export() {
    let f = Fixture::new();
    f.image("png", 0);
    let input = f.input();
    let image =
        f.db.with(|db| image_repository::list(db, &f.id))
            .unwrap()
            .remove(0);
    fs::remove_file(f.media.image_path(&image).unwrap()).unwrap();
    assert_eq!(f.run(&input).unwrap_err(), AppError::ExportBlocked);
    f.empty();
}

#[test]
fn corrupt_hash_or_size_fails_and_cleans_partial_output() {
    for sql in [
        "UPDATE product_images SET sha256=printf('%064d',0)",
        "UPDATE product_images SET file_size_bytes=file_size_bytes+1",
    ] {
        let f = Fixture::new();
        f.image("png", 0);
        f.sql(sql);
        let before = f.dump();
        assert_eq!(f.run(&f.input()).unwrap_err(), AppError::ExportImageInvalid);
        f.empty();
        assert_eq!(before, f.dump());
    }
}

#[test]
fn existing_folder_is_never_overwritten_and_retry_can_succeed_elsewhere() {
    let f = Fixture::new();
    let input = f.input();
    let path = folder(f.run(&input).unwrap());
    fs::write(path.join("sentinel"), b"preserve").unwrap();
    assert_eq!(f.run(&input).unwrap_err(), AppError::ExportCollision);
    assert_eq!(fs::read(path.join("sentinel")).unwrap(), b"preserve");
    let other = f.root.join("other");
    fs::create_dir(&other).unwrap();
    let _permit = f.media.begin_import().unwrap();
    folder(export::export(&f.db, &f.media, &input, &other).unwrap());
}

#[test]
fn product_changes_during_copy_prevent_publication() {
    let f = Fixture::new();
    let input = f.input();
    let _permit = f.media.begin_import().unwrap();
    let result = export::export_with(&f.db, &f.media, &input, &f.out, || {
        f.sql("UPDATE products SET price_yen=500")
    });
    assert_eq!(result.unwrap_err(), AppError::StaleData);
    f.empty();
}

#[test]
fn media_permit_contention_fails_then_releases_for_retry() {
    let f = Fixture::new();
    let input = f.input();
    let permit = f.media.begin_import().unwrap();
    assert_eq!(f.run(&input).unwrap_err(), AppError::MediaBusy);
    drop(permit);
    folder(f.run(&input).unwrap());
}

#[test]
fn managed_data_and_non_directory_destinations_are_rejected() {
    let f = Fixture::new();
    for path in [
        f.root.join("data"),
        f.root.join("data/media"),
        f.root.join("data/media/originals"),
        f.root.join("data/test.sqlite3"),
        f.out.join("missing"),
    ] {
        assert_eq!(
            export_storage::destination(&path, &f.root.join("data")).unwrap_err(),
            AppError::ExportDestination
        );
    }
}

#[test]
fn workspace_only_removes_its_own_files_and_reports_unknown_entries() {
    let f = Fixture::new();
    let mut w = Workspace::new(&f.out).unwrap();
    w.images().unwrap();
    drop(w.file("product.csv").unwrap());
    fs::write(w.path.join("unknown"), b"keep").unwrap();
    assert!(w.cleanup().is_err());
    assert_eq!(fs::read(w.path.join("unknown")).unwrap(), b"keep");
}

#[test]
fn cleanup_failure_is_reported_with_retained_folder_and_original_error() {
    let f = Fixture::new();
    let input = f.input();
    let _permit = f.media.begin_import().unwrap();
    let result = export::export_with(&f.db, &f.media, &input, &f.out, || {
        let entry = fs::read_dir(&f.out)
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        fs::write(entry.join("unknown"), b"preserve").unwrap();
        f.sql("UPDATE products SET price_yen=500");
    })
    .unwrap();
    match result {
        ExportReport::Failed {
            error: AppError::StaleData,
            retained_folder,
            cleanup_failed: true,
        } => assert_eq!(
            fs::read(PathBuf::from(retained_folder).join("unknown")).unwrap(),
            b"preserve"
        ),
        other => panic!("{other:?}"),
    }
}

#[test]
fn workspace_create_new_and_mid_write_failure_preserve_existing_files() {
    let f = Fixture::new();
    let mut w = Workspace::new(&f.out).unwrap();
    w.images().unwrap();
    drop(w.file("product.csv").unwrap());
    assert!(w.file("product.csv").is_err());
    assert!(w.file("missing/file.csv").is_err());
    w.cleanup().unwrap();
    f.empty();
}

#[test]
fn directory_finalize_collision_is_non_replacing_and_unknown_outcome_retains_files() {
    let f = Fixture::new();
    let w = Workspace::new(&f.out).unwrap();
    let target = f.out.join("finished");
    fs::create_dir(&target).unwrap();
    fs::write(target.join("keep"), b"keep").unwrap();
    assert_eq!(w.finish(&target).unwrap_err(), AppError::ExportCollision);
    assert!(w.path.exists());
    assert_eq!(fs::read(target.join("keep")).unwrap(), b"keep");
    let moved = f.out.join("moved");
    fs::rename(&w.path, &moved).unwrap();
    assert_eq!(
        w.finish(&f.out.join("new")).unwrap_err(),
        AppError::ExportUnknown
    );
    assert!(moved.exists());
}

#[cfg(windows)]
#[test]
fn junction_output_and_managed_image_parents_are_rejected() {
    use std::os::windows::process::CommandExt;
    let f = Fixture::new();
    let link = f.root.join("junction");
    assert!(
        std::process::Command::new("cmd")
            .args(["/c", "mklink", "/J"])
            .arg(&link)
            .arg(&f.out)
            .creation_flags(0x08000000)
            .output()
            .unwrap()
            .status
            .success()
    );
    assert_eq!(
        export_storage::destination(&link, &f.root.join("data")).unwrap_err(),
        AppError::ExportDestination
    );
    // Remove only the junction itself, never the target tree.
    fs::remove_dir(&link).unwrap();
    f.image("png", 0);
    let image =
        f.db.with(|db| image_repository::list(db, &f.id))
            .unwrap()
            .remove(0);
    let parent = f
        .media
        .image_path(&image)
        .unwrap()
        .parent()
        .unwrap()
        .to_owned();
    let moved = f.root.join("moved-images");
    fs::rename(&parent, &moved).unwrap();
    let output = std::process::Command::new("cmd")
        .args(["/d", "/c", "mklink", "/J"])
        .arg(
            parent
                .to_string_lossy()
                .trim_start_matches(r"\\?\")
                .replace('/', "\\"),
        )
        .arg(&moved)
        .creation_flags(0x08000000)
        .output()
        .unwrap();
    assert!(output.status.success(), "{:?}", output);
    let p = preview::get(&f.db, &f.media, &f.id).unwrap();
    assert_eq!(
        p.status,
        crate::domain::listing_preview::ListingStatus::Blocked
    );
    assert!(matches!(f.run(&f.input()), Err(AppError::ExportBlocked)));
    f.empty();
    fs::remove_dir(parent).unwrap();
}
