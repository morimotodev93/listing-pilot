use super::*;
use crate::{domain::batch_export::BatchExportInput, services::batch_export_service as batch};

fn second(f: &Fixture) -> String {
    f.db.with(|db| {
        product_service::create(db, serde_json::from_value(json!({"sku":"second", "name":"second", "listing_title":"second", "listing_description":"description", "price_yen":42})).unwrap()).map(|p| p.id)
    }).unwrap()
}
fn input(f: &Fixture, ids: &[String]) -> BatchExportInput {
    let _permit = f.media.begin_import().unwrap();
    let p = batch::get(&f.db, &f.media, ids).unwrap();
    BatchExportInput {
        product_ids: ids.to_vec(),
        snapshot_token: p.snapshot_token.clone(),
        warning_token: Some(p.snapshot_token),
    }
}
fn run(f: &Fixture, input: &BatchExportInput) -> Result<ExportReport, AppError> {
    let _permit = f.media.begin_import()?;
    batch::export(&f.db, &f.media, input, &f.out)
}
fn batch_folder(report: ExportReport) -> PathBuf {
    match report {
        ExportReport::Success {
            folder,
            product_count: 2,
            ..
        } => folder.into(),
        other => panic!("{other:?}"),
    }
}

#[test]
fn invalid_empty_duplicate_noncanonical_missing_and_limit_selection() {
    let f = Fixture::new();
    for ids in [
        vec![],
        vec![f.id.clone(), f.id.clone()],
        vec!["../bad".into()],
        vec![f.id.to_uppercase()],
        vec![Uuid::nil().simple().to_string()],
    ] {
        assert_eq!(
            batch::get(&f.db, &f.media, &ids).unwrap_err(),
            AppError::InvalidExportSelection
        );
    }
    assert_eq!(
        batch::get(&f.db, &f.media, &[Uuid::new_v4().to_string()]).unwrap_err(),
        AppError::ProductNotFound
    );
    let ids = (0..21)
        .map(|_| Uuid::new_v4().to_string())
        .collect::<Vec<_>>();
    assert_eq!(
        batch::normalize(&ids).unwrap_err(),
        AppError::ExportProductLimit
    );
    assert_eq!(batch::normalize(&ids[..20]).unwrap().len(), 20);
    f.empty();
}

#[test]
fn order_independent_confirmation_set_changes_and_warning_binding() {
    let f = Fixture::new();
    let other = second(&f);
    let ids = vec![f.id.clone(), other.clone()];
    let mut i = input(&f, &ids);
    let reverse = input(&f, &[other, f.id.clone()]);
    assert_eq!(i.snapshot_token, reverse.snapshot_token);
    i.warning_token = None;
    assert_eq!(run(&f, &i).unwrap_err(), AppError::ExportWarningRequired);
    i.warning_token = Some("old".into());
    assert_eq!(run(&f, &i).unwrap_err(), AppError::ExportWarningRequired);
    i.warning_token = Some(i.snapshot_token.clone());
    i.product_ids.pop();
    assert_eq!(run(&f, &i).unwrap_err(), AppError::StaleData);
    f.empty();
    batch_folder(run(&f, &reverse).unwrap());
}

#[test]
fn image_count_and_byte_limits_boundaries_and_invalid_sizes() {
    let f = Fixture::new();
    f.image("png", 1);
    let mut s = preview::snapshot(&f.db, &f.id).unwrap();
    let mut image = s.images[0].clone();
    image.file_size_bytes = 1;
    s.images = vec![image; 200];
    assert_eq!(batch::limits(&[s.clone()]).unwrap(), 200);
    s.images.push(s.images[0].clone());
    assert_eq!(
        batch::limits(&[s.clone()]).unwrap_err(),
        AppError::ExportImageLimit
    );
    s.images.truncate(1);
    s.images[0].file_size_bytes = batch::MAX_BATCH_BYTES as i64;
    assert_eq!(batch::limits(&[s.clone()]).unwrap(), 1);
    s.images[0].file_size_bytes += 1;
    assert_eq!(
        batch::limits(&[s.clone()]).unwrap_err(),
        AppError::ExportSizeLimit
    );
    for size in [0, -1] {
        s.images[0].file_size_bytes = size;
        assert_eq!(
            batch::limits(&[s.clone()]).unwrap_err(),
            AppError::ExportImageInvalid
        );
    }
    s.images[0].file_size_bytes = i64::MAX;
    assert_eq!(batch::limits(&[s]).unwrap_err(), AppError::ExportSizeLimit);
}

