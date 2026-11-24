//! Scanning local folders for images.

use std::path::Path;

use crate::model::Photo;

const EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "heic", "heif", "webp", "bmp", "tif", "tiff"];

pub fn is_image(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

/// FNV-1a: a stable id for a path that doesn't change between Rust versions.
pub fn stable_id(s: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

pub fn scan_folder(source_id: &str, root: &Path, recursive: bool) -> Vec<Photo> {
    let walker = walkdir::WalkDir::new(root)
        .max_depth(if recursive { usize::MAX } else { 1 })
        .follow_links(true);
    let mut photos: Vec<Photo> = walker
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file() && is_image(e.path()))
        .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
        .filter_map(|e| {
            let path = e.path();
            let size = imagesize::size(path).ok()?;
            let path_str = path.to_string_lossy().into_owned();
            Some(Photo {
                id: stable_id(&path_str),
                source_id: source_id.to_string(),
                filename: e.file_name().to_string_lossy().into_owned(),
                path: path_str,
                width: size.width as u32,
                height: size.height as u32,
                created: None,
            })
        })
        .collect();
    photos.sort_by(|a, b| a.path.cmp(&b.path));
    photos
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_id_is_stable() {
        assert_eq!(stable_id("/a/b.jpg"), stable_id("/a/b.jpg"));
        assert_ne!(stable_id("/a/b.jpg"), stable_id("/a/c.jpg"));
    }

    #[test]
    fn recognizes_images() {
        assert!(is_image(Path::new("x/IMG_1.JPG")));
        assert!(is_image(Path::new("x/a.heic")));
        assert!(!is_image(Path::new("x/a.mov")));
        assert!(!is_image(Path::new("x/noext")));
    }
}
