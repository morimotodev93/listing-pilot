use super::*;
use crate::{
    domain::{
        image::ProductImage,
        image_url::{GetImageUrlsInput, ImageUrlSetting},
        shopify_image_export::*,
    },
    infrastructure::db::image_url_repository as urls,
    services::{
        image_url_service, shopify_csv, shopify_export_service as plain, shopify_image_csv as csv,
        shopify_image_export_service as service,
    },
};
const A: &str = "a.myshopify.com";
const B: &str = "b.myshopify.com";

fn seed(f: &Fixture, id: &str, count: usize, store: &str) {
    f.db.with_mut(|db| {
        let tx = db.transaction()?;
        let offset = image_repository::list(&tx, id)?.len();
        for i in 0..count {
            let image_id = Uuid::new_v4().to_string();
            let hash = format!("{:064x}", offset + i + 1);
            image_repository::create(
                &tx,
                &ProductImage {
                    id: image_id.clone(),
                    product_id: id.into(),
                    original_filename: format!("mock-{i}.png"),
                    relative_path: MediaStorage::relative_path(id, &image_id, "png")?,
                    mime_type: "image/png".into(),
                    file_size_bytes: 2_000_000_000,
                    width: 2,
                    height: 3,
                    sha256: hash.clone(),
                    sort_order: i as i64,
                    created_at: "2026-11-20T00:00:00Z".into(),
                },
            )?;
            urls::save(
                &tx,
                &ImageUrlSetting {
                    image_id,
                    platform: "shopify".into(),
                    store_key: store.into(),
                    url: format!("https://cdn.shopify.com/photo-{i}.png?v=1&width=600"),
                    image_sha256: hash,
                    alt_text: if i == 0 {
                        "".into()
                    } else {
                        "日本語,\"alt\"".into()
                    },
                    updated_at: "2026-11-20T00:00:00Z".into(),
                },
            )?;
        }
        tx.commit()?;
        Ok(())
    })
    .unwrap();
}
fn get(f: &Fixture, ids: &[String], store: &str) -> ShopifyImagePreview {
    service::get(
        &f.db,
        &ShopifyImagePreviewInput {
            product_ids: ids.to_vec(),
            store_key: store.into(),
        },
    )
    .unwrap()
}
fn input(f: &Fixture, ids: &[String], store: &str) -> ShopifyImageExportInput {
    let p = get(f, ids, store);
    ShopifyImageExportInput {
        product_ids: ids.to_vec(),
        store_key: store.into(),
        snapshot_token: p.snapshot_token.clone(),
        warning_token: Some(p.snapshot_token),
    }
}
fn run(f: &Fixture, input: &ShopifyImageExportInput) -> Result<ExportReport, AppError> {
    let _permit = f.media.begin_import()?;
    service::export(&f.db, &f.media, input, &f.out)
}
fn parse(value: &str) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut cell = String::new();
    let mut quoted = false;
    let mut chars = value.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                chars.next();
                cell.push('"');
            }
            '"' => quoted = !quoted,
            ',' if !quoted => row.push(std::mem::take(&mut cell)),
            '\n' if !quoted => {
                row.push(std::mem::take(&mut cell));
                rows.push(std::mem::take(&mut row));
            }
            _ => cell.push(c),
        }
    }
    assert!(!quoted && cell.is_empty() && row.is_empty());
    rows
}
fn all_settings(f: &Fixture) -> Vec<ImageUrlSetting> {
    f.db.with(|db| urls::for_product(db, &f.id)).unwrap()
}
fn output(report: ExportReport) -> PathBuf {
    match report {
        ExportReport::Success { folder, .. } => folder.into(),
        other => panic!("{other:?}"),
    }
}

