use crate::{
    domain::{
        image::{ImportStatus, MAX_FILE_BYTES, ProductImage},
        product::{Product, ProductInput},
    },
    error::AppError,
    infrastructure::{
        db::{Database, connection, image_repository as repository, migrations},
        media_storage::{MediaStorage, check_dimensions},
    },
    services::{image_service, product_service},
};
use image::{ImageFormat, RgbImage};
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use uuid::Uuid;

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("listing-pilot-images-{}", Uuid::new_v4()));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).expect("remove isolated test directory");
    }
}
// Field drop order closes files/SQLite before Temp removes the directory on Windows.
struct Fixture {
    media: MediaStorage,
    db: Database,
    temp: Temp,
    product: Product,
}

fn image_ids(images: &[ProductImage]) -> Vec<String> {
    images.iter().map(|image| image.id.clone()).collect()
}

impl Fixture {
    fn ordered(&self) -> Vec<ProductImage> {
        self.db
            .with(|db| repository::list(db, &self.product.id))
            .unwrap()
    }

    fn import_distinct(&self, start: u8, count: u8) {
        let files: Vec<_> = (start..start + count)
            .map(|color| {
                let path = self.temp.0.join(format!("image-{color}.png"));
                RgbImage::from_pixel(3, 2, image::Rgb([color, 120, 200]))
                    .save(&path)
                    .unwrap();
                path
            })
            .collect();
        assert!(
            self.import(&files)
                .items
                .iter()
                .all(|item| item.status == ImportStatus::Imported)
        );
    }

    fn reorder(
        &self,
        expected: &[String],
        ordered: &[String],
    ) -> Result<Vec<ProductImage>, AppError> {
        image_service::reorder(&self.db, &self.media, &self.product.id, expected, ordered)
            .map(|images| images.into_iter().map(|preview| preview.image).collect())
    }
}

#[test]
fn reorder_four_images_preserves_metadata_bytes_product_and_restart_order() {
    let fixture = Fixture::new();
    fixture.import_distinct(0, 4);
    let before = fixture.ordered();
    let originals: Vec<_> = before
        .iter()
        .map(|image| fs::read(fixture.media.image_path(image).unwrap()).unwrap())
        .collect();
    let expected = image_ids(&before);
    let ordered = vec![
        expected[3].clone(),
        expected[0].clone(),
        expected[2].clone(),
        expected[1].clone(),
    ];
    let saved = fixture.reorder(&expected, &ordered).unwrap();
    assert_eq!(image_ids(&saved), ordered);
    assert_eq!(
        saved
            .iter()
            .map(|image| image.sort_order)
            .collect::<Vec<_>>(),
        vec![0, 1, 2, 3]
    );
    assert_eq!(fixture.ordered(), saved);
    assert_eq!(fixture.reorder(&ordered, &ordered).unwrap(), saved);
    fixture
        .db
        .with(|db| {
            assert_eq!(
                product_service::get(db, &fixture.product.id)?,
                fixture.product
            );
            Ok(())
        })
        .unwrap();
    for (image, bytes) in before.iter().zip(originals) {
        let after = saved.iter().find(|after| after.id == image.id).unwrap();
        let mut unchanged = after.clone();
        unchanged.sort_order = image.sort_order;
        assert_eq!(&unchanged, image);
        assert_eq!(
            fs::read(fixture.media.image_path(after).unwrap()).unwrap(),
            bytes
        );
        assert_eq!(
            fs::read(fixture.temp.0.join(&image.original_filename)).unwrap(),
            bytes
        );
        assert_eq!(after.sha256, format!("{:x}", Sha256::digest(&bytes)));
    }
    let Fixture {
        media,
        db,
        temp,
        product,
    } = fixture;
    drop(db);
    drop(media);
    let reopened = connection::open(&temp.0.join("test.sqlite3")).unwrap();
    let media =
        image_service::initialize_after_preflight(&reopened, &temp.0.join("media")).unwrap();
    assert_eq!(repository::list(&reopened, &product.id).unwrap(), saved);
    assert!(saved.iter().all(|image| media.preview(image).is_some()));
    drop(media);
    drop(reopened);
}

#[test]
fn reorder_rejects_invalid_sets_duplicates_and_stale_snapshots_without_writes() {
    let fixture = Fixture::new();
    fixture.import_distinct(0, 4);
    let before = fixture.ordered();
    let expected = image_ids(&before);
    let other = fixture
        .db
        .with(|db| product_service::create(db, input("other-order")))
        .unwrap();
    let source = fixture.temp.0.join("image-0.png");
    image_service::import_files(&fixture.db, &fixture.media, &other.id, &[source]).unwrap();
    let other_before = fixture
        .db
        .with(|db| repository::list(db, &other.id))
        .unwrap();
    let mut duplicate = expected.clone();
    duplicate[1] = duplicate[0].clone();
    let mut extra = expected.clone();
    extra.push(Uuid::new_v4().to_string());
    let mut foreign = expected.clone();
    foreign[1] = other_before[0].id.clone();
    let mut unknown = expected.clone();
    unknown[1] = Uuid::new_v4().to_string();
    for invalid in [
        duplicate.clone(),
        expected[..3].to_vec(),
        extra,
        foreign,
        unknown,
        vec![],
    ] {
        assert_eq!(
            fixture.reorder(&expected, &invalid),
            Err(AppError::InvalidImageOrder)
        );
        assert_eq!(fixture.ordered(), before);
    }
    assert_eq!(
        fixture.reorder(&duplicate, &expected),
        Err(AppError::InvalidImageOrder)
    );
    let mut stale = expected.clone();
    stale.reverse();
    for snapshot in [stale, expected[..3].to_vec(), vec![]] {
        assert_eq!(
            fixture.reorder(&snapshot, &expected),
            Err(AppError::StaleData)
        );
        assert_eq!(fixture.ordered(), before);
    }
    assert!(matches!(
        image_service::reorder(
            &fixture.db,
            &fixture.media,
            &Uuid::new_v4().to_string(),
            &[],
            &[]
        ),
        Err(AppError::ProductNotFound)
    ));
    assert_eq!(
        fixture
            .db
            .with(|db| repository::list(db, &other.id))
            .unwrap(),
        other_before
    );
    // A prior successful write also invalidates the original UI snapshot.
    let mut ordered = expected.clone();
    ordered.swap(0, 1);
    let saved = fixture.reorder(&expected, &ordered).unwrap();
    assert_eq!(
        fixture.reorder(&expected, &expected),
        Err(AppError::StaleData)
    );
    assert_eq!(fixture.ordered(), saved);
}

#[test]
fn reorder_rolls_back_partial_sql_failure_and_releases_guard() {
    let fixture = Fixture::new();
    fixture.import_distinct(0, 4);
    let before = fixture.ordered();
    let expected = image_ids(&before);
    fixture.db.with(|db| {
        // The second UPDATE fails, after the first has changed a row.
        db.execute_batch("CREATE TRIGGER fail_order BEFORE UPDATE OF sort_order ON product_images WHEN NEW.sort_order = 1 BEGIN SELECT RAISE(FAIL, 'injected reorder failure'); END;")?;
        Ok(())
    }).unwrap();
    let mut ordered = expected.clone();
    ordered.reverse();
    assert_eq!(
        fixture.reorder(&expected, &ordered),
        Err(AppError::DatabaseOperation)
    );
    assert_eq!(fixture.ordered(), before);
    fixture
        .db
        .with(|db| {
            db.execute_batch("DROP TRIGGER fail_order")?;
            Ok(())
        })
        .unwrap();
    assert_eq!(
        image_ids(&fixture.reorder(&expected, &ordered).unwrap()),
        ordered
    );
}

#[test]
fn reorder_and_import_share_nonblocking_guard_before_database_access() {
    let fixture = Fixture::new();
    fixture.import_distinct(0, 4);
    let before = fixture.ordered();
    let expected = image_ids(&before);
    let permit = fixture.media.begin_import().unwrap();
    {
        // A busy operation must return before trying to lock the already-held DB.
        let _db_guard = fixture.db.0.lock().unwrap();
        assert_eq!(
            fixture.reorder(&expected, &expected),
            Err(AppError::MediaBusy)
        );
        assert!(matches!(
            image_service::import_files(&fixture.db, &fixture.media, &fixture.product.id, &[]),
            Err(AppError::MediaBusy)
        ));
    }
    drop(permit);
    assert_eq!(fixture.reorder(&expected, &expected).unwrap(), before);
    fixture.import_distinct(4, 1);
    let after = fixture.ordered();
    assert_eq!(&after[..4], &before);
    assert_eq!(after[4].sort_order, 4);
    assert_eq!(
        fixture.reorder(&expected, &expected),
        Err(AppError::StaleData)
    );
}

#[test]
fn reorder_empty_single_and_more_than_twenty_images() {
    let fixture = Fixture::new();
    assert_eq!(fixture.reorder(&[], &[]).unwrap(), vec![]);
    fixture.import_distinct(0, 1);
    let single = fixture.ordered();
    let ids = image_ids(&single);
    assert_eq!(fixture.reorder(&ids, &ids).unwrap(), single);
    fixture.import_distinct(1, 20);
    let expected = image_ids(&fixture.ordered());
    let mut ordered = expected.clone();
    ordered.reverse();
    let saved = fixture.reorder(&expected, &ordered).unwrap();
    assert_eq!(image_ids(&saved), ordered);
    assert_eq!(saved.len(), 21);
    assert!(
        saved
            .iter()
            .enumerate()
            .all(|(index, image)| image.sort_order == index as i64)
    );
}

#[test]
fn reorder_uses_created_at_and_id_to_break_sort_order_ties() {
    let fixture = Fixture::new();
    fixture.import_distinct(0, 4);
    fixture
        .db
        .with(|db| {
            db.execute(
                "UPDATE product_images SET sort_order = 9, created_at = '2026-11-10T00:00:00Z'",
                [],
            )?;
            Ok(())
        })
        .unwrap();
    let before = fixture.ordered();
    let expected = image_ids(&before);
    let mut sorted = expected.clone();
    sorted.sort();
    assert_eq!(expected, sorted);
    sorted.reverse();
    assert_eq!(
        image_ids(&fixture.reorder(&expected, &sorted).unwrap()),
        sorted
    );
}
fn input(sku: &str) -> ProductInput {
    ProductInput {
        sku: sku.into(),
        name: "画像テスト".into(),
        brand: None,
        price_yen: None,
        description_note: None,
    }
}
impl Fixture {
    fn new() -> Self {
        let temp = Temp::new();
        let connection = connection::open(&temp.0.join("test.sqlite3")).unwrap();
        let product = product_service::create(&connection, input("image-test")).unwrap();
        let db = Database(Arc::new(Mutex::new(Ok(connection))));
        let media = MediaStorage::open(&temp.0.join("media")).unwrap();
        Self {
            media,
            db,
            temp,
            product,
        }
    }
    fn source(&self, name: &str, format: ImageFormat) -> PathBuf {
        let path = self.temp.0.join(name);
        RgbImage::from_pixel(3, 2, image::Rgb([40, 120, 200]))
            .save_with_format(&path, format)
            .unwrap();
        path
    }
    fn images(&self) -> Vec<ProductImage> {
        self.db.with(repository::all).unwrap()
    }
    fn import(&self, files: &[PathBuf]) -> crate::domain::image::ImportReport {
        image_service::import_files(&self.db, &self.media, &self.product.id, files).unwrap()
    }
    fn count(&self, subdir: &str) -> usize {
        fs::read_dir(self.temp.0.join("media").join(subdir))
            .unwrap()
            .count()
    }
}

