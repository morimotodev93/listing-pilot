use crate::{
    error::AppError,
    infrastructure::media_storage::{directory, safe_regular_file},
};
use std::{
    fs::{self, File, OpenOptions},
    path::{Path, PathBuf},
};
use uuid::Uuid;

pub(crate) fn destination(parent: &Path, data: &Path) -> Result<PathBuf, AppError> {
    directory(parent, false).map_err(|_| AppError::ExportDestination)?;
    let parent = fs::canonicalize(parent).map_err(|_| AppError::ExportDestination)?;
    let data = fs::canonicalize(data).map_err(|_| AppError::ExportDestination)?;
    if parent.starts_with(data) {
        return Err(AppError::ExportDestination);
    }
    Ok(parent)
}

pub(crate) struct Workspace {
    pub path: PathBuf,
    files: Vec<PathBuf>,
    directories: Vec<PathBuf>,
}

impl Workspace {
    pub fn new(parent: &Path) -> Result<Self, AppError> {
        let path = parent.join(format!(".ListingPilot-export-{}", Uuid::new_v4()));
        fs::create_dir(&path).map_err(|e| write_error("staging_create", &e))?;
        Ok(Self {
            path,
            files: vec![],
            directories: vec![],
        })
    }

    fn create_directory(&mut self, path: PathBuf) -> Result<(), AppError> {
        directory(path.parent().ok_or(AppError::ExportWrite)?, false)?;
        fs::create_dir(&path).map_err(|e| write_error("directory_create", &e))?;
        self.directories.push(path);
        Ok(())
    }

    pub fn images(&mut self) -> Result<(), AppError> {
        self.create_directory(self.path.join("images"))
    }

    pub fn product(&mut self, id: &str) -> Result<(), AppError> {
        if !canonical_uuid(id) {
            return Err(AppError::ExportWrite);
        }
        self.create_directory(self.path.join(id))?;
        self.create_directory(self.path.join(id).join("images"))
    }

    pub fn file(&mut self, relative: &str) -> Result<File, AppError> {
        validate_file(relative)?;
        let path = self.path.join(relative);
        let parent = path.parent().ok_or(AppError::ExportWrite)?;
        if parent != self.path && !self.directories.iter().any(|p| p == parent) {
            return Err(AppError::ExportWrite);
        }
        directory(parent, false)?;
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| write_error("file_create", &e))?;
        self.files.push(path);
        Ok(file)
    }

    // No recursive deletion: only files successfully created by this operation.
    // Unknown entries, links and directories are retained for manual inspection.
    pub fn cleanup(&self) -> Result<(), AppError> {
        directory(&self.path, false)?;
        for file in self.files.iter().rev() {
            safe_regular_file(file)?;
            fs::remove_file(file).map_err(|_| AppError::ExportWrite)?;
        }
        for path in self.directories.iter().rev() {
            directory(path, false)?;
            fs::remove_dir(path).map_err(|e| write_error("cleanup_directory", &e))?;
        }
        fs::remove_dir(&self.path).map_err(|_| AppError::ExportWrite)
    }

    pub fn finish(&self, target: &Path) -> Result<(), AppError> {
        directory(&self.path, false).map_err(|_| AppError::ExportUnknown)?;
        directory(target.parent().ok_or(AppError::ExportDestination)?, false)?;
        #[cfg(windows)]
        let result = crate::infrastructure::media_storage::place_new(&self.path, target);
        #[cfg(not(windows))]
        let result: std::io::Result<()> = Err(std::io::ErrorKind::Unsupported.into());
        match result {
            Ok(()) => Ok(()),
            Err(e) => {
                eprintln!(
                    "Export stage=final_move kind={:?} os={:?}",
                    e.kind(),
                    e.raw_os_error()
                );
                // Never clean up after an uncertain move outcome.
                if fs::symlink_metadata(&self.path).is_err() {
                    return Err(AppError::ExportUnknown);
                }
                if e.kind() == std::io::ErrorKind::AlreadyExists
                    || fs::symlink_metadata(target).is_ok()
                {
                    Err(AppError::ExportCollision)
                } else {
                    Err(AppError::ExportWrite)
                }
            }
        }
    }
}

// Fixed export layout only. No arbitrary paths, user filenames or platform separators.
fn validate_file(relative: &str) -> Result<(), AppError> {
    let parts: Vec<_> = relative.split('/').collect();
    let parts = if parts.len() > 1 && Uuid::parse_str(parts[0]).is_ok() {
        if !canonical_uuid(parts[0]) {
            return Err(AppError::ExportWrite);
        }
        &parts[1..]
    } else {
        &parts[..]
    };
    let valid = match parts {
        [name] => matches!(*name, "product.csv" | "attributes.csv" | "images.csv"),
        ["images", name] => name.split_once('-').is_some_and(|(number, rest)| {
            number.len() >= 4
                && number.bytes().all(|b| b.is_ascii_digit())
                && rest.rsplit_once('.').is_some_and(|(id, ext)| {
                    canonical_uuid(id) && matches!(ext, "jpg" | "png" | "webp")
                })
        }),
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(AppError::ExportWrite)
    }
}

fn canonical_uuid(value: &str) -> bool {
    Uuid::parse_str(value).is_ok_and(|id| id.to_string() == value)
}

pub(crate) fn write_error(stage: &str, error: &std::io::Error) -> AppError {
    // Deliberately omit paths and OS error text from diagnostics.
    eprintln!(
        "Export stage={stage} kind={:?} os={:?}",
        error.kind(),
        error.raw_os_error()
    );
    AppError::ExportWrite
}