#[test]
fn thirteen_headers_one_four_images_first_row_and_continuations_round_trip() {
    for count in [1, 4] {
        let f = Fixture::new();
        seed(&f, &f.id, count, A);
        let before = f.dump();
        let settings = all_settings(&f);
        let i = input(&f, std::slice::from_ref(&f.id), A);
        let report = run(&f, &i).unwrap();
        assert!(
            matches!(&report,ExportReport::Success{product_count:1,image_count,..} if *image_count==count)
        );
        let path = output(report);
        let bytes = fs::read(path.join("shopify-products.csv")).unwrap();
        assert!(!bytes.starts_with(&[0xef, 0xbb, 0xbf]));
        assert!(!bytes.contains(&b'\r'));
        let text = String::from_utf8(bytes).unwrap();
        let rows = parse(&text);
        assert_eq!(
            rows[0],
            [
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
                "Product image URL",
                "Image position",
                "Image alt text"
            ]
        );
        assert_eq!(rows.len(), count + 1);
        for (index, row) in rows[1..].iter().enumerate() {
            assert_eq!(row.len(), 13);
            assert_eq!(row[1], format!("lp-{}", f.id));
            assert_eq!(row[11], (index + 1).to_string());
            assert_eq!(
                row[10],
                format!("https://cdn.shopify.com/photo-{index}.png?v=1&width=600")
            );
            assert_eq!(row[12], if index == 0 { "" } else { "日本語,\"alt\"" });
            if index == 0 {
                assert_eq!(row[0], "公開,\"タイトル\"");
                assert_eq!(row[2], "日本語\n二行目");
                assert_eq!(
                    &row[5..10],
                    ["Title", "Default Title", "0", "false", "draft"]
                );
            } else {
                for column in [0, 2, 3, 4, 5, 6, 7, 8, 9] {
                    assert_eq!(row[column], "");
                }
            }
        }
        assert_eq!(fs::read_dir(&path).unwrap().count(), 1);
        assert_eq!(f.dump(), before);
        assert!(settings == all_settings(&f));
        assert!(!text.contains("media/"));
        assert!(!text.contains("極秘メモ"));
        // These originals do not exist. CSV-only output never reads or copies them.
        assert_eq!(
            fs::read_dir(f.root.join("data/media/originals"))
                .unwrap()
                .count(),
            0
        );
    }
}

#[test]
fn canonical_product_order_and_current_image_order_determine_positions() {
    let f = Fixture::new();
    let other =
        f.db.with(|db| {
            product_service::create(
                db,
                serde_json::from_value(json!({"sku":"other","name":"他商品","price_yen":9}))
                    .unwrap(),
            )
            .map(|p| p.id)
        })
        .unwrap();
    seed(&f, &f.id, 4, A);
    seed(&f, &other, 1, A);
    let mut ids = vec![f.id.clone(), other];
    let p = get(&f, &ids, A);
    ids.sort();
    assert_eq!(
        p.products
            .iter()
            .map(|p| p.product.product_id.clone())
            .collect::<Vec<_>>(),
        ids
    );
    let original = f.db.with(|db| image_repository::list(db, &f.id)).unwrap();
    let mut order: Vec<_> = original.iter().map(|i| i.id.clone()).collect();
    order.reverse();
    f.db.with_mut(|db| {
        let tx = db.transaction()?;
        image_repository::update_order(&tx, &f.id, &order)?;
        tx.commit()?;
        Ok(())
    })
    .unwrap();
    let p = get(&f, &ids, A);
    let product = p
        .products
        .iter()
        .find(|p| p.product.product_id == f.id)
        .unwrap();
    assert_eq!(product.main_image_id.as_ref(), Some(&order[0]));
    assert_eq!(
        product
            .images
            .iter()
            .map(|i| i.image_id.clone())
            .collect::<Vec<_>>(),
        order
    );
    let rows = parse(&csv::generate(&p.products).unwrap());
    let product_rows: Vec<_> = rows[1..]
        .iter()
        .filter(|r| r[1] == format!("lp-{}", f.id))
        .collect();
    assert!(product_rows[0][10].contains("photo-3.png"));
    assert_eq!(product_rows[0][11], "1");
}

#[test]
fn image_zero_missing_url_and_other_store_only_are_blocked_not_warnings() {
    for count in [0, 1] {
        let f = Fixture::new();
        if count > 0 {
            seed(&f, &f.id, 1, B);
        }
        let p = get(&f, std::slice::from_ref(&f.id), A);
        assert!(p.blocked);
        let code = if count == 0 {
            ImageExportIssueCode::ImagesMissing
        } else {
            ImageExportIssueCode::UrlMissing
        };
        assert_eq!(p.products[0].image_issues[0].code, code);
        assert_eq!(p.products[0].url_configured_count, 0);
        let i = input(&f, std::slice::from_ref(&f.id), A);
        assert!(
            matches!(run(&f,&i),Err(AppError::ShopifyImageExportBlocked{reason,..}) if reason==code)
        );
        f.empty();
    }
}