#[test]
fn migration_zero_one_two_and_existing_product_preservation() {
    for version in [0, 1, 2] {
        let mut db = Connection::open_in_memory().unwrap();
        let product = if version > 0 {
            db.execute_batch(include_str!("../migrations/0001_create_products.sql"))
                .unwrap();
            db.pragma_update(None, "user_version", 1).unwrap();
            let product = product_service::create(&db, input("preserved")).unwrap();
            if version == 2 {
                db.execute_batch(include_str!("../migrations/0002_create_product_images.sql"))
                    .unwrap();
                db.pragma_update(None, "user_version", 2).unwrap();
            }
            Some(product)
        } else {
            None
        };
        migrations::apply(&mut db).unwrap();
        assert_eq!(
            db.pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
                .unwrap(),
            3
        );
        assert!(repository::all(&db).unwrap().is_empty());
        if let Some(product) = product {
            assert_eq!(product_service::get(&db, &product.id).unwrap(), product);
        }
    }
}

#[test]
fn migration_two_failure_rolls_back_schema_and_keeps_version_one() {
    let mut db = Connection::open_in_memory().unwrap();
    db.execute_batch(include_str!("../migrations/0001_create_products.sql"))
        .unwrap();
    db.pragma_update(None, "user_version", 1).unwrap();
    let product = product_service::create(&db, input("keep")).unwrap();
    // Force failure after CREATE TABLE has succeeded.
    db.execute_batch("CREATE INDEX idx_product_images_product_order ON products(id)")
        .unwrap();
    assert!(migrations::apply(&mut db).is_err());
    assert_eq!(
        db.pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert!(db.prepare("SELECT * FROM product_images").is_err());
    assert_eq!(product_service::get(&db, &product.id).unwrap(), product);
}

#[test]
fn jpeg_png_webp_import_and_original_byte_hash_identity() {
    let fixture = Fixture::new();
    for (index, (format, mime)) in [
        (ImageFormat::Jpeg, "image/jpeg"),
        (ImageFormat::Png, "image/png"),
        (ImageFormat::WebP, "image/webp"),
    ]
    .into_iter()
    .enumerate()
    {
        // Deliberately misleading extension: detection must use the bytes.
        let source = fixture.source(&format!("写真{index}.txt"), format);
        let before = fs::read(&source).unwrap();
        let report = fixture.import(std::slice::from_ref(&source));
        assert_eq!(report.items[0].status, ImportStatus::Imported);
        let images = fixture
            .db
            .with(|db| repository::list(db, &fixture.product.id))
            .unwrap();
        let image = images.last().unwrap();
        assert_eq!(image.mime_type, mime);
        assert_eq!((image.width, image.height), (3, 2));
        assert_eq!(image.file_size_bytes as usize, before.len());
        assert_eq!(image.sort_order as usize, index);
        assert_eq!(
            fs::read(fixture.media.image_path(image).unwrap()).unwrap(),
            before
        );
        assert_eq!(image.sha256, format!("{:x}", Sha256::digest(&before)));
        assert_eq!(fs::read(&source).unwrap(), before);
        assert!(image.relative_path.starts_with("media/originals/"));
        assert!(Uuid::parse_str(&image.id).is_ok());
        assert!(chrono::DateTime::parse_from_rfc3339(&image.created_at).is_ok());
    }
    assert_eq!(fixture.count(".staging"), 0);
}

#[test]
fn duplicate_content_is_skipped_per_product_and_allowed_for_other_products() {
    let fixture = Fixture::new();
    let source = fixture.source("source.png", ImageFormat::Png);
    let alias = fixture.temp.0.join("renamed.jpg");
    fs::copy(&source, &alias).unwrap();
    let report = fixture.import(&[source, alias.clone()]);
    assert_eq!(report.items[0].status, ImportStatus::Imported);
    assert_eq!(report.items[1].status, ImportStatus::Duplicate);
    let other = fixture
        .db
        .with(|db| product_service::create(db, input("other")))
        .unwrap();
    let report =
        image_service::import_files(&fixture.db, &fixture.media, &other.id, &[alias]).unwrap();
    assert_eq!(report.items[0].status, ImportStatus::Imported);
    let images = fixture.images();
    assert_eq!(images.len(), 2);
    assert_eq!(images[0].sha256, images[1].sha256);
    assert_ne!(images[0].relative_path, images[1].relative_path);
    assert_eq!(fixture.count(".staging"), 0);
}

#[test]
fn image_unique_constraints_never_become_duplicate_sku() {
    let fixture = Fixture::new();
    fixture.import(&[fixture.source("source.png", ImageFormat::Png)]);
    let mut duplicate = fixture.images().remove(0);
    duplicate.id = Uuid::new_v4().to_string();
    duplicate.relative_path =
        MediaStorage::relative_path(&duplicate.product_id, &duplicate.id, "png").unwrap();
    assert_eq!(
        fixture.db.with(|db| repository::create(db, &duplicate)),
        Err(AppError::DuplicateImage)
    );
    duplicate.product_id = fixture
        .db
        .with(|db| product_service::create(db, input("other")))
        .unwrap()
        .id;
    duplicate.relative_path = fixture.images()[0].relative_path.clone();
    assert_eq!(
        fixture.db.with(|db| repository::create(db, &duplicate)),
        Err(AppError::DatabaseOperation)
    );
}

#[test]
fn missing_product_is_rejected_by_service_and_database_foreign_key() {
    let fixture = Fixture::new();
    let missing = Uuid::new_v4().to_string();
    assert!(matches!(
        image_service::import_files(&fixture.db, &fixture.media, &missing, &[]),
        Err(AppError::ProductNotFound)
    ));
    assert!(matches!(
        image_service::list(&fixture.db, &fixture.media, &missing),
        Err(AppError::ProductNotFound)
    ));
    fixture.import(&[fixture.source("source.png", ImageFormat::Png)]);
    let mut image = fixture.images().remove(0);
    image.id = Uuid::new_v4().to_string();
    image.product_id = missing;
    image.relative_path = MediaStorage::relative_path(&image.product_id, &image.id, "png").unwrap();
    assert_eq!(
        fixture.db.with(|db| repository::create(db, &image)),
        Err(AppError::DatabaseOperation)
    );
}

#[test]
fn invalid_unsupported_corrupt_images_and_partial_failure_are_independent() {
    let fixture = Fixture::new();
    let unsupported = fixture.temp.0.join("gif.png");
    fs::write(&unsupported, b"GIF89a\x01\x00\x01\x00").unwrap();
    let corrupt = fixture.temp.0.join("broken.png");
    fs::write(&corrupt, b"\x89PNG\r\n\x1a\ninvalid").unwrap();
    let text = fixture.temp.0.join("fake.jpg");
    fs::write(&text, b"not an image").unwrap();
    let good = fixture.source("good.png", ImageFormat::Png);
    let report = fixture.import(&[
        unsupported,
        corrupt,
        text,
        good,
        fixture.temp.0.join("missing.png"),
        fixture.temp.0.clone(),
    ]);
    assert_eq!(
        report
            .items
            .iter()
            .map(|item| item.error.clone())
            .collect::<Vec<_>>(),
        vec![
            Some(AppError::UnsupportedImage),
            Some(AppError::InvalidImage),
            Some(AppError::UnsupportedImage),
            None,
            Some(AppError::FileRead),
            Some(AppError::UnsafeMediaPath)
        ]
    );
    assert_eq!(fixture.images().len(), 1);
    assert_eq!(fixture.count(".staging"), 0);
}

#[test]
fn byte_and_batch_limits_prevent_import_without_leaving_files() {
    let fixture = Fixture::new();
    let oversized = fixture.temp.0.join("oversized.png");
    fs::File::create(&oversized)
        .unwrap()
        .set_len(MAX_FILE_BYTES + 1)
        .unwrap();
    assert_eq!(
        fixture.import(&[oversized]).items[0].error,
        Some(AppError::ImageTooLarge)
    );
    let source = fixture.source("source.png", ImageFormat::Png);
    assert!(matches!(
        image_service::import_files(
            &fixture.db,
            &fixture.media,
            &fixture.product.id,
            &vec![source.clone(); 21]
        ),
        Err(AppError::TooManyImages)
    ));
    let report = fixture.import(&vec![source; 20]);
    assert_eq!(report.items.len(), 20);
    assert_eq!(
        report
            .items
            .iter()
            .filter(|item| item.status == ImportStatus::Imported)
            .count(),
        1
    );
    assert_eq!(fixture.count(".staging"), 0);
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb88320u32 & (0u32.wrapping_sub(crc & 1)));
        }
    }
    !crc
}

#[test]
fn excessive_pixel_dimensions_are_rejected_before_decoding() {
    assert!(check_dimensions(8000, 5000).is_ok());
    assert_eq!(
        check_dimensions(u32::MAX, u32::MAX),
        Err(AppError::ImageTooLarge)
    );
    assert_eq!(check_dimensions(0, 1), Err(AppError::InvalidImage));
    let fixture = Fixture::new();
    let source = fixture.source("huge-header.png", ImageFormat::Png);
    let mut bytes = fs::read(&source).unwrap();
    bytes[16..20].copy_from_slice(&10000u32.to_be_bytes());
    bytes[20..24].copy_from_slice(&4001u32.to_be_bytes());
    let crc = crc32(&bytes[12..29]);
    bytes[29..33].copy_from_slice(&crc.to_be_bytes());
    fs::write(&source, bytes).unwrap();
    assert_eq!(
        fixture.import(&[source]).items[0].error,
        Some(AppError::ImageTooLarge)
    );
    assert!(fixture.images().is_empty());
    assert_eq!(fixture.count(".staging"), 0);
}

#[test]
fn staging_write_and_final_placement_failures_keep_source_and_clean_staging() {
    let fixture = Fixture::new();
    let source = fixture.source("source.png", ImageFormat::Png);
    let bytes = fs::read(&source).unwrap();
    let staging = fixture.temp.0.join("media/.staging");
    fs::remove_dir(&staging).unwrap();
    fs::write(&staging, b"blocked").unwrap();
    assert_eq!(
        fixture.import(std::slice::from_ref(&source)).items[0].error,
        Some(AppError::UnsafeMediaPath)
    );
    fs::remove_file(&staging).unwrap();
    fs::create_dir(&staging).unwrap();
    fs::write(
        fixture
            .temp
            .0
            .join("media/originals")
            .join(&fixture.product.id),
        b"blocked",
    )
    .unwrap();
    assert_eq!(
        fixture.import(std::slice::from_ref(&source)).items[0].error,
        Some(AppError::UnsafeMediaPath)
    );
    assert_eq!(fixture.count(".staging"), 0);
    assert!(fixture.images().is_empty());
    assert_eq!(fs::read(source).unwrap(), bytes);
}

