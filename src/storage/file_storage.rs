use std::path::{Path, PathBuf};

use tokio::fs;

use crate::error::AppError;

pub async fn ensure_storage_directories(storage_root: &str) -> Result<(), AppError> {
    let originals_dir = Path::new(storage_root).join("originals");
    let thumbnails_dir = Path::new(storage_root).join("thumbnails");

    match fs::create_dir_all(&originals_dir).await {
        Ok(_) => {}
        Err(error) => {
            return Err(AppError::Storage(format!(
                "failed to create originals directory: {error}"
            )));
        }
    }

    match fs::create_dir_all(&thumbnails_dir).await {
        Ok(_) => {}
        Err(error) => {
            return Err(AppError::Storage(format!(
                "failed to create thumbnails directory: {error}"
            )));
        }
    }

    Ok(())
}

pub fn build_original_path(storage_root: &str, id: uuid::Uuid, original_filename: &str) -> PathBuf {
    let extension = extract_extension(original_filename);

    match extension {
        Some(ext) => Path::new(storage_root)
            .join("originals")
            .join(format!("{id}.{ext}")),
        None => Path::new(storage_root)
            .join("originals")
            .join(id.to_string()),
    }
}

pub fn build_thumbnail_path(storage_root: &str, id: uuid::Uuid) -> PathBuf {
    Path::new(storage_root)
        .join("thumbnails")
        .join(format!("{id}.png"))
}

pub fn path_to_string(path: &Path) -> Result<String, AppError> {
    match path.to_str() {
        Some(value) => Ok(value.to_string()),
        None => Err(AppError::Storage("path contains invalid UTF-8".to_string())),
    }
}

fn extract_extension(filename: &str) -> Option<String> {
    let path = Path::new(filename);

    match path.extension() {
        Some(extension) => match extension.to_str() {
            Some(value) => {
                let trimmed = value.trim();

                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_lowercase())
                }
            }
            None => None,
        },
        None => None,
    }
}
