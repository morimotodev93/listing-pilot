use crate::{
    domain::image::{MAX_FILE_BYTES, MAX_PIXELS, ProductImage},
    error::AppError,
};
use image::{ImageDecoder, ImageFormat, ImageReader, Limits};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs::{self, File, Metadata, OpenOptions},
    io::{BufReader, Read, Write},
    path::{Component, Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use uuid::Uuid;

pub struct MediaStorage {
    root: PathBuf,
    // OS lock lives as long as the storage. A second process cannot import or reconcile.
    _lock: File,
    importing: Arc<AtomicBool>,
}

pub struct ImportPermit(Arc<AtomicBool>);
impl Drop for ImportPermit {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

pub struct StagedImage {
    path: PathBuf,
    pub mime_type: &'static str,
    pub extension: &'static str,
    pub size: u64,
    pub width: u32,
    pub height: u32,
    pub sha256: String,
}

impl Drop for StagedImage {
    fn drop(&mut self) {
        // Only our UUID staging file; failures are recovered at next startup.
        if safe_regular_file(&self.path).is_ok() {
            let _ = fs::remove_file(&self.path);
        }
    }
}

fn is_link(metadata: &Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0 // all reparse points, including junctions
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

// Inspect each component, including parents: never follow symlinks/junctions.
fn check_components(path: &Path, create: bool) -> Result<(), AppError> {
    if !path.is_absolute() {
        return Err(AppError::UnsafeMediaPath);
    }
    let mut current = PathBuf::new();
    for part in path.components() {
        match part {
            Component::ParentDir | Component::CurDir => return Err(AppError::UnsafeMediaPath),
            Component::Prefix(_) | Component::RootDir => {
                current.push(part);
                continue;
            }
            Component::Normal(_) => current.push(part),
        }
        let metadata = match fs::symlink_metadata(&current) {
            Ok(metadata) => metadata,
            Err(error) if create && error.kind() == std::io::ErrorKind::NotFound => {
                match fs::create_dir(&current) {
                    Ok(()) => (),
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => (),
                    Err(_) => return Err(AppError::FileSave),
                }
                fs::symlink_metadata(&current).map_err(|_| AppError::FileSave)?
            }
            Err(_) => return Err(AppError::FileRead),
        };
        if is_link(&metadata) {
            return Err(AppError::UnsafeMediaPath);
        }
    }
    Ok(())
}

fn directory(path: &Path, create: bool) -> Result<(), AppError> {
    check_components(path, create)?;
    if !fs::symlink_metadata(path)
        .map_err(|_| AppError::FileRead)?
        .is_dir()
    {
        return Err(AppError::UnsafeMediaPath);
    }
    Ok(())
}

fn safe_regular_file(path: &Path) -> Result<Metadata, AppError> {
    check_components(path, false)?;
    let metadata = fs::symlink_metadata(path).map_err(|_| AppError::FileRead)?;
    if !metadata.is_file() || is_link(&metadata) {
        return Err(AppError::UnsafeMediaPath);
    }
    Ok(metadata)
}

fn open_read(path: &Path) -> Result<File, AppError> {
    safe_regular_file(path)?;
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.custom_flags(0x00200000); // FILE_FLAG_OPEN_REPARSE_POINT
    }
    let file = options.open(path).map_err(|_| AppError::FileRead)?;
    let metadata = file.metadata().map_err(|_| AppError::FileRead)?;
    if !metadata.is_file() || is_link(&metadata) {
        return Err(AppError::UnsafeMediaPath);
    }
    Ok(file)
}

fn valid_uuid(value: &str) -> bool {
    Uuid::parse_str(value).is_ok_and(|id| id.to_string() == value)
}

fn image_error(error: image::ImageError) -> AppError {
    match error {
        image::ImageError::Limits(_) => AppError::ImageTooLarge,
        _ => AppError::InvalidImage,
    }
}

#[cfg(windows)]
fn place_new(source: &Path, destination: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn MoveFileExW(existing: *const u16, new: *const u16, flags: u32) -> i32;
    }

    // canonicalize supplies Windows verbatim paths, preserving long-path support.
    // Only the destination parent exists; the final name is created by the move.
    let source = fs::canonicalize(source)?;
    let destination = fs::canonicalize(
        destination
            .parent()
            .ok_or(std::io::ErrorKind::InvalidInput)?,
    )?
    .join(
        destination
            .file_name()
            .ok_or(std::io::ErrorKind::InvalidInput)?,
    );
    let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let destination: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    // SAFETY: both pointers refer to live, NUL-terminated UTF-16 buffers.
    // flags=0 disallows replacement and cross-volume copy/delete fallbacks.
    let moved = unsafe { MoveFileExW(source.as_ptr(), destination.as_ptr(), 0) };
    if moved == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(not(windows))]
fn place_new(source: &Path, destination: &Path) -> std::io::Result<()> {
    // Atomic create-without-replacement on the same volume. StagedImage's Drop
    // removes the staging link after registration (or on failure).
    fs::hard_link(source, destination)
}

pub fn filename(path: &Path) -> String {
    path.file_name()
        .map(|name| {
            name.to_string_lossy()
                .chars()
                .filter(|c| !c.is_control())
                .take(255)
                .collect()
        })
        .unwrap_or_else(|| "画像".into())
}

pub fn check_dimensions(width: u32, height: u32) -> Result<(), AppError> {
    if width == 0 || height == 0 {
        return Err(AppError::InvalidImage);
    }
    if u64::from(width) * u64::from(height) > MAX_PIXELS {
        return Err(AppError::ImageTooLarge);
    }
    Ok(())
}

impl MediaStorage {
    pub fn open(root: &Path) -> Result<Self, AppError> {
        directory(root, true)?;
        let lock_path = root.join(".lock");
        if lock_path.try_exists().map_err(|_| AppError::FileRead)? {
            safe_regular_file(&lock_path)?;
        }
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            options.custom_flags(0x00200000);
        }
        let lock = options
            .open(lock_path)
            .map_err(|_| AppError::MediaInitialization)?;
        if is_link(&lock.metadata().map_err(|_| AppError::MediaInitialization)?) {
            return Err(AppError::UnsafeMediaPath);
        }
        lock.try_lock().map_err(|_| AppError::MediaBusy)?;
        for child in ["originals", ".staging", "quarantine"] {
            directory(&root.join(child), true)?;
        }
        Ok(Self {
            root: root.to_path_buf(),
            _lock: lock,
            importing: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn begin_import(&self) -> Result<ImportPermit, AppError> {
        self.importing
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| AppError::MediaBusy)?;
        Ok(ImportPermit(Arc::clone(&self.importing)))
    }

    pub fn stage(&self, source: &Path) -> Result<StagedImage, AppError> {
        let input = open_read(source)?;
        if input.metadata().map_err(|_| AppError::FileRead)?.len() > MAX_FILE_BYTES {
            return Err(AppError::ImageTooLarge);
        }
        let staging = self.root.join(".staging");
        directory(&staging, false)?;
        let mut staged = StagedImage {
            path: staging.join(format!("{}.tmp", Uuid::new_v4())),
            mime_type: "",
            extension: "",
            size: 0,
            width: 0,
            height: 0,
            sha256: String::new(),
        };
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&staged.path)
            .map_err(|_| AppError::FileSave)?;
        let mut input = input.take(MAX_FILE_BYTES + 1);
        let mut buffer = [0u8; 64 * 1024];
        loop {
            let read = input.read(&mut buffer).map_err(|_| AppError::FileRead)?;
            if read == 0 {
                break;
            }
            staged.size += read as u64;
            if staged.size > MAX_FILE_BYTES {
                return Err(AppError::ImageTooLarge);
            }
            output
                .write_all(&buffer[..read])
                .map_err(|_| AppError::FileSave)?;
        }
        output.sync_all().map_err(|_| AppError::FileSave)?;
        drop(output);
        let mut reader = ImageReader::new(BufReader::new(open_read(&staged.path)?))
            .with_guessed_format()
            .map_err(|_| AppError::InvalidImage)?;
        let format = reader.format().ok_or(AppError::UnsupportedImage)?;
        (staged.mime_type, staged.extension) = match format {
            ImageFormat::Jpeg => ("image/jpeg", "jpg"),
            ImageFormat::Png => ("image/png", "png"),
            ImageFormat::WebP => ("image/webp", "webp"),
            _ => return Err(AppError::UnsupportedImage),
        };
        // Check the product of dimensions before allocating a decoded image.
        let mut header_limits = Limits::default();
        header_limits.max_alloc = Some(256 * 1024 * 1024);
        reader.limits(header_limits);
        let decoder = reader.into_decoder().map_err(image_error)?;
        (staged.width, staged.height) = decoder.dimensions();
        check_dimensions(staged.width, staged.height)?;
        drop(decoder);
        let mut limits = Limits::default();
        limits.max_alloc = Some(256 * 1024 * 1024);
        limits.max_image_width = Some(staged.width);
        limits.max_image_height = Some(staged.height);
        let mut reader = ImageReader::with_format(BufReader::new(open_read(&staged.path)?), format);
        reader.limits(limits);
        reader.decode().map_err(image_error)?;
        let mut hash = Sha256::new();
        let mut input = open_read(&staged.path)?;
        loop {
            let read = input.read(&mut buffer).map_err(|_| AppError::FileRead)?;
            if read == 0 {
                break;
            }
            hash.update(&buffer[..read]);
        }
        staged.sha256 = format!("{:x}", hash.finalize());
        Ok(staged)
    }

    pub fn relative_path(
        product_id: &str,
        image_id: &str,
        extension: &str,
    ) -> Result<String, AppError> {
        if !valid_uuid(product_id)
            || !valid_uuid(image_id)
            || !["jpg", "png", "webp"].contains(&extension)
        {
            return Err(AppError::UnsafeMediaPath);
        }
        Ok(format!(
            "media/originals/{product_id}/{image_id}.{extension}"
        ))
    }

    pub fn image_path(&self, image: &ProductImage) -> Result<PathBuf, AppError> {
        let extension = match image.mime_type.as_str() {
            "image/jpeg" => "jpg",
            "image/png" => "png",
            "image/webp" => "webp",
            _ => return Err(AppError::UnsafeMediaPath),
        };
        let expected = Self::relative_path(&image.product_id, &image.id, extension)?;
        if expected != image.relative_path {
            return Err(AppError::UnsafeMediaPath);
        }
        Ok(self.root.join(
            expected
                .strip_prefix("media/")
                .ok_or(AppError::UnsafeMediaPath)?,
        ))
    }

    pub fn place(&self, staged: &StagedImage, image: &ProductImage) -> Result<(), AppError> {
        let destination = self.image_path(image)?;
        directory(destination.parent().ok_or(AppError::UnsafeMediaPath)?, true)?;
        safe_regular_file(&staged.path)?;
        // The OS checks destination absence as part of placement. A separate
        // existence check would race and could hide errors other than NotFound.
        place_new(&staged.path, &destination).map_err(|_| AppError::FileSave)
    }

    // Missing is accepted only after checking every existing ancestor without
    // following links. Permission/I/O errors must never become "missing".
    pub fn deletion_file_exists(&self, image: &ProductImage) -> Result<bool, AppError> {
        let path = self.image_path(image)?;
        directory(&self.root, false)?;
        let relative = path
            .strip_prefix(&self.root)
            .map_err(|_| AppError::UnsafeMediaPath)?;
        let mut current = self.root.clone();
        for component in relative.components() {
            current.push(component);
            let metadata = match fs::symlink_metadata(&current) {
                Ok(metadata) => metadata,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
                Err(_) => return Err(AppError::FileRead),
            };
            if is_link(&metadata)
                || (current != path && !metadata.is_dir())
                || (current == path && !metadata.is_file())
            {
                return Err(AppError::UnsafeMediaPath);
            }
        }
        open_read(&path)?;
        Ok(true)
    }

    pub fn quarantine_deleted(&self, image: &ProductImage) -> Result<bool, AppError> {
        if !self.deletion_file_exists(image)? {
            return Ok(false);
        }
        let source = self.image_path(image)?;
        let destination = self
            .root
            .join("quarantine/deleted")
            .join(&image.product_id)
            .join(source.file_name().ok_or(AppError::UnsafeMediaPath)?);
        directory(destination.parent().ok_or(AppError::UnsafeMediaPath)?, true)?;
        // Revalidate source after destination creation. No copy/delete fallback.
        if !self.deletion_file_exists(image)? {
            return Ok(false);
        }
        #[cfg(windows)]
        let moved = place_new(&source, &destination);
        // Other platforms retain the file until an atomic no-replace move is
        // implemented; hard-link plus unlink is not an atomic move.
        #[cfg(not(windows))]
        let moved: std::io::Result<()> = Err(std::io::ErrorKind::Unsupported.into());
        match moved {
            Ok(()) => Ok(true),
            Err(_) => match self.deletion_file_exists(image) {
                Ok(false) => Ok(false),
                _ => Err(AppError::FileSave),
            },
        }
    }

    pub fn remove_unregistered(&self, image: &ProductImage) -> Result<(), AppError> {
        let path = self.image_path(image)?;
        safe_regular_file(&path)?;
        fs::remove_file(path).map_err(|_| AppError::FileSave)
    }

    pub fn preview(&self, image: &ProductImage) -> Option<String> {
        let path = self.image_path(image).ok()?;
        open_read(&path).ok()?;
        path.to_str().map(str::to_owned)
    }

    // Only called before publishing the storage to Commands. ImportPermit and the
    // process lock also protect direct/test callers from racing reconciliation.
    pub fn reconcile(&self, images: &[ProductImage]) -> Result<usize, AppError> {
        let _permit = self.begin_import()?;
        // Validate the entire reference set before moving anything.
        let referenced = images
            .iter()
            .map(|image| self.image_path(image))
            .collect::<Result<HashSet<_>, _>>()?;
        let mut candidates = vec![];
        let staging = self.root.join(".staging");
        directory(&staging, false)?;
        for entry in fs::read_dir(staging).map_err(|_| AppError::FileRead)? {
            let path = entry.map_err(|_| AppError::FileRead)?.path();
            safe_regular_file(&path)?;
            candidates.push(path);
        }
        let originals = self.root.join("originals");
        directory(&originals, false)?;
        for entry in fs::read_dir(originals).map_err(|_| AppError::FileRead)? {
            let product_dir = entry.map_err(|_| AppError::FileRead)?.path();
            directory(&product_dir, false)?;
            for entry in fs::read_dir(product_dir).map_err(|_| AppError::FileRead)? {
                let path = entry.map_err(|_| AppError::FileRead)?.path();
                safe_regular_file(&path)?;
                if !referenced.contains(&path) {
                    candidates.push(path);
                }
            }
        }
        let quarantine = self.root.join("quarantine");
        directory(&quarantine, false)?;
        for path in &candidates {
            safe_regular_file(path)?;
            // UUID destination prevents clobbering previous recovery artifacts.
            let destination = quarantine.join(format!("{}.recovered", Uuid::new_v4()));
            fs::rename(path, destination).map_err(|_| AppError::FileSave)?;
        }
        Ok(candidates.len())
    }
}