#[test]
fn database_failure_compensates_only_unregistered_file_and_keeps_previous_success() {
    let fixture = Fixture::new();
    let first = fixture.source("first.png", ImageFormat::Png);
    fixture.import(&[first]);
    let registered = fixture.images().remove(0);
    fixture.db.with(|db| { db.execute_batch("CREATE TRIGGER reject_image BEFORE INSERT ON product_images BEGIN SELECT RAISE(ABORT, 'test failure'); END;")?; Ok(()) }).unwrap();
    let second = fixture.source("second.jpg", ImageFormat::Jpeg);
    assert_eq!(
        fixture.import(&[second]).items[0].error,
        Some(AppError::DatabaseOperation)
    );
    assert_eq!(fixture.images(), vec![registered.clone()]);
    assert!(fixture.media.preview(&registered).is_some());
    assert_eq!(
        fixture.count(&format!("originals/{}", fixture.product.id)),
        1
    );
    assert_eq!(fixture.count(".staging"), 0);
}

#[test]
fn ambiguous_insert_error_checks_reference_and_preserves_committed_file() {
    let fixture = Fixture::new();
    // SQLite FAIL preserves changes made by the statement before this trigger.
    fixture.db.with(|db| { db.execute_batch("CREATE TRIGGER ambiguous AFTER INSERT ON product_images BEGIN SELECT RAISE(FAIL, 'ambiguous result'); END;")?; Ok(()) }).unwrap();
    let report = fixture.import(&[fixture.source("source.png", ImageFormat::Png)]);
    assert_eq!(report.items[0].status, ImportStatus::Imported);
    let images = fixture.images();
    assert_eq!(images.len(), 1);
    assert!(fixture.media.preview(&images[0]).is_some());
}

#[test]
fn crash_leftovers_and_orphans_are_quarantined_while_registered_files_remain() {
    let fixture = Fixture::new();
    let source = fixture.source("source.png", ImageFormat::Png);
    fixture.import(std::slice::from_ref(&source));
    let images = fixture.images();
    // Reproduce a crash after staging and after rename but before DB insertion.
    let leftover = fixture.temp.0.join("media/.staging/crashed.tmp");
    fs::write(&leftover, b"incomplete").unwrap();
    let orphan = fixture.temp.0.join(format!(
        "media/originals/{}/{}.png",
        fixture.product.id,
        Uuid::new_v4()
    ));
    fs::copy(&source, &orphan).unwrap();
    assert_eq!(fixture.media.reconcile(&images).unwrap(), 2);
    assert!(!leftover.exists() && !orphan.exists());
    assert_eq!(fixture.count("quarantine"), 2);
    assert!(fixture.media.preview(&images[0]).is_some());
    assert_eq!(fixture.media.reconcile(&images).unwrap(), 0);
    assert_eq!(fixture.count("quarantine"), 2);
}

#[test]
fn recovery_accepts_safe_unexpected_file_at_originals_root() {
    let f = Fixture::new();
    f.import_distinct(0, 1);
    let images = f.ordered();
    let stray = f.temp.0.join("media/originals/notes.txt");
    fs::write(&stray, b"retain notes").unwrap();
    assert_eq!(f.media.reconcile(&images), Ok(1));
    assert!(!stray.exists());
    assert!(f.media.preview(&images[0]).is_some());
}

#[test]
#[cfg(windows)]
fn recovery_preserves_registered_image_with_uppercase_disk_extension() {
    let f = Fixture::new();
    f.import_distinct(0, 1);
    let images = f.ordered();
    let registered = f.media.image_path(&images[0]).unwrap();
    let renamed = registered.with_extension("PNG");
    fs::rename(&registered, &renamed).unwrap();
    assert!(
        registered.exists(),
        "test requires case-insensitive directory"
    );
    assert_eq!(f.media.reconcile(&images), Ok(0));
    assert!(f.media.preview(&images[0]).is_some());
    assert_eq!(
        format!("{:x}", Sha256::digest(fs::read(registered).unwrap())),
        images[0].sha256
    );
    assert_eq!(f.count("quarantine"), 0);
}

#[test]
fn product_edit_conflict_keeps_delete_snapshot_and_latest_product_consistent() {
    let f = Fixture::new();
    f.import_distinct(0, 1);
    let old_delete = product_delete_input(&f);
    let updated =
        f.db.with(|db| {
            product_service::update(db, &f.product.id, input("edited"), &f.product.updated_at)
        })
        .unwrap();
    assert_eq!(delete_product(&f, &old_delete), Err(AppError::StaleData));
    let latest_delete = product_delete_input(&f);
    assert_eq!(
        f.db.with(|db| product_service::update(
            db,
            &f.product.id,
            input("stale"),
            &f.product.updated_at
        )),
        Err(AppError::StaleData)
    );
    assert_eq!(
        f.db.with(|db| product_service::get(db, &f.product.id))
            .unwrap(),
        updated
    );
    assert!(delete_product(&f, &latest_delete).unwrap().deleted);
}

#[test]
fn recovery_rejects_nested_directory_before_moving_any_files() {
    let f = Fixture::new();
    let keep = f.temp.0.join("media/.staging/keep.tmp");
    fs::write(&keep, b"keep").unwrap();
    fs::create_dir_all(f.temp.0.join("media/originals/unexpected/nested")).unwrap();
    assert_eq!(f.media.reconcile(&[]), Err(AppError::UnsafeMediaPath));
    assert_eq!(fs::read(keep).unwrap(), b"keep");
    assert_eq!(f.count("quarantine"), 0);
}

#[test]
fn recovery_accepts_unexpected_flat_directory_without_deleting_contents() {
    let f = Fixture::new();
    let stray = f.temp.0.join("media/originals/unexpected");
    fs::create_dir(&stray).unwrap();
    fs::write(stray.join("notes.txt"), b"safe content").unwrap();
    assert_eq!(f.media.reconcile(&[]), Ok(1));
    assert!(stray.is_dir());
    let recovered = fs::read_dir(f.temp.0.join("media/quarantine"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    assert_eq!(fs::read(recovered).unwrap(), b"safe content");
}

#[cfg(windows)]
#[test]
fn recovery_rejects_junctions_before_any_move() {
    let f = Fixture::new();
    let target = f.temp.0.join("isolated-target");
    fs::create_dir(&target).unwrap();
    fs::write(target.join("keep.txt"), b"keep target").unwrap();
    let stray = f.temp.0.join("media/.staging/keep.tmp");
    fs::write(&stray, b"keep staged").unwrap();
    let link = fs::canonicalize(f.temp.0.join("media/originals"))
        .unwrap()
        .join("linked");
    let result = std::process::Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(&link)
        .arg(&target)
        .output()
        .unwrap();
    assert!(result.status.success());
    let result = f.media.reconcile(&[]);
    fs::remove_dir(&link).unwrap();
    assert_eq!(result, Err(AppError::UnsafeMediaPath));
    assert_eq!(fs::read(stray).unwrap(), b"keep staged");
    assert_eq!(fs::read(target.join("keep.txt")).unwrap(), b"keep target");
    assert_eq!(f.count("quarantine"), 0);
}

#[cfg(windows)]
#[test]
fn recovery_rejects_unreadable_reference_before_moving_orphans() {
    use std::os::windows::fs::OpenOptionsExt;
    let f = Fixture::new();
    f.import_distinct(0, 1);
    let images = f.ordered();
    let path = f.media.image_path(&images[0]).unwrap();
    let stray = f.temp.0.join("media/.staging/keep.tmp");
    fs::write(&stray, b"keep").unwrap();
    let held = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&path)
        .unwrap();
    assert_eq!(f.media.reconcile(&images), Err(AppError::FileRead));
    assert!(stray.exists());
    assert_eq!(f.count("quarantine"), 0);
    drop(held);
    assert_eq!(f.media.reconcile(&images), Ok(1));
}

#[test]
fn startup_failure_preserves_database_and_does_not_publish_writable_connection() {
    let temp = Temp::new();
    let path = temp.0.join("listing-pilot.sqlite3");
    fs::write(&path, b"corrupt DB retained").unwrap();
    assert!(matches!(
        crate::services::startup_service::initialize(&temp.0),
        Err(AppError::DatabaseInitialization)
    ));
    assert_eq!(fs::read(&path).unwrap(), b"corrupt DB retained");
    assert!(!temp.0.join("media").exists());
}

// Invoked by dataset_lock_survives_two_processes_and_abnormal_exit. A normally
// run test has no environment variable and does not create any app data.
#[test]
fn dataset_lock_child() {
    use std::io::{Read, Write};
    let Some(directory) = std::env::var_os("LISTING_PILOT_TEST_DATASET") else {
        return;
    };
    let (_db, _media) =
        crate::services::startup_service::initialize(&PathBuf::from(directory)).unwrap();
    println!("DATASET_LOCK_READY");
    std::io::stdout().flush().unwrap();
    let mut release = [0u8];
    std::io::stdin().read_exact(&mut release).unwrap();
}

#[test]
fn dataset_lock_survives_two_processes_and_abnormal_exit() {
    use std::io::{BufRead, BufReader, Write};
    use std::process::{Command, Stdio};
    let temp = Temp::new();
    let product;
    let images;
    {
        let (db, media) = crate::services::startup_service::initialize(&temp.0).unwrap();
        product = product_service::create(&db, input("two-process")).unwrap();
        let db = Database(Arc::new(Mutex::new(Ok(db))));
        let source = temp.0.join("source.png");
        RgbImage::new(2, 2).save(&source).unwrap();
        image_service::import_files(&db, &media, &product.id, &[source]).unwrap();
        images = db.with(repository::all).unwrap();
    }
    for terminate in [false, true] {
        let before = fs::read(temp.0.join("listing-pilot.sqlite3")).unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "image_tests::dataset_lock_child", "--nocapture"])
            .env("LISTING_PILOT_TEST_DATASET", &temp.0)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let mut stdout = BufReader::new(child.stdout.take().unwrap());
        let mut line = String::new();
        loop {
            assert!(
                stdout.read_line(&mut line).unwrap() > 0,
                "child exited before acquiring lock"
            );
            if line.contains("DATASET_LOCK_READY") {
                break;
            }
            line.clear();
        }
        let blocked = crate::services::startup_service::initialize(&temp.0);
        assert!(matches!(blocked, Err(AppError::MediaBusy)));
        assert!(matches!(
            MediaStorage::open(&temp.0.join("media")),
            Err(AppError::MediaBusy)
        ));
        let db = Database(Arc::new(Mutex::new(Err(AppError::MediaBusy))));
        assert_eq!(
            db.with(|db| product_service::create(db, input("blocked"))),
            Err(AppError::MediaBusy)
        );
        assert_eq!(
            db.with(|db| product_service::update(
                db,
                &product.id,
                input("blocked"),
                &product.updated_at
            )),
            Err(AppError::MediaBusy)
        );
        assert_eq!(
            db.with_mut(|db| {
                db.execute("DELETE FROM products", [])?;
                Ok(())
            }),
            Err(AppError::MediaBusy)
        );
        assert_eq!(
            fs::read(temp.0.join("listing-pilot.sqlite3")).unwrap(),
            before
        );
        if terminate {
            child.kill().unwrap();
        } else {
            child.stdin.take().unwrap().write_all(b"x").unwrap();
        }
        let status = child.wait().unwrap();
        assert_eq!(status.success(), !terminate);
        let (db, media) = crate::services::startup_service::initialize(&temp.0).unwrap();
        assert_eq!(product_service::get(&db, &product.id).unwrap(), product);
        assert_eq!(repository::all(&db).unwrap(), images);
        assert_eq!(
            format!(
                "{:x}",
                Sha256::digest(fs::read(media.image_path(&images[0]).unwrap()).unwrap())
            ),
            images[0].sha256
        );
    }
}

