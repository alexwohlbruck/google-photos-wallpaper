# Pinwheel

Rotates your desktop wallpaper through photos from Google Photos and local folders, on a schedule. A shared Rust core with native UIs. **macOS is the only frontend so far**; Windows and Linux are planned.

> This is a from-scratch rewrite. The original Python/Eel/Vue app is on `master`.

## How Google Photos works now

In March 2025 Google removed the API scopes that let third-party apps browse a user's library and albums. The only supported way to read your photos now is the **Google Photos Picker**:

1. The app opens a Google-hosted picker in your browser.
2. You choose individual photos (up to 2,000 per session; date headers select a whole day). The picker has no way to choose an album.
3. The app downloads them right away (the download links expire after 60 minutes) and caches them locally.

So the app can't follow an album automatically. If you add photos to an album later, pick them again. Local folders don't have this limit, and they work well with anything that syncs to disk.

## Shared album links (unofficial)

An album shared by link is a public web page that lists its photos. Paste the link, and the app reads that page (and fetches later pages the way Google's web UI does), downloads the photos, and re-syncs every 6 hours. New photos appear, and photos removed from the album get removed. It needs no sign-in, but it relies on Google's page format, which can change without notice. The parser matches the shape of photo entries rather than fixed positions, and when a sync fails the album keeps its photos and the error is shown on the source.

## Wallpaper ratings (macOS)

Each photo gets a 0–1 rating for how well it works as a wallpaper, computed entirely on-device:

1. **Vision** (Neural Engine): Apple's aesthetics model (overall quality, plus a flag for screenshots/receipts), scene labels, faces and people.
2. **Apple Intelligence's on-device language model** puts the photo in a wallpaper-relevant category (landscape, cityscape, portrait, group photo, food, screenshot…) based on those signals. It can't see pixels yet, and small models are more reliable at choosing a category than inventing a number. Categories the labels don't support are rejected. Without Apple Intelligence, a keyword mapping of the labels is used instead.
3. The score blends how wallpaper-friendly the category is (60%) with image quality (40%). It's capped at 0.35 when people are the subject, and screenshots get 0.02.

The app shows ratings as 1–5 stars with a plain-language tier ("Great wallpaper", "Good", "Okay", "Not ideal", "Not a wallpaper"). Ratings can hide photos below a chosen tier and, in random order, make better-rated photos come up more often (weight = score²). Bump `PhotoRater.version` to re-rate everything after changing the algorithm.

## Names and tags (macOS)

The same on-device pass also:

- **Names each photo** like a wallpaper ("Teal Lagoon", "Mountain Mist", "Ocean Whisper"). The language model suggests four names from the photo's contents, category and main colours, and the app picks one that no other photo uses. (Listing names already in use in the prompt doesn't work: the small model copies them.) Without Apple Intelligence, a name is built from the main colour and the most specific label.
- **Tags each photo** (People, Landscapes, Water & Beaches, Skies & Sunsets, Nature, Flowers, Pets & Animals, City, Architecture, Art & Color, Food, Night, Snow, Vehicles, Indoors, Documents) from Vision's labels, its people detection, and the final category. Tags appear in the sidebar. Right-click a tag to use **only** those photos, or **never** use them.

You can rename any photo and set its stars yourself. Your names and ratings are stored separately from the automatic ones (`overrides` in library.json), so re-rating never overwrites them; choose "Automatic" or "Use Suggested Name" to go back.

## Architecture

```
core/                 Rust: sources, library + cache, filters, history,
                      scheduler, Google OAuth (PKCE + loopback) + Picker API,
                      built-in wallpaper setters (Windows, GNOME/KDE/XFCE/…, macOS fallback)
uniffi-bindgen/       Generates foreign-language bindings for the core
apps/macos/           SwiftUI menu bar app (links the core as a static lib via UniFFI)
apps/windows/         WinUI 3 (C#), planned. Bindings via uniffi-bindgen-cs
apps/linux/           GTK 4 + libadwaita (Rust, uses the core crate directly), planned
```

Frontends implement one callback interface, `EngineDelegate`:

| method | purpose |
|---|---|
| `set_wallpaper(path) -> bool` | Set the desktop picture natively. Return `false` to use the core's built-in setter. |
| `open_url(url)` | Open the sign-in / picker page in the browser. |
| `state_changed()` | Re-read `status()` / `sources()` and refresh the UI. |

Ratings are platform-specific (they need the OS's models), so frontends compute them and store them through `photos_to_rate()` / `set_scores()`. The core uses them for filtering and weighting.

All other logic is in `Engine`. Long calls (`google_sign_in`, `google_import`, `add_local_folder`, `rescan`) block, so frontends run them off the UI thread. `cancel()` aborts them.

State is stored as JSON in the OS data directory (`~/Library/Application Support/com.alexwohlbruck.Pinwheel`, `%APPDATA%\alexwohlbruck\Pinwheel`, `~/.local/share/pinwheel`):
`settings.json`, `library.json`, `google_token.json` (mode 0600), `cache/google/`.

## Google OAuth client

1. In [Google Cloud Console](https://console.cloud.google.com/), enable the **Photos Picker API**.
2. Create an OAuth client of type **Desktop app**.
3. Provide it in one of these ways (first match wins):
   - env vars `PINWHEEL_GOOGLE_CLIENT_ID` / `PINWHEEL_GOOGLE_CLIENT_SECRET` at runtime
   - the app's Settings → Google → "Use your own OAuth client" (saved as `google_client.json`)
   - the same env vars at **build** time, to bake it into a release

The only scope requested is `photospicker.mediaitems.readonly`.

## Building

**Core + tests** (any OS):
```sh
cargo test -p pinwheel-core
```

**macOS** (Xcode 16+, macOS 15+, `brew install xcodegen`):
```sh
cd apps/macos
xcodegen generate
open Pinwheel.xcodeproj   # the pre-build step compiles the Rust core
```
To build a universal binary, add the Intel target with `rustup target add x86_64-apple-darwin`.

## Development

| env var / flag | effect |
|---|---|
| `PINWHEEL_DATA_DIR=/path` | Use a scratch library instead of the real one |
| `PINWHEEL_DRY_RUN=1` | macOS app: go through every action without changing the real desktop |
| `--show-library` | Open the library window at launch |
| `--show-menu` (Debug builds) | Show the menu bar panel in a normal window, for when the status item is hidden behind the notch |

The core ships a small CLI for poking at a library without a UI:
```sh
export PINWHEEL_DATA_DIR=/tmp/pinwheel
cargo run -p pinwheel-core --example pinwheel -- add-folder ~/Pictures
cargo run -p pinwheel-core --example pinwheel -- next        # prints the pick; add --apply to set it
cargo run -p pinwheel-core --example pinwheel -- status
cargo run -p pinwheel-core --example pinwheel -- sign-in     # then: import
```

## License

[MIT](LICENSE)
