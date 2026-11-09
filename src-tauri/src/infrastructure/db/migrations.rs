use rusqlite::{Connection, TransactionBehavior};

pub fn apply(connection: &mut Connection) -> rusqlite::Result<()> {
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let version: i64 = transaction.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if version > 2 {
        return Err(rusqlite::Error::InvalidQuery);
    }
    if version == 0 {
        transaction.execute_batch(include_str!("../../../migrations/0001_create_products.sql"))?;
        transaction.pragma_update(None, "user_version", 1)?;
    }
    if version < 2 {
        transaction.execute_batch(include_str!(
            "../../../migrations/0002_create_product_images.sql"
        ))?;
        transaction.pragma_update(None, "user_version", 2)?;
    }
    transaction.commit()
}
