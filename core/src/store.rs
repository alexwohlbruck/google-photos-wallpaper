//! On-disk state: settings.json and library.json in the app data directory.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::model::{Photo, PhotoScore, Settings, Source};
use crate::PinwheelError;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Library {
    pub sources: Vec<Source>,
    pub photos: Vec<Photo>,
    /// Photo ids the user never wants to see.
    pub excluded: Vec<String>,
    /// Recently shown photo ids, oldest first. `history_pos` indexes the current one.
    pub history: Vec<String>,
    pub history_pos: usize,
    pub current: Option<String>,
    /// Wallpaper ratings by photo id, from the frontend's on-device models.
    pub scores: std::collections::HashMap<String, PhotoScore>,
    /// The user's own names and ratings. Kept apart so re-rating never overwrites them.
    pub overrides: std::collections::HashMap<String, UserOverride>,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UserOverride {
    pub title: Option<String>,
    pub score: Option<f64>,
}

impl Library {
    /// The rating to use for a photo: the model's, with the user's name/rating on top.
    pub fn effective_score(&self, id: &str) -> Option<std::borrow::Cow<'_, PhotoScore>> {
        use std::borrow::Cow;
        let auto = self.scores.get(id);
        let Some(o) = self.overrides.get(id).filter(|o| o.title.is_some() || o.score.is_some()) else {
            return auto.map(Cow::Borrowed);
        };
        let mut sc = auto.cloned().unwrap_or(PhotoScore {
            photo_id: id.to_string(),
            value: 0.5,
            label: String::new(),
            version: 0,
            tags: vec![],
            title: None,
        });
        if let Some(v) = o.score {
            sc.value = v;
        }
        if let Some(t) = &o.title {
            sc.title = Some(t.clone());
        }
        Some(Cow::Owned(sc))
    }
}

pub struct Paths {
    pub root: PathBuf,
}

impl Paths {
    pub fn new(root: PathBuf) -> Result<Self, PinwheelError> {
        fs::create_dir_all(&root)?;
        let p = Self { root };
        fs::create_dir_all(p.google_cache())?;
        Ok(p)
    }

    /// `PINWHEEL_DATA_DIR` overrides the OS default (handy for testing with a scratch library).
    pub fn default_root() -> Result<PathBuf, PinwheelError> {
        if let Some(dir) = std::env::var_os("PINWHEEL_DATA_DIR").filter(|d| !d.is_empty()) {
            return Ok(PathBuf::from(dir));
        }
        let root = directories::ProjectDirs::from("com", "alexwohlbruck", "Pinwheel")
            .map(|d| d.data_dir().to_path_buf())
            .ok_or_else(|| PinwheelError::Io { message: "no home directory".into() })?;
        if let Some(old) = directories::ProjectDirs::from("com", "alexwohlbruck", "GooglePhotosWallpaper") {
            if let Err(e) = migrate(old.data_dir(), &root) {
                eprintln!("pinwheel: couldn't move data from {}: {e}", old.data_dir().display());
            }
        }
        Ok(root)
    }

    pub fn settings(&self) -> PathBuf { self.root.join("settings.json") }
    pub fn library(&self) -> PathBuf { self.root.join("library.json") }
    pub fn token(&self) -> PathBuf { self.root.join("google_token.json") }
    pub fn client_config(&self) -> PathBuf { self.root.join("google_client.json") }
    pub fn google_cache(&self) -> PathBuf { self.root.join("cache").join("google") }
    pub fn shared_cache(&self, source_id: &str) -> PathBuf {
        let safe: String = source_id.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect();
        self.root.join("cache").join("shared").join(safe)
    }
}

/// The app used to be called Google Photos Wallpaper. Move its data folder to the new
/// location once, and fix the absolute photo paths stored in library.json.
fn migrate(old: &Path, new: &Path) -> std::io::Result<()> {
    if new.exists() || !old.is_dir() {
        return Ok(());
    }
    if let Some(parent) = new.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::rename(old, new)?;
    let lib = new.join("library.json");
    if let Ok(text) = fs::read_to_string(&lib) {
        // Paths are JSON strings, so escape the prefixes the same way serde did.
        let enc = |p: &Path| serde_json::to_string(&p.to_string_lossy()).unwrap_or_default();
        let (from, to) = (enc(old), enc(new));
        let (from, to) = (from.trim_matches('"'), to.trim_matches('"'));
        fs::write(&lib, text.replace(from, to))?;
    }
    Ok(())
}

pub fn load<T: DeserializeOwned + Default>(path: &Path) -> T {
    fs::read(path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

/// Write atomically (temp file + rename) so a crash never leaves half a file.
pub fn save<T: Serialize>(path: &Path, value: &T) -> Result<(), PinwheelError> {
    let tmp = path.with_extension("json.tmp");
    {
        let mut f = open_private(&tmp)?;
        f.write_all(&serde_json::to_vec_pretty(value)?)?;
        f.sync_all()?;
    }
    fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_old_data_dir() {
        let base = std::env::temp_dir().join(format!("pinwheel-migrate-{}", fastrand::u64(..)));
        let (old, new) = (base.join("Old App"), base.join("nested").join("Pinwheel"));
        fs::create_dir_all(old.join("cache")).unwrap();
        let photo = old.join("cache").join("a.jpg");
        fs::write(&photo, b"x").unwrap();
        let lib = format!(r#"{{"photos":[{{"path":{}}}]}}"#, serde_json::to_string(&photo.to_string_lossy()).unwrap());
        fs::write(old.join("library.json"), lib).unwrap();

        migrate(&old, &new).unwrap();
        assert!(!old.exists());
        let text = fs::read_to_string(new.join("library.json")).unwrap();
        let v: serde_json::Value = serde_json::from_str(&text).unwrap();
        let moved = PathBuf::from(v["photos"][0]["path"].as_str().unwrap());
        assert_eq!(moved, new.join("cache").join("a.jpg"));
        assert!(moved.exists());

        // Second run is a no-op.
        migrate(&old, &new).unwrap();
    }
}

#[cfg(unix)]
fn open_private(path: &Path) -> std::io::Result<fs::File> {
    use std::os::unix::fs::OpenOptionsExt;
    fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(path)
}

#[cfg(not(unix))]
fn open_private(path: &Path) -> std::io::Result<fs::File> {
    fs::File::create(path)
}

pub fn load_settings(paths: &Paths) -> Settings { load(&paths.settings()) }
pub fn load_library(paths: &Paths) -> Library { load(&paths.library()) }
