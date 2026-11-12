use crate::{
    domain::product::{MAX_PRICE_YEN, ProductInput},
    error::AppError,
    infrastructure::db::{connection, migrations},
    services::product_service as service,
};
use rusqlite::Connection;

fn database() -> Connection {
    let mut connection = Connection::open_in_memory().expect("test database");
    connection
        .pragma_update(None, "foreign_keys", true)
        .expect("foreign keys");
    migrations::apply(&mut connection).expect("migration");
    connection
}
fn input(sku: &str) -> ProductInput {
    ProductInput {
        sku: sku.into(),
        name: "テスト商品".into(),
        brand: None,
        price_yen: Some(1200),
        description_note: None,
    }
}

#[test]
fn create_product() {
    let db = database();
    let product = service::create(&db, input("  SKU-001  ")).unwrap();
    assert_eq!(product.sku, "SKU-001");
    assert_eq!(
        uuid::Uuid::parse_str(&product.id)
            .unwrap()
            .get_version_num(),
        4
    );
    assert!(chrono::DateTime::parse_from_rfc3339(&product.created_at).is_ok());
    assert_eq!(product.created_at, product.updated_at);
}
#[test]
fn get_product() {
    let db = database();
    let product = service::create(&db, input("SKU")).unwrap();
    assert_eq!(service::get(&db, &product.id).unwrap(), product);
    assert_eq!(service::get(&db, "missing"), Err(AppError::ProductNotFound));
}
#[test]
fn list_products() {
    let db = database();
    assert!(service::list(&db).unwrap().is_empty());
    let first = service::create(&db, input("first")).unwrap();
    let second = service::create(&db, input("second")).unwrap();
    let products = service::list(&db).unwrap();
    assert_eq!(products.len(), 2);
    assert!(products.contains(&first) && products.contains(&second));
}
#[test]
fn update_product() {
    let db = database();
    let original = service::create(&db, input("original")).unwrap();
    let mut update = input("new");
    update.name = "変更後".into();
    update.brand = Some("ブランド".into());
    update.description_note = Some("メモ".into());
    update.price_yen = Some(0);
    let updated = service::update(&db, &original.id, update, &original.updated_at).unwrap();
    assert_eq!(updated.id, original.id);
    assert_eq!(updated.created_at, original.created_at);
    assert_eq!(updated.sku, "new");
    assert_eq!(updated.name, "変更後");
    assert_eq!(updated.price_yen, Some(0));
    assert_eq!(updated.brand.as_deref(), Some("ブランド"));
    assert_eq!(updated.description_note.as_deref(), Some("メモ"));
    assert_eq!(service::get(&db, &original.id).unwrap(), updated);
}
#[test]
fn stale_form_never_overwrites_newer_product() {
    let db = database();
    let original = service::create(&db, input("original")).unwrap();
    let latest = service::update(&db, &original.id, input("latest"), &original.updated_at).unwrap();
    assert_eq!(
        service::update(&db, &original.id, input("stale form"), &original.updated_at),
        Err(AppError::StaleData)
    );
    assert_eq!(service::get(&db, &original.id).unwrap(), latest);
    assert_eq!(
        service::update(&db, &original.id, input("stale"), ""),
        Err(AppError::StaleData)
    );
    assert_eq!(service::get(&db, &original.id).unwrap(), latest);
}

#[test]
fn conditional_update_guards_against_change_after_service_read() {
    let db = database();
    let original = service::create(&db, input("original")).unwrap();
    let mut stale = original.clone();
    stale.name = "stale".into();
    let latest = service::update(&db, &original.id, input("latest"), &original.updated_at).unwrap();
    assert_eq!(
        crate::infrastructure::db::product_repository::update(&db, &stale, &original.updated_at),
        Err(AppError::StaleData)
    );
    assert_eq!(service::get(&db, &original.id).unwrap(), latest);
    db.execute("DELETE FROM products", []).unwrap();
    assert_eq!(
        crate::infrastructure::db::product_repository::update(&db, &stale, &original.updated_at),
        Err(AppError::ProductNotFound)
    );
}

