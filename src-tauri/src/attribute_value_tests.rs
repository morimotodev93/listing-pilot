use crate::{
    domain::{attribute::*, attribute_value::*, product::*},
    error::AppError,
    infrastructure::db::{attribute_value_repository as values, connection},
    services::{attribute_service as attributes, product_service as products},
};
use rusqlite::Connection;

fn db() -> Connection {
    let mut db = Connection::open_in_memory().unwrap();
    db.pragma_update(None, "foreign_keys", true).unwrap();
    crate::infrastructure::db::migrations::apply(&mut db).unwrap();
    db
}
fn input(sku: &str) -> ProductInput {
    ProductInput {
        sku: sku.into(),
        name: "質量".into(),
        brand: None,
        price_yen: None,
        description_note: None,
    }
}
fn definition(db: &Connection, name: &str, kind: &str) -> AttributeDefinition {
    attributes::create(
        db,
        CreateAttributeInput {
            name: name.into(),
            data_type: kind.into(),
            unit: None,
        },
    )
    .unwrap()
}
fn payload(items: &[(&AttributeDefinition, &str)]) -> AttributeValuesInput {
    AttributeValuesInput {
        editable_definitions: items
            .iter()
            .map(|(a, _)| EditableDefinition {
                attribute_id: a.id.clone(),
                expected_updated_at: a.updated_at.clone(),
            })
            .collect(),
        values: items
            .iter()
            .map(|(a, v)| ProductAttributeValue {
                attribute_id: a.id.clone(),
                value: (*v).into(),
            })
            .collect(),
    }
}
fn update(
    db: &Connection,
    p: &Product,
    v: Option<AttributeValuesInput>,
) -> Result<Product, AppError> {
    products::update_with_values(db, &p.id, input(&p.sku), &p.updated_at, v)
}