#[test]
fn batch_csv_matches_single_byte_for_byte_and_new_batch_uuid_each_run() {
    let f = Fixture::new();
    let other = second(&f);
    let before = f.dump();
    let i = input(&f, &[f.id.clone(), other]);
    let a = batch_folder(run(&f, &i).unwrap());
    let b = batch_folder(run(&f, &i).unwrap());
    assert_ne!(a, b);
    let single = folder(f.run(&f.input()).unwrap());
    for name in ["product.csv", "attributes.csv", "images.csv"] {
        assert_eq!(
            fs::read(a.join(&f.id).join(name)).unwrap(),
            fs::read(single.join(name)).unwrap()
        );
    }
    assert_eq!(fs::read_dir(&a).unwrap().count(), 2);
    assert!(
        a.file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("ListingPilot_Batch_")
    );
    assert_eq!(f.dump(), before);
}

#[test]
fn ready_warning_mix_preserves_original_formats_order_and_main_image() {
    let f = Fixture::new();
    let other = second(&f);
    for (color, ext) in ["jpg", "png", "webp"].iter().enumerate() {
        f.image(ext, color as u8);
    }
    f.sql("UPDATE product_images SET sort_order=7-sort_order");
    let before = f.dump();
    let i = input(&f, &[other.clone(), f.id.clone()]);
    let p = batch::get(&f.db, &f.media, &i.product_ids).unwrap();
    assert!(p.has_warnings);
    assert!(!p.blocked);
    assert_eq!(p.image_count, 3);
    assert!(
        p.products
            .iter()
            .any(|p| p.preview.status == crate::domain::listing_preview::ListingStatus::Ready)
    );
    let out = batch_folder(run(&f, &i).unwrap());
    let images = f.db.with(|db| image_repository::list(db, &f.id)).unwrap();
    let image_csv = fs::read_to_string(out.join(&f.id).join("images.csv")).unwrap();
    for (index, image) in images.iter().enumerate() {
        let original = f.media.image_path(image).unwrap();
        let name = format!(
            "images/{:04}-{}.{}",
            index + 1,
            image.id,
            original.extension().unwrap().to_str().unwrap()
        );
        assert_eq!(
            fs::read(original).unwrap(),
            fs::read(out.join(&f.id).join(&name)).unwrap()
        );
        assert!(image_csv.contains(&format!("\"{index}\",\"{name}\",\"{}\"", image.sha256)));
        if index == 0 {
            assert!(
                fs::read_to_string(out.join(&f.id).join("product.csv"))
                    .unwrap()
                    .contains(&name)
            );
        }
    }
    assert_eq!(
        fs::read_dir(out.join(other).join("images"))
            .unwrap()
            .count(),
        0
    );
    assert_eq!(f.dump(), before);
}

