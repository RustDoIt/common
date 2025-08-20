use std::fs;
use std::path::Path;
use crate::types::{MediaFile, TextFile};

/// Converts a file path into a `MediaFile`.
///
/// # Errors
///
/// Returns an error if the file cannot be read, parsed, or converted
/// into a `MediaFile`.
pub fn file_to_media_file(file_path: &str) -> Result<MediaFile, Box<dyn std::error::Error>> {
    let filename = Path::new(file_path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown")
        .to_string();

    let data = fs::read(file_path)?;
    Ok(MediaFile::from_u8(filename, &data))
}

/// Converts a file path into a `TextFile`.
///
/// # Errors
///
/// Returns an error if the file cannot be read, parsed, or converted
/// into a `TextFile`.
pub fn file_to_text_file(file_path: &str) -> Result<TextFile, Box<dyn std::error::Error>> {
    let filename = Path::new(file_path)
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown")
        .to_string();

    let content = fs::read_to_string(file_path)?;

    Ok(TextFile::new(filename, content, vec![]))
}