#[test]
fn saved_url_alt_and_hash_are_revalidated_with_safe_context_not_raw_values() {
    for (mutation, expected) in [
        (
            "UPDATE image_url_settings SET url='https://cdn.shopify.com/secret%0a'",
            ImageExportIssueCode::InvalidUrl,
        ),
        (
            "UPDATE image_url_settings SET url='https://cdn.shopify.com/'||printf('%04096d',0)",
            ImageExportIssueCode::InvalidUrl,
        ),
        (
            "UPDATE image_url_settings SET image_sha256=lower(hex(zeroblob(32)))",
            ImageExportIssueCode::ShaMismatch,
        ),
        (
            "UPDATE image_url_settings SET alt_text='=secret'",
            ImageExportIssueCode::InvalidAlt,
        ),
        (
            "UPDATE image_url_settings SET alt_text=char(10)||'secret'",
            ImageExportIssueCode::InvalidAlt,
        ),
        (
            "UPDATE image_url_settings SET alt_text=printf('%0513d',0)",
            ImageExportIssueCode::InvalidAlt,
        ),
        (
            "UPDATE image_url_settings SET url='',alt_text='secret'",
            ImageExportIssueCode::UrlMissing,
        ),
    ] {
        let f = Fixture::new();
        seed(&f, &f.id, 1, A);
        f.sql(&format!(
            "PRAGMA ignore_check_constraints=ON; {mutation}; PRAGMA ignore_check_constraints=OFF;"
        ));
        let p = get(&f, std::slice::from_ref(&f.id), A);
        assert!(p.blocked);
        assert!(
            p.products[0]
                .image_issues
                .iter()
                .any(|i| i.code == expected)
        );
        let serialized = serde_json::to_string(&p).unwrap();
        assert!(!serialized.contains("secret"));
        assert!(!serialized.contains("https://"));
        assert!(!serialized.contains("media/"));
        let i = input(&f, std::slice::from_ref(&f.id), A);
        let error = run(&f, &i).unwrap_err();
        let value = serde_json::to_value(error).unwrap();
        assert_eq!(value["product_id"], f.id);
        assert_eq!(value["position"], 1);
        f.empty();
    }
}

#[test]
fn duplicate_detection_is_per_product_and_selected_store_after_normalization() {
    let f = Fixture::new();
    seed(&f, &f.id, 2, A);
    f.sql("UPDATE image_url_settings SET url='https://cdn.shopify.com/same.png'");
    let p = get(&f, std::slice::from_ref(&f.id), A);
    assert!(p.blocked);
    assert!(
        p.products[0]
            .image_issues
            .iter()
            .any(|i| i.code == ImageExportIssueCode::DuplicateUrl)
    );
    f.sql("PRAGMA ignore_check_constraints=ON; UPDATE image_url_settings SET url='HTTPS://CDN.SHOPIFY.COM:443/same.png' WHERE rowid=(SELECT min(rowid) FROM image_url_settings); PRAGMA ignore_check_constraints=OFF;");
    let p = get(&f, std::slice::from_ref(&f.id), A);
    assert!(p.blocked);
    assert!(
        p.products[0]
            .image_issues
            .iter()
            .any(|i| i.code == ImageExportIssueCode::DuplicateUrl)
    );
    // The DB constraints normally keep canonical URLs; revalidation still catches equivalent duplicates.
}

#[test]
fn same_url_across_products_and_unrelated_store_settings_are_allowed() {
    let f = Fixture::new();
    let other =
        f.db.with(|db| {
            product_service::create(
                db,
                serde_json::from_value(json!({"sku":"other","name":"他","price_yen":1})).unwrap(),
            )
            .map(|p| p.id)
        })
        .unwrap();
    seed(&f, &f.id, 1, A);
    seed(&f, &other, 1, A);
    let p = get(&f, &[f.id.clone(), other], A);
    assert!(!p.blocked);
    f.sql("INSERT INTO image_url_settings SELECT image_id,platform,'b.myshopify.com',url,image_sha256,alt_text,updated_at FROM image_url_settings");
    let token = get(&f, std::slice::from_ref(&f.id), A).snapshot_token;
    f.sql("UPDATE image_url_settings SET url='https://cdn.shopify.com/other.png' WHERE store_key='b.myshopify.com'");
    assert_eq!(
        get(&f, std::slice::from_ref(&f.id), A).snapshot_token,
        token
    );
}