#[test]
fn update_token_advances_even_when_previous_timestamp_is_in_future() {
    let db = database();
    let original = service::create(&db, input("clock")).unwrap();
    let future = "2099-01-01T00:00:00.000000000Z";
    db.execute("UPDATE products SET updated_at = ?1", [future])
        .unwrap();
    let updated = service::update(&db, &original.id, input("clock"), future).unwrap();
    assert_eq!(updated.updated_at, "2099-01-01T00:00:00.000000001Z");
    assert_eq!(
        service::update(&db, &original.id, input("stale"), future),
        Err(AppError::StaleData)
    );
}
#[test]
fn duplicate_sku_is_rejected_on_create_and_update() {
    let db = database();
    service::create(&db, input("SKU")).unwrap();
    assert_eq!(
        service::create(&db, input(" SKU ")),
        Err(AppError::DuplicateSku)
    );
    let other = service::create(&db, input("sku")).unwrap(); // case-sensitive
    assert_eq!(
        service::update(&db, &other.id, input("SKU"), &other.updated_at),
        Err(AppError::DuplicateSku)
    );
    assert_eq!(service::get(&db, &other.id).unwrap(), other);
}
#[test]
fn blank_required_fields_are_rejected() {
    let db = database();
    for value in ["", "  ", "\t\n", "\u{3000}"] {
        assert_eq!(
            service::create(&db, input(value)),
            Err(AppError::RequiredField)
        );
        let mut data = input("SKU");
        data.name = value.into();
        assert_eq!(service::create(&db, data), Err(AppError::RequiredField));
    }
}
#[test]
fn invalid_prices_are_rejected() {
    let db = database();
    for price in [-1, MAX_PRICE_YEN + 1] {
        let mut data = input("SKU");
        data.price_yen = Some(price);
        assert_eq!(service::create(&db, data), Err(AppError::InvalidPrice));
    }
    let mut data = input("max");
    data.price_yen = Some(MAX_PRICE_YEN);
    assert_eq!(
        service::create(&db, data).unwrap().price_yen,
        Some(MAX_PRICE_YEN)
    );
}
#[test]
fn update_missing_product() {
    assert_eq!(
        service::update(&database(), "missing", input("SKU"), "old"),
        Err(AppError::ProductNotFound)
    );
}
#[test]
fn repeated_migrations_preserve_data() {
    let mut db = database();
    let product = service::create(&db, input("SKU")).unwrap();
    migrations::apply(&mut db).unwrap();
    migrations::apply(&mut db).unwrap();
    assert_eq!(service::get(&db, &product.id).unwrap(), product);
    assert_eq!(
        db.pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
            .unwrap(),
        3
    );
}
#[test]
fn file_database_survives_close_and_reopen() {
    let directory =
        std::env::temp_dir().join(format!("listing-pilot-test-{}", uuid::Uuid::new_v4()));
    let path = directory.join("listing-pilot.sqlite3");
    let product = {
        let db = connection::open(&path).unwrap();
        assert_eq!(
            db.pragma_query_value(None, "foreign_keys", |row| row.get::<_, i64>(0))
                .unwrap(),
            1
        );
        service::create(&db, input("persistent")).unwrap()
    };
    {
        let db = connection::open(&path).unwrap();
        assert_eq!(service::get(&db, &product.id).unwrap(), product);
    }
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(directory).unwrap();
}
#[test]
fn optional_fields_can_be_empty() {
    let db = database();
    let mut data = input("optional");
    data.price_yen = None;
    data.brand = Some("  ".into());
    data.description_note = Some("".into());
    let product = service::create(&db, data).unwrap();
    assert_eq!(product.price_yen, None);
    assert_eq!(product.brand, None);
    assert_eq!(product.description_note, None);
}
#[test]
fn migration_failure_rolls_back_schema_and_version() {
    let mut db = Connection::open_in_memory().unwrap();
    db.execute_batch(
        "CREATE TABLE products (sentinel TEXT); INSERT INTO products VALUES ('keep');",
    )
    .unwrap();
    assert!(migrations::apply(&mut db).is_err());
    assert_eq!(
        db.pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        db.query_row("SELECT sentinel FROM products", [], |row| row
            .get::<_, String>(0))
            .unwrap(),
        "keep"
    );
}
#[test]
fn newer_schema_is_rejected_without_modification() {
    let mut db = database();
    db.pragma_update(None, "user_version", 4).unwrap();
    assert!(migrations::apply(&mut db).is_err());
    assert_eq!(
        db.pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
            .unwrap(),
        4
    );
}
#[test]
fn initialization_failure_is_reported() {
    let path = std::env::temp_dir().join(format!("listing-pilot-invalid-{}", uuid::Uuid::new_v4()));
    std::fs::write(&path, "not a SQLite database").unwrap();
    assert!(matches!(
        connection::open(&path),
        Err(AppError::DatabaseInitialization)
    ));
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "not a SQLite database"
    );
    std::fs::remove_file(path).unwrap();
}
#[test]
fn schema_constraints_protect_direct_writes() {
    let db = database();
    for (sku, name, price) in [
        ("", "name", "1"),
        ("sku", "", "1"),
        ("sku", "name", "-1"),
        ("sku", "name", "1.5"),
    ] {
        assert!(
            db.execute(
                "INSERT INTO products VALUES ('id', ?1, ?2, NULL, ?3, NULL, 'utc', 'utc')",
                rusqlite::params![sku, name, price]
            )
            .is_err()
        );
    }
}
