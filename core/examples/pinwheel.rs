//! Small dev CLI over the core, for poking at a library without a UI.
//!
//!   PINWHEEL_DATA_DIR=/tmp/pinwheel cargo run -p pinwheel-core --example pinwheel -- add-folder ~/Pictures
//!   PINWHEEL_DATA_DIR=/tmp/pinwheel cargo run -p pinwheel-core --example pinwheel -- status
//!
//! `next`/`previous`/`shuffle` only print their pick unless `--apply` is given,
//! so they don't touch your real desktop by accident.

use std::sync::Arc;

use pinwheel_core::{Engine, EngineDelegate};

struct Cli {
    apply: bool,
}

impl EngineDelegate for Cli {
    fn set_wallpaper(&self, _path: String) -> bool {
        // true = "handled", which skips the built-in setter.
        !self.apply
    }
    fn open_url(&self, url: String) {
        println!("open: {url}");
        if std::env::var_os("PINWHEEL_NO_BROWSER").is_some() {
            return;
        }
        let _ = std::process::Command::new(if cfg!(target_os = "macos") { "open" } else { "xdg-open" }).arg(&url).status();
    }
    fn state_changed(&self) {}
}

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let apply = args.iter().any(|a| a == "--apply");
    args.retain(|a| a != "--apply");
    let engine = Engine::new(None, Arc::new(Cli { apply })).expect("open data dir");

    let result: Result<(), pinwheel_core::PinwheelError> = (|| {
        match args.first().map(String::as_str) {
            Some("add-folder") => {
                let path = args.get(1).expect("add-folder <path>");
                let s = engine.add_local_folder(path.clone(), true)?;
                println!("added {} ({} photos)", s.name, s.photo_count);
            }
            Some("add-album") => {
                let url = args.get(1).expect("add-album <shared album link>");
                let s = engine.add_shared_album(url.clone())?;
                println!("added {} ({} photos)", s.name, s.photo_count);
            }
            Some("sync") => {
                for s in engine.sources() {
                    if matches!(s.kind, pinwheel_core::SourceKind::SharedAlbum { .. }) {
                        engine.sync_source(s.id.clone())?;
                    }
                }
            }
            Some("to-rate") => {
                let v: u32 = args.get(1).and_then(|v| v.parse().ok()).unwrap_or(1);
                println!("{} photos to rate (unrated_count={})", engine.photos_to_rate(v, 10_000).len(), engine.status().unrated_count);
            }
            Some("rescan") => engine.rescan()?,
            Some("sign-in") => engine.google_sign_in()?,
            Some("import") => println!("imported {}", engine.google_import()?),
            Some(cmd @ ("next" | "previous" | "shuffle")) => {
                let p = match cmd {
                    "next" => engine.next()?,
                    "previous" => engine.previous()?,
                    _ => engine.shuffle()?,
                };
                println!("{}", p.path);
            }
            _ => {
                let st = engine.status();
                println!("data dir:  {}", engine.data_dir());
                println!("eligible:  {}", st.eligible_count);
                println!("current:   {:?}", st.current.map(|p| p.path));
                println!("google:    configured={} signed_in={}", st.google_configured, st.google_signed_in);
                for s in engine.sources() {
                    println!(
                        "source:    {} [{}] {} photos  synced={:?} error={:?}",
                        s.name, if s.enabled { "on" } else { "off" }, s.photo_count, s.last_synced_unix, s.sync_error
                    );
                }
            }
        }
        Ok(())
    })();
    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
