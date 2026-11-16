use crate::{
    domain::{
        attribute::AttributeDataType,
        attribute_value,
        listing_preview::{ListingPreview, TitleSource},
    },
    error::AppError,
    services::listing_preview_service::Snapshot,
};
use uuid::Uuid;

pub(crate) fn uuid(value: &str) -> Result<(), AppError> {
    if Uuid::parse_str(value).is_ok_and(|id| id.to_string() == value) {
        Ok(())
    } else {
        Err(AppError::ExportImageInvalid)
    }
}

pub(crate) fn text(value: &str, column: &str) -> Result<(), AppError> {
    // Ignore leading whitespace, controls and invisible format characters when
    // detecting a formula. Also inspect every line after embedded CR/LF.
    for line in value.split(['\r', '\n']) {
        let first = line.chars().find(|c| !c.is_whitespace() && !c.is_control()
            && !matches!(c, '\u{feff}' | '\u{200b}'..='\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2060}'..='\u{206f}'));
        if first.is_some_and(|c| matches!(c, '=' | '+' | '-' | '@')) {
            return Err(AppError::CsvFormulaRisk {
                column: column.to_owned(),
            });
        }
    }
    Ok(())
}

fn row(output: &mut String, cells: &[&str]) {
    output.push_str(
        &cells
            .iter()
            .map(|v| format!("\"{}\"", v.replace('"', "\"\"")))
            .collect::<Vec<_>>()
            .join(","),
    );
    output.push_str("\r\n");
}

pub(crate) fn generate(
    p: &ListingPreview,
    snapshot: &Snapshot,
    files: &[String],
) -> Result<[String; 3], AppError> {
    uuid(&p.product_id)?;
    for (value, column) in [
        (&*p.sku, "product.sku"),
        (&*p.title, "product.title"),
        (
            p.description.as_deref().unwrap_or(""),
            "product.description",
        ),
    ] {
        text(value, column)?;
    }
    let mut product = String::from(
        "\u{feff}product_id,sku,title,title_source,description,price_yen,main_image_path,image_count\r\n",
    );
    let price = p.price_yen.map(|v| v.to_string()).unwrap_or_default();
    let source = match p.title_source {
        TitleSource::ListingTitle => "listing_title",
        TitleSource::ProductName => "product_name",
    };
    row(
        &mut product,
        &[
            &p.product_id,
            &p.sku,
            &p.title,
            source,
            p.description.as_deref().unwrap_or(""),
            &price,
            files.first().map(String::as_str).unwrap_or(""),
            &files.len().to_string(),
        ],
    );
    let mut attributes = String::from("\u{feff}product_id,attribute_id,name,value,unit\r\n");
    for a in &p.attributes {
        uuid(&a.attribute_id)?;
        text(&a.name, "attributes.name")?;
        text(a.unit.as_deref().unwrap_or(""), "attributes.unit")?;
        let definition = snapshot
            .definitions
            .iter()
            .find(|d| d.id == a.attribute_id)
            .ok_or(AppError::StaleData)?;
        if definition.data_type == AttributeDataType::Number {
            if attribute_value::normalize(&a.value, AttributeDataType::Number)?.as_deref()
                != Some(a.value.as_str())
            {
                return Err(AppError::InvalidAttributeValue);
            }
        } else {
            text(&a.value, "attributes.value")?;
        }
        row(
            &mut attributes,
            &[
                &p.product_id,
                &a.attribute_id,
                &a.name,
                &a.value,
                a.unit.as_deref().unwrap_or(""),
            ],
        );
    }
    let mut images =
        String::from("\u{feff}product_id,image_id,sort_order,relative_file,sha256\r\n");
    for (i, (image, file)) in snapshot.images.iter().zip(files).enumerate() {
        uuid(&image.id)?;
        if image.product_id != p.product_id
            || image.sha256.len() != 64
            || !image
                .sha256
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        {
            return Err(AppError::ExportImageInvalid);
        }
        row(
            &mut images,
            &[
                &p.product_id,
                &image.id,
                &i.to_string(),
                file,
                &image.sha256,
            ],
        );
    }
    Ok([product, attributes, images])
}
