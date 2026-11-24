//! Pinwheel core.
//!
//! Everything that isn't UI lives here: photo sources, the local library and
//! cache, the rotation schedule, Google sign-in and the Photos Picker, and
//! setting the wallpaper. Native frontends talk to [`Engine`] through UniFFI
//! (Swift on macOS, C# on Windows) or directly as a Rust crate (GTK on Linux).

mod google;
mod model;
mod scan;
mod shared;
mod store;
mod wallpaper;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub use model::*;
use store::{Library, Paths};

uniffi::setup_scaffolding!();

pub const GOOGLE_SOURCE_ID: &str = "google";
const SHARED_PREFIX: &str = "shared:";
/// After a failed background sync, wait this long before trying again.
const SYNC_RETRY: Duration = Duration::from_secs(30 * 60);
const HISTORY_LIMIT: usize = 200;

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum PinwheelError {
    #[error("{message}")]
    Io { message: String },
    #[error("{message}")]
    Network { message: String },
    #[error("sign-in failed: {message}")]
    Auth { message: String },
    #[error("couldn't set the wallpaper: {message}")]
    Wallpaper { message: String },
    #[error("Google sign-in isn't configured in this build (no OAuth client id)")]
    GoogleNotConfigured,
    #[error("not signed in to Google")]
    NotSignedIn,
    #[error("cancelled")]
    Cancelled,
    #[error("no photos match your sources and filters")]
    NoPhotos,
    #[error("not found")]
    NotFound,
    #[error("another task is already running")]
    Busy,
    #[error("{message}")]
    SharedAlbum { message: String },
}

impl From<std::io::Error> for PinwheelError {
    fn from(e: std::io::Error) -> Self { PinwheelError::Io { message: e.to_string() } }
}
impl From<serde_json::Error> for PinwheelError {
    fn from(e: serde_json::Error) -> Self { PinwheelError::Io { message: e.to_string() } }
}

/// Implemented by each frontend. Every method may be called from a background thread.
#[uniffi::export(with_foreign)]
pub trait EngineDelegate: Send + Sync {
    /// Set the desktop picture. Return false to fall back to the core's built-in setter.
    fn set_wallpaper(&self, path: String) -> bool;
    /// Open a URL in the user's browser (Google sign-in and the photo picker).
    fn open_url(&self, url: String);
    /// Something changed; call `Engine::status()` / `sources()` and refresh the UI.
    fn state_changed(&self);
}

struct State {
    settings: Settings,
    library: Library,
    next_change: Option<SystemTime>,
}

struct Inner {
    paths: Paths,
    state: Mutex<State>,
    timer: Condvar,
    google: Mutex<google::Google>,
    delegate: Arc<dyn EngineDelegate>,
    activity: Mutex<Option<String>>,
    busy: AtomicBool,
    cancel: AtomicBool,
    started: AtomicBool,
    /// One shared-album sync at a time.
    sync_lock: Mutex<()>,
    /// Last background sync attempt per source, for retry backoff.
    sync_attempts: Mutex<std::collections::HashMap<String, SystemTime>>,
    /// Set when the computer wakes, so the sync loop checks right away instead of finishing its nap.
    sync_wake: (Mutex<bool>, Condvar),
}

#[derive(uniffi::Object)]
pub struct Engine {
    inner: Arc<Inner>,
}

#[derive(Clone, Copy)]
enum Step {
    Next,
    Previous,
    Shuffle,
}

#[uniffi::export]
impl Engine {
    /// `data_dir`: where settings and the photo cache live. None = the OS default
    /// (`~/Library/Application Support/...`, `%APPDATA%\...`, `~/.local/share/...`).
    #[uniffi::constructor]
    pub fn new(data_dir: Option<String>, delegate: Arc<dyn EngineDelegate>) -> Result<Arc<Self>, PinwheelError> {
        let root = match data_dir {
            Some(d) => PathBuf::from(d),
            None => Paths::default_root()?,
        };
        let paths = Paths::new(root)?;
        let settings = store::load_settings(&paths);
        let library = store::load_library(&paths);
        let google = google::Google::new(google::ClientConfig::resolve(&paths.client_config()), paths.token());
        Ok(Arc::new(Self {
            inner: Arc::new(Inner {
                paths,
                state: Mutex::new(State { settings, library, next_change: None }),
                timer: Condvar::new(),
                google: Mutex::new(google),
                delegate,
                activity: Mutex::new(None),
                busy: AtomicBool::new(false),
                cancel: AtomicBool::new(false),
                started: AtomicBool::new(false),
                sync_lock: Mutex::new(()),
                sync_attempts: Mutex::new(Default::default()),
                sync_wake: (Mutex::new(false), Condvar::new()),
            }),
        }))
    }

    /// Start the rotation timer. Call once after the UI is up.
    pub fn start(&self) {
        if self.inner.started.swap(true, Ordering::SeqCst) {
            return;
        }
        let change_now = self.inner.lock().settings.change_on_launch;
        self.inner.reset_timer();
        let inner = self.inner.clone();
        std::thread::Builder::new()
            .name("pinwheel-scheduler".into())
            .spawn(move || inner.scheduler_loop())
            .expect("spawn scheduler");
        let inner = self.inner.clone();
        std::thread::Builder::new()
            .name("pinwheel-sync".into())
            .spawn(move || inner.sync_loop())
            .expect("spawn sync");
        if change_now {
            let inner = self.inner.clone();
            std::thread::spawn(move || {
                let _ = inner.step(Step::Next);
            });
        }
    }

    pub fn data_dir(&self) -> String {
        self.inner.paths.root.to_string_lossy().into_owned()
    }

    // ---- Settings ----

    pub fn settings(&self) -> Settings {
        self.inner.lock().settings.clone()
    }