#[test]
fn separate_group_same_names_old_and_archived_values_excluded_precision_preserved() {
    let f = Fixture::new();
    let other = second(&f);
    let group = Uuid::new_v4().to_string();
    let a = Uuid::new_v4().to_string();
    let b = Uuid::new_v4().to_string();
    let c = Uuid::new_v4().to_string();
    let default = crate::domain::attribute_group::DEFAULT_GROUP_ID;
    f.sql(&format!("INSERT INTO attribute_groups VALUES('{group}','group',NULL,'c','u'); UPDATE products SET group_id='{group}' WHERE id='{other}';
        INSERT INTO attribute_definitions VALUES('{a}','same','number','cm',NULL,'c','u','{default}'),('{b}','same','text',NULL,NULL,'c','u','{group}'),('{c}','hidden','text',NULL,'archived','c','u','{group}');
        INSERT INTO product_attribute_values VALUES('{}','{a}','-000123.450000'),('{other}','{a}','123'),('{other}','{b}','current'),('{other}','{c}','=danger');", f.id));
    let before = f.dump();
    let out = batch_folder(run(&f, &input(&f, &[f.id.clone(), other.clone()])).unwrap());
    let first = fs::read_to_string(out.join(&f.id).join("attributes.csv")).unwrap();
    let second = fs::read_to_string(out.join(other).join("attributes.csv")).unwrap();
    assert!(first.contains("-000123.450000"));
    assert!(first.contains(&a));
    assert!(!first.contains(&b));
    assert!(second.contains(&b));
    assert!(second.contains("current"));
    assert!(!second.contains(&a));
    assert!(!second.contains(&c));
    assert_eq!(before, f.dump());
}

#[test]
fn csv_formula_in_any_product_aborts_all_before_dialog_or_staging() {
    let f = Fixture::new();
    let other = second(&f);
    f.sql(&format!(
        "UPDATE products SET listing_title='=formula' WHERE id='{other}'"
    ));
    let before = f.dump();
    let i = input(&f, &[f.id.clone(), other.clone()]);
    let expected = AppError::BatchCsvFormulaRisk {
        product_id: other,
        column: "product.title".into(),
    };
    assert_eq!(batch::preflight(&f.db, &f.media, &i).unwrap_err(), expected);
    assert_eq!(run(&f, &i).unwrap_err(), expected);
    f.empty();
    assert_eq!(f.dump(), before);
}

#[test]
fn batch_formula_error_identifies_product_and_column_without_changing_single_contract() {
    for (field, column) in [
        ("sku", "product.sku"),
        ("listing_description", "product.description"),
    ] {
        let f = Fixture::new();
        let other = second(&f);
        f.sql(&format!(
            "UPDATE products SET {field}='=private-formula' WHERE id='{other}'"
        ));
        let i = input(&f, &[other.clone(), f.id.clone()]);
        let error = run(&f, &i).unwrap_err();
        assert_eq!(
            serde_json::to_value(&error).unwrap(),
            json!({
                "code": "batch_csv_formula_risk", "product_id": other, "column": column
            })
        );
        let single = preview::get(&f.db, &f.media, &other).unwrap();
        let single_input = ExportInput {
            product_id: other,
            snapshot_token: single.snapshot_token.clone(),
            warning_token: Some(single.snapshot_token),
        };
        assert_eq!(
            serde_json::to_value(f.run(&single_input).unwrap_err()).unwrap(),
            json!({
                "code": "csv_formula_risk", "column": column
            })
        );
        f.empty();
    }
}

#[test]
fn blocked_member_stops_entire_batch_and_image_recovery_invalidates_token() {
    let f = Fixture::new();
    let other = second(&f);
    f.image("png", 1);
    let images = f.db.with(|db| image_repository::list(db, &f.id)).unwrap();
    let path = f.media.image_path(&images[0]).unwrap();
    let bytes = fs::read(&path).unwrap();
    fs::remove_file(&path).unwrap();
    let ids = vec![f.id.clone(), other];
    let i = input(&f, &ids);
    assert!(batch::get(&f.db, &f.media, &ids).unwrap().blocked);
    assert_eq!(run(&f, &i).unwrap_err(), AppError::ExportBlocked);
    f.empty();
    fs::write(path, bytes).unwrap();
    assert_eq!(run(&f, &i).unwrap_err(), AppError::StaleData);
    f.empty();
}

#[test]
fn metadata_and_group_mutations_invalidate_whole_batch() {
    for change in [
        "UPDATE products SET price_yen=99",
        "UPDATE product_images SET sort_order=99",
        "UPDATE attribute_groups SET name='renamed'",
        "UPDATE attribute_definitions SET name='renamed'",
        "UPDATE product_attribute_values SET value='new'",
    ] {
        let f = Fixture::new();
        let other = second(&f);
        f.image("png", 1);
        let a = Uuid::new_v4().to_string();
        let default = crate::domain::attribute_group::DEFAULT_GROUP_ID;
        f.sql(&format!("INSERT INTO attribute_definitions VALUES('{a}','field','text',NULL,NULL,'c','u','{default}'); INSERT INTO product_attribute_values VALUES('{other}','{a}','old');"));
        let i = input(&f, &[other, f.id.clone()]);
        f.sql(change);
        assert_eq!(run(&f, &i).unwrap_err(), AppError::StaleData);
        f.empty();
    }
}

#[test]
fn membership_and_definition_archive_changes_invalidate_whole_batch() {
    let f = Fixture::new();
    let other = second(&f);
    let group = Uuid::new_v4().to_string();
    f.sql(&format!(
        "INSERT INTO attribute_groups VALUES('{group}','new',NULL,'c','u');"
    ));
    let i = input(&f, &[f.id.clone(), other.clone()]);
    f.sql(&format!(
        "UPDATE products SET group_id='{group}' WHERE id='{other}'"
    ));
    assert_eq!(run(&f, &i).unwrap_err(), AppError::StaleData);
    f.empty();
}

#[test]
fn later_image_hash_failure_cleans_entire_batch_without_data_changes() {
    let f = Fixture::new();
    let other = second(&f);
    f.image("png", 1);
    for sql in [
        "UPDATE product_images SET sha256=printf('%064d',0)",
        "UPDATE product_images SET file_size_bytes=file_size_bytes+1",
    ] {
        f.sql(sql);
        let before = f.dump();
        let i = input(&f, &[f.id.clone(), other.clone()]);
        assert_eq!(run(&f, &i).unwrap_err(), AppError::ExportImageInvalid);
        f.empty();
        assert_eq!(before, f.dump());
    }
}

#[test]
fn final_recheck_rejects_changes_from_separate_connection_without_db_mutex_during_copy() {
    let f = Fixture::new();
    let other = second(&f);
    let i = input(&f, &[f.id.clone(), other]);
    let _permit = f.media.begin_import().unwrap();
    let result = batch::export_with(&f.db, &f.media, &i, &f.out, || {
        // with() would deadlock if the export retained its DB mutex while writing.
        f.db.with(|_| Ok(())).unwrap();
        let db = connection::open(&f.root.join("data/test.sqlite3")).unwrap();
        db.execute("UPDATE products SET price_yen=101", []).unwrap();
    });
    assert_eq!(result.unwrap_err(), AppError::StaleData);
    f.empty();
}

#[test]
fn cleanup_failure_retains_unknown_nested_file_and_original_error() {
    let f = Fixture::new();
    let other = second(&f);
    let i = input(&f, &[f.id.clone(), other]);
    let _permit = f.media.begin_import().unwrap();
    let result = batch::export_with(&f.db, &f.media, &i, &f.out, || {
        let staging = fs::read_dir(&f.out)
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        fs::write(staging.join(&f.id).join("unknown"), b"keep").unwrap();
        f.sql("UPDATE products SET name='changed'");
    })
    .unwrap();
    match result {
        ExportReport::Failed {
            error: AppError::StaleData,
            retained_folder,
            cleanup_failed: true,
        } => assert_eq!(
            fs::read(PathBuf::from(retained_folder).join(&f.id).join("unknown")).unwrap(),
            b"keep"
        ),
        other => panic!("{other:?}"),
    }
    assert!(!fs::read_dir(&f.out).unwrap().any(|p| {
        p.unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with("ListingPilot_Batch_")
    }));
}

#[test]
fn uncertain_finalization_retains_moved_workspace_and_never_reports_success() {
    let f = Fixture::new();
    let other = second(&f);
    let i = input(&f, &[f.id.clone(), other]);
    let moved = f.out.join("unknown-move");
    let _permit = f.media.begin_import().unwrap();
    let result = batch::export_with(&f.db, &f.media, &i, &f.out, || {
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
        result,
        ExportReport::Failed {
            error: AppError::ExportUnknown,
            cleanup_failed: false,
            ..
        }
    ));
    assert!(moved.join(&f.id).join("product.csv").exists());
}

#[test]
fn nested_workspace_rejects_paths_unknown_directories_and_keeps_collisions() {
    let f = Fixture::new();
    let id = Uuid::new_v4().to_string();
    let mut w = Workspace::new(&f.out).unwrap();
    assert!(w.product("../bad").is_err());
    w.product(&id).unwrap();
    for name in [
        "../product.csv",
        "images/../product.csv",
        "C:\\product.csv",
        "images\\bad.png",
        "unknown.csv",
        "product.csv:stream",
        "images/0001-not-a-uuid.png",
    ] {
        assert!(w.file(name).is_err(), "{name}");
    }
    fs::create_dir(w.path.join("unknown")).unwrap();
    assert!(w.file("unknown/product.csv").is_err());
    drop(w.file(&format!("{id}/product.csv")).unwrap());
    assert!(w.file(&format!("{id}/product.csv")).is_err());
    let target = f.out.join("existing");
    fs::create_dir(&target).unwrap();
    fs::write(target.join("keep"), b"keep").unwrap();
    assert_eq!(w.finish(&target).unwrap_err(), AppError::ExportCollision);
    assert!(w.cleanup().is_err());
    assert!(w.path.join("unknown").exists());
    assert_eq!(fs::read(target.join("keep")).unwrap(), b"keep");
}

#[cfg(windows)]
#[test]
fn nested_junction_is_never_written_through_or_cleaned_recursively() {
    use std::os::windows::process::CommandExt;
    let f = Fixture::new();
    let mut w = Workspace::new(&f.out).unwrap();
    let id = Uuid::new_v4().to_string();
    w.product(&id).unwrap();
    let images = w.path.join(&id).join("images");
    fs::remove_dir(&images).unwrap();
    let outside = f.root.join("outside");
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("keep"), b"preserve").unwrap();
    let result = std::process::Command::new("cmd")
        .args(["/d", "/c", "mklink", "/J"])
        .arg(&images)
        .arg(&outside)
        .creation_flags(0x08000000)
        .output()
        .unwrap();
    assert!(result.status.success());
    let name = format!("{id}/images/0001-{}.png", Uuid::new_v4());
    assert!(w.file(&name).is_err());
    assert!(w.cleanup().is_err());
    assert_eq!(fs::read(outside.join("keep")).unwrap(), b"preserve");
    assert_eq!(fs::read_dir(&outside).unwrap().count(), 1);
    // Only remove the test-created junction itself; never its target.
    fs::remove_dir(&images).unwrap();
    fs::create_dir(&images).unwrap();
    w.cleanup().unwrap();
}

#[test]
fn second_product_copy_failure_removes_already_written_first_product() {
    let f = Fixture::new();
    let other = second(&f);
    let mut ids = vec![f.id.clone(), other];
    ids.sort();
    let source = f.root.join("second.png");
    image::RgbImage::from_pixel(2, 2, image::Rgb([5, 6, 7]))
        .save(&source)
        .unwrap();
    image_service::import_files(&f.db, &f.media, &ids[1], &[source]).unwrap();
    f.sql("UPDATE product_images SET sha256=printf('%064d',0)");
    let before = f.dump();
    let i = input(&f, &ids);
    assert_eq!(run(&f, &i).unwrap_err(), AppError::ExportImageInvalid);
    f.empty();
    assert_eq!(before, f.dump());
}
