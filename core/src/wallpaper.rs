//! Built-in wallpaper setters. Frontends can override this through
//! `EngineDelegate::set_wallpaper` (macOS does, using NSWorkspace).

use crate::PinwheelError;

pub fn set(path: &str) -> Result<(), PinwheelError> {
    imp::set(path)
}

fn fail(msg: impl Into<String>) -> PinwheelError {
    PinwheelError::Wallpaper { message: msg.into() }
}

#[cfg(target_os = "windows")]
mod imp {
    use super::*;
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SystemParametersInfoW, SPIF_SENDCHANGE, SPIF_UPDATEINIFILE, SPI_SETDESKWALLPAPER,
    };

    pub fn set(path: &str) -> Result<(), PinwheelError> {
        let wide: Vec<u16> = std::ffi::OsStr::new(path).encode_wide().chain(Some(0)).collect();
        let ok = unsafe {
            SystemParametersInfoW(SPI_SETDESKWALLPAPER, 0, wide.as_ptr() as *mut _, SPIF_UPDATEINIFILE | SPIF_SENDCHANGE)
        };
        if ok == 0 { Err(fail(std::io::Error::last_os_error().to_string())) } else { Ok(()) }
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use super::*;
    use std::process::Command;

    /// Fallback only; the Swift app uses NSWorkspace, which needs no permissions.
    pub fn set(path: &str) -> Result<(), PinwheelError> {
        let escaped = path.replace('\\', "\\\\").replace('"', "\\\"");
        let script = format!("tell application \"System Events\" to tell every desktop to set picture to POSIX file \"{escaped}\"");
        let out = Command::new("osascript").args(["-e", &script]).output()?;
        if out.status.success() { Ok(()) } else { Err(fail(String::from_utf8_lossy(&out.stderr))) }
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
mod imp {
    use super::*;
    use std::process::Command;

    fn run(cmd: &str, args: &[&str]) -> bool {
        Command::new(cmd).args(args).output().map(|o| o.status.success()).unwrap_or(false)
    }

    pub fn set(path: &str) -> Result<(), PinwheelError> {
        let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default().to_lowercase();
        let uri = format!("file://{path}");

        if desktop.contains("kde") {
            if run("plasma-apply-wallpaperimage", &[path]) {
                return Ok(());
            }
        } else if desktop.contains("xfce") {
            // Set every monitor/workspace property that holds an image.
            let out = Command::new("xfconf-query").args(["-c", "xfce4-desktop", "-l"]).output()?;
            let props = String::from_utf8_lossy(&out.stdout);
            let mut any = false;
            for p in props.lines().filter(|l| l.ends_with("/last-image")) {
                any |= run("xfconf-query", &["-c", "xfce4-desktop", "-p", p, "-s", path]);
            }
            if any {
                return Ok(());
            }
        } else if desktop.contains("cinnamon") {
            if run("gsettings", &["set", "org.cinnamon.desktop.background", "picture-uri", &uri]) {
                return Ok(());
            }
        } else if desktop.contains("mate") {
            if run("gsettings", &["set", "org.mate.background", "picture-filename", path]) {
                return Ok(());
            }
        }

        // GNOME and friends. picture-uri-dark exists on GNOME 42+; ignore failure on older versions.
        if run("gsettings", &["set", "org.gnome.desktop.background", "picture-uri", &uri]) {
            let _ = run("gsettings", &["set", "org.gnome.desktop.background", "picture-uri-dark", &uri]);
            return Ok(());
        }
        // wlroots compositors and bare X11.
        if run("swww", &["img", path]) || run("feh", &["--bg-fill", path]) {
            return Ok(());
        }
        Err(fail(format!("don't know how to set the wallpaper on desktop '{desktop}'")))
    }
}
