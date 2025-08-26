use std::fs::{self, File as StdFile};
use std::path::Path;
use crate::types::{MediaFile, TextFile, File};
use std::io::Write;

/// Saves a [`File`] into a directory named `cached_files_{notification_from}`.
///
/// The function writes the [`TextFile`] content and appends a line
/// for each attached [`MediaFile`].
///
/// # Errors
///
/// Returns an error if the directory cannot be created or if the file cannot
/// be created or written to.
pub fn save_file(notification_from: &u8, file: &File) -> std::io::Result<()> {
    let dir_name = format!("cached_files_{notification_from}");
    let dir_path = Path::new(&dir_name);
    fs::create_dir_all(dir_path)?;

    let file_name = format!("{}_{}", file.text_file.id, file.text_file.title);
    let file_path = dir_path.join(file_name);

    let mut f = StdFile::create(file_path)?;
    writeln!(f, "{}", file.text_file.content)?;
    for media_file in &file.media_files {
        writeln!(f, "MediaFile attached: {}_{}", media_file.id, media_file.title)?;
    }
    Ok(())
}

/// Saves a list of [`File`]s into `cached_files_{notification_from}` by
/// delegating to [`save_file`] for each element.
///
/// # Errors
///
/// Returns an error if saving any single file fails.
pub fn save_files(notification_from: &u8, files: &Vec<File>) -> std::io::Result<()> {
    for file in files {
        save_file(notification_from, file)?;
    }
    Ok(())
}

/// Saves a [`TextFile`] into `cached_files_{notification_from}`.
///
/// The function writes the text content and appends a line for each
/// attached [`MediaReference`].
///
/// # Errors
///
/// Returns an error if the directory cannot be created or if the file cannot
/// be created or written to.
pub fn save_text_file(notification_from: &u8, file: &TextFile) -> std::io::Result<()> {
    let dir_name = format!("cached_files_{notification_from}");
    let dir_path = Path::new(&dir_name);
    fs::create_dir_all(dir_path)?;

    let file_name = format!("{}_{}", file.id, file.title);
    let file_path = dir_path.join(file_name);

    let mut f = StdFile::create(file_path)?;
    writeln!(f, "{}", file.content)?;
    for media_ref in &file.media_refs {
        writeln!(f, "MediaFile attached: {}_{}", media_ref.location, media_ref.id)?;
    }
    Ok(())
}

/// Saves a list of [`TextFile`]s by delegating to [`save_text_file`].
///
/// # Errors
///
/// Returns an error if saving any single file fails.
pub fn save_text_files(notification_from: &u8, files: &Vec<TextFile>) -> std::io::Result<()> {
    for file in files {
        save_text_file(notification_from, file)?;
    }
    Ok(())
}

/// Saves a single [`MediaFile`] into `cached_files_{notification_from}`.
///
/// The file is written as `{id}_{title}` and its binary content flushed.
///
/// # Errors
///
/// Returns an error if the directory cannot be created or if the file cannot
/// be created or written to.
pub fn save_media_file(notification_from: &u8, file: &MediaFile) -> std::io::Result<()> {
    let dir_name = format!("cached_files_{notification_from}");
    let dir_path = Path::new(&dir_name);
    fs::create_dir_all(dir_path)?;

    let file_name = format!("{}_{}", file.id, file.title);
    let file_path = dir_path.join(file_name);

    let mut f = StdFile::create(file_path)?;
    for chunk in &file.content {
        f.write_all(chunk)?;
    }
    Ok(())
}

/// Saves a list of [`MediaFile`]s by delegating to [`save_media_file`].
///
/// # Errors
///
/// Returns an error if saving any single file fails.
pub fn save_media_files(notification_from: &u8, files: &[MediaFile]) -> std::io::Result<()> {
    for file in files {
        save_media_file(notification_from, file)?;
    }
    Ok(())
}

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

#[cfg(test)]
mod file_conversion_tests {
    use std::fs;
    use std::io::Write;
    use tempfile::{NamedTempFile, tempdir};
    use crate::file_conversion::{file_to_media_file, file_to_text_file};

    #[test]
    /// Tests `file_to_text_file` conversion function
    fn test_text_file_conversion() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let test_content = "This is a test file\nwith multiple lines\nand some content";
        temp_file.write_all(test_content.as_bytes()).unwrap();

        let file_path = temp_file.path().to_str().unwrap();
        let result = file_to_text_file(file_path);

        assert!(result.is_ok());
        let text_file = result.unwrap();
        assert_eq!(text_file.content, test_content);
        assert!(!text_file.title.is_empty());
        assert!(text_file.media_refs.is_empty());
    }

    #[test]
    /// Tests `file_to_media_file` conversion function
    fn test_media_file_conversion() {
        // Create a temporary binary file
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test_image.png");
        let test_data = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        fs::write(&file_path, &test_data).unwrap();
        let result = file_to_media_file(file_path.to_str().unwrap());

        assert!(result.is_ok());
        let media_file = result.unwrap();
        assert_eq!(media_file.title, "test_image.png");
        assert_eq!(media_file.get_size(), test_data.len());
        let total_size: usize = media_file.content.iter().map(Vec::len).sum();
        assert_eq!(total_size, test_data.len());
    }

    #[test]
    /// Tests `file_to_media_file` conversion function with a large file
    fn test_large_file_conversion() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("large_file.bin");
        let large_data = vec![0xAB; 5000]; // 5KB file
        fs::write(&file_path, &large_data).unwrap();

        let result = file_to_media_file(file_path.to_str().unwrap());

        assert!(result.is_ok());
        let media_file = result.unwrap();
        assert_eq!(media_file.get_size(), large_data.len());

        let expected_chunks = large_data.len().div_ceil(1024); // Ceiling division
        assert_eq!(media_file.content.len(), expected_chunks);
    }

    #[test]
    /// Tests `file_to_text_file` and `file_to_media_file` conversion function with an empty file
    fn test_empty_file_conversion() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("empty.txt");
        fs::write(&file_path, b"").unwrap();

        let result = file_to_text_file(file_path.to_str().unwrap());
        assert!(result.is_ok());
        let text_file = result.unwrap();
        assert!(text_file.content.is_empty());

        let result = file_to_media_file(file_path.to_str().unwrap());
        assert!(result.is_ok());
        let media_file = result.unwrap();
        assert_eq!(media_file.get_size(), 0);
    }

    #[test]
    /// Tests `file_to_text_file` and `file_to_media_file` conversion function with a non-existent file
    fn test_nonexistent_file_error() {
        let result = file_to_text_file("/nonexistent/path/file.txt");
        assert!(result.is_err());
        let result = file_to_media_file("/nonexistent/path/file.bin");
        assert!(result.is_err());
    }

    #[test]
    /// Tests file name extraction in conversion functions
    fn test_file_name_extraction() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test_document.txt");
        fs::write(&file_path, "content").unwrap();

        let result = file_to_text_file(file_path.to_str().unwrap());
        assert!(result.is_ok());
        let text_file = result.unwrap();
        assert_eq!(text_file.title, "test_document");

        let result = file_to_media_file(file_path.to_str().unwrap());
        assert!(result.is_ok());
        let media_file = result.unwrap();
        assert_eq!(media_file.title, "test_document.txt");
    }
}