#[test]
fn reorder_commit_failure_rolls_back_order_without_moving_files() {
    let f = Fixture::new();
    f.import_distinct(0, 2);
    let images = f.ordered();
    let ids = image_ids(&images);
    f.db.with(|db| {
        db.execute_batch("CREATE TABLE deferred_failure (id TEXT REFERENCES products(id) DEFERRABLE INITIALLY DEFERRED); CREATE TRIGGER fail_order_commit AFTER UPDATE OF sort_order ON product_images BEGIN INSERT INTO deferred_failure VALUES ('missing'); END;")?;
        Ok(())
    }).unwrap();
    assert!(f.reorder(&ids, &[ids[1].clone(), ids[0].clone()]).is_err());
    assert_eq!(f.ordered(), images);
    assert_eq!(f.count("quarantine"), 0);
    assert!(images.iter().all(|image| f.media.preview(image).is_some()));
}

#[test]
fn media_process_lock_and_import_permit_prevent_racing_recovery() {
    let fixture = Fixture::new();
    assert!(matches!(
        MediaStorage::open(&fixture.temp.0.join("media")),
        Err(AppError::MediaBusy)
    ));
    let permit = fixture.media.begin_import().unwrap();
    assert!(matches!(
        fixture.media.begin_import(),
        Err(AppError::MediaBusy)
    ));
    assert_eq!(fixture.media.reconcile(&[]), Err(AppError::MediaBusy));
    drop(permit);
    assert_eq!(fixture.media.reconcile(&[]).unwrap(), 0);
}

#[test]
fn missing_or_tampered_database_paths_never_expose_arbitrary_files() {
    let fixture = Fixture::new();
    fixture.import(&[fixture.source("source.png", ImageFormat::Png)]);
    let image = fixture.images().remove(0);
    fs::remove_file(fixture.media.image_path(&image).unwrap()).unwrap();
    let previews = image_service::list(&fixture.db, &fixture.media, &fixture.product.id).unwrap();
    assert!(previews[0].preview_path.is_none());
    for path in [
        "../../secret.png",
        "C:/secret.png",
        "media/.staging/secret.png",
        "media/quarantine/secret.png",
    ] {
        let mut invalid = image.clone();
        invalid.relative_path = path.into();
        assert_eq!(
            fixture.media.image_path(&invalid),
            Err(AppError::UnsafeMediaPath)
        );
        assert!(fixture.media.preview(&invalid).is_none());
        assert_eq!(
            fixture.media.reconcile(&[invalid]),
            Err(AppError::UnsafeMediaPath)
        );
    }
    assert!(MediaStorage::relative_path("..", &image.id, "png").is_err());
    assert!(MediaStorage::relative_path(&image.product_id, &image.id, "../png").is_err());
}

#[test]
fn sqlite_and_image_files_survive_close_and_reopen() {
    let temp = Temp::new();
    let db_path = temp.0.join("persistent.sqlite3");
    let media_root = temp.0.join("media");
    let source = temp.0.join("source.png");
    RgbImage::new(2, 2).save(&source).unwrap();
    let (product, images) = {
        let connection = connection::open(&db_path).unwrap();
        let product = product_service::create(&connection, input("persistent")).unwrap();
        let db = Database(Arc::new(Mutex::new(Ok(connection))));
        let media = MediaStorage::open(&media_root).unwrap();
        image_service::import_files(&db, &media, &product.id, std::slice::from_ref(&source))
            .unwrap();
        (product, db.with(repository::all).unwrap())
    };
    let connection = connection::open(&db_path).unwrap();
    assert_eq!(
        product_service::get(&connection, &product.id).unwrap(),
        product
    );
    assert_eq!(repository::all(&connection).unwrap(), images);
    let media = MediaStorage::open(&media_root).unwrap();
    assert_eq!(media.reconcile(&images).unwrap(), 0);
    assert_eq!(
        fs::read(media.image_path(&images[0]).unwrap()).unwrap(),
        fs::read(source).unwrap()
    );
}

#[test]
fn cancelled_import_is_a_normal_empty_result() {
    let report = crate::domain::image::ImportReport::cancelled();
    assert!(report.cancelled && report.items.is_empty());
}

#[test]
fn failed_database_initialization_never_touches_media() {
    let temp = Temp::new();
    let root = temp.0.join("media");
    assert!(matches!(
        image_service::initialize(&Err(AppError::DatabaseInitialization), &root),
        Err(AppError::DatabaseInitialization)
    ));
    assert!(!root.exists());
    fs::create_dir_all(root.join(".staging")).unwrap();
    let leftover = root.join(".staging/keep.tmp");
    fs::write(&leftover, b"retain").unwrap();
    let corrupt_db = temp.0.join("corrupt.sqlite3");
    fs::write(&corrupt_db, b"not SQLite").unwrap();
    assert!(image_service::initialize(&connection::open(&corrupt_db), &root).is_err());
    assert_eq!(fs::read(leftover).unwrap(), b"retain");
    assert!(!root.join("quarantine").exists());
}

#[test]
fn incomplete_reference_query_prevents_media_recovery() {
    let temp = Temp::new();
    // An unavailable metadata table must never be interpreted as zero references.
    let database = Ok(Connection::open_in_memory().unwrap());
    let root = temp.0.join("media");
    assert!(matches!(
        image_service::initialize(&database, &root),
        Err(AppError::DatabaseOperation)
    ));
    assert!(!root.exists());
}

#[test]
fn initialization_refreshes_references_after_previous_owner_releases_lock() {
    let temp = Temp::new();
    let path = temp.0.join("race.sqlite3");
    let root = temp.0.join("media");
    let connection_a = connection::open(&path).unwrap();
    let product = product_service::create(&connection_a, input("race")).unwrap();
    let database_a = Database(Arc::new(Mutex::new(Ok(connection_a))));
    let connection_b = connection::open(&path).unwrap();
    let media_a = MediaStorage::open(&root).unwrap();

    // B's preflight completes while A still owns the lock, before A registers.
    let stale = repository::all(&connection_b).unwrap();
    assert!(stale.is_empty());
    let source = temp.0.join("late.png");
    RgbImage::new(2, 2).save(&source).unwrap();
    let report =
        image_service::import_files(&database_a, &media_a, &product.id, &[source]).unwrap();
    assert_eq!(report.items[0].status, ImportStatus::Imported);
    let images = database_a.with(repository::all).unwrap();
    let saved = media_a.image_path(&images[0]).unwrap();
    let bytes = fs::read(&saved).unwrap();
    drop(media_a);

    // Resume the same initialization phase used by initialize(), with no sleeps
    // or scheduling assumptions. B must re-query after acquiring the OS lock.
    let media_b = image_service::initialize_after_preflight(&connection_b, &root).unwrap();
    assert_eq!(repository::all(&connection_b).unwrap(), images);
    assert_eq!(fs::read(saved).unwrap(), bytes);
    assert!(media_b.preview(&images[0]).is_some());
    assert_eq!(fs::read_dir(root.join("quarantine")).unwrap().count(), 0);
}

#[test]
fn failed_locked_reference_refresh_preserves_staging_and_originals() {
    let temp = Temp::new();
    let connection = connection::open(&temp.0.join("refresh.sqlite3")).unwrap();
    let root = temp.0.join("media");
    assert!(repository::all(&connection).unwrap().is_empty()); // successful preflight
    drop(MediaStorage::open(&root).unwrap());
    let staging = root.join(".staging/keep.tmp");
    fs::write(&staging, b"incomplete").unwrap();
    let product_dir = root.join("originals").join(Uuid::new_v4().to_string());
    fs::create_dir(&product_dir).unwrap();
    let orphan = product_dir.join("keep.png");
    fs::write(&orphan, b"keep original").unwrap();
    // A deterministic DB failure between preflight and the protected re-query.
    connection
        .execute_batch("DROP TABLE product_images")
        .unwrap();
    assert!(matches!(
        image_service::initialize_after_preflight(&connection, &root),
        Err(AppError::DatabaseOperation)
    ));
    assert_eq!(fs::read(staging).unwrap(), b"incomplete");
    assert_eq!(fs::read(orphan).unwrap(), b"keep original");
    assert_eq!(fs::read_dir(root.join("quarantine")).unwrap().count(), 0);
    // The failed initialization also released its OS lock.
    assert!(MediaStorage::open(&root).is_ok());
}

#[test]
fn placement_never_replaces_an_existing_file_or_directory_and_cleans_staging() {
    let fixture = Fixture::new();
    let source = fixture.source("source.png", ImageFormat::Png);
    fixture.import(std::slice::from_ref(&source));
    let registered = fixture.images().remove(0);
    let saved = fixture.media.image_path(&registered).unwrap();
    let before = fs::read(&saved).unwrap();
    let staged = fixture.media.stage(&source).unwrap();
    // Exercise the OS no-replacement operation directly via place(): there is no
    // racy destination-existence precheck in production code.
    assert_eq!(
        fixture.media.place(&staged, &registered),
        Err(AppError::FileSave)
    );
    assert_eq!(fs::read(&saved).unwrap(), before);
    assert_eq!(fixture.count(".staging"), 1);
    drop(staged);
    assert_eq!(fixture.count(".staging"), 0);

    let mut image = registered.clone();
    image.id = Uuid::new_v4().to_string();
    image.relative_path = MediaStorage::relative_path(&image.product_id, &image.id, "png").unwrap();
    let occupied = fixture.media.image_path(&image).unwrap();
    fs::create_dir(&occupied).unwrap();
    let staged = fixture.media.stage(&source).unwrap();
    assert_eq!(
        fixture.media.place(&staged, &image),
        Err(AppError::FileSave)
    );
    drop(staged);
    assert!(occupied.is_dir());
    assert_eq!(fixture.count(".staging"), 0);
    assert_eq!(fixture.images(), vec![registered]);
    assert_eq!(fs::read(source).unwrap(), before);
}

#[cfg(windows)]
#[test]
fn windows_placement_supports_long_paths_and_unicode_filenames() {
    let temp = Temp::new();
    let connection = connection::open(&temp.0.join("long-path.sqlite3")).unwrap();
    let product = product_service::create(&connection, input("long-path")).unwrap();
    let database = Database(Arc::new(Mutex::new(Ok(connection))));
    let root = temp
        .0
        .join("a".repeat(120))
        .join("b".repeat(120))
        .join("media");
    let media = MediaStorage::open(&root).unwrap();
    let source = temp.0.join("長い名前📸.png");
    RgbImage::new(2, 2).save(&source).unwrap();
    let report = image_service::import_files(
        &database,
        &media,
        &product.id,
        std::slice::from_ref(&source),
    )
    .unwrap();
    assert_eq!(report.items[0].status, ImportStatus::Imported);
    let image = database.with(repository::all).unwrap().remove(0);
    let saved = media.image_path(&image).unwrap();
    assert!(saved.as_os_str().len() > 260);
    assert_eq!(fs::read(saved).unwrap(), fs::read(source).unwrap());
    assert_eq!(image.original_filename, "長い名前📸.png");
}

