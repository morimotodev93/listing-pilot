use crate::infrastructure::db::migrations;
use crate::{
    domain::{attribute::*, attribute_group::*, attribute_value::*, product::*},
    error::AppError,
    infrastructure::db::{
        attribute_group_repository as groups_db, attribute_repository as definitions_db,
        attribute_value_repository as values_db,
    },
    services::{
        attribute_group_service as groups, attribute_service as definitions,
        product_service as products,
    },
};
use rusqlite::{Connection, types::Value};
use serde_json::json;

const DEFAULT: &str = "7a8f0c3e-4d5b-4c6a-9e20-1234567890ab";

fn rows(db: &Connection, sql: &str) -> Vec<Vec<Value>> {
    let mut statement = db.prepare(sql).unwrap();
    let count = statement.column_count();
    statement
        .query_map([], |r| (0..count).map(|i| r.get(i)).collect())
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap()
}

fn legacy(version: i64) -> Connection {
    let db = Connection::open_in_memory().unwrap();
    db.pragma_update(None, "foreign_keys", true).unwrap();
    let sql = [
        include_str!("../migrations/0001_create_products.sql"),
        include_str!("../migrations/0002_create_product_images.sql"),
        include_str!("../migrations/0003_create_custom_attributes.sql"),
        include_str!("../migrations/0004_add_listing_content.sql"),
    ];
    for migration in sql.iter().take(version as usize) {
        db.execute_batch(migration).unwrap();
    }
    if version >= 1 {
        db.execute_batch("INSERT INTO products(id,sku,name,brand,price_yen,description_note,created_at,updated_at) VALUES ('p','SKU','商品','ブランド',123,'内部メモ','created','updated'),('q','SKU2','別商品',NULL,0,NULL,'c','u');").unwrap();
    }
    if version >= 2 {
        db.execute("INSERT INTO product_images VALUES ('i','p','写真.png','media/originals/p/i.png','image/png',12,2,3,?1,0,'created')", ["a".repeat(64)]).unwrap();
    }
    if version >= 3 {
        db.execute_batch("INSERT INTO attribute_definitions VALUES ('a','素材','text',NULL,NULL,'c','u'),('b','重量','number','g','archived','c2','u2'); INSERT INTO product_attribute_values VALUES ('p','a','陶器'),('p','b','000123.450000'),('q','a','木'),('q','b','-1.000001');").unwrap();
    }
    if version >= 4 {
        db.execute_batch(
            "UPDATE products SET listing_title='公開名',listing_description='公開説明';",
        )
        .unwrap();
    }
    db.pragma_update(None, "user_version", version).unwrap();
    db
}

#[test]
fn schema_v5_migrates_all_versions_preserving_every_column_and_foreign_key() {
    for version in 0..=4 {
        let mut db = legacy(version);
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
            .map(|t| rows(&db, &format!("SELECT * FROM {t} ORDER BY 1,2")))
            .collect();
        migrations::apply(&mut db).unwrap();
        assert_eq!(
            rows(&db, "PRAGMA user_version"),
            vec![vec![Value::Integer(6)]]
        );
        for (table, original) in tables.iter().zip(before) {
            let after = rows(&db, &format!("SELECT * FROM {table} ORDER BY 1,2"));
            assert_eq!(after.len(), original.len());
            for (new, old) in after.iter().zip(original) {
                assert_eq!(&new[..old.len()], old);
                if *table == "products" || *table == "attribute_definitions" {
                    assert_eq!(new.last(), Some(&Value::Text(DEFAULT.into())));
                }
            }
        }
        assert_eq!(
            rows(&db, "SELECT id,name,archived_at FROM attribute_groups"),
            vec![vec![
                Value::Text(DEFAULT.into()),
                Value::Text("既定".into()),
                Value::Null
            ]]
        );
        assert!(rows(&db, "PRAGMA foreign_key_check").is_empty());
        assert_eq!(
            rows(&db, "PRAGMA foreign_keys"),
            vec![vec![Value::Integer(1)]]
        );
        assert_eq!(
            rows(&db, "PRAGMA integrity_check"),
            vec![vec![Value::Text("ok".into())]]
        );
        let schema = rows(&db, "SELECT name,sql FROM sqlite_master ORDER BY name");
        let data = rows(&db, "SELECT * FROM attribute_groups");
        migrations::apply(&mut db).unwrap();
        assert_eq!(
            rows(&db, "SELECT name,sql FROM sqlite_master ORDER BY name"),
            schema
        );
        assert_eq!(rows(&db, "SELECT * FROM attribute_groups"), data);
        db.pragma_update(None, "user_version", 7).unwrap();
        assert!(migrations::apply(&mut db).is_err());
        assert_eq!(
            rows(&db, "PRAGMA user_version"),
            vec![vec![Value::Integer(7)]]
        );
        assert_eq!(
            rows(&db, "SELECT name,sql FROM sqlite_master ORDER BY name"),
            schema
        );
    }
}

