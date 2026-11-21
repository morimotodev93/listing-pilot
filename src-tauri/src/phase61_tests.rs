//! Disk-backed release audit. Retain only newly synthesized evidence under an
//! ignored UUID root; never resolve Tauri's normal app directories or reuse data.
use crate::{
    error::AppError,
    infrastructure::{db::migrations, media_storage},
    services::startup_service,
};
use rusqlite::{Connection, OpenFlags, params, types::Value};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, io::Write, path::Path};
use uuid::Uuid;

const TABLES: [&str; 5] = [
    "products",
    "product_images",
    "attribute_groups",
    "attribute_definitions",
    "product_attribute_values",
];
const V5: [&str; 5] = [
    include_str!("../migrations/0001_create_products.sql"),
    include_str!("../migrations/0002_create_product_images.sql"),
    include_str!("../migrations/0003_create_custom_attributes.sql"),
    include_str!("../migrations/0004_add_listing_content.sql"),
    include_str!("../migrations/0005_add_attribute_groups.sql"),
];

fn evidence_root() -> std::path::PathBuf {
    let parent = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join(".verification/phase61");
    // Reject links/reparse points in every component before creating test data.
    media_storage::directory(&parent, true).unwrap();
    let root = parent.join(Uuid::new_v4().to_string());
    fs::create_dir(&root).unwrap();
    println!("Phase 6.1 synthetic evidence: {}", root.display());
    root
}

fn write_new(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .unwrap();
    file.write_all(bytes).unwrap();
    file.sync_all().unwrap();
}

fn json(path: &Path, value: &impl Serialize) {
    write_new(path, &serde_json::to_vec_pretty(value).unwrap());
}

fn readonly(directory: &Path) -> Connection {
    Connection::open_with_flags(
        directory.join("listing-pilot.sqlite3"),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap()
}

fn rows(db: &Connection, sql: &str) -> Vec<Vec<String>> {
    let mut stmt = db.prepare(sql).unwrap();
    let count = stmt.column_count();
    stmt.query_map([], |row| {
        (0..count)
            .map(|i| row.get::<_, Value>(i).map(|value| format!("{value:?}")))
            .collect()
    })
    .unwrap()
    .collect::<rusqlite::Result<_>>()
    .unwrap()
}

fn snapshot(db: &Connection) -> BTreeMap<String, Vec<Vec<String>>> {
    TABLES
        .into_iter()
        .map(|table| {
            (
                table.to_owned(),
                rows(db, &format!("SELECT * FROM {table} ORDER BY 1,2")),
            )
        })
        .chain([(
            "image_display_order".into(),
            rows(db, "SELECT product_id,id,sort_order FROM product_images ORDER BY product_id,sort_order,created_at,id"),
        )])
        .collect()
}

fn schema(db: &Connection) -> Vec<Vec<String>> {
    rows(
        db,
        "SELECT type,name,tbl_name,sql FROM sqlite_schema ORDER BY type,name",
    )
}

fn version(db: &Connection) -> i64 {
    db.pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap()
}

fn healthy(db: &Connection) {
    assert_eq!(rows(db, "PRAGMA integrity_check"), [["Text(\"ok\")"]]);
    assert!(rows(db, "PRAGMA foreign_key_check").is_empty());
}

// Path, size and SHA-256 for every file, including hidden files and quarantine.
fn manifest(root: &Path) -> BTreeMap<String, (u64, String)> {
    fn walk(root: &Path, dir: &Path, files: &mut BTreeMap<String, (u64, String)>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                media_storage::directory(&path, false).unwrap();
                walk(root, &path, files);
            } else {
                media_storage::safe_regular_file(&path).unwrap();
                let bytes = fs::read(&path).unwrap();
                files.insert(
                    path.strip_prefix(root)
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .replace('\\', "/"),
                    (bytes.len() as u64, format!("{:x}", Sha256::digest(bytes))),
                );
            }
        }
    }
    let mut files = BTreeMap::new();
    walk(root, root, &mut files);
    files
}

fn copy_new_tree(source: &Path, destination: &Path) {
    fs::create_dir(destination).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let path = entry.unwrap().path();
        let target = destination.join(path.file_name().unwrap());
        if path.is_dir() {
            media_storage::directory(&path, false).unwrap();
            copy_new_tree(&path, &target);
        } else {
            media_storage::safe_regular_file(&path).unwrap();
            write_new(&target, &fs::read(path).unwrap());
        }
    }
}

