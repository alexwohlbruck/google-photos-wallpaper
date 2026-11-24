//! Google Photos shared-album links (unofficial).
//!
//! Google offers no API for albums anymore, but an album shared "by link" is a
//! public web page whose HTML embeds the album's photo list as JSON. We read
//! that page, page through the rest with the same RPC the web UI uses, and
//! download each photo. It needs no sign-in, and new photos added to the album
//! show up on the next sync.
//!
//! This depends on Google's web page format, which can change at any time.
//! The parser looks for the *shape* of a photo entry rather than fixed array
//! positions so small layout changes don't break it.

use std::path::Path;
use std::time::Duration;

use serde_json::Value;

use crate::model::Photo;
use crate::PinwheelError;

/// A plain (non-browser) user agent gets an HTTP redirect from short links
/// instead of the JavaScript page served to desktop browsers.
const UA: &str = "Pinwheel/0.1 (+https://github.com/alexwohlbruck/google-photos-wallpaper)";
const MAX_EDGE: u32 = 6144;
const MAX_PAGES: usize = 200;

#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub id: String,
    pub url: String,
    pub width: u32,
    pub height: u32,
    pub taken_ms: Option<i64>,
    pub video: bool,
}

#[derive(Debug, Clone)]
pub struct Album {
    pub title: String,
    pub items: Vec<Item>,
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(60)))
        .user_agent(UA)
        .max_redirects(0)
        .build()
        .into()
}

fn net(context: &str, e: impl std::fmt::Display) -> PinwheelError {
    PinwheelError::Network { message: format!("{context}: {e}") }
}

fn bad(msg: &str) -> PinwheelError {
    PinwheelError::SharedAlbum { message: msg.to_string() }
}

/// Is this something we can sync? Returns a cleaned-up URL.
pub fn normalize(url: &str) -> Result<String, PinwheelError> {
    let url = url.trim();
    let url = if url.starts_with("http") { url.to_string() } else { format!("https://{url}") };
    let lower = url.to_ascii_lowercase();
    if lower.starts_with("https://photos.app.goo.gl/") || lower.starts_with("https://photos.google.com/share/") {
        Ok(url)
    } else if lower.starts_with("https://photos.google.com/album/") || lower.contains("photos.google.com/u/") {
        Err(bad("That's a private album link. In Google Photos, open the album, choose Share → Create link, and paste that link."))
    } else {
        Err(bad("Paste a Google Photos shared album link (photos.app.goo.gl/… or photos.google.com/share/…)."))
    }
}

/// Follow a short link to the canonical photos.google.com/share/… URL.
fn resolve(agent: &ureq::Agent, url: &str) -> Result<String, PinwheelError> {
    let mut current = url.to_string();
    for _ in 0..5 {
        if current.starts_with("https://photos.google.com/share/") {
            return Ok(current);
        }
        let resp = agent.get(&current).call().map_err(|e| net("open album link", e))?;
        match resp.headers().get("location").and_then(|v| v.to_str().ok()) {
            Some(loc) if resp.status().is_redirection() => current = loc.to_string(),
            _ => return Err(bad("The link didn't lead to a shared album. Check that link sharing is on.")),
        }
    }
    Err(bad("Too many redirects while opening the album link."))
}

/// Fetch the album's title and every item, following pagination.
pub fn fetch(url: &str) -> Result<Album, PinwheelError> {
    let agent = agent();
    let share_url = resolve(&agent, &normalize(url)?)?;
    let (album_id, key) = parse_share_url(&share_url).ok_or_else(|| bad("Unrecognized shared album URL."))?;

    let mut resp = agent
        .get(&share_url)
        .header("Accept-Language", "en-US,en;q=0.8")
        .call()
        .map_err(|e| match e {
            ureq::Error::StatusCode(404) => bad("Album not found. It may have been deleted or unshared."),
            e => net("load album", e),
        })?;
    let html = resp
        .body_mut()
        .with_config()
        .limit(64 * 1024 * 1024)
        .read_to_string()
        .map_err(|e| net("load album", e))?;

    let data = extract_init_data(&html, "ds:1")
        .ok_or_else(|| bad("Couldn't read the album page. Google may have changed its format, or the album is no longer shared."))?;

    let title = album_title(&data).or_else(|| html_title(&html)).unwrap_or_else(|| "Shared album".into());
    let mut items = Vec::new();
    collect_items(&data, &mut items);
    let mut token = page_token(&data);

    let mut pages = 0;
    while let Some(t) = token.take() {
        pages += 1;
        if pages > MAX_PAGES {
            break;
        }
        let page = fetch_page(&agent, &album_id, &key, &t)?;
        let before = items.len();
        collect_items(&page, &mut items);
        token = page_token(&page);
        if items.len() == before {
            break; // No progress; don't loop forever on a format change.
        }
    }

    dedupe(&mut items);
    if items.is_empty() {
        return Err(bad("The album is empty, or its page format wasn't recognized."));
    }
    Ok(Album { title, items })
}