#[cfg(windows)]
#[test]
fn directory_junctions_are_rejected_without_following_them() {
    use std::process::Command;
    let fixture = Fixture::new();
    let outside = fixture.temp.0.join("outside");
    fs::create_dir(&outside).unwrap();
    let source = outside.join("source.png");
    RgbImage::new(1, 1).save(&source).unwrap();
    let junction = fixture.temp.0.join("linked");
    let status = Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(&junction)
        .arg(&outside)
        .output()
        .unwrap();
    assert!(status.status.success(), "junction creation failed");
    assert_eq!(
        fixture.import(&[junction.join("source.png")]).items[0].error,
        Some(AppError::UnsafeMediaPath)
    );
    assert!(matches!(
        MediaStorage::open(&junction.join("media")),
        Err(AppError::UnsafeMediaPath)
    ));
    assert!(source.exists());
    fs::remove_dir(junction).unwrap();
}

fn deleted_path(fixture: &Fixture, image: &ProductImage) -> PathBuf {
    fixture
        .temp
        .0
        .join("media/quarantine/deleted")
        .join(&image.product_id)
        .join(format!("{}.png", image.id))
}
fn delete_image(
    fixture: &Fixture,
    image: &ProductImage,
) -> Result<crate::domain::image::DeleteImageReport, AppError> {
    image_service::delete(&fixture.db, &fixture.media, &fixture.product.id, &image.id)
}

#[test]
fn delete_main_normalizes_order_preserves_other_products_metadata_sources_and_restart() {
    let fixture = Fixture::new();
    fixture.import_distinct(0, 4);
    let before = fixture.ordered();
    let other = fixture
        .db
        .with(|db| product_service::create(db, input("other-delete")))
        .unwrap();
    image_service::import_files(
        &fixture.db,
        &fixture.media,
        &other.id,
        &[fixture.temp.0.join("image-0.png")],
    )
    .unwrap();
    let other_before = fixture
        .db
        .with(|db| repository::list(db, &other.id))
        .unwrap();
    let bytes = fs::read(fixture.media.image_path(&before[0]).unwrap()).unwrap();
    let report = delete_image(&fixture, &before[0]).unwrap();
    assert!(report.deleted);
    assert_eq!(
        (
            report.affected_image_count,
            report.quarantined_image_count,
            report.missing_image_count,
            report.pending_quarantine_count
        ),
        (1, 1, 0, 0)
    );
    assert!(!fixture.media.image_path(&before[0]).unwrap().exists());
    assert_eq!(fs::read(deleted_path(&fixture, &before[0])).unwrap(), bytes);
    let saved = fixture.ordered();
    for (index, image) in saved.iter().enumerate() {
        let mut expected = before[index + 1].clone();
        expected.sort_order = index as i64;
        assert_eq!(image, &expected);
        assert_eq!(
            format!(
                "{:x}",
                Sha256::digest(fs::read(fixture.media.image_path(image).unwrap()).unwrap())
            ),
            image.sha256
        );
    }
    assert_eq!(saved[0].id, before[1].id);
    for image in &before {
        assert_eq!(
            format!(
                "{:x}",
                Sha256::digest(fs::read(fixture.temp.0.join(&image.original_filename)).unwrap())
            ),
            image.sha256
        );
    }
    fixture
        .db
        .with(|db| {
            assert_eq!(
                product_service::get(db, &fixture.product.id)?,
                fixture.product
            );
            assert_eq!(repository::list(db, &other.id)?, other_before);
            Ok(())
        })
        .unwrap();
    let destination = deleted_path(&fixture, &before[0]);
    let Fixture {
        media,
        db,
        temp,
        product,
    } = fixture;
    drop(db);
    drop(media);
    let reopened = connection::open(&temp.0.join("test.sqlite3")).unwrap();
    let media =
        image_service::initialize_after_preflight(&reopened, &temp.0.join("media")).unwrap();
    assert_eq!(repository::list(&reopened, &product.id).unwrap(), saved);
    assert_eq!(fs::read(destination).unwrap(), bytes);
    assert!(saved.iter().all(|image| media.preview(image).is_some()));
    drop(media);
    drop(reopened);
}

#[test]
fn delete_last_and_confirmed_missing_files_including_missing_product_directory() {
    for missing in 0..3 {
        let fixture = Fixture::new();
        fixture.import_distinct(0, 1);
        let image = fixture.ordered().remove(0);
        let path = fixture.media.image_path(&image).unwrap();
        if missing > 0 {
            fs::remove_file(&path).unwrap();
        }
        if missing == 2 {
            fs::remove_dir(path.parent().unwrap()).unwrap();
        }
        let report = delete_image(&fixture, &image).unwrap();
        assert_eq!(report.missing_image_count, usize::from(missing > 0));
        assert_eq!(report.quarantined_image_count, usize::from(missing == 0));
        assert!(fixture.ordered().is_empty());
        assert!(
            image_service::list(&fixture.db, &fixture.media, &fixture.product.id)
                .unwrap()
                .is_empty()
        );
    }
}

#[test]
fn delete_rejects_unknown_invalid_and_other_product_ids_without_changes() {
    let fixture = Fixture::new();
    fixture.import_distinct(0, 1);
    let before = fixture.ordered();
    let other = fixture
        .db
        .with(|db| product_service::create(db, input("other")))
        .unwrap();
    for (product, image) in [
        (&fixture.product.id, Uuid::new_v4().to_string()),
        (&other.id, before[0].id.clone()),
        (&fixture.product.id, "../unsafe".into()),
    ] {
        assert_eq!(
            image_service::delete(&fixture.db, &fixture.media, product, &image),
            Err(AppError::ImageNotFound)
        );
    }
    assert_eq!(fixture.ordered(), before);
    assert!(fixture.media.preview(&before[0]).is_some());
}

#[test]
fn delete_sql_failures_rollback_delete_and_all_normalization_updates() {
    for trigger in [
        "BEFORE DELETE ON product_images",
        "BEFORE UPDATE OF sort_order ON product_images WHEN NEW.sort_order = 1",
    ] {
        let fixture = Fixture::new();
        fixture.import_distinct(0, 4);
        let before = fixture.ordered();
        fixture.db.with(|db| { db.execute_batch(&format!("CREATE TRIGGER fail_delete {trigger} BEGIN SELECT RAISE(ABORT, 'injected'); END;"))?; Ok(()) }).unwrap();
        assert_eq!(
            delete_image(&fixture, &before[0]),
            Err(AppError::DatabaseOperation)
        );
        assert_eq!(fixture.ordered(), before);
        for image in &before {
            assert!(fixture.media.preview(image).is_some());
        }
        assert_eq!(fixture.count("quarantine"), 0);
        fixture
            .db
            .with(|db| {
                db.execute_batch("DROP TRIGGER fail_delete")?;
                Ok(())
            })
            .unwrap();
        assert!(delete_image(&fixture, &before[0]).unwrap().deleted);
    }
}

#[test]
fn delete_commit_failure_rechecks_rolled_back_references_and_preserves_file() {
    let fixture = Fixture::new();
    fixture.import_distinct(0, 1);
    let before = fixture.ordered();
    fixture.db.with(|db| {
        db.execute_batch("CREATE TABLE commit_blocker (image_id TEXT REFERENCES product_images(id) DEFERRABLE INITIALLY DEFERRED)")?;
        db.execute("INSERT INTO commit_blocker VALUES (?1)", [&before[0].id])?; Ok(())
    }).unwrap();
    assert_eq!(
        delete_image(&fixture, &before[0]),
        Err(AppError::DatabaseOperation)
    );
    assert_eq!(fixture.ordered(), before);
    assert!(fixture.media.preview(&before[0]).is_some());
    assert_eq!(fixture.count("quarantine"), 0);
}

#[test]
fn delete_commit_ambiguity_checks_id_path_and_unknown_database_state() {
    let fixture = Fixture::new();
    fixture.import_distinct(0, 1);
    let image = fixture.ordered().remove(0);
    let resolve = || {
        image_service::resolve_delete_commit(&fixture.db, &image, Err(AppError::DatabaseOperation))
    };
    assert_eq!(resolve(), Err(AppError::DatabaseOperation));
    fixture
        .db
        .with(|db| {
            repository::delete(db, &image.product_id, &image.id)?;
            Ok(())
        })
        .unwrap();
    assert_eq!(resolve(), Ok(()));
    let mut replacement = image.clone();
    replacement.id = Uuid::new_v4().to_string();
    fixture
        .db
        .with(|db| repository::create(db, &replacement))
        .unwrap();
    assert_eq!(resolve(), Err(AppError::DatabaseOperation)); // same path, different ID
    fixture
        .db
        .with(|db| {
            repository::delete(db, &replacement.product_id, &replacement.id)?;
            replacement.relative_path =
                MediaStorage::relative_path(&image.product_id, &replacement.id, "png")?;
            replacement.id = image.id.clone();
            repository::create(db, &replacement)
        })
        .unwrap();
    assert_eq!(resolve(), Err(AppError::DatabaseOperation)); // same ID, different path
    fixture
        .db
        .with(|db| {
            db.execute_batch("BEGIN IMMEDIATE")?;
            Ok(())
        })
        .unwrap();
    assert_eq!(resolve(), Err(AppError::ImageDeletionUnknown));
    fixture
        .db
        .with(|db| {
            db.execute_batch("ROLLBACK; DROP TABLE product_images")?;
            Ok(())
        })
        .unwrap();
    assert_eq!(resolve(), Err(AppError::ImageDeletionUnknown));
    assert!(fixture.media.preview(&image).is_some());
}

