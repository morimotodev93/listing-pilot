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
}

impl Workspace {
    pub fn new(parent: &Path) -> Result<Self, AppError> {
        let path = parent.join(format!(".ListingPilot-export-{}", Uuid::new_v4()));
        fs::create_dir(&path).map_err(|_| AppError::ExportWrite)?;
        Ok(Self {
            path,
            files: vec![],
        })
    }

    pub fn images(&self) -> Result<(), AppError> {
        directory(&self.path, false)?;
        fs::create_dir(self.path.join("images")).map_err(|_| AppError::ExportWrite)
    }

    pub fn file(&mut self, relative: &str) -> Result<File, AppError> {
        let path = self.path.join(relative);
        directory(path.parent().ok_or(AppError::ExportWrite)?, false)?;
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|_| AppError::ExportWrite)?;
        self.files.push(path);
        Ok(file)
    }

    // No recursive deletion: only files successfully created by this operation.
    // Unknown entries, links and directories are retained for manual inspection.
    pub fn cleanup(&self) -> Result<(), AppError> {
        directory(&self.path, false)?;
        for file in &self.files {
            safe_regular_file(file)?;
            fs::remove_file(file).map_err(|_| AppError::ExportWrite)?;
        }
        let images = self.path.join("images");
        match fs::symlink_metadata(&images) {
            Ok(_) => {
                directory(&images, false)?;
                fs::remove_dir(images).map_err(|_| AppError::ExportWrite)?;
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(_) => return Err(AppError::ExportWrite),
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