/// One page of additional items via the web UI's batchexecute RPC.
fn fetch_page(agent: &ureq::Agent, album_id: &str, key: &str, token: &str) -> Result<Value, PinwheelError> {
    let inner = serde_json::to_string(&serde_json::json!([album_id, token, null, key])).unwrap();
    let freq = serde_json::to_string(&serde_json::json!([[["snAcKc", inner, null, "generic"]]])).unwrap();
    let url = format!(
        "https://photos.google.com/_/PhotosUi/data/batchexecute?rpcids=snAcKc&source-path={}&hl=en&rt=c",
        urlencoding::encode(&format!("/share/{album_id}"))
    );
    let body = agent
        .post(&url)
        .header("Accept-Language", "en-US,en;q=0.8")
        .send_form([("f.req", freq.as_str())])
        .map_err(|e| net("load more of the album", e))?
        .body_mut()
        .with_config()
        .limit(64 * 1024 * 1024)
        .read_to_string()
        .map_err(|e| net("load more of the album", e))?;
    parse_batchexecute(&body, "snAcKc").ok_or_else(|| bad("Couldn't read the next page of the album."))
}

/// Download one item scaled to MAX_EDGE on its long side (always JPEG).
pub fn download(item: &Item, source_id: &str, dir: &Path) -> Result<Photo, PinwheelError> {
    std::fs::create_dir_all(dir)?;
    let safe: String = item.id.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_').collect();
    let path = dir.join(format!("{safe}.jpg"));
    if !path.exists() {
        let tmp = path.with_extension("part");
        let mut resp = agent()
            .get(&format!("{}=w{MAX_EDGE}-h{MAX_EDGE}", item.url))
            .call()
            .map_err(|e| net("download photo", e))?;
        {
            let mut f = std::fs::File::create(&tmp)?;
            std::io::copy(&mut resp.body_mut().as_reader(), &mut f)?;
        }
        std::fs::rename(&tmp, &path)?;
    }
    let (width, height) = imagesize::size(&path)
        .map(|s| (s.width as u32, s.height as u32))
        .unwrap_or((item.width, item.height));
    Ok(Photo {
        id: format!("s:{}", item.id),
        source_id: source_id.to_string(),
        path: path.to_string_lossy().into_owned(),
        filename: format!("{safe}.jpg"),
        width,
        height,
        created: item.taken_ms.map(rfc3339_from_ms),
    })
}

// ---- Parsing ----

fn parse_share_url(url: &str) -> Option<(String, String)> {
    let rest = url.strip_prefix("https://photos.google.com/share/")?;
    let (id, query) = rest.split_once('?').unwrap_or((rest, ""));
    let key = query.split('&').find_map(|kv| kv.strip_prefix("key="))?.to_string();
    Some((id.trim_end_matches('/').to_string(), key))
}

/// The JSON passed to `AF_initDataCallback({key: '<key>', ..., data: <JSON>, sideChannel: ...})`.
fn extract_init_data(html: &str, key: &str) -> Option<Value> {
    let marker = format!("AF_initDataCallback({{key: '{key}'");
    let start = html.find(&marker)?;
    let rest = &html[start..];
    let data_at = rest.find("data:")? + "data:".len();
    // Let serde find where the JSON value ends instead of guessing with string search.
    let mut de = serde_json::Deserializer::from_str(&rest[data_at..]).into_iter::<Value>();
    de.next()?.ok()
}

/// batchexecute responses: `)]}'` then length-prefixed chunks; the payload is
/// a JSON string inside `["wrb.fr", "<rpcid>", "<json>", ...]`.
fn parse_batchexecute(body: &str, rpcid: &str) -> Option<Value> {
    for line in body.lines() {
        let line = line.trim();
        if !line.starts_with('[') || !line.contains("wrb.fr") {
            continue;
        }
        let Ok(Value::Array(outer)) = serde_json::from_str::<Value>(line) else { continue };
        for entry in outer {
            let Value::Array(e) = entry else { continue };
            if e.first().and_then(Value::as_str) == Some("wrb.fr") && e.get(1).and_then(Value::as_str) == Some(rpcid) {
                if let Some(payload) = e.get(2).and_then(Value::as_str) {
                    return serde_json::from_str(payload).ok();
                }
            }
        }
    }
    None
}

/// Find every `["AF1Q…", ["https://lh3.googleusercontent.com/…", w, h, …], taken_ms, …]`.
fn collect_items(v: &Value, out: &mut Vec<Item>) {
    let Value::Array(arr) = v else {
        if let Value::Object(map) = v {
            for child in map.values() {
                collect_items(child, out);
            }
        }
        return;
    };
    if let Some(item) = as_item(arr) {
        out.push(item);
        return;
    }
    for child in arr {
        collect_items(child, out);
    }
}

