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
                migrations::apply(&mut db).unwrap();
            }
            Some(product)
        } else {
            None
        };
        migrations::apply(&mut db).unwrap();
        assert_eq!(
            db.pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
                .unwrap(),
            2
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
