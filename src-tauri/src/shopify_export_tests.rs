use super::*;
use crate::{
    domain::{
        listing_preview::{IssueCode, ListingStatus},
        shopify_export::*,
    },
    services::{batch_export_service, shopify_csv as csv, shopify_export_service as shopify},
};

fn input(f: &Fixture, ids: &[String]) -> ShopifyExportInput {
    let p = shopify::get(&f.db, ids).unwrap();
    ShopifyExportInput {
        product_ids: ids.to_vec(),
        snapshot_token: p.snapshot_token.clone(),
        warning_token: Some(p.snapshot_token),
    }
}
fn run(f: &Fixture, i: &ShopifyExportInput) -> Result<ExportReport, AppError> {
    let _permit = f.media.begin_import()?;
    shopify::export(&f.db, &f.media, i, &f.out)
}
fn second(f: &Fixture) -> String {
    f.db.with(|db| {
        product_service::create(
            db,
            serde_json::from_value(json!({"sku":"second", "name":"管理用二番目", "price_yen":42}))
                .unwrap(),
        )
        .map(|p| p.id)
    })
    .unwrap()
}
// Independent CSV reader used to assert actual field round trips, including multiline cells.
fn parse_csv(value: &str) -> Vec<Vec<String>> {
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
    assert!(!quoted);
    assert!(cell.is_empty());
    assert!(row.is_empty());
    rows
}

#[test]
fn two_products_one_utf8_lf_csv_exact_headers_and_round_trip_without_private_data() {
    let f = Fixture::new();
    let other = second(&f);
    f.sql("UPDATE products SET brand='日本語,\"ブランド\"' WHERE sku='日本語SKU'");
    let ids = vec![other.clone(), f.id.clone()];
    let i = input(&f, &ids);
    let before = f.dump();
    let report = run(&f, &i).unwrap();
    let output = match report {
        ExportReport::Success {
            folder,
            product_count: 2,
            attribute_count: 0,
            image_count: 0,
            has_warnings: true,
        } => PathBuf::from(folder),
        other => panic!("{other:?}"),
    };
    assert_eq!(fs::read_dir(&output).unwrap().count(), 1);
    assert!(
        output
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("ListingPilot_Shopify_")
    );
    let bytes = fs::read(output.join("shopify-products.csv")).unwrap();
    assert!(!bytes.starts_with(&[0xef, 0xbb, 0xbf]));
    assert!(!bytes.contains(&b'\r'));
    let text = String::from_utf8(bytes).unwrap();
    let rows = parse_csv(&text);
    assert_eq!(rows.len(), 3);
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
            "Status"
        ]
    );
    let mut sorted = ids;
    sorted.sort();
    for (id, row) in sorted.iter().zip(&rows[1..]) {
        assert_eq!(row.len(), 10);
        assert_eq!(row[1], format!("lp-{id}"));
        assert_eq!(&row[5..7], ["Title", "Default Title"]);
        assert_eq!(&row[8..10], ["false", "draft"]);
        if id == &f.id {
            assert_eq!(row[0], "公開,\"タイトル\"");
            assert_eq!(row[2], "日本語\n二行目");
            assert_eq!(row[3], "日本語,\"ブランド\"");
            assert_eq!(row[4], "日本語SKU");
            assert_eq!(row[7], "0");
        } else {
            assert_eq!(row[0], "管理用二番目");
            assert_eq!(row[2], "");
            assert_eq!(row[3], "");
            assert_eq!(row[7], "42");
        }
    }
    assert!(!text.contains("極秘メモ"));
    assert!(!text.contains("group"));
    assert!(!text.contains("media/"));
    assert_eq!(f.dump(), before);
    let second = match run(&f, &i).unwrap() {
        ExportReport::Success { folder, .. } => PathBuf::from(folder),
        r => panic!("{r:?}"),
    };
    assert_ne!(output, second);
    assert_eq!(
        fs::read(second.join("shopify-products.csv")).unwrap(),
        text.as_bytes()
    );
    // Existing generic bytes remain unchanged.
    let generic = folder(f.run(&f.input()).unwrap());
    let bytes = fs::read(generic.join("product.csv")).unwrap();
    assert!(bytes.starts_with(&[0xef, 0xbb, 0xbf]));
    assert!(bytes.ends_with(b"\r\n"));
}

