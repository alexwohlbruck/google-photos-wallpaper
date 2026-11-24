//! Google sign-in (installed-app OAuth with PKCE and a loopback redirect) and the
//! Google Photos Picker API.
//!
//! The Library API scopes that could list a user's albums were removed on
//! 2025-03-31, so the picker is the only way to read a user's photos. The user
//! picks items in a Google-hosted page; we download them right away because
//! their base URLs expire after 60 minutes.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::model::Photo;
use crate::store;
use crate::PinwheelError;

pub const SCOPE: &str = "https://www.googleapis.com/auth/photospicker.mediaitems.readonly";
const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const PICKER: &str = "https://photospicker.googleapis.com/v1";
/// Downloads are capped at this size on the long edge. Large enough for 5K/6K displays.
const MAX_EDGE: u32 = 6144;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    pub client_id: String,
    /// Desktop OAuth clients get a "secret" Google says isn't treated as one.
    #[serde(default)]
    pub client_secret: String,
}

impl ClientConfig {
    /// Resolve in order: env vars, `google_client.json` in the data dir, build-time env.
    pub fn resolve(config_path: &Path) -> Option<Self> {
        if let Ok(id) = std::env::var("PINWHEEL_GOOGLE_CLIENT_ID") {
            let secret = std::env::var("PINWHEEL_GOOGLE_CLIENT_SECRET").unwrap_or_default();
            return Some(Self { client_id: id, client_secret: secret });
        }
        if let Some(c) = std::fs::read(config_path).ok().and_then(|b| serde_json::from_slice::<Self>(&b).ok()) {
            return Some(c);
        }
        option_env!("PINWHEEL_GOOGLE_CLIENT_ID").map(|id| Self {
            client_id: id.to_string(),
            client_secret: option_env!("PINWHEEL_GOOGLE_CLIENT_SECRET").unwrap_or("").to_string(),
        })
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Token {
    pub access_token: String,
    pub refresh_token: Option<String>,
    /// Unix seconds.
    pub expires_at: i64,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: i64,
    refresh_token: Option<String>,
}

fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

fn api_err(context: &str, e: ureq::Error) -> PinwheelError {
    PinwheelError::Network { message: format!("{context}: {e}") }
}

pub struct Google {
    pub client: Option<ClientConfig>,
    token_path: PathBuf,
}

impl Google {
    pub fn new(client: Option<ClientConfig>, token_path: PathBuf) -> Self {
        Self { client, token_path }
    }

    pub fn signed_in(&self) -> bool {
        self.load_token().map(|t| t.refresh_token.is_some()).unwrap_or(false)
    }

    fn load_token(&self) -> Option<Token> {
        std::fs::read(&self.token_path).ok().and_then(|b| serde_json::from_slice(&b).ok())
    }

    pub fn sign_out(&self) {
        if let Some(t) = self.load_token() {
            // Best effort: revoke at Google so the grant disappears from the account page.
            let tok = t.refresh_token.unwrap_or(t.access_token);
            let _ = ureq::post("https://oauth2.googleapis.com/revoke").send_form([("token", tok.as_str())]);
        }
        let _ = std::fs::remove_file(&self.token_path);
    }

    fn client(&self) -> Result<&ClientConfig, PinwheelError> {
        self.client.as_ref().ok_or(PinwheelError::GoogleNotConfigured)
    }

    /// Run the browser sign-in flow. Blocks until the browser redirects back,
    /// `cancel` is set, or five minutes pass.
    pub fn sign_in(&self, open_url: &dyn Fn(String), cancel: &AtomicBool) -> Result<(), PinwheelError> {
        let client = self.client()?;
        let listener = TcpListener::bind("127.0.0.1:0")?;
        listener.set_nonblocking(true)?;
        let redirect = format!("http://127.0.0.1:{}", listener.local_addr()?.port());

        let verifier = random_token(48)?;
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        let state = random_token(16)?;
        let url = format!(
            "{AUTH_URL}?client_id={}&redirect_uri={}&response_type=code&scope={}&code_challenge={challenge}\
             &code_challenge_method=S256&access_type=offline&prompt=consent&state={state}",
            urlencoding::encode(&client.client_id),
            urlencoding::encode(&redirect),
            urlencoding::encode(SCOPE),
        );
        open_url(url);

        let deadline = Instant::now() + Duration::from_secs(300);
        loop {
            if cancel.load(Ordering::SeqCst) {
                return Err(PinwheelError::Cancelled);
            }
            if Instant::now() > deadline {
                return Err(PinwheelError::Auth { message: "timed out waiting for sign-in".into() });
            }
            let (mut stream, _) = match listener.accept() {
                Ok(s) => s,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(200));
                    continue;
                }
                Err(e) => return Err(e.into()),
            };
            stream.set_nonblocking(false)?;
            stream.set_read_timeout(Some(Duration::from_secs(5)))?;
            let mut line = String::new();
            BufReader::new(&stream).read_line(&mut line)?;
            // "GET /?code=...&state=... HTTP/1.1"
            let target = line.split_whitespace().nth(1).unwrap_or("");
            let params = parse_query(target);
            let get = |k: &str| params.iter().find(|(n, _)| n == k).map(|(_, v)| v.clone());

            let result: Result<(), PinwheelError> = match (get("code"), get("error")) {
                (Some(_), _) if get("state").as_deref() != Some(state.as_str()) => {
                    Err(PinwheelError::Auth { message: "state mismatch".into() })
                }
                // Exchange before answering so the page only says "signed in" when it's true.
                (Some(code), _) => self.exchange(client, &code, &redirect, &verifier),
                (None, Some(err)) => Err(PinwheelError::Auth { message: err }),
                // Favicon or a stray request: ignore it and keep waiting.
                (None, None) => {
                    let _ = stream.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n");
                    continue;
                }
            };
            let body = match &result {
                Ok(()) => "Signed in. You can close this tab and return to the app.".to_string(),
                Err(e) => format!("{e}. You can close this tab and try again from the app."),
            };
            let html = format!(
                "<!doctype html><meta charset=utf-8><title>Pinwheel</title>\
                 <body style=\"font:16px system-ui;margin:4em;text-align:center\">{}</body>",
                body.replace('&', "&amp;").replace('<', "&lt;")
            );
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{html}",
                html.len()
            );
            return result;
        }
    }

    fn exchange(&self, client: &ClientConfig, code: &str, redirect: &str, verifier: &str) -> Result<(), PinwheelError> {
        let resp: TokenResponse = ureq::post(TOKEN_URL)
            .send_form([
                ("code", code),
                ("client_id", client.client_id.as_str()),
                ("client_secret", client.client_secret.as_str()),
                ("redirect_uri", redirect),
                ("grant_type", "authorization_code"),
                ("code_verifier", verifier),
            ])
            .map_err(|e| api_err("token exchange", e))?
            .body_mut()
            .read_json()
            .map_err(|e| api_err("token exchange", e))?;
        self.save_token(&Token {
            access_token: resp.access_token,
            refresh_token: resp.refresh_token,
            expires_at: now() + resp.expires_in,
        })
    }

    fn save_token(&self, t: &Token) -> Result<(), PinwheelError> {
        store::save(&self.token_path, t)
    }

    /// A valid access token, refreshing it if it expires within a minute.
    fn access_token(&self) -> Result<String, PinwheelError> {
        let mut t = self.load_token().ok_or(PinwheelError::NotSignedIn)?;
        if t.expires_at - 60 > now() {
            return Ok(t.access_token);
        }
        let refresh = t.refresh_token.clone().ok_or(PinwheelError::NotSignedIn)?;
        let client = self.client()?;
        let resp: TokenResponse = ureq::post(TOKEN_URL)
            .send_form([
                ("refresh_token", refresh.as_str()),
                ("client_id", client.client_id.as_str()),
                ("client_secret", client.client_secret.as_str()),
                ("grant_type", "refresh_token"),
            ])
            .map_err(|e| match e {
                // invalid_grant: the user revoked access. Drop the token so the UI shows "Sign in".
                ureq::Error::StatusCode(400) => {
                    let _ = std::fs::remove_file(&self.token_path);
                    PinwheelError::NotSignedIn
                }
                e => api_err("token refresh", e),
            })?
            .body_mut()
            .read_json()
            .map_err(|e| api_err("token refresh", e))?;
        t.access_token = resp.access_token;
        t.expires_at = now() + resp.expires_in;
        if resp.refresh_token.is_some() {
            t.refresh_token = resp.refresh_token;
        }
        self.save_token(&t)?;
        Ok(t.access_token)
    }

    /// Open a picker session, wait for the user to finish, and download every
    /// picked photo into `cache_dir`. `progress` gets human-readable updates.
    pub fn pick(
        &self,
        source_id: &str,
        cache_dir: &Path,
        open_url: &dyn Fn(String),
        progress: &dyn Fn(String),
        cancel: &AtomicBool,
    ) -> Result<Vec<Photo>, PinwheelError> {
        let token = self.access_token()?;
        let session: Session = ureq::post(format!("{PICKER}/sessions"))
            .header("Authorization", format!("Bearer {token}"))
            .send_json(serde_json::json!({}))
            .map_err(|e| api_err("create picker session", e))?
            .body_mut()
            .read_json()
            .map_err(|e| api_err("create picker session", e))?;

        let result = self.run_session(&session, source_id, cache_dir, open_url, progress, cancel);
        // Sessions hold the user's selection server-side; always clean up.
        if let Ok(token) = self.access_token() {
            let _ = ureq::delete(format!("{PICKER}/sessions/{}", session.id))
                .header("Authorization", format!("Bearer {token}"))
                .call();
        }
        result
    }

    fn run_session(
        &self,
        session: &Session,
        source_id: &str,
        cache_dir: &Path,
        open_url: &dyn Fn(String),
        progress: &dyn Fn(String),
        cancel: &AtomicBool,
    ) -> Result<Vec<Photo>, PinwheelError> {
        open_url(format!("{}/autoclose", session.picker_uri.trim_end_matches('/')));
        progress("Waiting for you to pick photos in your browser…".into());

        let poll = session.polling_config.as_ref();
        let interval = poll.and_then(|p| parse_duration(&p.poll_interval)).unwrap_or(Duration::from_secs(3));
        let timeout = poll.and_then(|p| parse_duration(&p.timeout_in)).unwrap_or(Duration::from_secs(1800));
        let deadline = Instant::now() + timeout;

        loop {
            sleep_cancellable(interval, cancel)?;
            if Instant::now() > deadline {
                return Err(PinwheelError::Network { message: "the picker session timed out".into() });
            }
            let s: Session = ureq::get(format!("{PICKER}/sessions/{}", session.id))
                .header("Authorization", format!("Bearer {}", self.access_token()?))
                .call()
                .map_err(|e| api_err("poll picker session", e))?
                .body_mut()
                .read_json()
                .map_err(|e| api_err("poll picker session", e))?;
            if s.media_items_set {
                break;
            }
        }

        let mut items = Vec::new();
        let mut page_token: Option<String> = None;
        loop {
            let mut req = ureq::get(format!("{PICKER}/mediaItems"))
                .header("Authorization", format!("Bearer {}", self.access_token()?))
                .query("sessionId", &session.id)
                .query("pageSize", "100");
            if let Some(t) = &page_token {
                req = req.query("pageToken", t);
            }
            let page: MediaPage = req
                .call()
                .map_err(|e| api_err("list picked items", e))?
                .body_mut()
                .read_json()
                .map_err(|e| api_err("list picked items", e))?;
            items.extend(page.media_items);
            match page.next_page_token {
                Some(t) if !t.is_empty() => page_token = Some(t),
                _ => break,
            }
        }

        let photos_only: Vec<_> = items.into_iter().filter(|i| i.kind.as_deref() != Some("VIDEO")).collect();
        let total = photos_only.len();
        let mut out = Vec::with_capacity(total);
        for (n, item) in photos_only.into_iter().enumerate() {
            if cancel.load(Ordering::SeqCst) {
                break; // Keep what's already downloaded.
            }
            progress(format!("Downloading {} of {total}…", n + 1));
            match self.download(&item, source_id, cache_dir) {
                Ok(p) => out.push(p),
                Err(e) => eprintln!("pinwheel: skipping {}: {e}", item.id),
            }
        }
        Ok(out)
    }

    fn download(&self, item: &PickedItem, source_id: &str, cache_dir: &Path) -> Result<Photo, PinwheelError> {
        let file = &item.media_file;
        let meta = file.media_file_metadata.as_ref();
        let (w, h) = (meta.and_then(|m| m.width).unwrap_or(0), meta.and_then(|m| m.height).unwrap_or(0));
        // Ask for the image scaled to MAX_EDGE on its long side. The response is
        // always JPEG, so HEIC originals work on every OS.
        let (tw, th) = fit(w, h, MAX_EDGE);
        let url = format!("{}=w{tw}-h{th}", file.base_url);

        let safe_id: String = item.id.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_').collect();
        let path = cache_dir.join(format!("{safe_id}.jpg"));
        let tmp = path.with_extension("part");
        let mut resp = ureq::get(&url)
            .header("Authorization", format!("Bearer {}", self.access_token()?))
            .call()
            .map_err(|e| api_err("download photo", e))?;
        {
            let mut f = std::fs::File::create(&tmp)?;
            std::io::copy(&mut resp.body_mut().as_reader(), &mut f)?;
        }
        std::fs::rename(&tmp, &path)?;

        let (width, height) = imagesize::size(&path).map(|s| (s.width as u32, s.height as u32)).unwrap_or((tw, th));
        Ok(Photo {
            id: format!("g:{}", item.id),
            source_id: source_id.to_string(),
            path: path.to_string_lossy().into_owned(),
            filename: file.filename.clone().unwrap_or_else(|| format!("{safe_id}.jpg")),
            width,
            height,
            created: item.create_time.clone(),
        })
    }
}

