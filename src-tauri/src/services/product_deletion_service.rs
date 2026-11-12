use crate::{
    domain::{
        image::{DeleteImageReport, ProductImage},
        product::{DeleteProductInput, Product, ProductDeletePreview},
    },
    error::AppError,
    infrastructure::{
        db::{
            Database, attribute_value_repository as values, image_repository as images,
            product_repository as products,
        },
        media_storage::MediaStorage,
    },
};
use std::collections::HashSet;
use uuid::Uuid;

fn valid_uuid(id: &str) -> bool {
    Uuid::parse_str(id).is_ok_and(|uuid| uuid.to_string() == id)
}

pub fn preview(database: &Database, product_id: &str) -> Result<ProductDeletePreview, AppError> {
    if !valid_uuid(product_id) {
        return Err(AppError::ProductNotFound);
    }
    database.with_mut(|connection| {
        let transaction = connection.transaction()?;
        let product = products::get(&transaction, product_id)?;
        let current = images::list(&transaction, product_id)?;
        let result = ProductDeletePreview {
            product_id: product.id.clone(),
            name: product.name.clone(),
            sku: product.sku.clone(),
            updated_at: product.updated_at.clone(),
            image_count: current.len(),
            image_ids: current.into_iter().map(|image| image.id).collect(),
            attribute_values: values::list(&transaction, product_id)?,
            product,
        };
        transaction.commit()?;
        Ok(result)
    })
}

fn validate(
    input: &DeleteProductInput,
    product: &Product,
    current: &[ProductImage],
) -> Result<(), AppError> {
    if product != &input.expected_product
        || product.updated_at != input.expected_updated_at
        || input
            .expected_image_ids
            .iter()
            .collect::<HashSet<_>>()
            .len()
            != input.expected_image_ids.len()
        || !current
            .iter()
            .map(|image| &image.id)
            .eq(input.expected_image_ids.iter())
        || current
            .iter()
            .any(|image| image.product_id != input.product_id || !valid_uuid(&image.id))
    {
        return Err(AppError::StaleData);
    }
    Ok(())
}

pub fn delete(
    database: &Database,
    media: &MediaStorage,
    input: &DeleteProductInput,
) -> Result<DeleteImageReport, AppError> {
    let _permit = media.begin_import()?;
    if !valid_uuid(&input.product_id) || input.expected_image_ids.iter().any(|id| !valid_uuid(id)) {
        return Err(AppError::StaleData);
    }
    let checked = database.with_mut(|connection| {
        let transaction = connection.transaction()?;
        let product = products::get(&transaction, &input.product_id).map_err(stale_missing)?;
        let current = images::list(&transaction, &input.product_id)?;
        validate(input, &product, &current)?;
        if values::list(&transaction, &input.product_id)? != input.expected_attribute_values {
            return Err(AppError::StaleData);
        }
        transaction.commit()?;
        Ok(current)
    })?;
    for image in &checked {
        media.deletion_file_exists(image)?;
    }
    let commit = database.with_mut(|connection| {
        let transaction =
            connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let product = products::get(&transaction, &input.product_id).map_err(stale_missing)?;
        let current = images::list(&transaction, &input.product_id)?;
        validate(input, &product, &current)?;
        if values::list(&transaction, &input.product_id)? != input.expected_attribute_values {
            return Err(AppError::StaleData);
        }
        if current != checked {
            return Err(AppError::StaleData);
        }
        values::delete_product(
            &transaction,
            &input.product_id,
            input.expected_attribute_values.len(),
        )?;
        for image in &current {
            images::delete(&transaction, &input.product_id, &image.id)?;
        }
        products::delete(&transaction, &input.product_id)?;
        // Also reject triggers that silently retain/recreate records or references.
        if has_references(&transaction, &input.product_id, &checked)? {
            return Err(AppError::StaleData);
        }
        Ok(transaction.commit().map_err(AppError::from))
    })?;
    resolve_commit(
        database,
        &input.product_id,
        &checked,
        &input.expected_attribute_values,
        commit,
    )?;
    let mut report = DeleteImageReport {
        deleted: true,
        affected_image_count: checked.len(),
        quarantined_image_count: 0,
        missing_image_count: 0,
        pending_quarantine_count: 0,
    };
    for image in &checked {
        match media.quarantine_deleted(image) {
            Ok(true) => report.quarantined_image_count += 1,
            Ok(false) => report.missing_image_count += 1,
            Err(_) => report.pending_quarantine_count += 1,
        }
    }
    Ok(report)
}

fn stale_missing(error: AppError) -> AppError {
    if error == AppError::ProductNotFound {
        AppError::StaleData
    } else {
        error
    }
}

fn has_references(
    connection: &rusqlite::Connection,
    product_id: &str,
    checked: &[ProductImage],
) -> Result<bool, AppError> {
    let exists: bool = connection.query_row("SELECT EXISTS(SELECT 1 FROM products WHERE id = ?1) OR EXISTS(SELECT 1 FROM product_images WHERE product_id = ?1) OR EXISTS(SELECT 1 FROM product_attribute_values WHERE product_id = ?1)", [product_id], |row| row.get(0))?;
    if exists {
        return Ok(true);
    }
    for image in checked {
        if images::references_image_or_path(connection, image)? {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(crate) fn resolve_commit(
    database: &Database,
    product_id: &str,
    checked: &[ProductImage],
    checked_values: &[crate::domain::attribute_value::ProductAttributeValue],
    commit: Result<(), AppError>,
) -> Result<(), AppError> {
    if let Err(error) = commit {
        // Re-read all references in one snapshot after the transaction was dropped.
        match database.with_mut(|connection| {
            if !connection.is_autocommit() {
                return Err(AppError::ProductDeletionUnknown);
            }
            let transaction = connection.transaction()?;
            let references = has_references(&transaction, product_id, checked)?;
            if references {
                // A full rollback is a normal DB error; partial or reassigned
                // references need an explicit restart/state-check instruction.
                products::get(&transaction, product_id)?;
                if images::list(&transaction, product_id)? != checked
                    || values::list(&transaction, product_id)? != checked_values
                {
                    return Err(AppError::ProductDeletionUnknown);
                }
            }
            transaction.commit()?;
            Ok(references)
        }) {
            Ok(false) => (),
            Ok(true) => return Err(error),
            Err(_) => return Err(AppError::ProductDeletionUnknown),
        }
    }
    Ok(())
}