#[test]
fn fallback_description_warning_null_price_blocked_and_zero_valid() {
    let f = Fixture::new();
    f.sql("UPDATE products SET listing_title=NULL,listing_description=NULL,price_yen=NULL");
    let ids = vec![f.id.clone()];
    let p = shopify::get(&f.db, &ids).unwrap();
    assert!(p.blocked && p.has_warnings);
    let product = &p.products[0];
    assert_eq!(product.title, "管理名");
    assert_eq!(product.description, None);
    assert_eq!(product.vendor, None);
    assert_eq!(product.status, ListingStatus::Blocked);
    assert_eq!(product.issues.len(), 3);
    assert!(
        product
            .issues
            .iter()
            .any(|i| i.code == IssueCode::TitleFallback)
    );
    let i = input(&f, &ids);
    assert_eq!(run(&f, &i).unwrap_err(), AppError::ShopifyExportBlocked);
    f.empty();
    f.sql("UPDATE products SET price_yen=0");
    let p = shopify::get(&f.db, &ids).unwrap();
    assert!(!p.blocked && p.has_warnings);
    let mut i = input(&f, &ids);
    i.warning_token = None;
    assert_eq!(
        shopify::preflight(&f.db, &i).unwrap_err(),
        AppError::ExportWarningRequired
    );
    i.warning_token = Some("old".into());
    assert_eq!(run(&f, &i).unwrap_err(), AppError::ExportWarningRequired);
    f.empty();
    i.warning_token = Some(i.snapshot_token.clone());
    folder(run(&f, &i).unwrap());
}

#[test]
fn selection_validation_and_unknown_input_fields() {
    let f = Fixture::new();
    for ids in [
        vec![],
        vec![f.id.clone(), f.id.clone()],
        vec!["../bad".into()],
        vec![f.id.to_uppercase()],
        vec![Uuid::nil().simple().to_string()],
    ] {
        assert_eq!(
            shopify::get(&f.db, &ids).unwrap_err(),
            AppError::InvalidExportSelection
        );
    }
    assert_eq!(
        shopify::get(&f.db, &[Uuid::new_v4().to_string()]).unwrap_err(),
        AppError::ProductNotFound
    );
    assert_eq!(
        shopify::get(
            &f.db,
            &(0..21)
                .map(|_| Uuid::new_v4().to_string())
                .collect::<Vec<_>>()
        )
        .unwrap_err(),
        AppError::ExportProductLimit
    );
    assert!(
        serde_json::from_value::<ShopifyPreviewInput>(json!({"productIds":[f.id],"path":"bad"}))
            .is_err()
    );
    assert!(
        serde_json::from_value::<ShopifyExportInput>(
            json!({"productIds":[f.id],"snapshotToken":"a","warningToken":null,"path":"bad"})
        )
        .is_err()
    );
    f.empty();
}

#[test]
fn twenty_products_supported_in_canonical_order() {
    let f = Fixture::new();
    let mut ids = vec![f.id.clone()];
    for n in 0..19 {
        ids.push(
            f.db.with(|db| {
                product_service::create(
                    db,
                    serde_json::from_value(
                        json!({"sku":format!("sku-{n}"),"name":"商品","price_yen":1}),
                    )
                    .unwrap(),
                )
                .map(|p| p.id)
            })
            .unwrap(),
        );
    }
    let p = shopify::get(&f.db, &ids).unwrap();
    ids.sort();
    assert_eq!(
        p.products
            .iter()
            .map(|p| p.product_id.clone())
            .collect::<Vec<_>>(),
        ids
    );
    assert_eq!(parse_csv(&csv::generate(&p.products).unwrap()).len(), 21);
}

#[test]
fn formula_checks_preview_and_execution_for_all_text_columns_and_line_boundaries() {
    for (field, column) in [
        ("sku", "SKU"),
        ("listing_title", "Title"),
        ("brand", "Vendor"),
        ("listing_description", "Description"),
    ] {
        for value in [
            "=secret",
            "  +secret",
            "\u{feff}\u{200b}-secret",
            "\t@secret",
            "日本語\n \u{2060}=secret",
            "ok\r+secret",
        ] {
            let f = Fixture::new();
            let ids = vec![f.id.clone()];
            let i = input(&f, &ids);
            let expected = AppError::ShopifyCsvFormulaRisk {
                product_id: f.id.clone(),
                column: column.into(),
            };
            // Exercise leading ASCII spaces too, even though normal persistence trims them.
            let mut snapshot = preview::snapshot(&f.db, &f.id).unwrap();
            match field {
                "sku" => snapshot.product.sku = value.into(),
                "listing_title" => snapshot.product.listing_title = Some(value.into()),
                "brand" => snapshot.product.brand = Some(value.into()),
                "listing_description" => snapshot.product.listing_description = Some(value.into()),
                _ => unreachable!(),
            }
            assert_eq!(shopify::build(&[snapshot]).unwrap_err(), expected);
            if value.starts_with(' ') {
                continue;
            }
            f.db.with(|db| {
                db.execute(&format!("UPDATE products SET {field}=?1"), [value])?;
                Ok(())
            })
            .unwrap();
            assert_eq!(shopify::get(&f.db, &ids).unwrap_err(), expected);
            assert_eq!(shopify::preflight(&f.db, &i).unwrap_err(), expected);
            assert_eq!(run(&f, &i).unwrap_err(), expected);
            f.empty();
            let error = serde_json::to_string(&expected).unwrap();
            assert!(!error.contains("secret"));
            assert!(!error.contains("media"));
        }
    }
    let f = Fixture::new();
    f.sql("UPDATE products SET listing_title=NULL,name='=secret'");
    assert!(
        matches!(shopify::get(&f.db,std::slice::from_ref(&f.id)),Err(AppError::ShopifyCsvFormulaRisk{column,..}) if column=="Title")
    );
}