/// Scale (w, h) down so the long edge is at most `max`. Unknown sizes get `max x max`.
fn fit(w: u32, h: u32, max: u32) -> (u32, u32) {
    if w == 0 || h == 0 {
        return (max, max);
    }
    let long = w.max(h);
    if long <= max {
        return (w, h);
    }
    let scale = max as f64 / long as f64;
    (((w as f64 * scale).round() as u32).max(1), ((h as f64 * scale).round() as u32).max(1))
}

fn sleep_cancellable(d: Duration, cancel: &AtomicBool) -> Result<(), PinwheelError> {
    let end = Instant::now() + d;
    while Instant::now() < end {
        if cancel.load(Ordering::SeqCst) {
            return Err(PinwheelError::Cancelled);
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Ok(())
}

/// Google durations are strings like "5s" or "1.5s".
fn parse_duration(s: &str) -> Option<Duration> {
    s.strip_suffix('s')?.parse::<f64>().ok().filter(|v| *v > 0.0).map(Duration::from_secs_f64)
}

fn parse_query(target: &str) -> Vec<(String, String)> {
    let q = target.split_once('?').map(|(_, q)| q).unwrap_or("");
    q.split('&')
        .filter_map(|kv| kv.split_once('='))
        .map(|(k, v)| {
            let v = v.replace('+', " ");
            (k.to_string(), urlencoding::decode(&v).map(|c| c.into_owned()).unwrap_or(v))
        })
        .collect()
}

fn random_token(bytes: usize) -> Result<String, PinwheelError> {
    let mut buf = vec![0u8; bytes];
    getrandom::fill(&mut buf).map_err(|e| PinwheelError::Io { message: e.to_string() })?;
    Ok(URL_SAFE_NO_PAD.encode(buf))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Session {
    id: String,
    #[serde(default)]
    picker_uri: String,
    #[serde(default)]
    polling_config: Option<PollingConfig>,
    #[serde(default)]
    media_items_set: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PollingConfig {
    #[serde(default)]
    poll_interval: String,
    #[serde(default)]
    timeout_in: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MediaPage {
    #[serde(default)]
    media_items: Vec<PickedItem>,
    next_page_token: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PickedItem {
    id: String,
    create_time: Option<String>,
    #[serde(rename = "type")]
    kind: Option<String>,
    media_file: MediaFile,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MediaFile {
    base_url: String,
    filename: Option<String>,
    media_file_metadata: Option<MediaFileMetadata>,
}

#[derive(Deserialize)]
struct MediaFileMetadata {
    width: Option<u32>,
    height: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations() {
        assert_eq!(parse_duration("5s"), Some(Duration::from_secs(5)));
        assert_eq!(parse_duration("1.5s"), Some(Duration::from_millis(1500)));
        assert_eq!(parse_duration("bogus"), None);
    }

    #[test]
    fn fits_long_edge() {
        assert_eq!(fit(8000, 4000, 6144), (6144, 3072));
        assert_eq!(fit(3000, 2000, 6144), (3000, 2000));
        assert_eq!(fit(0, 0, 6144), (6144, 6144));
    }

    #[test]
    fn parses_redirect() {
        let q = parse_query("/?state=abc&code=4%2F0Ab_x&scope=a+b");
        assert_eq!(q[1], ("code".into(), "4/0Ab_x".into()));
        assert_eq!(q[2], ("scope".into(), "a b".into()));
    }

    #[test]
    fn picked_item_json() {
        let j = r#"{"mediaItems":[{"id":"AB1","createTime":"2024-01-01T00:00:00Z","type":"PHOTO",
            "mediaFile":{"baseUrl":"https://x","mimeType":"image/jpeg","filename":"a.jpg",
            "mediaFileMetadata":{"width":4032,"height":3024}}}],"nextPageToken":"n"}"#;
        let p: MediaPage = serde_json::from_str(j).unwrap();
        assert_eq!(p.media_items[0].media_file.media_file_metadata.as_ref().unwrap().width, Some(4032));
    }
}
