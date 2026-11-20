use crate::{
    domain::product::{ListingTextInput, ProductInput},
    error::AppError,
    infrastructure::db::{connection, migrations},
    services::product_service as products,
};
use rusqlite::{Connection, types::Value};
use serde_json::{Value as Json, json};

fn input(extra: Json) -> ProductInput {
    let mut value = json!({"sku":"SKU", "name":"管理名", "description_note":"内部専用"});
    value
        .as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    serde_json::from_value(value).unwrap()
}
fn database() -> Connection {
    let mut db = Connection::open_in_memory().unwrap();
    db.pragma_update(None, "foreign_keys", true).unwrap();
    migrations::apply(&mut db).unwrap();
    db
}
fn rows(db: &Connection, sql: &str) -> Vec<Vec<Value>> {
    let mut stmt = db.prepare(sql).unwrap();
    let count = stmt.column_count();
    stmt.query_map([], |r| (0..count).map(|i| r.get(i)).collect())
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap()
}
fn legacy(db: &Connection, version: i64) {
    if version >= 1 {
        db.execute_batch(include_str!("../migrations/0001_create_products.sql"))
            .unwrap();
        db.execute_batch("INSERT INTO products VALUES ('p','SKU','管理名','ブランド',123,'内部専用','created','updated');").unwrap();
    }
    if version >= 2 {
        db.execute_batch(include_str!("../migrations/0002_create_product_images.sql"))
            .unwrap();
        db.execute("INSERT INTO product_images VALUES ('i','p','写真.png','media/originals/p/i.png','image/png',12,2,3,?1,0,'created')", ["a".repeat(64)]).unwrap();
    }
    if version >= 3 {
        db.execute_batch(include_str!(
            "../migrations/0003_create_custom_attributes.sql"
        ))
        .unwrap();
        db.execute_batch("INSERT INTO attribute_definitions VALUES ('a','素材','text',NULL,NULL,'created','updated'); INSERT INTO product_attribute_values VALUES ('p','a','陶器');").unwrap();
    }
    db.pragma_update(None, "user_version", version).unwrap();
}

#[test]
fn all_old_versions_migrate_preserving_all_columns_and_related_rows() {
    for version in 0..=3 {
        let mut db = Connection::open_in_memory().unwrap();
        db.pragma_update(None, "foreign_keys", true).unwrap();
        legacy(&db, version);
        let tables: Vec<_> = [
            "products",
            "product_images",
            "attribute_definitions",
            "product_attribute_values",
        ]
        .into_iter()
        .take(match version {
            0 => 0,
            1 => 1,
            2 => 2,
            _ => 4,
        })
        .collect();
        let before: Vec<_> = tables
            .iter()
            .map(|t| rows(&db, &format!("SELECT * FROM {t}")))
            .collect();
        migrations::apply(&mut db).unwrap();
        assert_eq!(
            rows(&db, "PRAGMA user_version"),
            vec![vec![Value::Integer(6)]]
        );
        for (table, original) in tables.iter().zip(before) {
            let after = rows(&db, &format!("SELECT * FROM {table}"));
            if *table == "products" {
                assert_eq!(&after[0][..8], original[0]);
                assert_eq!(&after[0][8..10], &[Value::Null, Value::Null]);
                assert_eq!(
                    after[0][10],
                    Value::Text(crate::domain::attribute_group::DEFAULT_GROUP_ID.into())
                );
            } else if *table == "attribute_definitions" {
                assert_eq!(&after[0][..7], original[0]);
                assert_eq!(
                    after[0][7],
                    Value::Text(crate::domain::attribute_group::DEFAULT_GROUP_ID.into())
                );
            } else {
                assert_eq!(after, original);
            }
        }
        let before = rows(&db, "SELECT * FROM products");
        migrations::apply(&mut db).unwrap();
        assert_eq!(rows(&db, "SELECT * FROM products"), before);
        assert!(rows(&db, "PRAGMA foreign_key_check").is_empty());
        assert_eq!(
            rows(&db, "PRAGMA integrity_check"),
            vec![vec![Value::Text("ok".into())]]
        );
    }
}