    pub fn update_settings(&self, settings: Settings) -> Result<(), PinwheelError> {
        {
            let mut s = self.inner.lock();
            s.settings = settings;
            store::save(&self.inner.paths.settings(), &s.settings)?;
        }
        self.inner.reset_timer();
        self.inner.notify();
        Ok(())
    }

    /// Call when the computer wakes from sleep. A change that came due while asleep happens now
    /// (once, not once per missed interval), and shared albums that are due sync shortly after,
    /// once the network is back.
    pub fn wake(&self) {
        // Timer deadlines are wall-clock times, so the scheduler just needs a nudge.
        self.inner.timer.notify_all();
        let (flag, cv) = &self.inner.sync_wake;
        *flag.lock().unwrap() = true;
        cv.notify_all();
    }

    pub fn set_paused(&self, paused: bool) -> Result<(), PinwheelError> {
        let mut s = self.settings();
        s.paused = paused;
        self.update_settings(s)
    }

    pub fn status(&self) -> Status {
        let s = self.inner.lock();
        let current = s.library.current.as_ref().and_then(|id| s.library.photos.iter().find(|p| &p.id == id)).cloned();
        let google = self.inner.google.lock().unwrap();
        Status {
            current,
            next_change_unix: s.next_change.map(unix),
            eligible_count: eligible(&s).len() as u32,
            google_signed_in: google.signed_in(),
            google_configured: google.client.is_some(),
            activity: self.inner.activity.lock().unwrap().clone(),
            unrated_count: rating_queue(&s, 0, usize::MAX).len() as u32,
        }
    }

    // ---- Rotation ----

    pub fn next(&self) -> Result<Photo, PinwheelError> { self.inner.step(Step::Next) }
    pub fn previous(&self) -> Result<Photo, PinwheelError> { self.inner.step(Step::Previous) }
    /// Jump to a random photo regardless of the configured order.
    pub fn shuffle(&self) -> Result<Photo, PinwheelError> { self.inner.step(Step::Shuffle) }

    pub fn set_photo(&self, photo_id: String) -> Result<Photo, PinwheelError> {
        let photo = {
            let s = self.inner.lock();
            s.library.photos.iter().find(|p| p.id == photo_id).cloned().ok_or(PinwheelError::NotFound)?
        };
        self.inner.show(photo, true)
    }

    /// Never show this photo again. If it's the current wallpaper, move on.
    pub fn exclude_photo(&self, photo_id: String) -> Result<(), PinwheelError> {
        let was_current = {
            let mut s = self.inner.lock();
            if !s.library.excluded.contains(&photo_id) {
                s.library.excluded.push(photo_id.clone());
            }
            self.inner.save_library(&s)?;
            s.library.current.as_deref() == Some(photo_id.as_str())
        };
        self.inner.notify();
        if was_current {
            let _ = self.next();
        }
        Ok(())
    }

    pub fn include_photo(&self, photo_id: String) -> Result<(), PinwheelError> {
        {
            let mut s = self.inner.lock();
            s.library.excluded.retain(|id| id != &photo_id);
            self.inner.save_library(&s)?;
        }
        self.inner.notify();
        Ok(())
    }

    // ---- Ratings ----

    /// Photos that still need a rating at `version` (none at all, or an older one).
    /// Ones currently in rotation come first.
    pub fn photos_to_rate(&self, version: u32, limit: u32) -> Vec<Photo> {
        let s = self.inner.lock();
        rating_queue(&s, version, limit as usize).into_iter().cloned().collect()
    }

    pub fn set_scores(&self, scores: Vec<PhotoScore>) -> Result<(), PinwheelError> {
        {
            let mut s = self.inner.lock();
            for sc in scores {
                s.library.scores.insert(sc.photo_id.clone(), sc);
            }
            self.inner.save_library(&s)?;
        }
        self.inner.notify();
        Ok(())
    }

    /// Give a photo your own name, or `None` to go back to the suggested one.
    pub fn set_photo_title(&self, photo_id: String, title: Option<String>) -> Result<(), PinwheelError> {
        let title = title.map(|t| t.trim().to_string()).filter(|t| !t.is_empty());
        self.edit_override(photo_id, |o| o.title = title)
    }

    /// Rate a photo yourself (0...1), or `None` to go back to the automatic rating.
    pub fn set_photo_score(&self, photo_id: String, score: Option<f64>) -> Result<(), PinwheelError> {
        let score = score.map(|v| v.clamp(0.0, 1.0));
        self.edit_override(photo_id, |o| o.score = score)
    }