#[test]
fn schema_v5_late_failure_rolls_back_every_step_and_all_data() {
    for version in 0..=4 {
        let mut db = legacy(version);
        // Fail at the final trigger, after both parent and child tables were replaced.
        db.execute_batch("CREATE TABLE sentinel(id TEXT); CREATE TRIGGER attribute_group_immutable AFTER INSERT ON sentinel BEGIN SELECT 1; END;").unwrap();
        let schema = rows(&db, "SELECT name,sql FROM sqlite_master ORDER BY name");
        let products = (version > 0).then(|| rows(&db, "SELECT * FROM products"));
        let values = (version >= 3).then(|| rows(&db, "SELECT * FROM product_attribute_values"));
        assert!(migrations::apply(&mut db).is_err());
        assert_eq!(
            rows(&db, "SELECT name,sql FROM sqlite_master ORDER BY name"),
            schema
        );
        assert_eq!(
            rows(&db, "PRAGMA user_version"),
            vec![vec![Value::Integer(version)]]
        );
        if let Some(before) = products {
            assert_eq!(rows(&db, "SELECT * FROM products"), before);
        }
        if let Some(before) = values {
            assert_eq!(rows(&db, "SELECT * FROM product_attribute_values"), before);
        }
        assert!(rows(&db, "PRAGMA foreign_key_check").is_empty());
    }
}

#[test]
fn schema_v5_enforces_group_references_and_scoped_names() {
    let mut db = legacy(4);
    migrations::apply(&mut db).unwrap();
    db.execute_batch("INSERT INTO attribute_groups VALUES('other','PC',NULL,'c','u'); INSERT INTO attribute_definitions VALUES('other-a','素材','text',NULL,NULL,'c','u','other');").unwrap();
    for sql in [
        "INSERT INTO attribute_definitions VALUES('dup','素材','text',NULL,'archived','c','u','other')",
        "INSERT INTO attribute_definitions VALUES('null','項目','text',NULL,NULL,'c','u',NULL)",
        "INSERT INTO attribute_definitions VALUES('missing','項目','text',NULL,NULL,'c','u','missing')",
        "UPDATE products SET group_id=NULL WHERE id='p'",
        "UPDATE products SET group_id='missing' WHERE id='p'",
        "INSERT INTO products(id,sku,name,created_at,updated_at) VALUES('new','new','新規','c','u')",
        "INSERT INTO products(id,sku,name,created_at,updated_at,group_id) VALUES('new','new','新規','c','u','missing')",
        "UPDATE attribute_definitions SET group_id='other' WHERE id='a'",
    ] {
        assert!(db.execute_batch(sql).is_err(), "{sql}");
    }
    assert!(rows(&db, "PRAGMA foreign_key_check").is_empty());
}

fn database() -> Connection {
    let mut db = legacy(0);
    migrations::apply(&mut db).unwrap();
    assert_eq!(DEFAULT, DEFAULT_GROUP_ID);
    db
}

fn group(db: &Connection, name: &str) -> AttributeGroup {
    groups::create(db, CreateAttributeGroupInput { name: name.into() }).unwrap()
}

fn attribute(db: &Connection, group: &str, name: &str) -> AttributeDefinition {
    definitions::create(
        db,
        CreateAttributeInput {
            group_id: Some(group.into()),
            name: name.into(),
            data_type: "text".into(),
            unit: None,
        },
    )
    .unwrap()
}