#[test]
fn handles_stable_but_confirmation_changes_with_name_sku_price_vendor_and_public_text() {
    for sql in [
        "UPDATE products SET name='new'",
        "UPDATE products SET sku='new'",
        "UPDATE products SET price_yen=8",
        "UPDATE products SET brand='new'",
        "UPDATE products SET listing_title='new'",
        "UPDATE products SET listing_description='new'",
    ] {
        let f = Fixture::new();
        let ids = vec![f.id.clone()];
        let i = input(&f, &ids);
        let p = shopify::get(&f.db, &ids).unwrap();
        f.sql(sql);
        let next = shopify::get(&f.db, &ids).unwrap();
        assert_eq!(next.products[0].handle, p.products[0].handle);
        assert_ne!(next.snapshot_token, p.snapshot_token);
        assert_eq!(
            shopify::preflight(&f.db, &i).unwrap_err(),
            AppError::StaleData
        );
        f.empty();
    }
}

#[test]
fn order_independent_set_bound_tokens_reject_generic_domains_and_warning_tokens() {
    let f = Fixture::new();
    let other = second(&f);
    let ids = vec![f.id.clone(), other.clone()];
    let mut i = input(&f, &ids);
    assert_eq!(
        i.snapshot_token,
        input(&f, &[other, f.id.clone()]).snapshot_token
    );
    let single = f.input();
    let batch = batch_export_service::get(&f.db, &f.media, &ids).unwrap();
    assert_ne!(i.snapshot_token, single.snapshot_token);
    assert_ne!(i.snapshot_token, batch.snapshot_token);
    for token in [single.snapshot_token, batch.snapshot_token] {
        i.warning_token = Some(token.clone());
        assert_eq!(
            shopify::preflight(&f.db, &i).unwrap_err(),
            AppError::ExportWarningRequired
        );
        let own = std::mem::replace(&mut i.snapshot_token, token);
        assert_eq!(
            shopify::preflight(&f.db, &i).unwrap_err(),
            AppError::StaleData
        );
        i.snapshot_token = own;
    }
    i.warning_token = Some(i.snapshot_token.clone());
    i.product_ids.pop();
    assert_eq!(run(&f, &i).unwrap_err(), AppError::StaleData);
    f.empty();
}

#[test]
fn missing_images_and_generic_image_limits_never_block_shopify_or_copy_images() {
    let f = Fixture::new();
    f.image("png", 1);
    let image =
        f.db.with(|db| image_repository::list(db, &f.id))
            .unwrap()
            .remove(0);
    let path = f.media.image_path(&image).unwrap();
    let bytes = fs::read(&path).unwrap();
    fs::remove_file(&path).unwrap();
    let ids = vec![f.id.clone()];
    let p = shopify::get(&f.db, &ids).unwrap();
    assert!(!p.blocked && !p.has_warnings);
    assert!(p.products[0].issues.is_empty());
    assert!(
        batch_export_service::get(&f.db, &f.media, &ids)
            .unwrap()
            .blocked
    );
    let i = input(&f, &ids);
    let output = folder(run(&f, &i).unwrap());
    assert_eq!(fs::read_dir(output).unwrap().count(), 1);
    fs::write(&path, &bytes).unwrap();
    // File availability is deliberately not an input to the CSV-only candidate token.
    assert_eq!(
        shopify::get(&f.db, &ids).unwrap().snapshot_token,
        p.snapshot_token
    );
    let mut snapshot = preview::snapshot(&f.db, &f.id).unwrap();
    let mut large = image;
    large.file_size_bytes = 2 * 1024 * 1024 * 1024;
    snapshot.images = vec![large; 201];
    let p = shopify::build(&[snapshot]).unwrap();
    assert!(!p.blocked);
    csv::generate(&p.products).unwrap();
    assert_eq!(fs::read(path).unwrap(), bytes);
}