    /// How many photos (in enabled sources) carry each tag, most common first.
    pub fn tag_counts(&self) -> Vec<TagCount> {
        let s = self.inner.lock();
        let enabled: Vec<&str> = s.library.sources.iter().filter(|src| src.enabled).map(|src| src.id.as_str()).collect();
        let mut counts: std::collections::HashMap<&str, u32> = Default::default();
        for p in s.library.photos.iter().filter(|p| enabled.contains(&p.source_id.as_str())) {
            if let Some(sc) = s.library.scores.get(&p.id) {
                for t in &sc.tags {
                    *counts.entry(t.as_str()).or_default() += 1;
                }
            }
        }
        let mut v: Vec<TagCount> = counts.into_iter().map(|(tag, count)| TagCount { tag: tag.to_string(), count }).collect();
        v.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.tag.cmp(&b.tag)));
        v
    }

    /// Forget all ratings so they're recomputed.
    pub fn clear_scores(&self) -> Result<(), PinwheelError> {
        {
            let mut s = self.inner.lock();
            s.library.scores.clear();
            self.inner.save_library(&s)?;
        }
        self.inner.notify();
        Ok(())
    }

    // ---- Library ----

    pub fn sources(&self) -> Vec<Source> {
        let s = self.inner.lock();
        s.library
            .sources
            .iter()
            .map(|src| Source {
                photo_count: s.library.photos.iter().filter(|p| p.source_id == src.id).count() as u32,
                ..src.clone()
            })
            .collect()
    }

    /// All photos, optionally for one source, with an `excluded` flag the UI can show.
    pub fn photos(&self, source_id: Option<String>) -> Vec<PhotoEntry> {
        let s = self.inner.lock();
        s.library
            .photos
            .iter()
            .filter(|p| source_id.as_ref().map_or(true, |id| &p.source_id == id))
            .map(|p| PhotoEntry {
                photo: p.clone(),
                excluded: s.library.excluded.contains(&p.id),
                passes_filters: passes(&s.settings.filters, p, s.library.effective_score(&p.id).as_deref()),
                score: score_of(&s, p),
                score_label: s.library.scores.get(&p.id).map(|sc| sc.label.clone()),
                tags: s.library.scores.get(&p.id).map(|sc| sc.tags.clone()).unwrap_or_default(),
                title: s.library.effective_score(&p.id).and_then(|sc| sc.title.clone()),
                custom_title: s.library.overrides.get(&p.id).is_some_and(|o| o.title.is_some()),
                custom_score: s.library.overrides.get(&p.id).is_some_and(|o| o.score.is_some()),
            })
            .collect()
    }

    /// Add a folder and scan it. Blocking; call off the main thread.
    pub fn add_local_folder(&self, path: String, recursive: bool) -> Result<Source, PinwheelError> {
        let root = PathBuf::from(&path);
        if !root.is_dir() {
            return Err(PinwheelError::NotFound);
        }
        let id = format!("folder:{}", scan::stable_id(&path));
        if self.inner.lock().library.sources.iter().any(|s| s.id == id) {
            return self.sources().into_iter().find(|s| s.id == id).ok_or(PinwheelError::NotFound);
        }
        let _task = self.inner.begin("Scanning folder…")?;
        let photos = scan::scan_folder(&id, &root, recursive);
        let name = root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| path.clone());
        let source = Source {
            id: id.clone(),
            name,
            kind: SourceKind::LocalFolder { path, recursive },
            enabled: true,
            photo_count: photos.len() as u32,
            last_synced_unix: None,
            sync_error: None,
        };
        {
            let mut s = self.inner.lock();
            s.library.sources.push(source.clone());
            s.library.photos.extend(photos);
            self.inner.save_library(&s)?;
        }
        Ok(source)
    }

    pub fn remove_source(&self, source_id: String) -> Result<(), PinwheelError> {
        let removed: Vec<Photo> = {
            let mut s = self.inner.lock();
            s.library.sources.retain(|src| src.id != source_id);
            let (gone, keep): (Vec<_>, Vec<_>) = std::mem::take(&mut s.library.photos).into_iter().partition(|p| p.source_id == source_id);
            s.library.photos = keep;
            self.inner.save_library(&s)?;
            gone
        };
        {
            let mut s = self.inner.lock();
            for p in &removed {
                s.library.scores.remove(&p.id);
            }
            let _ = self.inner.save_library(&s);
        }
        if source_id == GOOGLE_SOURCE_ID {
            for p in removed {
                let _ = std::fs::remove_file(&p.path);
            }
        } else if source_id.starts_with(SHARED_PREFIX) {
            let _ = std::fs::remove_dir_all(self.inner.paths.shared_cache(&source_id));
        }
        self.inner.notify();
        Ok(())
    }

    /// Add a Google Photos album shared by link and download its photos.
    /// Blocking; call off the main thread. `cancel()` aborts it.
    pub fn add_shared_album(&self, url: String) -> Result<Source, PinwheelError> {
        let url = shared::normalize(&url)?;
        let id = format!("{SHARED_PREFIX}{}", scan::stable_id(&url));
        if !self.inner.lock().library.sources.iter().any(|s| s.id == id) {
            let _task = self.inner.begin("Reading shared album…")?;
            let album = shared::fetch(&url)?;
            {
                let mut s = self.inner.lock();
                s.library.sources.push(Source {
                    id: id.clone(),
                    name: album.title.clone(),
                    kind: SourceKind::SharedAlbum { url: url.clone() },
                    enabled: true,
                    photo_count: 0,
                    last_synced_unix: None,
                    sync_error: None,
                });
                self.inner.save_library(&s)?;
            }
            self.inner.notify();
            let inner = self.inner.clone();
            self.inner.sync_album(&id, Some(album), &|m| inner.set_activity(Some(m)), Some(&self.inner.cancel))?;
        }
        self.sources().into_iter().find(|s| s.id == id).ok_or(PinwheelError::NotFound)
    }

    /// Re-read a shared album now: download new photos, drop removed ones. Blocking.
    pub fn sync_source(&self, source_id: String) -> Result<(), PinwheelError> {
        let _task = self.inner.begin("Syncing shared album…")?;
        let inner = self.inner.clone();
        self.inner.sync_album(&source_id, None, &|m| inner.set_activity(Some(m)), Some(&self.inner.cancel))
    }

    pub fn set_source_enabled(&self, source_id: String, enabled: bool) -> Result<(), PinwheelError> {
        {
            let mut s = self.inner.lock();
            let src = s.library.sources.iter_mut().find(|src| src.id == source_id).ok_or(PinwheelError::NotFound)?;
            src.enabled = enabled;
            self.inner.save_library(&s)?;
        }
        self.inner.notify();
        Ok(())
    }

    /// Delete one downloaded Google photo from the cache.
    pub fn remove_google_photo(&self, photo_id: String) -> Result<(), PinwheelError> {
        let photo = {
            let mut s = self.inner.lock();
            let idx = s.library.photos.iter().position(|p| p.id == photo_id && p.source_id == GOOGLE_SOURCE_ID).ok_or(PinwheelError::NotFound)?;
            let p = s.library.photos.remove(idx);
            self.inner.save_library(&s)?;
            p
        };
        let _ = std::fs::remove_file(&photo.path);
        self.inner.notify();
        Ok(())
    }

    /// Rescan local folders and drop cached Google photos whose files are gone. Blocking.
    pub fn rescan(&self) -> Result<(), PinwheelError> {
        let _task = self.inner.begin("Rescanning folders…")?;
        let sources = self.inner.lock().library.sources.clone();
        let mut fresh: Vec<(String, Vec<Photo>)> = Vec::new();
        for src in &sources {
            if let SourceKind::LocalFolder { path, recursive } = &src.kind {
                fresh.push((src.id.clone(), scan::scan_folder(&src.id, &PathBuf::from(path), *recursive)));
            }
        }
        let mut s = self.inner.lock();
        for (id, photos) in fresh {
            s.library.photos.retain(|p| p.source_id != id);
            s.library.photos.extend(photos);
        }
        s.library.photos.retain(|p| p.source_id != GOOGLE_SOURCE_ID || std::path::Path::new(&p.path).exists());
        self.inner.save_library(&s)?;
        drop(s);

        // Shared albums: errors are recorded on the source rather than failing the rescan.
        for src in sources.iter().filter(|s| matches!(s.kind, SourceKind::SharedAlbum { .. })) {
            let inner = self.inner.clone();
            let name = src.name.clone();
            let _ = self.inner.sync_album(&src.id, None, &move |m| inner.set_activity(Some(format!("{name}: {m}"))), Some(&self.inner.cancel));
        }
        Ok(())
    }

    // ---- Google ----

    /// Use a different OAuth client (for development builds). Saved to the data dir.
    pub fn set_google_client(&self, client_id: String, client_secret: String) -> Result<(), PinwheelError> {
        let cfg = google::ClientConfig { client_id, client_secret };
        store::save(&self.inner.paths.client_config(), &cfg)?;
        self.inner.google.lock().unwrap().client = Some(cfg);
        self.inner.notify();
        Ok(())
    }

    /// Browser sign-in. Blocking; call off the main thread. `cancel()` aborts it.
    pub fn google_sign_in(&self) -> Result<(), PinwheelError> {
        let _task = self.inner.begin("Waiting for Google sign-in in your browser…")?;
        let delegate = self.inner.delegate.clone();
        let google = self.google_snapshot();
        google.sign_in(&|url| delegate.open_url(url), &self.inner.cancel)
    }

    pub fn google_sign_out(&self) {
        self.google_snapshot().sign_out();
        self.inner.notify();
    }

    /// Open the Google Photos picker, wait for the user's selection, and
    /// download it into the cache. Returns how many photos were added.
    /// Blocking; call off the main thread. `cancel()` aborts it.
    pub fn google_import(&self) -> Result<u32, PinwheelError> {
        let _task = self.inner.begin("Opening Google Photos…")?;
        let delegate = self.inner.delegate.clone();
        let google = self.google_snapshot();
        let inner = self.inner.clone();
        let photos = google.pick(
            GOOGLE_SOURCE_ID,
            &self.inner.paths.google_cache(),
            &|url| delegate.open_url(url),
            &|msg| inner.set_activity(Some(msg)),
            &self.inner.cancel,
        )?;

        let mut s = self.inner.lock();
        if !s.library.sources.iter().any(|src| src.id == GOOGLE_SOURCE_ID) {
            s.library.sources.push(Source {
                id: GOOGLE_SOURCE_ID.into(),
                name: "Google Photos".into(),
                kind: SourceKind::GooglePhotos,
                enabled: true,
                photo_count: 0,
                last_synced_unix: None,
                sync_error: None,
            });
        }
        let mut added = 0;
        for p in photos {
            match s.library.photos.iter_mut().find(|e| e.id == p.id) {
                Some(existing) => *existing = p,
                None => {
                    s.library.photos.push(p);
                    added += 1;
                }
            }
        }
        self.inner.save_library(&s)?;
        Ok(added)
    }

    /// Abort a running sign-in, import, or scan.
    pub fn cancel(&self) {
        self.inner.cancel.store(true, Ordering::SeqCst);
    }
}