#[test]
fn zero_price_is_valid_null_price_blocks_the_whole_selection() {
    let f = Fixture::new();
    seed(&f, &f.id, 1, A);
    let ids = vec![f.id.clone()];
    assert!(!get(&f, &ids, A).blocked);
    assert!(service::preflight(&f.db, &input(&f, &ids, A)).is_ok());
    f.sql("UPDATE products SET price_yen=NULL");
    let p = get(&f, &ids, A);
    assert!(p.blocked);
    assert!(
        p.products[0]
            .product
            .issues
            .iter()
            .any(|i| i.code == crate::domain::listing_preview::IssueCode::PriceMissing)
    );
    assert_eq!(
        run(&f, &input(&f, &ids, A)).unwrap_err(),
        AppError::ShopifyExportBlocked
    );
    f.empty();
}

#[test]
fn exactly_250_images_allowed_251_rejected_without_generic_byte_or_image_limit() {
    for count in [250, 251] {
        let f = Fixture::new();
        seed(&f, &f.id, count, A);
        let ids = vec![f.id.clone()];
        let p = get(&f, &ids, A);
        assert_eq!(p.blocked, count > 250);
        assert_eq!(p.image_count, count);
        if count == 250 {
            assert_eq!(parse(&csv::generate(&p.products).unwrap()).len(), 251);
            assert!(matches!(
                run(&f, &input(&f, &ids, A)),
                Ok(ExportReport::Success {
                    image_count: 250,
                    ..
                })
            ));
        } else {
            assert_eq!(
                run(&f, &input(&f, &ids, A)).unwrap_err(),
                AppError::ShopifyProductImageLimit {
                    product_id: f.id.clone()
                }
            );
            f.empty();
        }
    }
}

#[test]
fn one_blocked_product_prevents_every_product_from_being_written() {
    let f = Fixture::new();
    seed(&f, &f.id, 1, A);
    let other =
        f.db.with(|db| {
            product_service::create(
                db,
                serde_json::from_value(json!({"sku":"other","name":"他","price_yen":1})).unwrap(),
            )
            .map(|p| p.id)
        })
        .unwrap();
    let ids = vec![f.id.clone(), other];
    assert!(get(&f, &ids, A).blocked);
    assert!(run(&f, &input(&f, &ids, A)).is_err());
    f.empty();
}

#[test]
fn csv_utf8_byte_limit_applies_to_actual_thirteen_column_rendering() {
    let f = Fixture::new();
    seed(&f, &f.id, 1, A);
    let mut p = get(&f, std::slice::from_ref(&f.id), A);
    p.products[0].product.description = Some(String::new());
    let base = csv::generate(&p.products).unwrap().len();
    let length = shopify_csv::MAX_BYTES - 1 - base;
    p.products[0].product.description = Some(format!(
        "{}{}",
        "あ".repeat(length / 3),
        "a".repeat(length % 3)
    ));
    assert_eq!(
        csv::generate(&p.products).unwrap().len(),
        shopify_csv::MAX_BYTES - 1
    );
    p.products[0]
        .product
        .description
        .as_mut()
        .unwrap()
        .push('x');
    assert_eq!(
        csv::generate(&p.products).err(),
        Some(AppError::ShopifyCsvSizeLimit)
    );
}

#[test]
fn all_relevant_changes_invalidate_token_and_old_warning_confirmation() {
    for mutation in [
        "name",
        "title",
        "sku",
        "price",
        "order",
        "add",
        "delete",
        "url",
        "alt",
        "clear",
        "sha",
        "mapping_sha",
        "timestamp",
    ] {
        let f = Fixture::new();
        seed(&f, &f.id, 2, A);
        let ids = vec![f.id.clone()];
        let i = input(&f, &ids, A);
        match mutation {
            "name"=>f.sql("UPDATE products SET name='changed'"), "title"=>f.sql("UPDATE products SET listing_title='changed'"),
            "sku"=>f.sql("UPDATE products SET sku='changed'"), "price"=>f.sql("UPDATE products SET price_yen=7"),
            "order"=>f.sql("UPDATE product_images SET sort_order=1-sort_order"),
            "add"=>seed(&f,&f.id,1,B), "delete"=>f.sql("DELETE FROM product_images WHERE rowid=(SELECT min(rowid) FROM product_images)"),
            "url"=>f.sql("UPDATE image_url_settings SET url=url||'&changed=1'"), "alt"=>f.sql("UPDATE image_url_settings SET alt_text='changed'"),
            "clear"=>f.sql("DELETE FROM image_url_settings"), "sha"=>f.sql("UPDATE product_images SET sha256=lower(hex(zeroblob(32))) WHERE rowid=(SELECT min(rowid) FROM product_images)"),
            "mapping_sha"=>f.sql("UPDATE image_url_settings SET image_sha256=lower(hex(zeroblob(32)))"),
            _=>f.sql("UPDATE image_url_settings SET updated_at='2099-01-01T00:00:00Z'"),
        }
        assert_ne!(
            get(&f, &ids, A).snapshot_token,
            i.snapshot_token,
            "{mutation}"
        );
        assert_eq!(
            service::preflight(&f.db, &i),
            Err(AppError::StaleData),
            "{mutation}"
        );
        f.empty();
    }
}