#[test]
fn delete_quarantine_collision_never_overwrites_and_restart_recovers_pending() {
    let fixture = Fixture::new();
    fixture.import_distinct(0, 1);
    let image = fixture.ordered().remove(0);
    let bytes = fs::read(fixture.media.image_path(&image).unwrap()).unwrap();
    let destination = deleted_path(&fixture, &image);
    fs::create_dir_all(destination.parent().unwrap()).unwrap();
    fs::write(&destination, b"previous quarantined image").unwrap();
    let report = delete_image(&fixture, &image).unwrap();
    assert_eq!(
        (
            report.deleted,
            report.pending_quarantine_count,
            report.missing_image_count
        ),
        (true, 1, 0)
    );
    assert!(fixture.ordered().is_empty());
    assert_eq!(
        fs::read(&destination).unwrap(),
        b"previous quarantined image"
    );
    assert_eq!(
        fs::read(fixture.media.image_path(&image).unwrap()).unwrap(),
        bytes
    );
    let Fixture {
        media, db, temp, ..
    } = fixture;
    drop(db);
    drop(media);
    let reopened = connection::open(&temp.0.join("test.sqlite3")).unwrap();
    let media =
        image_service::initialize_after_preflight(&reopened, &temp.0.join("media")).unwrap();
    let recovered = fs::read_dir(temp.0.join("media/quarantine"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| path.extension().is_some_and(|ext| ext == "recovered"))
        .unwrap();
    assert_eq!(fs::read(recovered).unwrap(), bytes);
    assert_eq!(
        fs::read(destination).unwrap(),
        b"previous quarantined image"
    );
    drop(media);
    drop(reopened);
}

#[test]
fn delete_and_import_reorder_share_guard_before_database_lock() {
    let fixture = Fixture::new();
    fixture.import_distinct(0, 1);
    let image = fixture.ordered().remove(0);
    let ids = vec![image.id.clone()];
    let permit = fixture.media.begin_import().unwrap();
    {
        let _db_lock = fixture.db.0.lock().unwrap();
        assert_eq!(delete_image(&fixture, &image), Err(AppError::MediaBusy));
        assert_eq!(fixture.reorder(&ids, &ids), Err(AppError::MediaBusy));
        assert!(matches!(
            image_service::import_files(&fixture.db, &fixture.media, &fixture.product.id, &[]),
            Err(AppError::MediaBusy)
        ));
    }
    drop(permit);
    assert!(delete_image(&fixture, &image).unwrap().deleted);
    fixture.import_distinct(1, 1);
}

#[test]
fn delete_rejects_unsafe_database_paths_and_non_regular_sources_before_db_changes() {
    for unsafe_path in [true, false] {
        let fixture = Fixture::new();
        fixture.import_distinct(0, 1);
        let image = fixture.ordered().remove(0);
        let path = fixture.media.image_path(&image).unwrap();
        if unsafe_path {
            fixture
                .db
                .with(|db| {
                    db.execute("UPDATE product_images SET relative_path = '../unsafe'", [])?;
                    Ok(())
                })
                .unwrap();
        } else {
            fs::remove_file(&path).unwrap();
            fs::create_dir(&path).unwrap();
        }
        assert_eq!(
            delete_image(&fixture, &image),
            Err(AppError::UnsafeMediaPath)
        );
        assert_eq!(fixture.ordered().len(), 1);
        assert!(path.exists());
    }
}

#[cfg(windows)]
#[test]
fn delete_rejects_junction_in_source_and_retains_pending_for_destination_junction() {
    use std::process::Command;
    for source_link in [true, false] {
        let fixture = Fixture::new();
        fixture.import_distinct(0, 1);
        let image = fixture.ordered().remove(0);
        let original = fixture.media.image_path(&image).unwrap();
        let bytes = fs::read(&original).unwrap();
        let target = fixture.temp.0.join("junction-target");
        fs::create_dir(&target).unwrap();
        let link = if source_link {
            fs::rename(&original, target.join(original.file_name().unwrap())).unwrap();
            fs::remove_dir(original.parent().unwrap()).unwrap();
            original.parent().unwrap().to_path_buf()
        } else {
            fixture
                .temp
                .0
                .join("media")
                .join("quarantine")
                .join("deleted")
        };
        // cmd's mklink treats forward slashes embedded in a path as switches.
        let link = fs::canonicalize(link.parent().unwrap())
            .unwrap()
            .join(link.file_name().unwrap());
        let result = Command::new("cmd")
            .args(["/c", "mklink", "/J"])
            .arg(&link)
            .arg(&target)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "junction creation failed: {:?}",
            result
        );
        if source_link {
            assert_eq!(
                delete_image(&fixture, &image),
                Err(AppError::UnsafeMediaPath)
            );
            assert_eq!(fixture.ordered().len(), 1);
        } else {
            let report = delete_image(&fixture, &image).unwrap();
            assert_eq!(report.pending_quarantine_count, 1);
            assert_eq!(fs::read(&original).unwrap(), bytes);
        }
        fs::remove_dir(link).unwrap();
        if source_link {
            assert_eq!(
                fs::read(target.join(original.file_name().unwrap())).unwrap(),
                bytes
            );
        } else {
            assert_eq!(fs::read_dir(target).unwrap().count(), 0);
        }
    }
}

#[cfg(windows)]
#[test]
fn delete_unreadable_source_is_not_missing_and_locked_move_is_pending() {
    use std::os::windows::fs::OpenOptionsExt;
    for share_mode in [0, 1] {
        let fixture = Fixture::new();
        fixture.import_distinct(0, 1);
        let image = fixture.ordered().remove(0);
        let path = fixture.media.image_path(&image).unwrap();
        let lock = fs::OpenOptions::new()
            .read(true)
            .share_mode(share_mode)
            .open(&path)
            .unwrap();
        if share_mode == 0 {
            assert_eq!(delete_image(&fixture, &image), Err(AppError::FileRead));
            assert_eq!(fixture.ordered().len(), 1);
        } else {
            let report = delete_image(&fixture, &image).unwrap();
            assert_eq!(
                (report.pending_quarantine_count, report.missing_image_count),
                (1, 0)
            );
            assert!(fixture.ordered().is_empty());
        }
        drop(lock);
        assert!(path.exists());
    }
}

#[test]
fn delete_finalization_rechecks_missing_and_unsafe_sources() {
    for unsafe_source in [true, false] {
        let fixture = Fixture::new();
        fixture.import_distinct(0, 1);
        let image = fixture.ordered().remove(0);
        fixture
            .db
            .with(|db| repository::delete(db, &image.product_id, &image.id))
            .unwrap();
        let path = fixture.media.image_path(&image).unwrap();
        fs::remove_file(&path).unwrap();
        if unsafe_source {
            fs::create_dir(&path).unwrap();
        }
        let report = image_service::finish_delete(&fixture.media, &image);
        assert_eq!(report.pending_quarantine_count, usize::from(unsafe_source));
        assert_eq!(report.missing_image_count, usize::from(!unsafe_source));
    }
}

// Phase 3.3: all fixtures below use dedicated temporary DB/media only.
fn product_delete_input(fixture: &Fixture) -> crate::domain::product::DeleteProductInput {
    let preview =
        crate::services::product_deletion_service::preview(&fixture.db, &fixture.product.id)
            .unwrap();
    assert_eq!(preview.image_count, preview.image_ids.len());
    assert_eq!(preview.image_ids, image_ids(&fixture.ordered()));
    crate::domain::product::DeleteProductInput {
        product_id: preview.product_id,
        expected_updated_at: preview.updated_at,
        expected_image_ids: preview.image_ids,
        expected_product: preview.product,
        expected_attribute_values: preview.attribute_values,
    }
}
fn delete_product(
    f: &Fixture,
    input: &crate::domain::product::DeleteProductInput,
) -> Result<crate::domain::image::DeleteImageReport, AppError> {
    crate::services::product_deletion_service::delete(&f.db, &f.media, input)
}

#[test]
fn product_delete_zero_one_four_images_preserves_sources_other_product_and_restart() {
    for count in [0, 1, 4] {
        let f = Fixture::new();
        f.import_distinct(0, count);
        let before = f.ordered();
        let other =
            f.db.with(|db| product_service::create(db, input("other-product-delete")))
                .unwrap();
        if count > 0 {
            assert_eq!(
                image_service::import_files(
                    &f.db,
                    &f.media,
                    &other.id,
                    &[f.temp.0.join("image-0.png")]
                )
                .unwrap()
                .items[0]
                    .status,
                ImportStatus::Imported
            );
        }
        let other_images = f.db.with(|db| repository::list(db, &other.id)).unwrap();
        let report = delete_product(&f, &product_delete_input(&f)).unwrap();
        assert!(report.deleted);
        assert_eq!(report.affected_image_count, count as usize);
        assert_eq!(report.quarantined_image_count, count as usize);
        assert_eq!(
            report.missing_image_count + report.pending_quarantine_count,
            0
        );
        assert!(f.ordered().is_empty());
        assert_eq!(
            f.db.with(|db| product_service::get(db, &f.product.id)),
            Err(AppError::ProductNotFound)
        );
        for image in &before {
            let bytes = fs::read(deleted_path(&f, image)).unwrap();
            assert_eq!(
                bytes,
                fs::read(f.temp.0.join(&image.original_filename)).unwrap()
            );
            assert_eq!(format!("{:x}", Sha256::digest(&bytes)), image.sha256);
        }
        let Fixture {
            media,
            db,
            temp,
            product,
        } = f;
        drop(media);
        drop(db);
        let connection = connection::open(&temp.0.join("test.sqlite3")).unwrap();
        // Fixture uses this exact path; a separate connection checks persistence.
        assert_eq!(
            product_service::get(&connection, &product.id),
            Err(AppError::ProductNotFound)
        );
        assert_eq!(product_service::get(&connection, &other.id).unwrap(), other);
        assert_eq!(
            repository::list(&connection, &other.id).unwrap(),
            other_images
        );
        let media = image_service::initialize(&Ok(connection), &temp.0.join("media")).unwrap();
        for image in &before {
            let path = temp
                .0
                .join("media/quarantine/deleted")
                .join(&image.product_id)
                .join(format!("{}.png", image.id));
            assert!(path.is_file());
        }
        for image in &other_images {
            assert!(media.preview(image).is_some());
        }
    }
}

#[test]
fn product_delete_rejects_stale_timestamp_and_same_timestamp_product_changes() {
    for column in ["updated_at", "name", "sku", "brand", "description_note"] {
        let f = Fixture::new();
        f.import_distinct(0, 2);
        let expected = product_delete_input(&f);
        f.db.with(|db| {
            db.execute(
                &format!("UPDATE products SET {column} = 'changed' WHERE id = ?1"),
                [&f.product.id],
            )?;
            Ok(())
        })
        .unwrap();
        assert_eq!(delete_product(&f, &expected), Err(AppError::StaleData));
        assert_eq!(f.ordered().len(), 2);
        assert!(
            f.ordered()
                .iter()
                .all(|image| f.media.preview(image).is_some())
        );
    }
}

#[test]
fn product_delete_rejects_added_removed_reordered_images() {
    for mode in 0..3 {
        let f = Fixture::new();
        f.import_distinct(0, 3);
        let expected = product_delete_input(&f);
        match mode {
            0 => f.import_distinct(5, 1),
            1 => {
                delete_image(&f, &f.ordered()[0]).unwrap();
            }
            _ => {
                let mut reversed = expected.expected_image_ids.clone();
                reversed.reverse();
                f.reorder(&expected.expected_image_ids, &reversed).unwrap();
            }
        }
        let before = f.ordered();
        assert_eq!(delete_product(&f, &expected), Err(AppError::StaleData));
        assert_eq!(f.ordered(), before);
    }
}

#[test]
fn product_delete_rejects_duplicate_foreign_invalid_and_missing_ids() {
    for mode in 0..6 {
        let f = Fixture::new();
        f.import_distinct(0, 2);
        let mut expected = product_delete_input(&f);
        match mode {
            0 => expected.expected_image_ids[1] = expected.expected_image_ids[0].clone(),
            1 => {
                let other =
                    f.db.with(|db| product_service::create(db, input("foreign-delete")))
                        .unwrap();
                image_service::import_files(
                    &f.db,
                    &f.media,
                    &other.id,
                    &[f.temp.0.join("image-0.png")],
                )
                .unwrap();
                expected.expected_image_ids[0] =
                    f.db.with(|db| repository::list(db, &other.id)).unwrap()[0]
                        .id
                        .clone();
            }
            2 => expected.product_id = "invalid".into(),
            3 => expected.expected_image_ids[0] = "invalid".into(),
            4 => expected.product_id = Uuid::new_v4().to_string(),
            _ => expected.expected_image_ids.clear(),
        }
        assert_eq!(delete_product(&f, &expected), Err(AppError::StaleData));
        assert_eq!(f.ordered().len(), 2);
    }
}

#[test]
fn product_preview_not_found_is_read_only() {
    let f = Fixture::new();
    for id in ["invalid".to_string(), Uuid::new_v4().to_string()] {
        assert!(matches!(
            crate::services::product_deletion_service::preview(&f.db, &id),
            Err(AppError::ProductNotFound)
        ));
    }
    assert_eq!(f.count("quarantine"), 0);
}