impl Engine {
    fn edit_override(&self, photo_id: String, f: impl FnOnce(&mut store::UserOverride)) -> Result<(), PinwheelError> {
        {
            let mut s = self.inner.lock();
            if !s.library.photos.iter().any(|p| p.id == photo_id) {
                return Err(PinwheelError::NotFound);
            }
            let o = s.library.overrides.entry(photo_id.clone()).or_default();
            f(o);
            if *o == store::UserOverride::default() {
                s.library.overrides.remove(&photo_id);
            }
            self.inner.save_library(&s)?;
        }
        self.inner.notify();
        Ok(())
    }

    fn google_snapshot(&self) -> google::Google {
        let g = self.inner.google.lock().unwrap();
        google::Google::new(g.client.clone(), self.inner.paths.token())
    }
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct PhotoEntry {
    pub photo: Photo,
    pub excluded: bool,
    pub passes_filters: bool,
    pub score: Option<f64>,
    pub score_label: Option<String>,
    pub tags: Vec<String>,
    pub title: Option<String>,
    /// The user renamed this photo.
    pub custom_title: bool,
    /// The user rated this photo themselves.
    pub custom_score: bool,
}

/// Marks the engine busy for one long task; clears activity and notifies on drop.
struct Task<'a>(&'a Inner);
impl Drop for Task<'_> {
    fn drop(&mut self) {
        self.0.busy.store(false, Ordering::SeqCst);
        self.0.set_activity(None);
    }
}

impl Inner {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn notify(&self) {
        self.delegate.state_changed();
    }

    fn set_activity(&self, a: Option<String>) {
        *self.activity.lock().unwrap() = a;
        self.notify();
    }

