use std::path::{Path, PathBuf};

use async_trait::async_trait;
use image::imageops::FilterType;

use crate::processing::{ImageProcessor, ProcessingError};

#[derive(Debug, Clone)]
pub struct ThumbnailProcessor {
    width: u32,
    height: u32,
}

impl ThumbnailProcessor {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub fn default_64x64() -> Self {
        Self {
            width: 64,
            height: 64,
        }
    }
}

#[async_trait]
impl ImageProcessor for ThumbnailProcessor {
    async fn process(&self, input_path: &Path, output_path: &Path) -> Result<(), ProcessingError> {
        let input = input_path.to_path_buf();
        let output = output_path.to_path_buf();
        let width = self.width;
        let height = self.height;

        let join_result =
            tokio::task::spawn_blocking(move || generate_thumbnail(input, output, width, height))
                .await;

        match join_result {
            Ok(processing_result) => processing_result,
            Err(error) => Err(ProcessingError::TaskJoin(error.to_string())),
        }
    }
}

fn generate_thumbnail(
    input_path: PathBuf,
    output_path: PathBuf,
    width: u32,
    height: u32,
) -> Result<(), ProcessingError> {
    let image = match image::open(&input_path) {
        Ok(value) => value,
        Err(error) => return Err(ProcessingError::ReadImage(error.to_string())),
    };

    match output_path.parent() {
        Some(parent) => match std::fs::create_dir_all(parent) {
            Ok(_) => {}
            Err(error) => return Err(ProcessingError::WriteImage(error.to_string())),
        },
        None => {}
    }

    let thumbnail = image.resize(width, height, FilterType::Triangle);

    match thumbnail.save(&output_path) {
        Ok(_) => Ok(()),
        Err(error) => Err(ProcessingError::WriteImage(error.to_string())),
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use image::{ImageBuffer, Rgba};
    use uuid::Uuid;

    use crate::processing::{ImageProcessor, ThumbnailProcessor};

    #[tokio::test]
    async fn thumbnail_processor_creates_resized_image() -> Result<(), Box<dyn std::error::Error>> {
        let test_dir = test_directory()?;
        let input_path = test_dir.join("input.png");
        let output_path = test_dir.join("thumbnail.png");

        create_test_image(&input_path)?;

        let processor = ThumbnailProcessor::default_64x64();

        processor.process(&input_path, &output_path).await?;

        let thumbnail = image::open(&output_path)?;

        assert_eq!(thumbnail.width(), 64);
        assert_eq!(thumbnail.height(), 64);

        remove_test_directory(&test_dir).await;

        Ok(())
    }

    #[tokio::test]
    async fn thumbnail_processor_returns_error_for_missing_input()
    -> Result<(), Box<dyn std::error::Error>> {
        let test_dir = test_directory()?;
        let input_path = test_dir.join("missing.png");
        let output_path = test_dir.join("thumbnail.png");

        let processor = ThumbnailProcessor::default_64x64();

        let result = processor.process(&input_path, &output_path).await;

        assert_eq!(result.is_err(), true);

        remove_test_directory(&test_dir).await;

        Ok(())
    }

    fn test_directory() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let dir = std::env::temp_dir().join(format!("image-indexer-demo-test-{}", Uuid::new_v4()));

        std::fs::create_dir_all(&dir)?;

        Ok(dir)
    }

    fn create_test_image(path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let image: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::from_pixel(128, 128, Rgba([255, 0, 0, 255]));

        image.save(path)?;

        Ok(())
    }

    async fn remove_test_directory(path: &PathBuf) {
        match tokio::fs::remove_dir_all(path).await {
            Ok(_) => {}
            Err(_) => {}
        }
    }
}
