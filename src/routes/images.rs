use actix_multipart::Multipart;
use actix_web::{HttpResponse, web};
use chrono::Utc;
use futures_util::StreamExt;
use serde::Serialize;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use crate::domain::ImageJob;
use crate::error::AppError;
use crate::storage::{build_original_path, ensure_storage_directories, path_to_string};

pub fn configure(config: &mut web::ServiceConfig) {
    config
        .route("/images", web::post().to(upload_image))
        .route("/images/{id}", web::get().to(get_image));
}

async fn upload_image(
    state: web::Data<crate::app_state::AppState>,
    mut payload: Multipart,
) -> Result<HttpResponse, AppError> {
    ensure_storage_directories(&state.config.storage_root).await?;

    let mut saved_upload: Option<SavedUpload> = None;

    while let Some(item_result) = payload.next().await {
        let mut field = match item_result {
            Ok(value) => value,
            Err(error) => {
                return Err(AppError::BadRequest(format!(
                    "failed to read multipart field: {error}"
                )));
            }
        };

        if saved_upload.is_some() {
            return Err(AppError::BadRequest(
                "only one image upload is supported".to_string(),
            ));
        }

        let original_filename = match field.content_disposition() {
            Some(content_disposition) => match content_disposition.get_filename() {
                Some(filename) => filename.to_string(),
                None => {
                    continue;
                }
            },
            None => {
                continue;
            }
        };

        let now = Utc::now();
        let image_id = Uuid::new_v4();
        let original_path =
            build_original_path(&state.config.storage_root, image_id, &original_filename);

        let mut file = match tokio::fs::File::create(&original_path).await {
            Ok(created_file) => created_file,
            Err(error) => {
                return Err(AppError::Storage(format!(
                    "failed to create uploaded file: {error}"
                )));
            }
        };

        let mut bytes_written: usize = 0;

        while let Some(chunk_result) = field.next().await {
            let chunk = match chunk_result {
                Ok(value) => value,
                Err(error) => {
                    remove_file_if_exists(&original_path).await;

                    return Err(AppError::BadRequest(format!(
                        "failed to read upload chunk: {error}"
                    )));
                }
            };

            bytes_written = bytes_written + chunk.len();

            if bytes_written > state.config.max_upload_bytes {
                remove_file_if_exists(&original_path).await;

                return Err(AppError::BadRequest(format!(
                    "upload exceeds maximum size of {} bytes",
                    state.config.max_upload_bytes
                )));
            }

            match file.write_all(&chunk).await {
                Ok(_) => {}
                Err(error) => {
                    remove_file_if_exists(&original_path).await;

                    return Err(AppError::Storage(format!(
                        "failed to write uploaded file: {error}"
                    )));
                }
            }
        }

        let stored_path = path_to_string(&original_path)?;

        let job = ImageJob {
            id: image_id,
            original_filename: original_filename.clone(),
            stored_path,
            thumbnail_path: None,
            status: crate::domain::ImageJobStatus::Pending,
            attempts: 0,
            max_attempts: state.config.max_attempts,
            last_error: None,
            next_retry_at: None,
            created_at: now,
            updated_at: now,
            indexed_at: None,
        };

        state.image_jobs.insert(&job).await?;

        saved_upload = Some(SavedUpload { job });
    }

    match saved_upload {
        Some(upload) => Ok(HttpResponse::Created().json(ImageJobResponse::from(upload.job))),
        None => Err(AppError::BadRequest(
            "multipart request did not contain an uploaded file".to_string(),
        )),
    }
}

async fn get_image(
    state: web::Data<crate::app_state::AppState>,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let id = path.into_inner();

    let found = state.image_jobs.find_by_id(id).await?;

    match found {
        Some(job) => Ok(HttpResponse::Ok().json(ImageJobResponse::from(job))),
        None => Err(AppError::NotFound(format!("image job {id} was not found"))),
    }
}

async fn remove_file_if_exists(path: &std::path::Path) {
    match tokio::fs::remove_file(path).await {
        Ok(_) => {}
        Err(_) => {}
    }
}

struct SavedUpload {
    job: ImageJob,
}

#[derive(Debug, Serialize)]
struct ImageJobResponse {
    id: Uuid,
    original_filename: String,
    stored_path: String,
    thumbnail_path: Option<String>,
    status: crate::domain::ImageJobStatus,
    attempts: u32,
    max_attempts: u32,
    last_error: Option<String>,
    next_retry_at: Option<chrono::DateTime<Utc>>,
    created_at: chrono::DateTime<Utc>,
    updated_at: chrono::DateTime<Utc>,
    indexed_at: Option<chrono::DateTime<Utc>>,
}

impl From<ImageJob> for ImageJobResponse {
    fn from(job: ImageJob) -> Self {
        Self {
            id: job.id,
            original_filename: job.original_filename,
            stored_path: job.stored_path,
            thumbnail_path: job.thumbnail_path,
            status: job.status,
            attempts: job.attempts,
            max_attempts: job.max_attempts,
            last_error: job.last_error,
            next_retry_at: job.next_retry_at,
            created_at: job.created_at,
            updated_at: job.updated_at,
            indexed_at: job.indexed_at,
        }
    }
}