    fn begin(&self, what: &str) -> Result<Task<'_>, PinwheelError> {
        if self.busy.swap(true, Ordering::SeqCst) {
            return Err(PinwheelError::Busy);
        }
        self.cancel.store(false, Ordering::SeqCst);
        self.set_activity(Some(what.to_string()));
        Ok(Task(self))
    }

    fn save_library(&self, s: &State) -> Result<(), PinwheelError> {
        store::save(&self.paths.library(), &s.library)
    }

    fn reset_timer(&self) {
        let mut s = self.lock();
        s.next_change = if s.settings.paused || s.settings.interval_minutes == 0 {
            None
        } else {
            Some(SystemTime::now() + Duration::from_secs(s.settings.interval_minutes as u64 * 60))
        };
        self.timer.notify_all();
    }

    /// Bring a shared album's photos in line with the album. `prefetched` skips the
    /// page fetch when the caller already has it.
    fn sync_album(
        &self,
        source_id: &str,
        prefetched: Option<shared::Album>,
        progress: &dyn Fn(String),
        cancel: Option<&AtomicBool>,
    ) -> Result<(), PinwheelError> {
        let _one = self.sync_lock.lock().unwrap_or_else(|e| e.into_inner());
        let url = {
            let s = self.lock();
            match s.library.sources.iter().find(|src| src.id == source_id).map(|src| &src.kind) {
                Some(SourceKind::SharedAlbum { url }) => url.clone(),
                _ => return Err(PinwheelError::NotFound),
            }
        };
        let result = (|| -> Result<(), PinwheelError> {
            let album = match prefetched {
                Some(a) => a,
                None => {
                    progress("Checking for new photos…".into());
                    shared::fetch(&url)?
                }
            };
            let have: std::collections::HashSet<String> = {
                let s = self.lock();
                s.library.photos.iter().filter(|p| p.source_id == source_id).map(|p| p.id.clone()).collect()
            };
            let wanted: Vec<&shared::Item> = album.items.iter().filter(|i| !i.video).collect();
            let new: Vec<&&shared::Item> = wanted.iter().filter(|i| !have.contains(&format!("s:{}", i.id))).collect();

            let dir = self.paths.shared_cache(source_id);
            let mut downloaded = Vec::new();
            let mut failures = 0;
            for (n, item) in new.iter().enumerate() {
                if cancel.is_some_and(|c| c.load(Ordering::SeqCst)) {
                    break;
                }
                progress(format!("Downloading {} of {}…", n + 1, new.len()));
                match shared::download(item, source_id, &dir) {
                    Ok(p) => downloaded.push(p),
                    Err(e) => {
                        failures += 1;
                        eprintln!("pinwheel: shared album download failed: {e}");
                    }
                }
                // Save progress in batches so a long first sync survives a quit.
                if downloaded.len() >= 25 {
                    self.commit_shared(source_id, &album, std::mem::take(&mut downloaded), false)?;
                }
            }
            self.commit_shared(source_id, &album, downloaded, true)?;
            if failures > 0 && failures == new.len() {
                return Err(PinwheelError::Network { message: format!("couldn't download any of {failures} new photos") });
            }
            Ok(())
        })();

        {
            let mut s = self.lock();
            if let Some(src) = s.library.sources.iter_mut().find(|src| src.id == source_id) {
                match &result {
                    Ok(()) => {
                        src.last_synced_unix = Some(unix(SystemTime::now()));
                        src.sync_error = None;
                    }
                    Err(PinwheelError::Cancelled) => {}
                    Err(e) => src.sync_error = Some(e.to_string()),
                }
            }
            let _ = self.save_library(&s);
        }
        self.notify();
        result
    }

    /// Add downloaded photos; when `prune`, also drop photos no longer in the album.
    fn commit_shared(&self, source_id: &str, album: &shared::Album, new: Vec<Photo>, prune: bool) -> Result<(), PinwheelError> {
        let mut removed = Vec::new();
        {
            let mut s = self.lock();
            if let Some(src) = s.library.sources.iter_mut().find(|src| src.id == source_id) {
                src.name = album.title.clone();
            }
            if prune {
                let keep: std::collections::HashSet<String> = album.items.iter().map(|i| format!("s:{}", i.id)).collect();
                let (gone, kept): (Vec<_>, Vec<_>) = std::mem::take(&mut s.library.photos)
                    .into_iter()
                    .partition(|p| p.source_id == source_id && !keep.contains(&p.id));
                s.library.photos = kept;
                removed = gone;
            }
            for p in new {
                if !s.library.photos.iter().any(|e| e.id == p.id) {
                    s.library.photos.push(p);
                }
            }
            self.save_library(&s)?;
        }
        for p in removed {
            let _ = std::fs::remove_file(&p.path);
        }
        self.notify();
        Ok(())
    }

    /// Background: sync shared albums every `sync_hours`, retrying failures after SYNC_RETRY.
    fn sync_loop(&self) {
        std::thread::sleep(Duration::from_secs(5)); // Let the UI settle after launch.
        loop {
            let (due, every): (Vec<String>, u64) = {
                let s = self.lock();
                let every = s.settings.sync_hours.max(1) as u64 * 3600;
                let now = unix(SystemTime::now());
                let attempts = self.sync_attempts.lock().unwrap();
                let due = s
                    .library
                    .sources
                    .iter()
                    .filter(|src| src.enabled && matches!(src.kind, SourceKind::SharedAlbum { .. }))
                    .filter(|src| src.last_synced_unix.map_or(true, |t| now - t >= every as i64))
                    .filter(|src| attempts.get(&src.id).map_or(true, |t| t.elapsed().unwrap_or_default() >= SYNC_RETRY))
                    .map(|src| src.id.clone())
                    .collect();
                (due, every)
            };
            for id in due {
                self.sync_attempts.lock().unwrap().insert(id.clone(), SystemTime::now());
                if let Err(e) = self.sync_album(&id, None, &|_| {}, None) {
                    eprintln!("pinwheel: background sync of {id} failed: {e}");
                }
            }
            // Nap until the next check, or until the computer wakes from sleep. (A plain sleep would
            // stall: its clock stops while the computer sleeps.)
            let (flag, cv) = &self.sync_wake;
            let mut woke = flag.lock().unwrap();
            if !*woke {
                woke = cv.wait_timeout(woke, Duration::from_secs(every.min(5 * 60))).unwrap().0;
            }
            if std::mem::take(&mut *woke) {
                drop(woke);
                // Give Wi-Fi a moment to reconnect.
                std::thread::sleep(Duration::from_secs(20));
            }
        }
    }

    fn scheduler_loop(&self) {
        let mut s = self.lock();
        loop {
            match s.next_change {
                None => s = self.timer.wait(s).unwrap_or_else(|e| e.into_inner()),
                Some(at) => match at.duration_since(SystemTime::now()) {
                    // Wake at least every 30s so sleep/wake and clock changes are noticed quickly.
                    Ok(left) => {
                        s = self.timer.wait_timeout(s, left.min(Duration::from_secs(30))).unwrap_or_else(|e| e.into_inner()).0
                    }
                    Err(_) => {
                        drop(s);
                        if let Err(e) = self.step(Step::Next) {
                            eprintln!("pinwheel: scheduled change failed: {e}");
                            self.reset_timer();
                        }
                        s = self.lock();
                    }
                },
            }
        }
    }

    fn step(&self, step: Step) -> Result<Photo, PinwheelError> {
        let (photo, from_history) = {
            let mut s = self.lock();
            let pool = eligible(&s);
            if pool.is_empty() {
                return Err(PinwheelError::NoPhotos);
            }
            let lib = &s.library;
            let current_idx = lib.current.as_ref().and_then(|c| pool.iter().position(|p| &p.id == c));

            // Walk the history first so previous/next retrace what was shown.
            let hist_target = match step {
                Step::Previous if lib.history_pos > 0 => Some(lib.history_pos - 1),
                Step::Next if lib.history_pos + 1 < lib.history.len() => Some(lib.history_pos + 1),
                _ => None,
            };
            let from_hist = hist_target.and_then(|i| {
                let id = &lib.history[i];
                pool.iter().find(|p| &p.id == id).map(|p| (i, (*p).clone()))
            });

            if let Some((i, p)) = from_hist {
                s.library.history_pos = i;
                (p, true)
            } else {
                let order = match step {
                    Step::Shuffle => Order::Random,
                    _ => s.settings.order,
                };
                let pick = match (order, step) {
                    (Order::Sequential, Step::Previous) => {
                        let i = current_idx.map(|i| (i + pool.len() - 1) % pool.len()).unwrap_or(pool.len() - 1);
                        pool[i].clone()
                    }
                    (Order::Sequential, _) => pool[current_idx.map(|i| (i + 1) % pool.len()).unwrap_or(0)].clone(),
                    (Order::Random, _) => {
                        let weight = |p: &Photo| {
                            if s.settings.prefer_high_scores {
                                // Unrated photos count as middling; squaring favors the best without starving the rest.
                                let v = lib.effective_score(&p.id).map_or(0.5, |sc| sc.value).max(0.05);
                                v * v
                            } else {
                                1.0
                            }
                        };
                        pick_random(&pool, &lib.history, lib.current.as_deref(), weight)
                    }
                };
                (pick, false)
            }
        };
        self.show(photo, !from_history)
    }

    fn show(&self, photo: Photo, push_history: bool) -> Result<Photo, PinwheelError> {
        // Don't hold the lock while calling out: the delegate may hop to the UI thread.
        if !self.delegate.set_wallpaper(photo.path.clone()) {
            wallpaper::set(&photo.path)?;
        }
        {
            let mut s = self.lock();
            let lib = &mut s.library;
            if push_history {
                let keep = if lib.history.is_empty() { 0 } else { lib.history_pos + 1 };
                lib.history.truncate(keep);
                lib.history.push(photo.id.clone());
                if lib.history.len() > HISTORY_LIMIT {
                    let drop_n = lib.history.len() - HISTORY_LIMIT;
                    lib.history.drain(..drop_n);
                }
                lib.history_pos = lib.history.len() - 1;
            }
            lib.current = Some(photo.id.clone());
            self.save_library(&s)?;
        }
        self.reset_timer();
        self.notify();
        Ok(photo)
    }
}

