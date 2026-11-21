use super::export_csv;
use crate::{domain::shopify_export::ShopifyProductPreview, error::AppError};

pub(crate) const HEADERS: [&str; 10] = [
    "Title",
    "URL handle",
    "Description",
    "Vendor",
    "SKU",
    "Option1 name",
    "Option1 value",
    "Price",
    "Published on online store",
    "Status",
];
pub(crate) const MAX_BYTES: usize = 15_000_000;

pub(crate) fn check_size(bytes: usize) -> Result<(), AppError> {
    if bytes >= MAX_BYTES {
        Err(AppError::ShopifyCsvSizeLimit)
    } else {
        Ok(())
    }
}

pub(crate) fn row(output: &mut String, cells: &[&str]) {
    for (index, cell) in cells.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push('"');
        output.push_str(&cell.replace('"', "\"\""));
        output.push('"');
    }
    output.push('\n');
}

// Also validates blocked rows during preview; missing prices are never rendered as zero.
pub(crate) fn validate(p: &ShopifyProductPreview) -> Result<(), AppError> {
    export_csv::uuid(&p.product_id)?;
    if p.handle != format!("lp-{}", p.product_id) {
        return Err(AppError::InvalidExportSelection);
    }
    for (value, column) in [
        (p.title.as_str(), "Title"),
        (p.description.as_deref().unwrap_or(""), "Description"),
        (p.vendor.as_deref().unwrap_or(""), "Vendor"),
        (p.sku.as_str(), "SKU"),
        (p.handle.as_str(), "URL handle"),
        ("Title", "Option1 name"),
        ("Default Title", "Option1 value"),
        ("false", "Published on online store"),
        ("draft", "Status"),
    ] {
        export_csv::text(value, column).map_err(|_| AppError::ShopifyCsvFormulaRisk {
            product_id: p.product_id.clone(),
            column: column.to_owned(),
        })?;
    }
    if p.sku.is_empty() || p.title.is_empty() {
        return Err(AppError::RequiredField);
    }
    if p.price_yen.is_some_and(|price| price < 0) {
        return Err(AppError::InvalidPrice);
    }
    Ok(())
}

pub(crate) fn generate(products: &[ShopifyProductPreview]) -> Result<String, AppError> {
    let mut output = String::new();
    row(&mut output, &HEADERS);
    for p in products {
        validate(p)?;
        let price = p
            .price_yen
            .ok_or(AppError::ShopifyExportBlocked)?
            .to_string();
        row(
            &mut output,
            &[
                &p.title,
                &p.handle,
                p.description.as_deref().unwrap_or(""),
                p.vendor.as_deref().unwrap_or(""),
                &p.sku,
                "Title",
                "Default Title",
                &price,
                "false",
                "draft",
            ],
        );
        check_size(output.len())?;
    }
    Ok(output)
}