#[test]
fn second_alter_failure_rolls_back_first_alter_and_every_intermediate_migration() {
    for version in 1..=3 {
        let mut db = Connection::open_in_memory().unwrap();
        legacy(&db, version);
        db.execute_batch("ALTER TABLE products ADD COLUMN listing_description TEXT; UPDATE products SET listing_description = '保持';").unwrap();
        let schema = rows(&db, "SELECT name, sql FROM sqlite_master ORDER BY name");
        let before = rows(&db, "SELECT * FROM products");
        assert!(migrations::apply(&mut db).is_err());
        assert_eq!(
            rows(&db, "PRAGMA user_version"),
            vec![vec![Value::Integer(version)]]
        );
        assert_eq!(rows(&db, "SELECT * FROM products"), before);
        assert_eq!(
            rows(&db, "SELECT name, sql FROM sqlite_master ORDER BY name"),
            schema
        );
    }
}

#[test]
fn serde_distinguishes_omission_null_and_string_and_rejects_other_types() {
    assert_eq!(input(json!({})).listing_title, ListingTextInput::Omitted);
    assert_eq!(
        input(json!({"listing_title":null})).listing_title,
        ListingTextInput::Provided(None)
    );
    assert_eq!(
        input(json!({"listing_description":"文章"})).listing_description,
        ListingTextInput::Provided(Some("文章".into()))
    );
    for field in ["listing_title", "listing_description"] {
        for value in [json!(1), json!(true), json!([]), json!({})] {
            let mut raw = json!({"sku":"s", "name":"n"});
            raw[field] = value;
            assert!(serde_json::from_value::<ProductInput>(raw).is_err());
        }
    }
}

#[test]
fn title_unicode_boundary_trim_and_no_unicode_conversion() {
    let db = database();
    let title = format!("  {}　", "😀".repeat(200));
    let p = products::create(&db, input(json!({"listing_title":title}))).unwrap();
    assert_eq!(p.listing_title, Some("😀".repeat(200)));
    assert_eq!(p.listing_description, None);
    let p = products::update(
        &db,
        &p.id,
        input(json!({"listing_title":"　Ａe\u{0301}　"})),
        &p.updated_at,
    )
    .unwrap();
    assert_eq!(p.listing_title.as_deref(), Some("Ａe\u{0301}"));
}

#[test]
fn title_rejects_controls_before_trim_and_all_newline_forms() {
    for title in [
        "😀".repeat(201),
        "\na".into(),
        "a\r\n".into(),
        "a\t".into(),
        "\0".into(),
        "a\u{85}".into(),
        "a\u{2028}".into(),
        "\u{2029}a".into(),
        "a\u{7f}".into(),
    ] {
        assert_eq!(
            input(json!({"listing_title":title}))
                .validate()
                .unwrap_err(),
            AppError::InvalidListingTitle
        );
    }
}

#[test]
fn description_normalizes_crlf_before_counting_and_preserves_plain_text_and_tabs() {
    let db = database();
    let text = format!(" \r\n{}\r\n\t<b>&文章</b>\r\n ", "😀".repeat(9_987));
    let p = products::create(&db, input(json!({"listing_description":text}))).unwrap();
    assert_eq!(
        p.listing_description,
        Some(format!("{}\n\t<b>&文章</b>", "😀".repeat(9_987)))
    );
    let value = "😀".repeat(10_000);
    let p = products::update(
        &db,
        &p.id,
        input(json!({"listing_description":value})),
        &p.updated_at,
    )
    .unwrap();
    assert_eq!(p.listing_description.unwrap().chars().count(), 10_000);
}

#[test]
fn description_rejects_excess_length_and_invalid_controls_before_trim() {
    for text in [
        "😀".repeat(10_001),
        "\0text".into(),
        "text\u{7f}".into(),
        "\u{85}".into(),
        "text\r".into(),
        "\u{b}text".into(),
    ] {
        assert_eq!(
            input(json!({"listing_description":text}))
                .validate()
                .unwrap_err(),
            AppError::InvalidListingDescription
        );
    }
}