fn eligible(s: &State) -> Vec<&Photo> {
    let enabled: Vec<&str> = s.library.sources.iter().filter(|src| src.enabled).map(|src| src.id.as_str()).collect();
    let mut v: Vec<&Photo> = s
        .library
        .photos
        .iter()
        .filter(|p| enabled.contains(&p.source_id.as_str()))
        .filter(|p| !s.library.excluded.contains(&p.id))
        .filter(|p| passes(&s.settings.filters, p, s.library.effective_score(&p.id).as_deref()))
        .collect();
    // Sequential order: by source, then capture time or path.
    v.sort_by(|a, b| {
        let src = |p: &Photo| enabled.iter().position(|id| *id == p.source_id);
        src(a).cmp(&src(b)).then_with(|| a.created.cmp(&b.created)).then_with(|| a.path.cmp(&b.path))
    });
    v
}

/// Photos in enabled, non-excluded sources whose rating is missing or older than `version`.
/// version 0 means "any rating counts".
fn rating_queue<'a>(s: &'a State, version: u32, limit: usize) -> Vec<&'a Photo> {
    let enabled: Vec<&str> = s.library.sources.iter().filter(|src| src.enabled).map(|src| src.id.as_str()).collect();
    let needs = |p: &&Photo| match s.library.scores.get(&p.id) {
        None => true,
        Some(sc) => version > 0 && sc.version < version,
    };
    let mut v: Vec<&Photo> = s
        .library
        .photos
        .iter()
        .filter(|p| enabled.contains(&p.source_id.as_str()) && !s.library.excluded.contains(&p.id))
        .filter(needs)
        .collect();
    // Rate what's in rotation first; ignore the score filter itself when deciding.
    v.sort_by_key(|p| !passes(&Filters { min_score: 0.0, ..s.settings.filters.clone() }, p, None));
    v.truncate(limit);
    v
}