#[test]
fn stale_after_dialog_rejected_before_workspace_is_created() {
    let f = Fixture::new();
    let i = input(&f, std::slice::from_ref(&f.id));
    shopify::preflight(&f.db, &i).unwrap();
    let db = rusqlite::Connection::open(f.root.join("data/test.sqlite3")).unwrap();
    db.execute("UPDATE products SET listing_title='during dialog'", [])
        .unwrap();
    assert_eq!(run(&f, &i).unwrap_err(), AppError::StaleData);
    f.empty();
}

#[test]
fn final_recheck_observes_separate_connection_and_releases_mutex_during_csv_io() {
    let f = Fixture::new();
    let other = second(&f);
    let i = input(&f, &[f.id.clone(), other.clone()]);
    let _permit = f.media.begin_import().unwrap();
    let result = shopify::export_with(&f.db, &f.media, &i, &f.out, || {
        assert!(f.db.0.try_lock().is_ok());
        let db = rusqlite::Connection::open(f.root.join("data/test.sqlite3")).unwrap();
        db.execute("UPDATE products SET price_yen=77 WHERE id=?1", [other])
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
fn csv_size_limit_is_utf8_byte_length_and_strictly_below_fifteen_mb() {
    assert!(csv::check_size(csv::MAX_BYTES - 1).is_ok());
    assert_eq!(
        csv::check_size(csv::MAX_BYTES).unwrap_err(),
        AppError::ShopifyCsvSizeLimit
    );
    let f = Fixture::new();
    let mut snapshots = vec![preview::snapshot(&f.db, &f.id).unwrap()];
    snapshots[0].product.listing_description = Some("あ".repeat(csv::MAX_BYTES / 3));
    assert_eq!(
        shopify::build(&snapshots).unwrap_err(),
        AppError::ShopifyCsvSizeLimit
    );
}

#[test]
fn fixed_csv_root_only_create_new_and_existing_final_folder_preserved() {
    let f = Fixture::new();
    let mut w = Workspace::new(&f.out).unwrap();
    let id = Uuid::new_v4().to_string();
    w.product(&id).unwrap();
    for name in [
        format!("{id}/shopify-products.csv"),
        "images/shopify-products.csv".into(),
        "../shopify-products.csv".into(),
        "shopify-products.csv:stream".into(),
        "shopify-products.csv/extra".into(),
    ] {
        assert!(w.file(&name).is_err());
    }
    drop(w.file("shopify-products.csv").unwrap());
    assert!(w.file("shopify-products.csv").is_err());
    let target = f.out.join("existing");
    fs::create_dir(&target).unwrap();
    fs::write(target.join("keep"), b"preserve").unwrap();
    assert_eq!(w.finish(&target).unwrap_err(), AppError::ExportCollision);
    w.cleanup().unwrap();
    assert_eq!(fs::read(target.join("keep")).unwrap(), b"preserve");
}

#[test]
fn cleanup_failure_retains_unknown_files_with_original_error() {
    let f = Fixture::new();
    let i = input(&f, std::slice::from_ref(&f.id));
    let _permit = f.media.begin_import().unwrap();
    let report = shopify::export_with(&f.db, &f.media, &i, &f.out, || {
        let staging = fs::read_dir(&f.out)
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        fs::write(staging.join("unknown"), b"retain").unwrap();
        f.sql("UPDATE products SET price_yen=7");
    })
    .unwrap();
    match report {
        ExportReport::Failed {
            error: AppError::StaleData,
            retained_folder,
            cleanup_failed: true,
        } => {
            let path = PathBuf::from(retained_folder);
            assert_eq!(fs::read(path.join("unknown")).unwrap(), b"retain");
            assert!(!path.join("shopify-products.csv").exists());
        }
        other => panic!("{other:?}"),
    }
    assert!(!fs::read_dir(&f.out).unwrap().any(|e| {
        e.unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with("ListingPilot_Shopify_")
    }));
}

#[test]
fn uncertain_finish_retains_csv_and_reports_unknown() {
    let f = Fixture::new();
    let i = input(&f, std::slice::from_ref(&f.id));
    let _permit = f.media.begin_import().unwrap();
    let moved = f.out.join("unknown-move");
    let report = shopify::export_with(&f.db, &f.media, &i, &f.out, || {
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
fn app_data_destination_rejected_without_writes() {
    let f = Fixture::new();
    let i = input(&f, std::slice::from_ref(&f.id));
    let _permit = f.media.begin_import().unwrap();
    assert_eq!(
        shopify::export(&f.db, &f.media, &i, &f.root.join("data")).unwrap_err(),
        AppError::ExportDestination
    );
    f.empty();
}