fn product_input(group_id: Option<&str>) -> ProductInput {
    serde_json::from_value(json!({"sku":"SKU","name":"商品","group_id":group_id})).unwrap()
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

fn archived(
    db: &Connection,
    g: &AttributeGroup,
    archived: bool,
) -> Result<AttributeGroup, AppError> {
    groups::set_archived(
        db,
        SetAttributeGroupArchivedInput {
            id: g.id.clone(),
            archived,
            expected_updated_at: g.updated_at.clone(),
        },
    )
}

#[test]
fn group_names_validation_binary_uniqueness_default_identity_and_conflicts() {
    let db = database();
    let default = groups_db::get(&db, DEFAULT_GROUP_ID).unwrap();
    assert_eq!(
        archived(&db, &default, true),
        Err(AppError::DefaultAttributeGroup)
    );
    let renamed = groups::update(
        &db,
        UpdateAttributeGroupInput {
            id: default.id.clone(),
            name: "共通".into(),
            expected_updated_at: default.updated_at.clone(),
        },
    )
    .unwrap();
    assert_eq!(renamed.id, DEFAULT_GROUP_ID);
    assert!(renamed.updated_at > default.updated_at);
    assert_eq!(
        groups::update(
            &db,
            UpdateAttributeGroupInput {
                id: default.id.clone(),
                name: "古い入力".into(),
                expected_updated_at: default.updated_at.clone()
            }
        ),
        Err(AppError::StaleData)
    );
    assert_eq!(archived(&db, &default, false), Err(AppError::StaleData));
    assert_eq!(
        groups::create(
            &db,
            CreateAttributeGroupInput {
                name: " 共通 ".into()
            }
        ),
        Err(AppError::DuplicateAttributeGroupName)
    );
    for name in [
        "".into(),
        "　".into(),
        "\n名前".into(),
        "名前\t".into(),
        "名\u{2028}前".into(),
        "😀".repeat(81),
    ] {
        assert_eq!(
            groups::create(&db, CreateAttributeGroupInput { name }),
            Err(AppError::InvalidAttributeGroup)
        );
    }
    for name in ["PC", "pc", "Ａ", "A", "é", "e\u{301}", &"😀".repeat(80)] {
        group(&db, name);
    }
    let a = group(&db, "食器");
    let a = archived(&db, &a, true).unwrap();
    assert_eq!(
        groups::create(
            &db,
            CreateAttributeGroupInput {
                name: "食器".into()
            }
        ),
        Err(AppError::DuplicateAttributeGroupName)
    );
    let ordered = groups::list(&db).unwrap();
    assert_eq!(ordered, groups::list(&db).unwrap());
    assert_eq!(ordered.last(), Some(&a));
    let restored = archived(&db, &a, false).unwrap();
    assert_eq!(restored.archived_at, None);
    assert_eq!(restored.id, a.id);
    assert!(restored.updated_at > a.updated_at);
    let missing = uuid::Uuid::new_v4().to_string();
    assert_eq!(
        groups::require_active(&db, &missing),
        Err(AppError::AttributeGroupNotFound)
    );
    assert_eq!(
        groups::require_active(&db, "invalid"),
        Err(AppError::InvalidAttributeGroup)
    );
}

#[test]
fn scoped_field_names_rename_and_archive_preserve_immutable_membership() {
    let db = database();
    let g = group(&db, "PC");
    let a = attribute(&db, DEFAULT_GROUP_ID, "カラー");
    let b = attribute(&db, &g.id, "カラー");
    let c = attribute(&db, &g.id, "素材");
    let rename = |name: &str| {
        definitions::update(
            &db,
            UpdateAttributeInput {
                id: c.id.clone(),
                name: name.into(),
                expected_updated_at: c.updated_at.clone(),
            },
        )
    };
    assert_eq!(rename("カラー"), Err(AppError::DuplicateAttributeName));
    let c = rename("色").unwrap();
    assert_eq!(c.group_id, g.id);
    let hidden = definitions::set_archived(
        &db,
        SetAttributeArchivedInput {
            id: b.id.clone(),
            archived: true,
            expected_updated_at: b.updated_at.clone(),
        },
    )
    .unwrap();
    assert_eq!(hidden.group_id, g.id);
    assert_eq!(
        definitions::create(
            &db,
            CreateAttributeInput {
                group_id: Some(g.id.clone()),
                name: a.name.clone(),
                data_type: "text".into(),
                unit: None
            }
        ),
        Err(AppError::DuplicateAttributeName)
    );
    let g = archived(&db, &g, true).unwrap();
    assert_eq!(
        definitions::create(
            &db,
            CreateAttributeInput {
                group_id: Some(g.id.clone()),
                name: "新規".into(),
                data_type: "text".into(),
                unit: None
            }
        ),
        Err(AppError::AttributeGroupArchived)
    );
    assert_eq!(
        definitions::update(
            &db,
            UpdateAttributeInput {
                id: c.id.clone(),
                name: "変更".into(),
                expected_updated_at: c.updated_at.clone()
            }
        ),
        Err(AppError::AttributeGroupArchived)
    );
    assert_eq!(
        definitions::set_archived(
            &db,
            SetAttributeArchivedInput {
                id: hidden.id.clone(),
                archived: false,
                expected_updated_at: hidden.updated_at.clone()
            }
        ),
        Err(AppError::AttributeGroupArchived)
    );
    archived(&db, &g, false).unwrap();
    assert_eq!(definitions_db::get(&db, &hidden.id).unwrap(), hidden);
    let missing = uuid::Uuid::new_v4().to_string();
    assert_eq!(
        definitions::create(
            &db,
            CreateAttributeInput {
                group_id: Some(missing),
                name: "項目".into(),
                data_type: "text".into(),
                unit: None
            }
        ),
        Err(AppError::AttributeGroupNotFound)
    );
}

#[test]
fn product_switch_preserves_values_and_restores_previous_group_without_implicit_clears() {
    let db = database();
    let g = group(&db, "食器");
    let a = attribute(&db, DEFAULT_GROUP_ID, "素材");
    let b = attribute(&db, &g.id, "素材");
    let p = products::create_with_values(&db, product_input(None), Some(payload(&[(&a, "木")])))
        .unwrap();
    assert_eq!(p.group_id, DEFAULT_GROUP_ID);
    let p = products::update(&db, &p.id, product_input(None), &p.updated_at).unwrap();
    assert_eq!(p.group_id, DEFAULT_GROUP_ID);
    let p = products::update_with_values(
        &db,
        &p.id,
        product_input(Some(&g.id)),
        &p.updated_at,
        Some(payload(&[(&b, "陶器")])),
    )
    .unwrap();
    assert_eq!(p.group_id, g.id);
    assert_eq!(values_db::list(&db, &p.id).unwrap().len(), 2);
    assert_eq!(archived(&db, &g, true), Err(AppError::AttributeGroupInUse));
    let snapshot = products::edit_snapshot(&db, &p.id).unwrap();
    assert_eq!(snapshot.definitions, vec![b.clone()]);
    let p = products::update(&db, &p.id, product_input(None), &p.updated_at).unwrap();
    assert_eq!(p.group_id, g.id); // A legacy edit must not reset membership.
    let p = products::update(
        &db,
        &p.id,
        product_input(Some(DEFAULT_GROUP_ID)),
        &p.updated_at,
    )
    .unwrap();
    let before = values_db::list(&db, &p.id).unwrap();
    let g = archived(&db, &g, true).unwrap();
    assert_eq!(values_db::list(&db, &p.id).unwrap(), before);
    archived(&db, &g, false).unwrap();
    let snapshot = products::edit_snapshot(&db, &p.id).unwrap();
    assert_eq!(snapshot.definitions, vec![a.clone()]);
    assert!(snapshot.values.contains(&ProductAttributeValue {
        attribute_id: a.id,
        value: "木".into()
    }));
    assert!(snapshot.values.contains(&ProductAttributeValue {
        attribute_id: b.id,
        value: "陶器".into()
    }));
    assert!(rows(&db, "PRAGMA foreign_key_check").is_empty());
}

#[test]
fn cross_group_update_clear_and_old_targets_during_switch_roll_back_whole_request() {
    let db = database();
    let g = group(&db, "PC");
    let a = attribute(&db, DEFAULT_GROUP_ID, "カラー");
    let b = attribute(&db, &g.id, "カラー");
    let p = products::create_with_values(&db, product_input(None), Some(payload(&[(&a, "赤")])))
        .unwrap();
    let before = values_db::list(&db, &p.id).unwrap();
    for (destination, target) in [(None, &b), (Some(g.id.as_str()), &a)] {
        for value in ["青", ""] {
            let mut changed = product_input(destination);
            changed.name = "商品名も変更".into();
            assert_eq!(
                products::update_with_values(
                    &db,
                    &p.id,
                    changed,
                    &p.updated_at,
                    Some(payload(&[(target, value)]))
                ),
                Err(AppError::InvalidAttributeValue)
            );
            assert_eq!(products::get(&db, &p.id).unwrap(), p);
            assert_eq!(values_db::list(&db, &p.id).unwrap(), before);
        }
    }
    // A valid new-group target must not be partially applied when any old target is included.
    assert_eq!(
        products::update_with_values(
            &db,
            &p.id,
            product_input(Some(&g.id)),
            &p.updated_at,
            Some(payload(&[(&b, "青"), (&a, "")]))
        ),
        Err(AppError::InvalidAttributeValue)
    );
    assert_eq!(products::get(&db, &p.id).unwrap(), p);
    assert_eq!(values_db::list(&db, &p.id).unwrap(), before);
    let mut new_product = product_input(Some(&g.id));
    new_product.sku = "new".into();
    assert_eq!(
        products::create_with_values(&db, new_product, Some(payload(&[(&a, "新規")]))),
        Err(AppError::InvalidAttributeValue)
    );
    assert_eq!(products::list(&db).unwrap(), vec![p]);
}

#[test]
fn invalid_inactive_product_assignment_and_stale_group_switch_leave_data_unchanged() {
    let db = database();
    let g = group(&db, "PC");
    let g = archived(&db, &g, true).unwrap();
    let missing = uuid::Uuid::new_v4().to_string();
    let p = products::create(&db, product_input(None)).unwrap();
    for (id, error) in [
        (&g.id, AppError::AttributeGroupArchived),
        (&missing, AppError::AttributeGroupNotFound),
    ] {
        assert_eq!(
            products::create(&db, product_input(Some(id))),
            Err(error.clone())
        );
        assert_eq!(
            products::update(&db, &p.id, product_input(Some(id)), &p.updated_at),
            Err(error)
        );
        assert_eq!(products::get(&db, &p.id).unwrap(), p);
    }
    archived(&db, &g, false).unwrap();
    let latest = products::update(&db, &p.id, product_input(None), &p.updated_at).unwrap();
    assert_eq!(
        products::update(&db, &p.id, product_input(Some(&g.id)), &p.updated_at),
        Err(AppError::StaleData)
    );
    assert_eq!(products::get(&db, &p.id).unwrap(), latest);
}

#[test]
fn sql_failure_during_group_switch_rolls_back_membership_product_and_values() {
    let db = database();
    let g = group(&db, "PC");
    let a = attribute(&db, &g.id, "カラー");
    let b = attribute(&db, &g.id, "素材");
    let p = products::create(&db, product_input(None)).unwrap();
    db.execute_batch(&format!("CREATE TRIGGER fail_value BEFORE INSERT ON product_attribute_values WHEN NEW.attribute_id='{}' BEGIN SELECT RAISE(ABORT,'injected'); END;",b.id)).unwrap();
    assert_eq!(
        products::update_with_values(
            &db,
            &p.id,
            product_input(Some(&g.id)),
            &p.updated_at,
            Some(payload(&[(&a, "赤"), (&b, "陶器")]))
        ),
        Err(AppError::DatabaseOperation)
    );
    assert_eq!(products::get(&db, &p.id).unwrap(), p);
    assert!(values_db::list(&db, &p.id).unwrap().is_empty());
}