fn score_of(s: &State, p: &Photo) -> Option<f64> {
    s.library.effective_score(&p.id).map(|sc| sc.value)
}

fn passes(f: &Filters, p: &Photo, score: Option<&PhotoScore>) -> bool {
    if f.min_score > 0.0 && score.is_some_and(|sc| sc.value < f.min_score) {
        return false;
    }
    // Tag rules only apply once a photo has been tagged.
    if let Some(sc) = score {
        if sc.tags.iter().any(|t| f.never_tags.contains(t)) {
            return false;
        }
        if !f.only_tags.is_empty() && !sc.tags.iter().any(|t| f.only_tags.contains(t)) {
            return false;
        }
    }
    if p.width == 0 || p.height == 0 {
        return true; // Unknown size; give it the benefit of the doubt.
    }
    let aspect = p.width as f64 / p.height as f64;
    p.width >= f.min_width
        && p.height >= f.min_height
        && (f.min_aspect <= 0.0 || aspect >= f.min_aspect - 1e-6)
        && (f.max_aspect <= 0.0 || aspect <= f.max_aspect + 1e-6)
}

/// Random pick that avoids the most recently shown photos when the pool allows.
fn pick_random(pool: &[&Photo], history: &[String], current: Option<&str>, weight: impl Fn(&Photo) -> f64) -> Photo {
    let avoid = (pool.len() / 2).min(history.len());
    let recent = &history[history.len() - avoid..];
    let fresh: Vec<&Photo> = pool.iter().copied().filter(|p| !recent.contains(&p.id) && Some(p.id.as_str()) != current).collect();
    if let Some(p) = weighted(&fresh, &weight) {
        return p.clone();
    }
    let others: Vec<&Photo> = pool.iter().copied().filter(|p| Some(p.id.as_str()) != current).collect();
    if let Some(p) = weighted(&others, &weight) {
        return p.clone();
    }
    pool[0].clone()
}

fn weighted<'a>(items: &[&'a Photo], weight: &impl Fn(&Photo) -> f64) -> Option<&'a Photo> {
    let weights: Vec<f64> = items.iter().map(|p| weight(p).max(0.0)).collect();
    let total: f64 = weights.iter().sum();
    if items.is_empty() {
        return None;
    }
    if total <= 0.0 {
        return Some(items[fastrand::usize(..items.len())]);
    }
    let mut r = fastrand::f64() * total;
    for (p, w) in items.iter().zip(&weights) {
        if r < *w {
            return Some(p);
        }
        r -= w;
    }
    items.last().copied()
}