#[test]
fn mode_store_selection_and_settings_save_tokens_cannot_be_reused() {
    let f = Fixture::new();
    seed(&f, &f.id, 1, A);
    f.sql("INSERT INTO image_url_settings SELECT image_id,platform,'b.myshopify.com',url,image_sha256,alt_text,updated_at FROM image_url_settings");
    let ids = vec![f.id.clone()];
    let mut i = input(&f, &ids, A);
    let original = i.snapshot_token.clone();
    assert_eq!(get(&f, &ids, " A.MYSHOPIFY.COM ").snapshot_token, original);
    assert_eq!(get(&f, &ids, A).snapshot_token, original);
    let plain_token = plain::get(&f.db, &ids).unwrap().snapshot_token;
    let setting_token = image_url_service::get(
        &f.db,
        &GetImageUrlsInput {
            product_id: f.id.clone(),
            store_key: A.into(),
        },
    )
    .unwrap()
    .snapshot_token;
    for token in [plain_token, setting_token] {
        i.snapshot_token = token;
        assert_eq!(service::preflight(&f.db, &i), Err(AppError::StaleData));
    }
    i.snapshot_token = original.clone();
    i.store_key = B.into();
    assert_eq!(service::preflight(&f.db, &i), Err(AppError::StaleData));
    let plain_input = crate::domain::shopify_export::ShopifyExportInput {
        product_ids: ids.clone(),
        snapshot_token: original,
        warning_token: None,
    };
    assert_eq!(
        plain::preflight(&f.db, &plain_input),
        Err(AppError::StaleData)
    );
    f.sql("UPDATE products SET listing_title=NULL");
    i = input(&f, &ids, A);
    i.warning_token = None;
    assert_eq!(
        service::preflight(&f.db, &i),
        Err(AppError::ExportWarningRequired)
    );
}

#[test]
fn consistent_read_never_mixes_listing_before_write_with_mapping_after_write() {
    let f = Fixture::new();
    seed(&f, &f.id, 1, A);
    f.sql("PRAGMA journal_mode=WAL");
    let old = all_settings(&f)[0].url.clone();
    let snapshot=f.db.with_mut(|db| {
        let tx=db.transaction()?;
        service::read_one_with(&tx,&f.id,A,|| {
            let mut writer=rusqlite::Connection::open(f.root.join("data/test.sqlite3")).unwrap();
            let change=writer.transaction().unwrap();
            change.execute("UPDATE products SET name='after' WHERE id=?1",[&f.id]).unwrap();
            change.execute("UPDATE image_url_settings SET url='https://cdn.shopify.com/after.png' WHERE store_key=?1",[A]).unwrap();change.commit().unwrap();
        })
    }).unwrap();
    assert_eq!(snapshot.listing.product.name, "管理名");
    assert_eq!(snapshot.settings[0].url, old);
    assert_eq!(
        get(&f, std::slice::from_ref(&f.id), A).products[0]
            .product
            .name,
        "after"
    );
    assert!(all_settings(&f)[0].url.contains("after.png"));
}

#[test]
fn final_recheck_observes_mapping_change_from_another_connection_without_holding_mutex() {
    let f = Fixture::new();
    seed(&f, &f.id, 1, A);
    let i = input(&f, std::slice::from_ref(&f.id), A);
    let _permit = f.media.begin_import().unwrap();
    let result = service::export_with(&f.db, &f.media, &i, &f.out, |_| {
        assert!(f.db.0.try_lock().is_ok());
        let db = rusqlite::Connection::open(f.root.join("data/test.sqlite3")).unwrap();
        db.execute("UPDATE image_url_settings SET url=url||'&concurrent=1'", [])
            .unwrap();
        let staging = fs::read_dir(&f.out)
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        assert!(staging.join("shopify-products.csv").exists());
    });
    assert_eq!(result.unwrap_err(), AppError::StaleData);
    f.empty();
}