#[test]
fn old_calls_preserve_each_field_and_explicit_null_or_blank_only_clears_its_field() {
    let db = database();
    let old = products::create(&db, input(json!({}))).unwrap();
    assert_eq!(old.listing_title, None);
    assert_eq!(old.listing_description, None); // Internal note is never copied.
    let mut p = products::update(
        &db,
        &old.id,
        input(json!({"listing_title":"公開名", "listing_description":"説明"})),
        &old.updated_at,
    )
    .unwrap();
    for extra in [
        json!({}),
        json!({"name":"管理名変更", "description_note":"内部変更"}),
        json!({"listing_title":"公開名"}),
        json!({"listing_description":"説明"}),
    ] {
        let next = products::update(&db, &p.id, input(extra), &p.updated_at).unwrap();
        assert_eq!(next.listing_title.as_deref(), Some("公開名"));
        assert_eq!(next.listing_description.as_deref(), Some("説明"));
        assert!(next.updated_at > p.updated_at);
        p = next;
    }
    for blank in [Json::Null, json!(""), json!("　 ")] {
        let cleared = products::update(
            &db,
            &p.id,
            input(json!({"listing_title":blank})),
            &p.updated_at,
        )
        .unwrap();
        assert_eq!(cleared.listing_title, None);
        assert_eq!(cleared.listing_description, p.listing_description);
        let cleared = products::update(
            &db,
            &cleared.id,
            input(json!({"listing_description":blank})),
            &cleared.updated_at,
        )
        .unwrap();
        assert_eq!(cleared.listing_description, None);
        p = products::update(
            &db,
            &cleared.id,
            input(json!({"listing_title":"公開名", "listing_description":"説明"})),
            &cleared.updated_at,
        )
        .unwrap();
    }
    let p = products::create(
        &db,
        input(json!({"sku":"blank", "listing_description":" \r\n\t　", "listing_title":"　 "})),
    )
    .unwrap();
    assert_eq!(p.listing_title, None);
    assert_eq!(p.listing_description, None);
}

#[test]
fn stale_or_invalid_save_preserves_all_product_fields() {
    let db = database();
    let p = products::create(
        &db,
        input(json!({"listing_title":"公開名", "listing_description":"説明"})),
    )
    .unwrap();
    let latest = products::update(
        &db,
        &p.id,
        input(json!({"listing_description":"最新説明"})),
        &p.updated_at,
    )
    .unwrap();
    assert_eq!(
        products::update(
            &db,
            &p.id,
            input(json!({"listing_title":null, "listing_description":"古い入力"})),
            &p.updated_at
        ),
        Err(AppError::StaleData)
    );
    assert_eq!(
        products::update(
            &db,
            &p.id,
            input(json!({"listing_title":"不正\n"})),
            &latest.updated_at
        ),
        Err(AppError::InvalidListingTitle)
    );
    assert_eq!(products::get(&db, &p.id).unwrap(), latest);
}

#[test]
fn file_restart_and_future_schema_rejection_preserve_listing_content() {
    let path = std::env::temp_dir().join(format!(
        "listing-pilot-listing-{}.sqlite3",
        uuid::Uuid::new_v4()
    ));
    let p = {
        let db = connection::open(&path).unwrap();
        products::create(
            &db,
            input(json!({"listing_title":"公開名", "listing_description":"一行目\r\n二行目"})),
        )
        .unwrap()
    };
    {
        let mut db = connection::open(&path).unwrap();
        assert_eq!(products::get(&db, &p.id).unwrap(), p);
        assert_eq!(products::list(&db).unwrap(), vec![p.clone()]);
        assert_eq!(products::edit_snapshot(&db, &p.id).unwrap().product, p);
        for version in [7, 99] {
            db.pragma_update(None, "user_version", version).unwrap();
            assert!(migrations::apply(&mut db).is_err());
            assert_eq!(products::get(&db, &p.id).unwrap(), p);
        }
    }
    std::fs::remove_file(path).unwrap();
}