#[test]
fn text_unicode_limits_trim_controls_and_unset() {
    for (v, expected) in [
        (" 00123　", Some("00123")),
        ("　 ", None),
        ("", None),
        ("陶器😀", Some("陶器😀")),
    ] {
        assert_eq!(
            normalize(v, AttributeDataType::Text).unwrap().as_deref(),
            expected
        );
    }
    assert!(normalize(&"😀".repeat(2000), AttributeDataType::Text).is_ok());
    for v in [
        "😀".repeat(2001),
        "a\n".into(),
        "\tb".into(),
        "a\0".into(),
        "a\u{2028}".into(),
        "a\u{85}".into(),
    ] {
        assert_eq!(
            normalize(&v, AttributeDataType::Text),
            Err(AppError::InvalidAttributeValue)
        );
    }
}
#[test]
fn decimal_grammar_boundaries_precision_and_leading_zeroes() {
    for v in [
        "0",
        "-0",
        "00123",
        "-0001.000001",
        "999999999999999.999999",
        "-999999999999999.999999",
        "1.0",
    ] {
        assert_eq!(
            normalize(&format!("　{v} "), AttributeDataType::Number)
                .unwrap()
                .as_deref(),
            Some(v)
        );
    }
    for v in [
        "1000000000000000",
        "1.1234567",
        "1.",
        ".1",
        "+1",
        "1e2",
        "NaN",
        "Infinity",
        "1,000",
        "１",
        "--1",
        "1.2.3",
        "-",
        "1\n",
    ] {
        assert_eq!(
            normalize(v, AttributeDataType::Number),
            Err(AppError::InvalidAttributeValue),
            "{v}"
        );
    }
    assert_eq!(normalize("　", AttributeDataType::Number).unwrap(), None);
}
#[test]
fn atomic_create_update_clear_partial_and_legacy_preserve_values() {
    let db = db();
    let a = definition(&db, "カラー", "text");
    let b = definition(&db, "重量", "number");
    let p = products::create_with_values(
        &db,
        input("a"),
        Some(payload(&[(&a, " 00123 "), (&b, "0")])),
    )
    .unwrap();
    let snap = products::edit_snapshot(&db, &p.id).unwrap();
    assert_eq!(snap.product, p);
    assert_eq!(snap.definitions.len(), 2);
    assert_eq!(snap.values.len(), 2);
    let p2 = update(&db, &p, Some(payload(&[(&a, "　"), (&b, "0001.000001")]))).unwrap();
    assert_ne!(p.updated_at, p2.updated_at);
    let p3 = update(&db, &p2, None).unwrap();
    let before = values::list(&db, &p.id).unwrap();
    let p4 = update(&db, &p3, Some(payload(&[]))).unwrap();
    assert_eq!(values::list(&db, &p.id).unwrap(), before);
    let p5 = update(&db, &p4, Some(payload(&[(&a, "　")]))).unwrap();
    let left = values::list(&db, &p.id).unwrap();
    assert_eq!(
        left,
        vec![ProductAttributeValue {
            attribute_id: b.id.clone(),
            value: "0001.000001".into()
        }]
    );
    assert_eq!(
        update(&db, &p4, Some(payload(&[(&b, "9")]))),
        Err(AppError::StaleData)
    );
    assert_eq!(products::get(&db, &p.id).unwrap(), p5);
    assert_eq!(values::list(&db, &p.id).unwrap(), left);
    let no_values = AttributeValuesInput {
        editable_definitions: payload(&[(&b, "")]).editable_definitions,
        values: vec![],
    };
    update(&db, &p5, Some(no_values)).unwrap();
    assert!(values::list(&db, &p.id).unwrap().is_empty());
}
#[test]
fn archived_rename_restore_and_definition_conflicts() {
    let db = db();
    let a = definition(&db, "素材", "text");
    let b = definition(&db, "重量", "number");
    let p =
        products::create_with_values(&db, input("a"), Some(payload(&[(&a, "陶器"), (&b, "350")])))
            .unwrap();
    let archived = attributes::set_archived(
        &db,
        SetAttributeArchivedInput {
            id: a.id.clone(),
            archived: true,
            expected_updated_at: a.updated_at.clone(),
        },
    )
    .unwrap();
    assert_eq!(
        update(&db, &p, Some(payload(&[(&a, "陶器")]))),
        Err(AppError::StaleData)
    );
    assert_eq!(
        update(&db, &p, Some(payload(&[(&archived, "陶器")]))),
        Err(AppError::StaleData)
    );
    let p = update(&db, &p, Some(payload(&[(&b, "0")]))).unwrap();
    assert!(
        products::edit_snapshot(&db, &p.id)
            .unwrap()
            .values
            .iter()
            .any(|v| v.value == "陶器")
    );
    let renamed = attributes::update(
        &db,
        UpdateAttributeInput {
            id: b.id.clone(),
            name: "質量".into(),
            expected_updated_at: b.updated_at.clone(),
        },
    )
    .unwrap();
    assert_eq!(
        update(&db, &p, Some(payload(&[(&b, "1")]))),
        Err(AppError::StaleData)
    );
    assert_eq!(
        products::create_with_values(&db, input("stale"), Some(payload(&[(&b, "1")]))),
        Err(AppError::StaleData)
    );
    assert_eq!(products::list(&db).unwrap().len(), 1);
    let restored = attributes::set_archived(
        &db,
        SetAttributeArchivedInput {
            id: a.id.clone(),
            archived: false,
            expected_updated_at: archived.updated_at,
        },
    )
    .unwrap();
    assert!(
        values::list(&db, &p.id)
            .unwrap()
            .iter()
            .any(|v| v.attribute_id == restored.id && v.value == "陶器")
    );
    update(&db, &p, Some(payload(&[(&renamed, "10")]))).unwrap();
}
#[test]
fn invalid_duplicate_missing_and_unmatched_ids_roll_back_product() {
    let db = db();
    let a = definition(&db, "a", "text");
    let p = products::create(&db, input("a")).unwrap();
    let mut duplicate_values = payload(&[(&a, "x")]);
    duplicate_values
        .values
        .push(duplicate_values.values[0].clone());
    let unmatched = AttributeValuesInput {
        editable_definitions: vec![],
        values: payload(&[(&a, "x")]).values,
    };
    let missing = AttributeDefinition {
        id: uuid::Uuid::new_v4().to_string(),
        ..a.clone()
    };
    for v in [
        payload(&[(&a, "x"), (&a, "y")]),
        duplicate_values,
        unmatched,
        payload(&[(&missing, "x")]),
        payload(&[(&a, "x\n")]),
    ] {
        assert!(update(&db, &p, Some(v)).is_err());
        assert_eq!(products::get(&db, &p.id).unwrap(), p);
        assert!(values::list(&db, &p.id).unwrap().is_empty());
    }
}
#[test]
fn sql_failure_after_first_value_rolls_back_create_and_update() {
    let db = db();
    let a = definition(&db, "a", "text");
    let b = definition(&db, "b", "number");
    let p =
        products::create_with_values(&db, input("a"), Some(payload(&[(&a, "keep"), (&b, "0")])))
            .unwrap();
    db.execute_batch(&format!("CREATE TRIGGER fail_value BEFORE INSERT ON product_attribute_values WHEN NEW.attribute_id='{}' BEGIN SELECT RAISE(ABORT,'injected'); END",b.id)).unwrap();
    let before = values::list(&db, &p.id).unwrap();
    assert_eq!(
        update(&db, &p, Some(payload(&[(&a, "changed"), (&b, "1")]))),
        Err(AppError::DatabaseOperation)
    );
    assert_eq!(products::get(&db, &p.id).unwrap(), p);
    assert_eq!(values::list(&db, &p.id).unwrap(), before);
    assert_eq!(
        products::create_with_values(&db, input("fail"), Some(payload(&[(&a, "new"), (&b, "1")]))),
        Err(AppError::DatabaseOperation)
    );
    assert_eq!(products::list(&db).unwrap(), vec![p]);
}
#[test]
fn restart_preserves_decimal_text_and_schema_three() {
    let path = std::env::temp_dir().join(format!(
        "listing-pilot-values-{}.sqlite3",
        uuid::Uuid::new_v4()
    ));
    let id = {
        let db = connection::open(&path).unwrap();
        let a = definition(&db, "n", "number");
        let b = definition(&db, "t", "text");
        products::create_with_values(
            &db,
            input("a"),
            Some(payload(&[(&a, "000123456789012.123456"), (&b, "00123")])),
        )
        .unwrap()
        .id
    };
    {
        let db = connection::open(&path).unwrap();
        let got = values::list(&db, &id).unwrap();
        assert!(got.iter().any(|v| v.value == "000123456789012.123456"));
        assert!(got.iter().any(|v| v.value == "00123"));
        assert_eq!(
            db.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
                .unwrap(),
            3
        );
    }
    std::fs::remove_file(path).unwrap();
}