#[test]
fn collision_preserves_existing_completed_output_and_cleans_only_new_workspace() {
    let f = Fixture::new();
    seed(&f, &f.id, 1, A);
    let i = input(&f, std::slice::from_ref(&f.id), A);
    let first = output(run(&f, &i).unwrap());
    let bytes = fs::read(first.join("shopify-products.csv")).unwrap();
    let _permit = f.media.begin_import().unwrap();
    let result = service::export_with(&f.db, &f.media, &i, &f.out, |target| {
        fs::create_dir(target).unwrap();
        fs::write(target.join("keep"), b"retain").unwrap();
    });
    assert_eq!(result.unwrap_err(), AppError::ExportCollision);
    assert_eq!(fs::read(first.join("shopify-products.csv")).unwrap(), bytes);
    let dirs: Vec<_> = fs::read_dir(&f.out)
        .unwrap()
        .map(|p| p.unwrap().path())
        .collect();
    assert_eq!(dirs.len(), 2);
    assert!(dirs.iter().any(|p| p.join("keep").exists()));
}

#[test]
fn cleanup_failure_keeps_unknown_file_and_reports_original_error() {
    let f = Fixture::new();
    seed(&f, &f.id, 1, A);
    let i = input(&f, std::slice::from_ref(&f.id), A);
    let _permit = f.media.begin_import().unwrap();
    let report = service::export_with(&f.db, &f.media, &i, &f.out, |_| {
        let staging = fs::read_dir(&f.out)
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        fs::write(staging.join("unknown"), b"retain").unwrap();
        f.sql("UPDATE image_url_settings SET alt_text='changed'");
    })
    .unwrap();
    match report {
        ExportReport::Failed {
            error: AppError::StaleData,
            cleanup_failed: true,
            retained_folder,
        } => {
            let path = PathBuf::from(retained_folder);
            assert!(path.join("unknown").exists());
            assert!(!path.join("shopify-products.csv").exists());
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn uncertain_final_move_retains_csv_and_reports_unknown_without_cleanup() {
    let f = Fixture::new();
    seed(&f, &f.id, 1, A);
    let i = input(&f, std::slice::from_ref(&f.id), A);
    let moved = f.out.join("unknown");
    let _permit = f.media.begin_import().unwrap();
    let report = service::export_with(&f.db, &f.media, &i, &f.out, |_| {
        let staging = fs::read_dir(&f.out)
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        fs::rename(staging, &moved).unwrap();
    })
    .unwrap();
    assert!(matches!(
        report,
        ExportReport::Failed {
            error: AppError::ExportUnknown,
            cleanup_failed: false,
            ..
        }
    ));
    assert!(moved.join("shopify-products.csv").exists());
}

#[test]
fn ten_column_bytes_token_and_image_independence_remain_unchanged() {
    let f = Fixture::new();
    let ids = vec![f.id.clone()];
    let p = plain::get(&f.db, &ids).unwrap();
    let bytes = shopify_csv::generate(&p.products).unwrap();
    let expected = format!(
        "\"Title\",\"URL handle\",\"Description\",\"Vendor\",\"SKU\",\"Option1 name\",\"Option1 value\",\"Price\",\"Published on online store\",\"Status\"\n\"公開,\"\"タイトル\"\"\",\"lp-{}\",\"日本語\n二行目\",\"\",\"日本語SKU\",\"Title\",\"Default Title\",\"0\",\"false\",\"draft\"\n",
        f.id
    );
    assert_eq!(bytes.as_bytes(), expected.as_bytes());
    let single = preview::snapshot(&f.db, &f.id).unwrap();
    let base = preview::build_for_csv(single).snapshot_token;
    use sha2::Digest;
    let mut hash = sha2::Sha256::new();
    hash.update(b"ListingPilot-shopify-csv-v1\0");
    hash.update(f.id.as_bytes());
    hash.update(base.as_bytes());
    assert_eq!(p.snapshot_token, format!("{:x}", hash.finalize()));
    seed(&f, &f.id, 1, A);
    let before = plain::get(&f.db, &ids).unwrap().snapshot_token;
    f.sql("DELETE FROM image_url_settings");
    let p = plain::get(&f.db, &ids).unwrap();
    assert_eq!(p.snapshot_token, before);
    assert_eq!(shopify_csv::generate(&p.products).unwrap(), bytes);
    let i = crate::domain::shopify_export::ShopifyExportInput {
        product_ids: ids,
        snapshot_token: p.snapshot_token,
        warning_token: None,
    };
    let _permit = f.media.begin_import().unwrap();
    let path = output(plain::export(&f.db, &f.media, &i, &f.out).unwrap());
    assert_eq!(
        fs::read(path.join("shopify-products.csv")).unwrap(),
        bytes.as_bytes()
    );
}

#[test]
fn dedicated_inputs_reject_unknown_fields_invalid_stores_and_invalid_selection() {
    assert!(
        serde_json::from_value::<ShopifyImagePreviewInput>(
            json!({"productIds":[],"storeKey":A,"path":"secret"})
        )
        .is_err()
    );
    assert!(serde_json::from_value::<ShopifyImageExportInput>(json!({"productIds":[],"storeKey":A,"snapshotToken":"x","warningToken":null,"mode":"plain"})).is_err());
    let f = Fixture::new();
    assert_eq!(
        service::get(
            &f.db,
            &ShopifyImagePreviewInput {
                product_ids: vec![f.id.clone()],
                store_key: String::new()
            }
        )
        .err(),
        Some(AppError::InvalidShopifyStore)
    );
    for ids in [vec![], vec![f.id.clone(), f.id.clone()], vec!["bad".into()]] {
        assert_eq!(
            service::get(
                &f.db,
                &ShopifyImagePreviewInput {
                    product_ids: ids,
                    store_key: A.into()
                }
            )
            .err(),
            Some(AppError::InvalidExportSelection)
        );
    }
}

#[test]
fn unsafe_export_destination_is_rejected_before_writing_anything() {
    let f = Fixture::new();
    seed(&f, &f.id, 1, A);
    let i = input(&f, std::slice::from_ref(&f.id), A);
    let _permit = f.media.begin_import().unwrap();
    assert_eq!(
        service::export(&f.db, &f.media, &i, &f.root.join("data")).unwrap_err(),
        AppError::ExportDestination
    );
    f.empty();
}

#[test]
fn alt_scalar_boundaries_and_empty_values_round_trip_without_generation() {
    let f = Fixture::new();
    seed(&f, &f.id, 1, A);
    for alt in [
        String::new(),
        "あ".repeat(125),
        "𠮷".repeat(512),
        "e\u{301}".repeat(256),
    ] {
        f.db.with(|db| {
            db.execute("UPDATE image_url_settings SET alt_text=?1", [&alt])?;
            Ok(())
        })
        .unwrap();
        let p = get(&f, std::slice::from_ref(&f.id), A);
        assert!(!p.blocked);
        assert_eq!(parse(&csv::generate(&p.products).unwrap())[1][12], alt);
    }
}

#[test]
fn invalid_image_uuid_membership_and_malformed_equal_hashes_are_blocked() {
    for mode in ["uuid", "membership", "hash"] {
        let f = Fixture::new();
        seed(&f, &f.id, 1, A);
        let mut snapshot =
            f.db.with_mut(|db| {
                let tx = db.transaction()?;
                service::read_one_with(&tx, &f.id, A, || {})
            })
            .unwrap();
        match mode {
            "uuid" => snapshot.listing.images[0].id = "secret/internal/path".into(),
            "membership" => snapshot.listing.images[0].product_id = Uuid::new_v4().to_string(),
            _ => {
                snapshot.listing.images[0].sha256 = "g".repeat(64);
                snapshot.settings[0].image_sha256 = "g".repeat(64);
            }
        }
        let p = service::build(&[snapshot], A).unwrap();
        assert!(p.blocked);
        assert!(
            !serde_json::to_string(&p)
                .unwrap()
                .contains("secret/internal/path")
        );
    }
}

#[test]
fn tied_sort_order_and_created_at_use_image_uuid_as_final_order() {
    let f = Fixture::new();
    seed(&f, &f.id, 4, A);
    f.sql("UPDATE product_images SET sort_order=0,created_at='same'");
    let p = get(&f, std::slice::from_ref(&f.id), A);
    let ids: Vec<_> = p.products[0]
        .images
        .iter()
        .map(|i| i.image_id.clone())
        .collect();
    let mut sorted = ids.clone();
    sorted.sort();
    assert_eq!(ids, sorted);
    assert_eq!(
        p.products[0]
            .images
            .iter()
            .map(|i| i.position)
            .collect::<Vec<_>>(),
        vec![1, 2, 3, 4]
    );
}

#[test]
fn final_recheck_covers_product_alt_images_and_mapping_presence() {
    for sql in [
        "UPDATE products SET name='changed'",
        "UPDATE image_url_settings SET alt_text='changed'",
        "DELETE FROM image_url_settings",
        "UPDATE product_images SET sort_order=sort_order+1",
    ] {
        let f = Fixture::new();
        seed(&f, &f.id, 1, A);
        let i = input(&f, std::slice::from_ref(&f.id), A);
        let _permit = f.media.begin_import().unwrap();
        let result = service::export_with(&f.db, &f.media, &i, &f.out, |_| f.sql(sql));
        assert_eq!(result.unwrap_err(), AppError::StaleData);
        f.empty();
    }
}

#[test]
fn changed_selection_and_old_warning_token_are_rejected() {
    let f = Fixture::new();
    seed(&f, &f.id, 1, A);
    f.sql("UPDATE products SET listing_title=NULL");
    let mut i = input(&f, std::slice::from_ref(&f.id), A);
    let old = i.snapshot_token.clone();
    f.sql("UPDATE image_url_settings SET alt_text='changed'");
    i.snapshot_token = get(&f, &i.product_ids, A).snapshot_token;
    assert_ne!(i.snapshot_token, old);
    assert_eq!(
        service::preflight(&f.db, &i),
        Err(AppError::ExportWarningRequired)
    );
    let other =
        f.db.with(|db| {
            product_service::create(
                db,
                serde_json::from_value(json!({"sku":"other","name":"他","price_yen":1})).unwrap(),
            )
            .map(|p| p.id)
        })
        .unwrap();
    seed(&f, &other, 1, A);
    i = input(&f, std::slice::from_ref(&f.id), A);
    i.product_ids.push(other);
    assert_eq!(service::preflight(&f.db, &i), Err(AppError::StaleData));
}

#[test]
fn twenty_products_supported_and_twenty_one_rejected() {
    let f = Fixture::new();
    let mut ids = vec![f.id.clone()];
    seed(&f, &f.id, 1, A);
    for i in 1..20 {
        let id =
            f.db.with(|db| {
                product_service::create(
                    db,
                    serde_json::from_value(
                        json!({"sku":format!("sku-{i}"),"name":"他","price_yen":1}),
                    )
                    .unwrap(),
                )
                .map(|p| p.id)
            })
            .unwrap();
        seed(&f, &id, 1, A);
        ids.push(id);
    }
    let p = get(&f, &ids, A);
    assert_eq!(p.products.len(), 20);
    assert_eq!(parse(&csv::generate(&p.products).unwrap()).len(), 21);
    ids.push(Uuid::new_v4().to_string());
    assert_eq!(
        service::get(
            &f.db,
            &ShopifyImagePreviewInput {
                product_ids: ids,
                store_key: A.into()
            }
        )
        .err(),
        Some(AppError::ExportProductLimit)
    );
}

#[test]
fn saved_url_4096_bytes_is_accepted_4097_is_blocked() {
    let f = Fixture::new();
    seed(&f, &f.id, 1, A);
    let prefix = "https://cdn.shopify.com/";
    let exact = format!("{prefix}{}", "a".repeat(4096 - prefix.len()));
    f.db.with(|db| {
        db.execute("UPDATE image_url_settings SET url=?1", [&exact])?;
        Ok(())
    })
    .unwrap();
    let p = get(&f, std::slice::from_ref(&f.id), A);
    assert!(!p.blocked);
    assert_eq!(parse(&csv::generate(&p.products).unwrap())[1][10], exact);
    f.sql("PRAGMA ignore_check_constraints=ON;UPDATE image_url_settings SET url=url||'a';PRAGMA ignore_check_constraints=OFF;");
    let p = get(&f, std::slice::from_ref(&f.id), A);
    assert!(p.blocked);
    assert_eq!(
        p.products[0].image_issues[0].code,
        ImageExportIssueCode::InvalidUrl
    );
}