fn as_item(arr: &[Value]) -> Option<Item> {
    let id = arr.first()?.as_str()?;
    if !id.starts_with("AF1Q") {
        return None;
    }
    let media = arr.get(1)?.as_array()?;
    let url = media.first()?.as_str()?;
    if !url.starts_with("https://lh3.googleusercontent.com/") {
        return None;
    }
    let width = media.get(1)?.as_u64()? as u32;
    let height = media.get(2)?.as_u64()? as u32;
    // Videos carry an extra metadata object keyed 76647426 (as observed in the web UI's data).
    let video = arr.iter().any(|v| v.as_object().is_some_and(|o| o.contains_key("76647426")));
    Some(Item {
        id: id.to_string(),
        url: url.to_string(),
        width,
        height,
        taken_ms: arr.get(2).and_then(Value::as_i64),
        video,
    })
}

fn page_token(data: &Value) -> Option<String> {
    data.get(2).and_then(Value::as_str).filter(|t| !t.is_empty()).map(str::to_string)
}

fn album_title(data: &Value) -> Option<String> {
    data.get(3)?.get(1)?.as_str().filter(|t| !t.is_empty()).map(str::to_string)
}

fn html_title(html: &str) -> Option<String> {
    let start = html.find("<title>")? + 7;
    let end = html[start..].find("</title>")? + start;
    let t = html[start..end].trim().trim_end_matches("- Google Photos").trim();
    (!t.is_empty()).then(|| t.to_string())
}

fn dedupe(items: &mut Vec<Item>) {
    let mut seen = std::collections::HashSet::new();
    items.retain(|i| seen.insert(i.id.clone()));
}

fn rfc3339_from_ms(ms: i64) -> String {
    // Civil-from-days (Howard Hinnant), to avoid pulling in a date crate for one format.
    let secs = ms.div_euclid(1000);
    let (days, sod) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", sod / 3600, sod % 3600 / 60, sod % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGE: &str = r#"<html><title>Trip - Google Photos</title><script>AF_initDataCallback({key: 'ds:1', hash: '2', data:[null,[["AF1QipA",["https://lh3.googleusercontent.com/pw/AAA",4080,3072,null],1768093198748,"x",-28800000,1,["AF1Qowner"],[[2]],2,{"15":1}],["AF1QipB",["https://lh3.googleusercontent.com/pw/BBB",1080,1920],1768093198000,"y",0,1,[],[],2,{"76647426":[1]}]],"NEXT",["AF1QipAlbum","Trip",[1,2],null,["https://lh3.googleusercontent.com/pw/COVER",100,100],["AF1Qowner","123"]]], sideChannel: {}});</script></html>"#;

    #[test]
    fn parses_album_page() {
        let data = extract_init_data(PAGE, "ds:1").unwrap();
        let mut items = vec![];
        collect_items(&data, &mut items);
        assert_eq!(items.len(), 2, "cover and owner entries aren't items");
        assert_eq!(items[0].id, "AF1QipA");
        assert_eq!((items[0].width, items[0].height), (4080, 3072));
        assert!(!items[0].video);
        assert!(items[1].video);
        assert_eq!(page_token(&data).as_deref(), Some("NEXT"));
        assert_eq!(album_title(&data).as_deref(), Some("Trip"));
        assert_eq!(html_title(PAGE).as_deref(), Some("Trip"));
    }

    #[test]
    fn parses_batchexecute() {
        let inner = r#"[null,[["AF1QipC",["https://lh3.googleusercontent.com/pw/CCC",10,20],5]],""]"#;
        let line = serde_json::to_string(&serde_json::json!([["wrb.fr", "snAcKc", inner, null, null, null, "generic"]])).unwrap();
        let body = format!(")]}}'\n\n{}\n{line}\n25\n[[\"e\",4,null,null,1]]\n", line.len());
        let v = parse_batchexecute(&body, "snAcKc").unwrap();
        let mut items = vec![];
        collect_items(&v, &mut items);
        assert_eq!(items[0].id, "AF1QipC");
        assert_eq!(page_token(&v), None);
    }

    #[test]
    fn urls() {
        assert!(normalize("photos.app.goo.gl/abc").is_ok());
        assert!(normalize("https://photos.google.com/share/AF1Q?key=k").is_ok());
        assert!(matches!(normalize("https://photos.google.com/album/AF1Q"), Err(PinwheelError::SharedAlbum { .. })));
        assert!(normalize("https://example.com").is_err());
        assert_eq!(
            parse_share_url("https://photos.google.com/share/AF1Qx?key=abc&foo=1"),
            Some(("AF1Qx".into(), "abc".into()))
        );
    }

    #[test]
    fn dates() {
        assert_eq!(rfc3339_from_ms(0), "1970-01-01T00:00:00Z");
        assert_eq!(rfc3339_from_ms(1768093198748), "2026-01-11T00:59:58Z");
    }
}