#[test]
fn product_delete_sql_image_partial_and_product_failures_rollback_every_record() {
    for trigger in [
        "BEFORE DELETE ON product_images WHEN OLD.sort_order = 1",
        "BEFORE DELETE ON products",
        "BEFORE DELETE ON products WHEN OLD.name IS NOT NULL",
    ] {
        let f = Fixture::new();
        f.import_distinct(0, 4);
        let expected = product_delete_input(&f);
        let before = f.ordered();
        f.db.with(|db| {
            db.execute_batch(&format!(
                "CREATE TRIGGER injected {trigger} BEGIN SELECT RAISE(ABORT, 'injected'); END"
            ))?;
            Ok(())
        })
        .unwrap();
        assert_eq!(
            delete_product(&f, &expected),
            Err(AppError::DatabaseOperation)
        );
        assert_eq!(f.ordered(), before);
        assert_eq!(
            f.db.with(|db| product_service::get(db, &f.product.id))
                .unwrap(),
            f.product
        );
        for image in &before {
            assert!(f.media.preview(image).is_some());
        }
        assert_eq!(f.count("quarantine"), 0);
    }
}

#[test]
fn schema_two_startup_migration_keeps_registered_images_and_original_bytes() {
    let fixture = Fixture::new();
    fixture.import_distinct(60, 2);
    let images = fixture.ordered();
    let bytes: Vec<_> = images
        .iter()
        .map(|i| fs::read(fixture.media.image_path(i).unwrap()).unwrap())
        .collect();
    // Reproduce the exact pre-4.1 schema inside this dedicated temporary fixture.
    fixture.db.with(|db| {
        db.execute_batch("DROP TABLE product_attribute_values; DROP TABLE attribute_definitions; PRAGMA user_version = 2;")?;
        Ok(())
    }).unwrap();
    let directory = fixture.temp.0.clone();
    let product = fixture.product.clone();
    drop(fixture.db);
    drop(fixture.media);
    fs::rename(
        directory.join("test.sqlite3"),
        directory.join("listing-pilot.sqlite3"),
    )
    .unwrap();
    let (db, media) = crate::services::startup_service::initialize(&directory).unwrap();
    assert_eq!(
        db.pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
            .unwrap(),
        3
    );
    assert_eq!(product_service::get(&db, &product.id).unwrap(), product);
    assert_eq!(repository::list(&db, &product.id).unwrap(), images);
    for (image, expected_bytes) in images.iter().zip(&bytes) {
        assert_eq!(
            &fs::read(media.image_path(image).unwrap()).unwrap(),
            expected_bytes
        );
    }
    assert_eq!(media.reconcile(&images).unwrap(), 0);
    drop(db);
    drop(media);
}

#[test]
fn attribute_value_delete_failure_preserves_product_images_bytes_after_restart() {
    use crate::{domain::attribute::CreateAttributeInput, services::attribute_service};
    let fixture = Fixture::new();
    fixture.import_distinct(40, 2);
    let images = fixture.ordered();
    let bytes: Vec<_> = images
        .iter()
        .map(|i| fs::read(fixture.media.image_path(i).unwrap()).unwrap())
        .collect();
    let attribute = fixture
        .db
        .with(|db| {
            let a = attribute_service::create(
                db,
                CreateAttributeInput {
                    name: "重量".into(),
                    data_type: "number".into(),
                    unit: Some("g".into()),
                },
            )?;
            db.execute(
                "INSERT INTO product_attribute_values VALUES (?1, ?2, '0.1234567890123456789')",
                rusqlite::params![fixture.product.id, a.id],
            )?;
            attribute_service::set_archived(
                db,
                crate::domain::attribute::SetAttributeArchivedInput {
                    id: a.id,
                    archived: true,
                    expected_updated_at: a.updated_at,
                },
            )
        })
        .unwrap();
    fixture.db.with(|db| { db.execute_batch("CREATE TRIGGER fail_value_delete BEFORE DELETE ON product_attribute_values BEGIN SELECT RAISE(ABORT, 'injected'); END")?; Ok(()) }).unwrap();
    let expected = product_delete_input(&fixture);
    assert_eq!(
        delete_product(&fixture, &expected),
        Err(AppError::DatabaseOperation)
    );
    assert_eq!(fixture.ordered(), images);
    fixture
        .db
        .with(|db| {
            assert_eq!(
                product_service::get(db, &fixture.product.id).unwrap(),
                fixture.product
            );
            assert_eq!(
                crate::infrastructure::db::attribute_repository::get(db, &attribute.id).unwrap(),
                attribute
            );
            assert_eq!(
                db.query_row("SELECT value FROM product_attribute_values", [], |r| r
                    .get::<_, String>(
                    0
                ))
                .unwrap(),
                "0.1234567890123456789"
            );
            Ok(())
        })
        .unwrap();
    for (image, expected_bytes) in images.iter().zip(&bytes) {
        assert_eq!(
            &fs::read(fixture.media.image_path(image).unwrap()).unwrap(),
            expected_bytes
        );
        assert!(!deleted_path(&fixture, image).exists());
    }
    let directory = fixture.temp.0.clone();
    let product = fixture.product.clone();
    // Release both the database and media lock before startup reconciliation.
    drop(fixture.db);
    drop(fixture.media);
    fs::rename(
        directory.join("test.sqlite3"),
        directory.join("listing-pilot.sqlite3"),
    )
    .unwrap();
    let (db, media) = crate::services::startup_service::initialize(&directory).unwrap();
    assert_eq!(repository::list(&db, &product.id).unwrap(), images);
    assert_eq!(product_service::get(&db, &product.id).unwrap(), product);
    assert_eq!(
        crate::infrastructure::db::attribute_repository::get(&db, &attribute.id).unwrap(),
        attribute
    );
    assert_eq!(
        db.query_row("SELECT value FROM product_attribute_values", [], |r| r
            .get::<_, String>(
            0
        ))
        .unwrap(),
        "0.1234567890123456789"
    );
    for (image, expected_bytes) in images.iter().zip(&bytes) {
        assert_eq!(
            &fs::read(media.image_path(image).unwrap()).unwrap(),
            expected_bytes
        );
    }
    drop(db);
    drop(media);
}

#[test]
fn product_delete_commit_failure_rolls_back_and_preserves_files() {
    let f = Fixture::new();
    f.import_distinct(0, 3);
    let expected = product_delete_input(&f);
    let before = f.ordered();
    f.db.with(|db| {
        db.execute_batch("CREATE TABLE blocker (product_id TEXT REFERENCES products(id) DEFERRABLE INITIALLY DEFERRED)")?;
        db.execute("INSERT INTO blocker VALUES (?1)", [&f.product.id])?; Ok(())
    }).unwrap();
    assert_eq!(
        delete_product(&f, &expected),
        Err(AppError::DatabaseOperation)
    );
    assert_eq!(f.ordered(), before);
    for image in before {
        assert!(f.media.preview(&image).is_some());
    }
}

#[test]
fn product_delete_commit_resolution_checks_product_all_ids_paths_and_unknown_state() {
    let f = Fixture::new();
    f.import_distinct(0, 3);
    let before = f.ordered();
    let resolve = || {
        crate::services::product_deletion_service::resolve_commit(
            &f.db,
            &f.product.id,
            &before,
            &[],
            Err(AppError::DatabaseOperation),
        )
    };
    assert_eq!(resolve(), Err(AppError::DatabaseOperation));
    f.db.with(|db| {
        for image in &before {
            repository::delete(db, &f.product.id, &image.id)?;
        }
        Ok(())
    })
    .unwrap();
    assert_eq!(resolve(), Err(AppError::ProductDeletionUnknown)); // partial state
    f.db.with(|db| crate::infrastructure::db::product_repository::delete(db, &f.product.id))
        .unwrap();
    assert_eq!(resolve(), Ok(())); // simulated error after successful commit
    let other =
        f.db.with(|db| product_service::create(db, input("resolve-other")))
            .unwrap();
    for same_id in [false, true] {
        let mut replacement = before[2].clone();
        replacement.product_id = other.id.clone();
        if same_id {
            replacement.relative_path =
                MediaStorage::relative_path(&other.id, &replacement.id, "png").unwrap();
        } else {
            replacement.id = Uuid::new_v4().to_string();
        }
        f.db.with(|db| repository::create(db, &replacement))
            .unwrap();
        assert_eq!(resolve(), Err(AppError::ProductDeletionUnknown));
        f.db.with(|db| repository::delete(db, &other.id, &replacement.id))
            .unwrap();
    }
    f.db.with(|db| {
        db.execute_batch("BEGIN IMMEDIATE")?;
        Ok(())
    })
    .unwrap();
    assert_eq!(resolve(), Err(AppError::ProductDeletionUnknown));
    f.db.with(|db| {
        db.execute_batch("ROLLBACK; DROP TABLE product_images")?;
        Ok(())
    })
    .unwrap();
    assert_eq!(resolve(), Err(AppError::ProductDeletionUnknown));
    for image in before {
        assert!(f.media.preview(&image).is_some());
    }
}

#[test]
fn product_delete_partial_quarantine_collision_and_missing_aggregate_and_restart_recover() {
    let f = Fixture::new();
    f.import_distinct(0, 4);
    let expected = product_delete_input(&f);
    let before = f.ordered();
    let collision = deleted_path(&f, &before[1]);
    fs::create_dir_all(collision.parent().unwrap()).unwrap();
    fs::write(&collision, b"existing").unwrap();
    fs::remove_file(f.media.image_path(&before[2]).unwrap()).unwrap();
    let report = delete_product(&f, &expected).unwrap();
    assert_eq!(
        (
            report.affected_image_count,
            report.quarantined_image_count,
            report.missing_image_count,
            report.pending_quarantine_count
        ),
        (4, 2, 1, 1)
    );
    assert_eq!(fs::read(&collision).unwrap(), b"existing");
    let retained = fs::read(f.media.image_path(&before[1]).unwrap()).unwrap();
    let Fixture {
        media, db, temp, ..
    } = f;
    drop(media);
    drop(db);
    let connection = connection::open(&temp.0.join("test.sqlite3")).unwrap();
    let media = image_service::initialize(&Ok(connection), &temp.0.join("media")).unwrap();
    assert_eq!(fs::read(collision).unwrap(), b"existing");
    assert!(
        fs::read_dir(temp.0.join("media/quarantine"))
            .unwrap()
            .filter_map(Result::ok)
            .any(|entry| entry
                .path()
                .extension()
                .is_some_and(|ext| ext == "recovered")
                && fs::read(entry.path()).unwrap() == retained)
    );
    assert!(media.preview(&before[1]).is_none());
}

#[test]
fn product_delete_all_missing_accepts_missing_product_directory() {
    let f = Fixture::new();
    f.import_distinct(0, 3);
    let expected = product_delete_input(&f);
    for image in f.ordered() {
        fs::remove_file(f.media.image_path(&image).unwrap()).unwrap();
    }
    fs::remove_dir(f.temp.0.join("media/originals").join(&f.product.id)).unwrap();
    let report = delete_product(&f, &expected).unwrap();
    assert_eq!(
        (
            report.affected_image_count,
            report.missing_image_count,
            report.quarantined_image_count,
            report.pending_quarantine_count
        ),
        (3, 3, 0, 0)
    );
}