fn seed_v5(root: &Path) -> std::path::PathBuf {
    let data = root.join("dataset");
    fs::create_dir(&data).unwrap();
    // Create the standard empty media layout and lock file without reconciliation.
    drop(media_storage::MediaStorage::open(&data.join("media")).unwrap());
    let mut db = Connection::open(data.join("listing-pilot.sqlite3")).unwrap();
    db.pragma_update(None, "foreign_keys", true).unwrap();
    let tx = db.transaction().unwrap();
    for sql in V5 {
        tx.execute_batch(sql).unwrap();
    }
    tx.pragma_update(None, "user_version", 5).unwrap();
    let group = Uuid::new_v4().to_string();
    let hidden_group = Uuid::new_v4().to_string();
    for (id, name, archived) in [
        (&group, "合成テスト項目", None),
        (
            &hidden_group,
            "以前の非表示グループ",
            Some("2026-11-01T00:00:00Z"),
        ),
    ] {
        tx.execute("INSERT INTO attribute_groups VALUES (?1,?2,?3,'2026-10-01T00:00:00Z','2026-11-01T00:00:00Z')", params![id,name,archived]).unwrap();
    }
    let definitions: Vec<_> = (0..3).map(|_| Uuid::new_v4().to_string()).collect();
    for (i, (name, kind, unit, archived, group_id)) in [
        ("素材", "text", None, None, &group),
        ("重量", "number", Some("g"), None, &group),
        (
            "旧項目",
            "text",
            None,
            Some("2026-11-01T00:00:00Z"),
            &hidden_group,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        tx.execute("INSERT INTO attribute_definitions VALUES (?1,?2,?3,?4,?5,'2026-10-01T00:00:00Z','2026-11-01T00:00:00Z',?6)", params![definitions[i],name,kind,unit,archived,group_id]).unwrap();
    }
    let products: Vec<_> = (0..2).map(|_| Uuid::new_v4().to_string()).collect();
    for (i, id) in products.iter().enumerate() {
        tx.execute("INSERT INTO products (id,sku,name,brand,price_yen,description_note,created_at,updated_at,listing_title,listing_description,group_id) VALUES (?1,?2,?3,'合成ブランド',?4,'内部メモ','2026-10-01T00:00:00Z','2026-11-01T00:00:00Z',?5,?6,?7)", params![id,format!("PH61-{:02}", i+1),format!("合成商品{}",i+1),[0,1280][i],format!("公開タイトル{}",i+1),"公開説明文\n2行目\t合成データ",group]).unwrap();
        for (attribute, value) in definitions
            .iter()
            .zip(["陶器", "0012.340000", "以前の保存値"])
        {
            tx.execute(
                "INSERT INTO product_attribute_values VALUES (?1,?2,?3)",
                params![id, attribute, value],
            )
            .unwrap();
        }
    }
    for i in 0..6 {
        let source = root.join(format!("sources/synthetic-{i}.png"));
        fs::create_dir_all(source.parent().unwrap()).unwrap();
        image::RgbImage::from_pixel(3, 2, image::Rgb([i as u8 * 30, 60, 90]))
            .save(&source)
            .unwrap();
        let bytes = fs::read(&source).unwrap();
        let image = Uuid::new_v4().to_string();
        let product = &products[usize::from(i > 0)];
        let relative = if i == 5 {
            format!("media/quarantine/deleted/{product}/{image}.png")
        } else {
            format!("media/originals/{product}/{image}.png")
        };
        write_new(&data.join(&relative), &bytes);
        assert_eq!(fs::read(data.join(&relative)).unwrap(), bytes);
        if i < 5 {
            let order = [0, 2, 0, 3, 1][i];
            tx.execute("INSERT INTO product_images VALUES (?1,?2,?3,?4,'image/png',?5,3,2,?6,?7,'2026-11-01T00:00:00Z')", params![image,product,format!("synthetic-{i}.png"),relative,bytes.len() as i64,format!("{:x}",Sha256::digest(bytes)),order]).unwrap();
        }
    }
    tx.commit().unwrap();
    healthy(&db);
    assert_eq!(version(&db), 5);
    drop(db);
    json(
        &root.join("sources-manifest.json"),
        &manifest(&root.join("sources")),
    );
    data
}

#[test]
fn v5_startup_migration_restart_and_backup_restore_preserve_dataset() {
    let root = evidence_root();
    let data = seed_v5(&root);
    let db = readonly(&data);
    let before = snapshot(&db);
    let old_schema = schema(&db);
    assert_eq!(version(&db), 5); // Before the actual startup service can migrate.
    json(&root.join("before-v5.json"), &before);
    json(&root.join("schema-v5.json"), &old_schema);
    drop(db);
    let files = manifest(&data);
    let media = manifest(&data.join("media"));
    let sources = manifest(&root.join("sources"));
    assert_eq!(files.len(), 8); // DB, lock, 5 originals, 1 deleted/quarantine image.
    json(&root.join("before-files.json"), &files);
    let backup = root.join("backup-v5");
    copy_new_tree(&data, &backup);
    assert_eq!(manifest(&backup), files);
    let restored = root.join("restored");
    copy_new_tree(&backup, &restored);
    assert_eq!(manifest(&restored), files);
    let restored_db = readonly(&restored);
    assert_eq!(version(&restored_db), 5);
    healthy(&restored_db);
    assert_eq!(snapshot(&restored_db), before);
    drop(restored_db);
    // No Tauri window/PathResolver is involved: both paths are fresh descendants
    // of the compile-time repository root, established before initialize().
    for target in [&data, &restored] {
        let (mut db, lock) = startup_service::initialize(target).unwrap();
        assert_eq!(version(&db), 6);
        healthy(&db);
        assert_eq!(snapshot(&db), before);
        let new_schema = schema(&db);
        assert!(old_schema.iter().all(|row| new_schema.contains(row)));
        assert!(rows(&db, "SELECT * FROM image_url_settings").is_empty());
        // The table, its PK autoindex, and its lookup index are the only additions.
        assert_eq!(new_schema.len(), old_schema.len() + 3);
        migrations::apply(&mut db).unwrap();
        assert_eq!(schema(&db), new_schema);
        drop((db, lock));
        let (db, lock) = startup_service::initialize(target).unwrap();
        assert_eq!(version(&db), 6);
        healthy(&db);
        assert_eq!(schema(&db), new_schema);
        assert_eq!(snapshot(&db), before);
        json(&target.with_extension("after-v6.json"), &snapshot(&db));
        drop((db, lock));
        assert_eq!(manifest(&target.join("media")), media);
        json(&target.with_extension("files-v6.json"), &manifest(target));
    }
    assert_eq!(manifest(&backup), files); // Backup was never migrated/opened.
    assert_eq!(manifest(&root.join("sources")), sources);
    json(
        &root.join("result.json"),
        &serde_json::json!({
            "result":"passed", "before_user_version":5, "after_user_version":6,
            "integrity_check":"ok", "foreign_key_check_rows":0,
            "products":2, "registered_images":5, "deleted_quarantine_images":1,
            "groups":3, "definitions":3, "values":6,
            "backup_file_count":files.len(), "backup_bytes":files.values().map(|(size,_)|size).sum::<u64>(),
            "old_rows_and_schema_preserved":true, "all_media_paths_sizes_sha256_preserved":true,
            "backup_restore_full_manifest_match":true, "restart_and_reapply":"passed",
            "ui_or_installer_started":false
        }),
    );
}

#[test]
fn v5_startup_failures_keep_database_and_every_media_file() {
    let root = evidence_root();
    for scenario in [
        "late-ddl",
        "foreign-key",
        "future-version",
        "media-locked",
        "corrupt-db",
    ] {
        let case = root.join(scenario);
        fs::create_dir(&case).unwrap();
        let data = seed_v5(&case);
        let db_path = data.join("listing-pilot.sqlite3");
        let db = Connection::open(&db_path).unwrap();
        match scenario {
            "late-ddl" => db
                .execute_batch("CREATE INDEX idx_image_url_settings_store ON products(id)")
                .unwrap(),
            "foreign-key" => db
                .execute_batch(
                    "PRAGMA foreign_keys=OFF; UPDATE product_images SET product_id='missing'",
                )
                .unwrap(),
            "future-version" => db.pragma_update(None, "user_version", 7).unwrap(),
            _ => (),
        }
        let before = snapshot(&db);
        let old_schema = schema(&db);
        let old_version = version(&db);
        drop(db);
        if scenario == "corrupt-db" {
            // Corrupt only this just-generated DB; record baseline after injection.
            let mut file = fs::OpenOptions::new().write(true).open(&db_path).unwrap();
            file.write_all(b"synthetic invalid SQLite header!").unwrap();
            file.sync_all().unwrap();
        }
        let files = manifest(&data);
        json(&case.join("before-files.json"), &files);
        let held_lock = (scenario == "media-locked")
            .then(|| media_storage::MediaStorage::open(&data.join("media")).unwrap());
        let failure = startup_service::initialize(&data);
        if scenario == "media-locked" {
            assert!(matches!(failure, Err(AppError::MediaBusy)));
        } else {
            assert!(matches!(failure, Err(AppError::DatabaseInitialization)));
        }
        // Windows denies reading the OS-locked .lock bytes. Release the test's
        // owner after observing rejection, then hash the entire dataset.
        drop(held_lock);
        assert_eq!(manifest(&data), files); // Includes the exact DB bytes.
        if scenario != "corrupt-db" {
            let db = readonly(&data);
            assert_eq!(version(&db), old_version);
            assert_eq!(schema(&db), old_schema);
            assert_eq!(snapshot(&db), before);
        }
        json(
            &case.join("result.json"),
            &serde_json::json!({
                "scenario":scenario, "result":"passed", "user_version_before":old_version,
                "all_files_including_db_byte_identical":true,
                "database_recreated":false, "media_reconciled":false
            }),
        );
    }
}
