use anyhow::Result;
use base64::{engine::general_purpose::STANDARD, Engine};
use image::imageops::FilterType;
use image::{GenericImageView, ImageReader};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::fs;
use std::path::Path;

/// Maximum dimensions for cover thumbnails
pub const THUMBNAIL_WIDTH: u32 = 200;
pub const THUMBNAIL_HEIGHT: u32 = 300;

/// Thumbnail metadata for cache invalidation
#[derive(Debug, Serialize, Deserialize)]
struct ThumbnailMetadata {
    /// Modification time of original cover.jpg
    cover_mtime: u64,
}

/// Get modification time of file in seconds since Unix epoch
fn get_file_mtime(path: &Path) -> Result<u64> {
    let metadata = fs::metadata(path)?;
    Ok(metadata
        .modified()?
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs())
}

/// Read thumbnail metadata from disk
fn read_thumbnail_metadata(thumb_dir: &Path) -> Option<ThumbnailMetadata> {
    let metadata_path = thumb_dir.join("thumbnail.json");
    if !metadata_path.exists() {
        return None;
    }

    let content = fs::read_to_string(&metadata_path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Write thumbnail metadata to disk
fn write_thumbnail_metadata(thumb_dir: &Path, metadata: &ThumbnailMetadata) -> Result<()> {
    let metadata_path = thumb_dir.join("thumbnail.json");
    let content = serde_json::to_string_pretty(metadata)?;
    fs::write(&metadata_path, content)?;
    Ok(())
}

/// Cover image data for sending to frontend
#[derive(Serialize, Type)]
pub struct CoverImageData {
    /// Base64-encoded data URL (e.g., "data:image/png;base64,...")
    pub data_url: String,
    /// Image width in pixels
    pub width: u32,
    /// Image height in pixels
    pub height: u32,
}

/// Convert raw RGBA pixels to a base64 data URL
///
/// Takes raw pixel data (RGBA format) and converts it to a PNG image encoded as a base64 data URL.
/// This can be directly used in HTML img src attributes or React components.
pub fn pixels_to_data_url(pixels: &[u8], width: u32, height: u32) -> Result<String> {
    use image::{ImageBuffer, RgbaImage};
    use std::io::Cursor;

    // Create image from raw pixels
    let img: RgbaImage = ImageBuffer::from_raw(width, height, pixels.to_vec())
        .ok_or_else(|| anyhow::anyhow!("Failed to create image from pixels"))?;

    // Encode to PNG in memory
    let mut png_data = Cursor::new(Vec::new());
    img.write_to(&mut png_data, image::ImageFormat::Png)?;

    // Encode to base64
    let base64_str = STANDARD.encode(png_data.into_inner());

    // Create data URL
    Ok(format!("data:image/png;base64,{}", base64_str))
}

/// Load cover image data as a base64 data URL for frontend display
///
/// This function loads a cover image, resizes it to thumbnail dimensions,
/// and returns it as a base64-encoded data URL ready for use in the frontend.
pub fn load_cover_as_data_url(path: &Path) -> Result<CoverImageData> {
    let (pixels, width, height) = load_cover_data(path)?;
    let data_url = pixels_to_data_url(&pixels, width, height)?;

    Ok(CoverImageData {
        data_url,
        width,
        height,
    })
}

/// Load cover image data (raw pixels) that can be sent across threads
///
/// This function loads an image from the given path and resizes it to fit within
/// THUMBNAIL_WIDTH x THUMBNAIL_HEIGHT while maintaining aspect ratio.
/// The resulting image is converted to RGBA format.
///
/// Returns (pixels, width, height) tuple.
pub fn load_cover_data(path: &Path) -> Result<(Vec<u8>, u32, u32)> {
    let cover_mtime = get_file_mtime(path)?;

    // 1. Determine if we can use a cached thumbnail
    let is_cover_jpg = path
        .file_name()
        .and_then(|f| f.to_str())
        .is_some_and(|s| s.to_lowercase() == "cover.jpg");

    let thumbnail_path = if is_cover_jpg {
        // Cover is in root, but thumbnail is cached in irie/ subfolder
        path.parent().map(|parent| parent.join("irie"))
    } else {
        None
    };

    // 2. Check if thumbnail is stale by comparing modification times
    let should_regenerate = if let Some(thumb_dir) = &thumbnail_path {
        let thumbnail_path = thumb_dir.join("thumbnail.jpg");

        if thumbnail_path.exists() {
            match read_thumbnail_metadata(thumb_dir) {
                Some(metadata) => metadata.cover_mtime != cover_mtime,
                None => true, // No metadata, regenerate
            }
        } else {
            true // No thumbnail, need to generate
        }
    } else {
        true // Not cover.jpg, always load directly
    };

    // 3. Try to load from cache if valid
    if !should_regenerate {
        if let Some(thumb_dir) = &thumbnail_path {
            let thumbnail_path = thumb_dir.join("thumbnail.jpg");
            if let Ok(img) = ImageReader::open(&thumbnail_path)?.decode() {
                let rgba = img.to_rgba8();
                let (width, height) = rgba.dimensions();
                let pixels = rgba.into_raw();
                return Ok((pixels, width, height));
            }
        }
    }

    // 4. Load and resize original
    let img = ImageReader::open(path)?.decode()?;
    let (width, height) = img.dimensions();

    let thumbnail = match (width > THUMBNAIL_WIDTH) || (height > THUMBNAIL_HEIGHT) {
        true => img.resize(THUMBNAIL_WIDTH, THUMBNAIL_HEIGHT, FilterType::Lanczos3),
        false => img,
    };

    // 5. Save to cache if applicable
    if let Some(thumb_dir) = &thumbnail_path {
        // Ensure irie/ directory exists before saving
        let thumbnail_path = thumb_dir.join("thumbnail.jpg");
        let _ = std::fs::create_dir_all(thumb_dir);

        // Save thumbnail
        let _ = thumbnail.save(&thumbnail_path);

        // Save metadata for cache invalidation
        let metadata = ThumbnailMetadata { cover_mtime };
        let _ = write_thumbnail_metadata(thumb_dir, &metadata);
    }

    let rgba = thumbnail.to_rgba8();
    let (width, height) = rgba.dimensions();
    let pixels = rgba.into_raw();

    Ok((pixels, width, height))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb};
    use std::fs;
    use tempfile::TempDir;

    fn create_test_image(path: &Path, color: [u8; 3]) -> Result<()> {
        let img = ImageBuffer::from_fn(100, 100, |_, _| Rgb(color));
        img.save(path)?;
        Ok(())
    }

    #[test]
    fn test_load_cover_data_generates_thumbnail() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let cover_path = temp_dir.path().join("cover.jpg");
        create_test_image(&cover_path, [255, 0, 0])?; // Red

        // Thumbnail should be in irie/ subfolder
        let thumbnail_path = temp_dir.path().join("irie/thumbnail.jpg");
        assert!(!thumbnail_path.exists());

        // Load it
        let _ = load_cover_data(&cover_path)?;

        // Thumbnail should now exist in irie/
        assert!(
            thumbnail_path.exists(),
            "Thumbnail should be created automatically"
        );

        Ok(())
    }

    #[test]
    fn test_load_cover_data_uses_existing_thumbnail() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let cover_path = temp_dir.path().join("cover.jpg");

        // Thumbnail should be in irie/ subfolder
        let irie_dir = temp_dir.path().join("irie");
        fs::create_dir(&irie_dir)?;
        let thumbnail_path = irie_dir.join("thumbnail.jpg");

        // Create a RED cover and a BLUE thumbnail
        create_test_image(&cover_path, [255, 0, 0])?; // Red
        create_test_image(&thumbnail_path, [0, 0, 255])?; // Blue

        // Create thumbnail.json with matching mtime
        let cover_mtime = get_file_mtime(&cover_path)?;
        let metadata = ThumbnailMetadata { cover_mtime };
        let metadata_path = irie_dir.join("thumbnail.json");
        fs::write(&metadata_path, serde_json::to_string_pretty(&metadata)?)?;

        // Load
        let (pixels, _w, _h) = load_cover_data(&cover_path)?;

        // Check first pixel - should be Blue (from thumbnail), not Red (from cover)
        // RGB or RGBA? load_cover_data converts to RGBA
        // Allow for slight variation due to JPEG compression (e.g. 254 instead of 255)
        assert!(
            pixels[0] < 10,
            "Red channel should be low (actual: {})",
            pixels[0]
        );
        assert!(
            pixels[1] < 10,
            "Green channel should be low (actual: {})",
            pixels[1]
        );
        assert!(
            pixels[2] > 240,
            "Blue channel should be high (actual: {})",
            pixels[2]
        );
        assert_eq!(pixels[3], 255); // Alpha should still be full

        Ok(())
    }
}
