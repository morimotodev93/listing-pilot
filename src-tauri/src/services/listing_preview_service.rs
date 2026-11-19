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
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Snapshot {
    pub group: crate::domain::attribute_group::AttributeGroup,
    pub product: crate::domain::product::Product,
    pub images: Vec<crate::domain::image::ProductImage>,
    pub definitions: Vec<crate::domain::attribute::AttributeDefinition>,
    pub values: Vec<crate::domain::attribute_value::ProductAttributeValue>,
}

pub(crate) fn read(
    connection: &rusqlite::Connection,
    product_id: &str,
) -> Result<Snapshot, AppError> {
    let product = product_repository::get(connection, product_id)?;
    Ok(Snapshot {
        group: super::attribute_group_service::require_active(connection, &product.group_id)?,
        product,
        images: image_repository::list(connection, product_id)?,
        definitions: attribute_repository::list(connection)?,
        values: attribute_value_repository::list(connection, product_id)?,
    })
}

pub(crate) fn snapshot(database: &Database, product_id: &str) -> Result<Snapshot, AppError> {
    database.with_mut(|connection| {
        let transaction = connection.transaction()?;
        let snapshot = read(&transaction, product_id)?;
        transaction.commit()?;
        Ok(snapshot)
    })
}

// Shared candidate contract for preview and future export. No stored status or writes.
pub fn get(
    database: &Database,
    media: &MediaStorage,
    product_id: &str,
) -> Result<ListingPreview, AppError> {
    // Same ordering as media mutations: permit -> DB -> release DB -> file I/O.
    // The OS lock is held by MediaStorage throughout its lifetime.
    let _permit = media.begin_import()?;
    let snapshot = snapshot(database, product_id)?;
    Ok(build(snapshot, media))
}

pub(crate) fn build(snapshot: Snapshot, media: &MediaStorage) -> ListingPreview {
    build_with(snapshot, |image| media.preview(image))
}

// CSV-only profiles reuse candidates and token construction without touching image files.
pub(crate) fn build_for_csv(snapshot: Snapshot) -> ListingPreview {
    build_with(snapshot, |_| None)
}

fn build_with(
    snapshot: Snapshot,
    image_path: impl Fn(&crate::domain::image::ProductImage) -> Option<String>,
) -> ListingPreview {
    // Opaque, process-version-local identity; no internal data is sent to React.
    let mut hash = Sha256::new();
    hash.update(format!("{snapshot:?}"));
    let Snapshot {
        group: _,
        product,
        images,
        definitions,
        values,
    } = snapshot;
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
            preview_path: image_path(&image),
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
        .filter(|d| d.group_id == product.group_id && d.archived_at.is_none())
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
    hash.update(format!("{images:?}{issues:?}"));
    ListingPreview {
        snapshot_token: format!("{:x}", hash.finalize()),
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
    }
}
