use crate::{
    domain::shopify_image_export::{ImageExportIssueCode, ShopifyImageProductPreview},
    error::AppError,
    services::{export_csv, shopify_csv},
};

pub(crate) const MAX_PRODUCT_IMAGES: usize = 250;

pub(crate) fn generate(products: &[ShopifyImageProductPreview]) -> Result<String, AppError> {
    let mut output = String::new();
    let mut headers = shopify_csv::HEADERS.to_vec();
    headers.extend(["Product image URL", "Image position", "Image alt text"]);
    shopify_csv::row(&mut output, &headers);
    for p in products {
        let product = &p.product;
        shopify_csv::validate(product)?;
        if p.images.len() > MAX_PRODUCT_IMAGES {
            return Err(AppError::ShopifyProductImageLimit {
                product_id: product.product_id.clone(),
            });
        }
        let price = product
            .price_yen
            .ok_or(AppError::ShopifyExportBlocked)?
            .to_string();
        if p.images.is_empty() || !p.image_issues.is_empty() {
            return Err(blocked(
                &product.product_id,
                None,
                None,
                ImageExportIssueCode::ImagesMissing,
            ));
        }
        for (index, image) in p.images.iter().enumerate() {
            let url = image.csv_url.as_deref().ok_or_else(|| {
                blocked(
                    &product.product_id,
                    Some(&image.image_id),
                    Some(image.position),
                    ImageExportIssueCode::UrlMissing,
                )
            })?;
            // The builder and renderer use the same Rust validators, including saved data.
            if crate::domain::image_url::image_url(url)?.is_none() {
                return Err(AppError::InvalidShopifyImageUrl);
            }
            crate::domain::image_url::alt_text(&image.csv_alt)?;
            export_csv::text(&image.csv_alt, "Image alt text")?;
            let position = (index + 1).to_string();
            let mut cells = if index == 0 {
                vec![
                    product.title.as_str(),
                    &product.handle,
                    product.description.as_deref().unwrap_or(""),
                    product.vendor.as_deref().unwrap_or(""),
                    &product.sku,
                    "Title",
                    "Default Title",
                    &price,
                    "false",
                    "draft",
                ]
            } else {
                vec!["", &product.handle, "", "", "", "", "", "", "", ""]
            };
            cells.extend([url, position.as_str(), image.csv_alt.as_str()]);
            shopify_csv::row(&mut output, &cells);
            shopify_csv::check_size(output.len())?;
        }
    }
    Ok(output)
}

pub(crate) fn blocked(
    product_id: &str,
    image_id: Option<&str>,
    position: Option<usize>,
    reason: ImageExportIssueCode,
) -> AppError {
    AppError::ShopifyImageExportBlocked {
        product_id: product_id.into(),
        image_id: image_id.map(str::to_owned),
        position,
        reason,
    }
}