#[test]
fn product_delete_unsafe_path_or_nonregular_file_aborts_all_before_db_change() {
    for mode in 0..3 {
        let f = Fixture::new();
        f.import_distinct(0, 3);
        let expected = product_delete_input(&f);
        let before = f.ordered();
        if mode == 0 {
            f.db.with(|db| {
                db.execute(
                    "UPDATE product_images SET relative_path = '../escape.png' WHERE id = ?1",
                    [&before[2].id],
                )?;
                Ok(())
            })
            .unwrap();
        } else if mode == 1 {
            let path = f.media.image_path(&before[2]).unwrap();
            fs::remove_file(&path).unwrap();
            fs::create_dir(path).unwrap();
        } else {
            f.db.with(|db| {
                db.execute(
                    "UPDATE product_images SET mime_type = 'image/jpeg' WHERE id = ?1",
                    [&before[2].id],
                )?;
                Ok(())
            })
            .unwrap();
        }
        assert_eq!(
            delete_product(&f, &expected),
            Err(AppError::UnsafeMediaPath)
        );
        assert_eq!(f.ordered().len(), 3);
        assert!(f.media.preview(&before[0]).is_some());
        assert_eq!(f.count("quarantine"), 0);
    }
}

#[test]
fn product_delete_zero_and_multiple_images_share_import_reorder_delete_permit() {
    for count in [0, 3] {
        let f = Fixture::new();
        f.import_distinct(0, count);
        let expected = product_delete_input(&f);
        let permit = f.media.begin_import().unwrap();
        // Holding DB too proves a busy operation rejects before attempting the DB lock.
        let lock = f.db.0.lock().unwrap();
        assert_eq!(delete_product(&f, &expected), Err(AppError::MediaBusy));
        assert!(matches!(
            image_service::reorder(&f.db, &f.media, &f.product.id, &[], &[]),
            Err(AppError::MediaBusy)
        ));
        assert!(matches!(
            image_service::import_files(&f.db, &f.media, &f.product.id, &[]),
            Err(AppError::MediaBusy)
        ));
        assert_eq!(
            image_service::delete(&f.db, &f.media, &f.product.id, &Uuid::new_v4().to_string()),
            Err(AppError::MediaBusy)
        );
        drop(lock);
        drop(permit);
        assert!(delete_product(&f, &expected).unwrap().deleted);
    }
}

#[cfg(windows)]
#[test]
fn product_delete_read_error_aborts_all_and_locked_move_continues_other_images() {
    use std::os::windows::fs::OpenOptionsExt;
    for share_mode in [0, 1] {
        let f = Fixture::new();
        f.import_distinct(0, 3);
        let expected = product_delete_input(&f);
        let before = f.ordered();
        let path = f.media.image_path(&before[1]).unwrap();
        let lock = fs::OpenOptions::new()
            .read(true)
            .share_mode(share_mode)
            .open(&path)
            .unwrap();
        if share_mode == 0 {
            assert_eq!(delete_product(&f, &expected), Err(AppError::FileRead));
            assert_eq!(f.ordered(), before);
            assert_eq!(f.count("quarantine"), 0);
        } else {
            let report = delete_product(&f, &expected).unwrap();
            assert_eq!(
                (
                    report.quarantined_image_count,
                    report.pending_quarantine_count,
                    report.missing_image_count
                ),
                (2, 1, 0)
            );
        }
        drop(lock);
        assert!(path.is_file());
    }
}

#[cfg(windows)]
#[test]
fn product_delete_rejects_source_reparse_point_and_retains_destination_reparse_point() {
    for source in [true, false] {
        let f = Fixture::new();
        f.import_distinct(0, 3);
        let expected = product_delete_input(&f);
        let target = f.temp.0.join("junction-target");
        fs::create_dir(&target).unwrap();
        let link = if source {
            let original = f.temp.0.join("media/originals").join(&f.product.id);
            for image in f.ordered() {
                let path = f.media.image_path(&image).unwrap();
                fs::rename(&path, target.join(path.file_name().unwrap())).unwrap();
            }
            fs::remove_dir(&original).unwrap();
            original
        } else {
            f.temp.0.join("media/quarantine/deleted")
        };
        let link = fs::canonicalize(link.parent().unwrap())
            .unwrap()
            .join(link.file_name().unwrap());
        assert!(
            std::process::Command::new("cmd")
                .args(["/c", "mklink", "/J"])
                .arg(&link)
                .arg(&target)
                .output()
                .unwrap()
                .status
                .success()
        );
        if source {
            assert_eq!(
                delete_product(&f, &expected),
                Err(AppError::UnsafeMediaPath)
            );
            assert_eq!(f.ordered().len(), 3);
        } else {
            let report = delete_product(&f, &expected).unwrap();
            assert_eq!(report.pending_quarantine_count, 3);
            assert_eq!(fs::read_dir(&target).unwrap().count(), 0);
        }
        fs::remove_dir(link).unwrap();
    }
}

// Phase 4.2: values participate in deletion snapshots, rollback and commit resolution.
fn seed_attribute_values(f: &Fixture) -> crate::domain::attribute::AttributeDefinition {
    f.db.with(|db| {
        let a = crate::services::attribute_service::create(
            db,
            crate::domain::attribute::CreateAttributeInput {
                name: "素材".into(),
                data_type: "text".into(),
                unit: None,
            },
        )?;
        crate::infrastructure::db::attribute_value_repository::set(
            db,
            &f.product.id,
            &a.id,
            Some("陶器"),
        )?;
        Ok(a)
    })
    .unwrap()
}
#[test]
fn product_delete_values_archived_and_images_preserves_other_product_and_definitions() {
    for count in [0, 2] {
        let f = Fixture::new();
        f.import_distinct(40, count);
        let images = f.ordered();
        let a = seed_attribute_values(&f);
        let other =
            f.db.with(|db| {
                let p = product_service::create(db, input("other-values"))?;
                crate::infrastructure::db::attribute_value_repository::set(
                    db,
                    &p.id,
                    &a.id,
                    Some("他商品"),
                )?;
                crate::services::attribute_service::set_archived(
                    db,
                    crate::domain::attribute::SetAttributeArchivedInput {
                        id: a.id.clone(),
                        archived: true,
                        expected_updated_at: a.updated_at.clone(),
                    },
                )?;
                Ok(p)
            })
            .unwrap();
        let expected = product_delete_input(&f);
        assert_eq!(expected.expected_attribute_values.len(), 1);
        let report = delete_product(&f, &expected).unwrap();
        assert!(report.deleted);
        assert_eq!(report.affected_image_count, count as usize);
        f.db.with(|db| {
            assert!(
                crate::infrastructure::db::attribute_value_repository::list(db, &f.product.id)?
                    .is_empty()
            );
            assert_eq!(
                crate::infrastructure::db::attribute_value_repository::list(db, &other.id)?[0]
                    .value,
                "他商品"
            );
            assert!(
                crate::infrastructure::db::attribute_repository::get(db, &a.id)?
                    .archived_at
                    .is_some()
            );
            Ok(())
        })
        .unwrap();
        for i in &images {
            assert!(deleted_path(&f, i).exists());
        }
    }
}
#[test]
fn product_delete_rejects_value_add_change_remove_without_updated_product_token() {
    for action in ["add", "change", "remove"] {
        let f = Fixture::new();
        f.import_distinct(40, 1);
        let images = f.ordered();
        let a = seed_attribute_values(&f);
        let expected = product_delete_input(&f);
        f.db.with(|db| {
            match action {
                "add" => {
                    let b = crate::services::attribute_service::create(
                        db,
                        crate::domain::attribute::CreateAttributeInput {
                            name: "追加".into(),
                            data_type: "text".into(),
                            unit: None,
                        },
                    )?;
                    crate::infrastructure::db::attribute_value_repository::set(
                        db,
                        &f.product.id,
                        &b.id,
                        Some("x"),
                    )?;
                }
                "change" => crate::infrastructure::db::attribute_value_repository::set(
                    db,
                    &f.product.id,
                    &a.id,
                    Some("変更"),
                )?,
                _ => crate::infrastructure::db::attribute_value_repository::set(
                    db,
                    &f.product.id,
                    &a.id,
                    None,
                )?,
            }
            Ok(())
        })
        .unwrap();
        assert_eq!(delete_product(&f, &expected), Err(AppError::StaleData));
        assert_eq!(f.ordered(), images);
        for i in &images {
            assert!(f.media.preview(i).is_some());
        }
    }
}
#[test]
fn product_delete_definition_rename_does_not_invalidate_value_snapshot() {
    let f = Fixture::new();
    let a = seed_attribute_values(&f);
    let expected = product_delete_input(&f);
    f.db.with(|db| {
        crate::services::attribute_service::update(
            db,
            crate::domain::attribute::UpdateAttributeInput {
                id: a.id,
                name: "名称変更".into(),
                expected_updated_at: a.updated_at,
            },
        )
    })
    .unwrap();
    assert!(delete_product(&f, &expected).unwrap().deleted);
}
#[test]
fn product_delete_late_failure_restores_deleted_values_and_images() {
    let f = Fixture::new();
    f.import_distinct(40, 2);
    seed_attribute_values(&f);
    let expected = product_delete_input(&f);
    let images = f.ordered();
    f.db.with(|db| {db.execute_batch("CREATE TRIGGER fail_product_values BEFORE DELETE ON products BEGIN SELECT RAISE(ABORT,'injected'); END")?;Ok(())}).unwrap();
    assert_eq!(
        delete_product(&f, &expected),
        Err(AppError::DatabaseOperation)
    );
    assert_eq!(f.ordered(), images);
    f.db.with(|db| {
        assert_eq!(
            crate::infrastructure::db::attribute_value_repository::list(db, &f.product.id)?,
            expected.expected_attribute_values
        );
        Ok(())
    })
    .unwrap();
    for i in &images {
        assert!(f.media.preview(i).is_some());
    }
}
#[test]
fn product_delete_commit_unknown_checks_value_references_and_full_rollback() {
    let f = Fixture::new();
    f.import_distinct(40, 1);
    let a = seed_attribute_values(&f);
    let images = f.ordered();
    let expected = product_delete_input(&f);
    let resolve = || {
        crate::services::product_deletion_service::resolve_commit(
            &f.db,
            &f.product.id,
            &images,
            &expected.expected_attribute_values,
            Err(AppError::DatabaseOperation),
        )
    };
    assert_eq!(resolve(), Err(AppError::DatabaseOperation));
    f.db.with(|db| {
        crate::infrastructure::db::attribute_value_repository::set(
            db,
            &f.product.id,
            &a.id,
            Some("変更"),
        )
    })
    .unwrap();
    assert_eq!(resolve(), Err(AppError::ProductDeletionUnknown));
    f.db.with(|db| {
        db.pragma_update(None, "foreign_keys", false)?;
        db.execute(
            "DELETE FROM product_images WHERE product_id=?1",
            [&f.product.id],
        )?;
        db.execute("DELETE FROM products WHERE id=?1", [&f.product.id])?;
        Ok(())
    })
    .unwrap();
    assert_eq!(resolve(), Err(AppError::ProductDeletionUnknown)); // orphan value still prohibits quarantine
    f.db.with(|db| {
        crate::infrastructure::db::attribute_value_repository::delete_product(db, &f.product.id, 1)
    })
    .unwrap();
    assert_eq!(resolve(), Ok(()));
    for i in &images {
        assert!(f.media.preview(i).is_some());
    } // resolver itself never moves files
}