fn unix(t: SystemTime) -> i64 {
    t.duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    struct Recorder(Mutex<Vec<String>>, AtomicUsize);
    impl EngineDelegate for Recorder {
        fn set_wallpaper(&self, path: String) -> bool {
            self.0.lock().unwrap().push(path);
            true
        }
        fn open_url(&self, _url: String) {}
        fn state_changed(&self) {
            self.1.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn png(path: &std::path::Path, w: u32, h: u32) {
        // Minimal PNG header: enough for imagesize to read dimensions.
        let mut b = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13, b'I', b'H', b'D', b'R'];
        b.extend_from_slice(&w.to_be_bytes());
        b.extend_from_slice(&h.to_be_bytes());
        b.extend_from_slice(&[8, 2, 0, 0, 0, 0, 0, 0, 0]);
        std::fs::write(path, b).unwrap();
    }

    fn setup() -> (Arc<Engine>, Arc<Recorder>, PathBuf) {
        let dir = std::env::temp_dir().join(format!("pinwheel-test-{}", fastrand::u64(..)));
        let pics = dir.join("pics");
        std::fs::create_dir_all(pics.join("sub")).unwrap();
        png(&pics.join("a.png"), 1920, 1080);
        png(&pics.join("b.png"), 3840, 2160);
        png(&pics.join("c.png"), 2560, 1440);
        png(&pics.join("portrait.png"), 1080, 1920);
        png(&pics.join("tiny.png"), 200, 100);
        png(&pics.join("sub").join("d.png"), 1920, 1200);
        let rec = Arc::new(Recorder(Mutex::new(vec![]), AtomicUsize::new(0)));
        let engine = Engine::new(Some(dir.join("data").to_string_lossy().into()), rec.clone()).unwrap();
        (engine, rec, pics)
    }

    #[test]
    fn folder_filters_and_sequence() {
        let (engine, rec, pics) = setup();
        let src = engine.add_local_folder(pics.to_string_lossy().into(), false).unwrap();
        assert_eq!(src.photo_count, 5); // sub/ not included
        // Default filters drop portrait and tiny.
        assert_eq!(engine.status().eligible_count, 3);

        let mut s = engine.settings();
        s.order = Order::Sequential;
        engine.update_settings(s).unwrap();

        let a = engine.next().unwrap();
        let b = engine.next().unwrap();
        let c = engine.next().unwrap();
        let wrap = engine.next().unwrap();
        assert_eq!(vec![&a.filename, &b.filename, &c.filename], vec!["a.png", "b.png", "c.png"]);
        assert_eq!(wrap.filename, "a.png");
        // Previous retraces history.
        assert_eq!(engine.previous().unwrap().filename, "c.png");
        assert_eq!(engine.previous().unwrap().filename, "b.png");
        // Next walks forward through history again.
        assert_eq!(engine.next().unwrap().filename, "c.png");
        assert_eq!(rec.0.lock().unwrap().len(), 7);
        assert_eq!(engine.status().current.unwrap().filename, "c.png");
    }

    #[test]
    fn exclude_and_persist() {
        let (engine, rec, pics) = setup();
        engine.add_local_folder(pics.to_string_lossy().into(), true).unwrap();
        assert_eq!(engine.status().eligible_count, 4);
        let shown = engine.shuffle().unwrap();
        engine.exclude_photo(shown.id.clone()).unwrap();
        assert_eq!(engine.status().eligible_count, 3);
        assert_ne!(engine.status().current.unwrap().id, shown.id, "moved off the excluded photo");
        assert!(rec.1.load(Ordering::SeqCst) > 0);

        // A new engine on the same data dir sees the same library.
        let again = Engine::new(Some(engine.data_dir()), rec.clone()).unwrap();
        assert_eq!(again.status().eligible_count, 3);
        assert_eq!(again.sources().len(), 1);
    }

    #[test]
    fn random_avoids_repeats() {
        let (engine, _rec, pics) = setup();
        engine.add_local_folder(pics.to_string_lossy().into(), true).unwrap();
        let mut last = String::new();
        for _ in 0..20 {
            let p = engine.shuffle().unwrap();
            assert_ne!(p.id, last);
            last = p.id;
        }
    }

    #[test]
    fn ratings_filter_and_weight() {
        let (engine, _rec, pics) = setup();
        engine.add_local_folder(pics.to_string_lossy().into(), true).unwrap();
        let queue = engine.photos_to_rate(1, 100);
        assert_eq!(queue.len(), 6);
        assert_eq!(engine.status().unrated_count, 6);
        // In-rotation photos come first (portrait and tiny are filtered by size/shape).
        assert!(queue[..4].iter().all(|p| p.filename != "portrait.png" && p.filename != "tiny.png"));

        let scores = queue
            .iter()
            .map(|p| PhotoScore {
                photo_id: p.id.clone(),
                value: if p.filename == "a.png" { 0.9 } else { 0.1 },
                label: "test".into(),
                version: 1,
                tags: if p.filename == "a.png" { vec!["test".into(), "landscape".into()] } else { vec!["test".into()] },
                title: Some(format!("Title of {}", p.filename)),
            })
            .collect();
        engine.set_scores(scores).unwrap();
        assert_eq!(engine.status().unrated_count, 0);
        assert_eq!(engine.tag_counts()[0].tag, "test");
        assert!(engine.photos_to_rate(1, 100).is_empty());
        assert_eq!(engine.photos_to_rate(2, 100).len(), 6, "newer algorithm re-rates");

        // Weighted random strongly prefers the 0.9 photo (weight 0.81 vs 0.01 each).
        let a_count = (0..200).filter(|_| engine.shuffle().unwrap().filename == "a.png").count();
        assert!(a_count > 60, "a.png picked {a_count}/200");

        let mut s = engine.settings();
        s.filters.min_score = 0.5;
        engine.update_settings(s).unwrap();
        assert_eq!(engine.status().eligible_count, 1);
        let entry = engine.photos(None).into_iter().find(|e| e.photo.filename == "a.png").unwrap();
        assert_eq!(entry.score, Some(0.9));
        assert!(entry.tags.contains(&"landscape".to_string()));
        assert_eq!(entry.title.as_deref(), Some("Title of a.png"));

        // User overrides win over the model, survive re-rating, and can be cleared.
        let b = engine.photos(None).into_iter().find(|e| e.photo.filename == "b.png").unwrap();
        engine.set_photo_title(b.photo.id.clone(), Some("  My Favorite  ".into())).unwrap();
        engine.set_photo_score(b.photo.id.clone(), Some(0.95)).unwrap();
        engine.set_scores(vec![PhotoScore { photo_id: b.photo.id.clone(), value: 0.1, label: "re-rated".into(), version: 2, tags: vec![], title: Some("Model Name".into()) }]).unwrap();
        let b2 = engine.photos(None).into_iter().find(|e| e.photo.id == b.photo.id).unwrap();
        assert_eq!((b2.title.as_deref(), b2.score, b2.custom_title, b2.custom_score), (Some("My Favorite"), Some(0.95), true, true));
        engine.set_photo_title(b.photo.id.clone(), None).unwrap();
        engine.set_photo_score(b.photo.id.clone(), None).unwrap();
        let b3 = engine.photos(None).into_iter().find(|e| e.photo.id == b.photo.id).unwrap();
        assert_eq!((b3.title.as_deref(), b3.score, b3.custom_title), (Some("Model Name"), Some(0.1), false));

        // Tag rules: "only" narrows, "never" excludes.
        let mut s = engine.settings();
        s.filters.min_score = 0.0;
        s.filters.only_tags = vec!["landscape".into()];
        engine.update_settings(s.clone()).unwrap();
        assert_eq!(engine.status().eligible_count, 1);
        s.filters.only_tags = vec![];
        s.filters.never_tags = vec!["landscape".into()];
        engine.update_settings(s).unwrap();
        assert_eq!(engine.status().eligible_count, 3);
        assert_eq!(entry.score_label.as_deref(), Some("test"));
    }

    #[test]
    fn disabled_source_and_no_photos() {
        let (engine, _rec, pics) = setup();
        let src = engine.add_local_folder(pics.to_string_lossy().into(), false).unwrap();
        engine.set_source_enabled(src.id.clone(), false).unwrap();
        assert!(matches!(engine.next(), Err(PinwheelError::NoPhotos)));
        engine.remove_source(src.id).unwrap();
        assert!(engine.sources().is_empty());
    }
}
