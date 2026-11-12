use crate::{
    domain::{attribute::*, product::ProductInput},
    error::AppError,
    infrastructure::db::{
        attribute_repository as repository, connection, image_repository, migrations,
    },
    services::{attribute_service as service, product_service},
};
use rusqlite::{Connection, params};
use std::path::PathBuf;

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let path =
            std::env::temp_dir().join(format!("listing-pilot-attributes-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

fn input(name: &str, data_type: &str, unit: Option<&str>) -> CreateAttributeInput {
    CreateAttributeInput {
        name: name.into(),
        data_type: data_type.into(),
        unit: unit.map(str::to_owned),
    }
}
fn rename(
    db: &Connection,
    attribute: &AttributeDefinition,
    name: &str,
) -> Result<AttributeDefinition, AppError> {
    service::update(
        db,
        UpdateAttributeInput {
            id: attribute.id.clone(),
            name: name.into(),
            expected_updated_at: attribute.updated_at.clone(),
        },
    )
}
fn archive(
    db: &Connection,
    attribute: &AttributeDefinition,
    archived: bool,
) -> Result<AttributeDefinition, AppError> {
    service::set_archived(
        db,
        SetAttributeArchivedInput {
            id: attribute.id.clone(),
            archived,
            expected_updated_at: attribute.updated_at.clone(),
        },
    )
}
fn product(db: &Connection) -> crate::domain::product::Product {
    product_service::create(
        db,
        ProductInput {
            sku: "SKU".into(),
            name: "商品".into(),
            brand: Some("ブランド".into()),
            price_yen: Some(1200),
            description_note: Some("説明\nメモ".into()),
        },
    )
    .unwrap()
}
fn version(db: &Connection) -> i64 {
    db.pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap()
}
fn schema(db: &Connection) -> Vec<(String, String)> {
    db.prepare("SELECT name, sql FROM sqlite_master WHERE sql IS NOT NULL ORDER BY name")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap()
}

#[test]
fn migrate_zero_one_two_to_three_preserves_every_product_and_image_column() {
    for initial in 0..=2 {
        let temp = Temp::new();
        let mut db = Connection::open(temp.0.join("test.sqlite3")).unwrap();
        db.pragma_update(None, "foreign_keys", true).unwrap();
        let before = if initial >= 1 {
            db.execute_batch(include_str!("../migrations/0001_create_products.sql"))
                .unwrap();
            Some(product(&db))
        } else {
            None
        };
        if initial == 2 {
            db.execute_batch(include_str!("../migrations/0002_create_product_images.sql"))
                .unwrap();
            db.execute("INSERT INTO product_images VALUES ('image', ?1, '写真.png', 'media/originals/p/i.png', 'image/png', 9, 2, 3, ?2, 4, '2026-01-01T00:00:00Z')", params![before.as_ref().unwrap().id, "a".repeat(64)]).unwrap();
        }
        let images = if initial == 2 {
            image_repository::all(&db).unwrap()
        } else {
            vec![]
        };
        db.pragma_update(None, "user_version", initial).unwrap();
        migrations::apply(&mut db).unwrap();
        assert_eq!(version(&db), 3);
        if let Some(before) = before {
            assert_eq!(product_service::get(&db, &before.id).unwrap(), before);
        }
        assert_eq!(image_repository::all(&db).unwrap(), images);
        assert!(service::list(&db).unwrap().is_empty());
        assert!(
            !db.prepare("PRAGMA foreign_key_check")
                .unwrap()
                .exists([])
                .unwrap()
        );
        assert_eq!(
            db.query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
                .unwrap(),
            "ok"
        );
    }
}

#[test]
fn migration_three_failure_rolls_back_all_intermediate_schema_and_existing_data() {
    for initial in 0..=2 {
        let temp = Temp::new();
        let mut db = Connection::open(temp.0.join("test.sqlite3")).unwrap();
        let before = if initial >= 1 {
            db.execute_batch(include_str!("../migrations/0001_create_products.sql"))
                .unwrap();
            Some(product(&db))
        } else {
            None
        };
        if initial == 2 {
            db.execute_batch(include_str!("../migrations/0002_create_product_images.sql"))
                .unwrap();
        }
        db.execute_batch("CREATE TABLE product_attribute_values (sentinel TEXT); INSERT INTO product_attribute_values VALUES ('keep');").unwrap();
        db.pragma_update(None, "user_version", initial).unwrap();
        let original_schema = schema(&db);
        assert!(migrations::apply(&mut db).is_err());
        assert_eq!(version(&db), initial);
        assert_eq!(schema(&db), original_schema);
        assert_eq!(
            db.query_row("SELECT sentinel FROM product_attribute_values", [], |r| {
                r.get::<_, String>(0)
            })
            .unwrap(),
            "keep"
        );
        if let Some(before) = before {
            assert_eq!(product_service::get(&db, &before.id).unwrap(), before);
        }
    }
}

#[test]
fn version_three_reapply_restart_and_future_version_rejection_preserve_data() {
    let temp = Temp::new();
    let path = temp.0.join("test.sqlite3");
    let (p, attribute, original_schema) = {
        let mut db = connection::open(&path).unwrap();
        let p = product(&db);
        let attribute = archive(
            &db,
            &service::create(&db, input("重量", "number", Some("g"))).unwrap(),
            true,
        )
        .unwrap();
        db.execute("INSERT INTO product_attribute_values VALUES (?1, ?2, '12345678901234567890.0123456789')", params![p.id, attribute.id]).unwrap();
        let original_schema = schema(&db);
        migrations::apply(&mut db).unwrap();
        assert_eq!(schema(&db), original_schema);
        (p, attribute, original_schema)
    };
    let mut db = connection::open(&path).unwrap();
    assert_eq!(version(&db), 3);
    assert_eq!(repository::get(&db, &attribute.id).unwrap(), attribute);
    assert_eq!(product_service::get(&db, &p.id).unwrap(), p);
    assert_eq!(
        db.query_row("SELECT value FROM product_attribute_values", [], |r| r
            .get::<_, String>(
            0
        ))
        .unwrap(),
        "12345678901234567890.0123456789"
    );
    for newer in [4, 99] {
        db.pragma_update(None, "user_version", newer).unwrap();
        assert!(migrations::apply(&mut db).is_err());
        assert_eq!(version(&db), newer);
        assert_eq!(schema(&db), original_schema);
        assert_eq!(repository::get(&db, &attribute.id).unwrap(), attribute);
        assert_eq!(product_service::get(&db, &p.id).unwrap(), p);
    }
}

#[test]
fn create_text_number_optional_units_unicode_and_binary_names() {
    let temp = Temp::new();
    let db = connection::open(&temp.0.join("test.sqlite3")).unwrap();
    let text = service::create(&db, input("　素材　", "text", None)).unwrap();
    assert_eq!(text.name, "素材");
    assert_eq!(text.data_type, AttributeDataType::Text);
    assert_eq!(text.unit, None);
    assert_eq!(text.archived_at, None);
    assert_eq!(
        uuid::Uuid::parse_str(&text.id).unwrap().get_version_num(),
        4
    );
    assert_eq!(text.created_at, text.updated_at);
    assert!(text.created_at.ends_with('Z'));
    assert!(chrono::DateTime::parse_from_rfc3339(&text.created_at).is_ok());
    for (name, unit, expected) in [
        ("重量", Some(" g "), Some("g")),
        ("容量", None, None),
        ("長さ", Some("　 "), None),
    ] {
        let a = service::create(&db, input(name, "number", unit)).unwrap();
        assert_eq!(a.data_type, AttributeDataType::Number);
        assert_eq!(a.unit.as_deref(), expected);
    }
    for name in ["Color", "color", "Ａ", "A", "é", "e\u{301}"] {
        service::create(&db, input(name, "text", None)).unwrap();
    }
    assert!(service::create(&db, input(&"😀".repeat(80), "text", None)).is_ok());
    assert!(service::create(&db, input("単位上限", "number", Some(&"😀".repeat(20)))).is_ok());
}

#[test]
fn reject_invalid_names_units_types_and_text_units_without_creating_rows() {
    let temp = Temp::new();
    let db = connection::open(&temp.0.join("test.sqlite3")).unwrap();
    for name in [
        "",
        "　 ",
        "\t\n",
        "名\0前",
        "a\u{7f}",
        "a\u{85}",
        "a\u{2028}",
        "a\u{2029}",
        "\n名前",
        "名前\r",
        &"😀".repeat(81),
    ] {
        assert_eq!(
            service::create(&db, input(name, "text", None)),
            Err(AppError::InvalidAttributeDefinition)
        );
    }
    for unit in ["\n", "g\t", "\0", "a\u{2028}", &"😀".repeat(21)] {
        assert_eq!(
            service::create(&db, input("重量", "number", Some(unit))),
            Err(AppError::InvalidAttributeDefinition)
        );
    }
    for data_type in ["", "Text", "NUMBER", "boolean"] {
        assert_eq!(
            service::create(&db, input("名前", data_type, None)),
            Err(AppError::InvalidAttributeDefinition)
        );
    }
    for unit in ["g", "", " "] {
        assert_eq!(
            service::create(&db, input("名前", "text", Some(unit))),
            Err(AppError::InvalidAttributeDefinition)
        );
    }
    assert!(service::list(&db).unwrap().is_empty());
}

#[test]
fn rename_duplicates_archive_restore_and_stale_updates_preserve_identity_and_values() {
    let temp = Temp::new();
    let db = connection::open(&temp.0.join("test.sqlite3")).unwrap();
    let original = service::create(&db, input("重量", "number", Some("g"))).unwrap();
    let other = service::create(&db, input("素材", "text", None)).unwrap();
    assert_eq!(
        service::create(&db, input(" 重量 ", "text", None)),
        Err(AppError::DuplicateAttributeName)
    );
    assert_eq!(
        rename(&db, &original, "素材"),
        Err(AppError::DuplicateAttributeName)
    );
    assert_eq!(
        rename(&db, &original, "\n"),
        Err(AppError::InvalidAttributeDefinition)
    );
    assert_eq!(repository::get(&db, &original.id).unwrap(), original);
    let p = product(&db);
    db.execute(
        "INSERT INTO product_attribute_values VALUES (?1, ?2, '1.234567890123456789')",
        params![p.id, original.id],
    )
    .unwrap();
    let updated = rename(&db, &original, "　質量　").unwrap();
    assert_eq!(
        updated,
        AttributeDefinition {
            name: "質量".into(),
            updated_at: updated.updated_at.clone(),
            ..original.clone()
        }
    );
    assert_ne!(updated.updated_at, original.updated_at);
    assert_eq!(rename(&db, &original, "古い入力"), Err(AppError::StaleData));
    assert_eq!(archive(&db, &original, true), Err(AppError::StaleData));
    let archived = archive(&db, &updated, true).unwrap();
    assert_eq!(archived.archived_at.as_ref(), Some(&archived.updated_at));
    assert_eq!(
        service::list(&db).unwrap(),
        vec![other.clone(), archived.clone()]
    );
    assert_eq!(
        service::create(&db, input("質量", "text", None)),
        Err(AppError::DuplicateAttributeName)
    );
    assert_eq!(
        rename(&db, &other, "質量"),
        Err(AppError::DuplicateAttributeName)
    );
    assert_eq!(archive(&db, &updated, false), Err(AppError::StaleData));
    let restored = archive(&db, &archived, false).unwrap();
    assert_eq!(restored.archived_at, None);
    assert_eq!(restored.id, original.id);
    assert_eq!(restored.created_at, original.created_at);
    assert_eq!(restored.unit, original.unit);
    assert_eq!(restored.data_type, original.data_type);
    assert_ne!(restored.updated_at, archived.updated_at);
    assert_eq!(
        db.query_row("SELECT value FROM product_attribute_values", [], |r| r
            .get::<_, String>(
            0
        ))
        .unwrap(),
        "1.234567890123456789"
    );
    assert_eq!(product_service::get(&db, &p.id).unwrap(), p);
    let missing = AttributeDefinition {
        id: uuid::Uuid::new_v4().to_string(),
        ..restored
    };
    assert_eq!(
        rename(&db, &missing, "不在"),
        Err(AppError::AttributeNotFound)
    );
    assert_eq!(
        archive(&db, &missing, true),
        Err(AppError::AttributeNotFound)
    );
}

#[test]
fn conditional_repository_updates_reject_changes_after_service_read() {
    let temp = Temp::new();
    let db = connection::open(&temp.0.join("test.sqlite3")).unwrap();
    let stale = service::create(&db, input("カラー", "text", None)).unwrap();
    let latest = rename(&db, &stale, "色").unwrap();
    assert_eq!(
        repository::rename(&db, &stale, &stale.updated_at),
        Err(AppError::StaleData)
    );
    assert_eq!(
        repository::set_archived(&db, &stale, &stale.updated_at),
        Err(AppError::StaleData)
    );
    assert_eq!(repository::get(&db, &stale.id).unwrap(), latest);
    db.execute(
        "DELETE FROM attribute_definitions WHERE id = ?1",
        [&stale.id],
    )
    .unwrap();
    assert_eq!(
        repository::rename(&db, &stale, &stale.updated_at),
        Err(AppError::AttributeNotFound)
    );
    assert_eq!(
        repository::set_archived(&db, &stale, &stale.updated_at),
        Err(AppError::AttributeNotFound)
    );
}

#[test]
fn timestamp_advances_at_same_time_and_backwards_clock_for_all_operations() {
    let previous = "2099-01-01T00:00:00.000000000Z";
    for now in [previous, "2020-01-01T00:00:00Z"] {
        assert_eq!(
            service::next_timestamp(
                previous,
                chrono::DateTime::parse_from_rfc3339(now)
                    .unwrap()
                    .with_timezone(&chrono::Utc)
            )
            .unwrap(),
            "2099-01-01T00:00:00.000000001Z"
        );
    }
    let temp = Temp::new();
    let db = connection::open(&temp.0.join("test.sqlite3")).unwrap();
    let mut a = service::create(&db, input("色", "text", None)).unwrap();
    db.execute(
        "UPDATE attribute_definitions SET updated_at = ?1",
        [previous],
    )
    .unwrap();
    a.updated_at = previous.into();
    let a = rename(&db, &a, "カラー").unwrap();
    assert!(a.updated_at.ends_with("000000001Z"));
    let a = archive(&db, &a, true).unwrap();
    assert!(a.updated_at.ends_with("000000002Z"));
    let a = archive(&db, &a, false).unwrap();
    assert!(a.updated_at.ends_with("000000003Z"));
}

#[test]
fn list_order_uses_archive_state_created_at_and_uuid_tiebreaker() {
    let temp = Temp::new();
    let db = connection::open(&temp.0.join("test.sqlite3")).unwrap();
    let mut attrs = (0..4)
        .map(|i| service::create(&db, input(&format!("項目{i}"), "text", None)).unwrap())
        .collect::<Vec<_>>();
    db.execute("UPDATE attribute_definitions SET created_at = 'same'", [])
        .unwrap();
    archive(&db, &attrs[0], true).unwrap();
    let archived = attrs.remove(0);
    attrs.sort_by(|a, b| a.id.cmp(&b.id));
    attrs.push(archived);
    assert_eq!(
        service::list(&db)
            .unwrap()
            .iter()
            .map(|a| &a.id)
            .collect::<Vec<_>>(),
        attrs.iter().map(|a| &a.id).collect::<Vec<_>>()
    );
}

#[test]
fn database_constraints_reject_invalid_definitions_orphan_duplicate_and_blank_values() {
    let temp = Temp::new();
    let db = connection::open(&temp.0.join("test.sqlite3")).unwrap();
    let p = product(&db);
    let a = service::create(&db, input("色", "text", None)).unwrap();
    for (name, data_type, unit) in [
        ("", "text", None),
        ("　", "text", None),
        ("型", "bool", None),
        ("単位", "text", Some("g")),
    ] {
        assert!(db.execute("INSERT INTO attribute_definitions VALUES ('invalid', ?1, ?2, ?3, NULL, 'utc', 'utc')", params![name, data_type, unit]).is_err());
    }
    assert!(db.execute("INSERT INTO attribute_definitions VALUES ('duplicate', '色', 'text', NULL, NULL, 'utc', 'utc')", []).is_err());
    for (pid, aid, value) in [
        ("missing", a.id.as_str(), "赤"),
        (p.id.as_str(), "missing", "赤"),
        (&p.id, &a.id, ""),
        (&p.id, &a.id, " \t\r\n　\u{a0}\u{2003}"),
    ] {
        assert!(
            db.execute(
                "INSERT INTO product_attribute_values VALUES (?1, ?2, ?3)",
                params![pid, aid, value]
            )
            .is_err()
        );
    }
    db.execute(
        "INSERT INTO product_attribute_values VALUES (?1, ?2, '赤')",
        params![p.id, a.id],
    )
    .unwrap();
    assert!(
        db.execute(
            "INSERT INTO product_attribute_values VALUES (?1, ?2, '青')",
            params![p.id, a.id]
        )
        .is_err()
    );
    assert!(
        db.execute("DELETE FROM attribute_definitions WHERE id = ?1", [&a.id])
            .is_err()
    );
    assert!(
        db.execute("DELETE FROM products WHERE id = ?1", [&p.id])
            .is_err()
    );
    assert_eq!(repository::get(&db, &a.id).unwrap(), a);
    assert_eq!(product_service::get(&db, &p.id).unwrap(), p);
}
