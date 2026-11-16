use crate::{
    domain::listing_preview::*,
    error::AppError,
    infrastructure::{
        db::{
            Database, attribute_repository, attribute_value_repository, image_repository,
            product_repository,
        },
        media_storage::MediaStorage,
    },
};

// Shared candidate contract for preview and future export. No stored status or writes.
pub fn get(
    database: &Database,
    media: &MediaStorage,
    product_id: &str,
) -> Result<ListingPreview, AppError> {
    // Same ordering as media mutations: permit -> DB -> release DB -> file I/O.
    // The OS lock is held by MediaStorage throughout its lifetime.
    let _permit = media.begin_import()?;
    let (product, images, definitions, values) = database.with_mut(|connection| {
        let transaction = connection.transaction()?;
        let snapshot = (
            product_repository::get(&transaction, product_id)?,
            image_repository::list(&transaction, product_id)?,
            attribute_repository::list(&transaction)?,
            attribute_value_repository::list(&transaction, product_id)?,
        );
        transaction.commit()?;
        Ok(snapshot)
    })?;
    let mut issues = Vec::new();
    let mut warning = |code, message| {
        issues.push(ListingIssue {
            code,
            severity: IssueSeverity::Warning,
            message,
        })
    };
    let (title, title_source) = match product.listing_title {
        Some(title) => (title, TitleSource::ListingTitle),
        None => {
            warning(
                IssueCode::TitleFallback,
                "公開用タイトルが未設定です。管理用の商品名を候補として表示しています。公開前に確認してください。",
            );
            (product.name, TitleSource::ProductName)
        }
    };
    if product.listing_description.is_none() {
        warning(IssueCode::DescriptionMissing, "公開用説明文が未設定です。");
    }
    if product.price_yen.is_none() {
        warning(IssueCode::PriceMissing, "価格が未設定です。");
    }
    if images.is_empty() {
        warning(IssueCode::ImagesMissing, "画像が登録されていません。");
    }
    let images: Vec<_> = images
        .into_iter()
        .map(|image| ListingPreviewImage {
            preview_path: media.preview(&image),
            image_id: image.id,
            sort_order: image.sort_order,
        })
        .collect();
    if images.iter().any(|image| image.preview_path.is_none()) {
        issues.push(ListingIssue {
            code: IssueCode::ImageUnavailable,
            severity: IssueSeverity::Error,
            message: "安全に読み取れない画像があります。画像の状態を確認してください。",
        });
    }
    let attributes = definitions
        .into_iter()
        .filter(|d| d.archived_at.is_none())
        .filter_map(|d| {
            values
                .iter()
                .find(|v| v.attribute_id == d.id)
                .map(|v| ListingPreviewAttribute {
                    attribute_id: d.id,
                    name: d.name,
                    unit: d.unit,
                    value: v.value.clone(),
                })
        })
        .collect();
    let status = if issues.iter().any(|i| i.severity == IssueSeverity::Error) {
        ListingStatus::Blocked
    } else if issues.is_empty() {
        ListingStatus::Ready
    } else {
        ListingStatus::Warning
    };
    Ok(ListingPreview {
        product_id: product.id,
        sku: product.sku,
        title,
        title_source,
        description: product.listing_description,
        price_yen: product.price_yen,
        attributes,
        images,
        status,
        issues,
    })
}
