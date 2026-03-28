// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Theme loading, listing, and default-theme scaffolding.
//!
//! Each theme lives in its own subdirectory of `themes_dir`:
//!
//! ```text
//! {themes_dir}/{folder}/
//!   theme.json
//!   css/
//!     main.css      ← main window styles (overrides :root variables)
//!     splash.css    ← splash screen styles (extends main.css)
//!   backgrounds/
//!     bg_main.png   ← optional
//!     bg_splash.png ← optional
//! ```
//!
//! Paths inside `theme.json` are relative to the theme folder and may start
//! with `"./"` (e.g. `"./css/main.css"`).

use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use crate::error::Result;

// ─── Public types ─────────────────────────────────────────────────────────────

/// Manifest file (`theme.json`) for a theme.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeMeta {
    pub name: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "ver_default")]
    pub version: String,
    #[serde(default)]
    pub main: ThemeMain,
    #[serde(default)]
    pub splash: ThemeSplash,
}

fn ver_default() -> String { "1.0.0".into() }

// ── main ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThemeMain {
    #[serde(default)]
    pub buttons: ThemeButtons,
    #[serde(default)]
    pub layout: ThemeLayout,
    #[serde(default)]
    pub backgrounds: ThemeMainBgs,
    #[serde(default)]
    pub css: ThemeMainCss,
}

/// Localised labels for every interactive element in the launcher.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThemeButtons {
    pub play:           Option<String>,
    pub install:        Option<String>,
    pub nav_play:       Option<String>,
    pub nav_settings:   Option<String>,
    pub save_settings:  Option<String>,
    pub detect_java:    Option<String>,
    pub create_profile: Option<String>,
    pub add_offline:    Option<String>,
    pub add_microsoft:  Option<String>,
}

/// Layout hints for the main window.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThemeLayout {
    /// `"left"` (default) or `"right"` — which side the sidebar appears on.
    pub sidebar_position:  Option<String>,
    /// `"stretch"` (default), `"left"`, `"center"`, or `"right"`.
    pub play_button_align: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThemeMainBgs {
    /// Relative path to the main window background image.
    pub main: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThemeMainCss {
    /// Relative path to the main window stylesheet.
    pub main: Option<String>,
}

// ── splash ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThemeSplash {
    #[serde(default)]
    pub texts: Vec<String>,
    #[serde(default)]
    pub backgrounds: ThemeSplashBgs,
    #[serde(default)]
    pub css: ThemeSplashCss,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThemeSplashBgs {
    /// Relative path to the splash screen background image.
    pub splash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThemeSplashCss {
    /// Relative path to the splash screen stylesheet.
    pub splash: Option<String>,
}

// ── transport ─────────────────────────────────────────────────────────────────

/// Full theme payload sent to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeData {
    pub folder_name: String,
    pub meta: ThemeMeta,
    /// Raw CSS for the main window.
    pub main_css: String,
    /// Raw CSS for the splash screen.
    pub splash_css: String,
    /// Optional data-URI for the main background.
    pub main_bg_data_uri: Option<String>,
    /// Optional data-URI for the splash background.
    pub splash_bg_data_uri: Option<String>,
}

/// Lightweight entry for the theme picker list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeEntry {
    pub folder_name: String,
    pub display_name: String,
    pub author: String,
}

// ─── Built-in theme content ───────────────────────────────────────────────────

const DARK_JSON: &str = r#"{
  "name": "Тёмная",
  "author": "XLauchez",
  "description": "Built-in dark theme",
  "version": "1.0.0",
  "main": {
    "buttons": {
      "play":           "Play",
      "install":        "Install",
      "nav_play":       "Play",
      "nav_settings":   "Settings",
      "save_settings":  "Save",
      "detect_java":    "Detect",
      "create_profile": "Create",
      "add_offline":    "+ Offline",
      "add_microsoft":  "+ Microsoft"
    },
    "layout": {
      "sidebar_position":  "left",
      "play_button_align": "stretch"
    },
    "backgrounds": {
      "main": null
    },
    "css": {
      "main": "./css/main.css"
    }
  },
  "splash": {
    "texts": [
      "Mining some code…",
      "Summoning diamonds…",
      "Preparing the Nether…",
      "Loading chunks…",
      "Crafting your world…",
      "Consulting the villagers…",
      "Smelting ores…"
    ],
    "backgrounds": {
      "splash": null
    },
    "css": {
      "splash": "./css/splash.css"
    }
  }
}"#;

const DARK_MAIN_CSS: &str = r#"/* XLauchez — Тёмная */
:root {
  --bg:      #111111;
  --surface: #191919;
  --line:    #242424;
  --accent:  #5dbb2c;
  --ms:      #0078d4;
  --danger:  #c03030;
  --text:    #c8c8c8;
  --dim:     #555555;
}
"#;

const DARK_SPLASH_CSS: &str =
    "/* XLauchez — Тёмная splash (extends main.css) */\n";

const LIGHT_JSON: &str = r#"{
  "name": "Светлая",
  "author": "XLauchez",
  "description": "Built-in light theme",
  "version": "1.0.0",
  "main": {
    "buttons": {
      "play":           "Play",
      "install":        "Install",
      "nav_play":       "Play",
      "nav_settings":   "Settings",
      "save_settings":  "Save",
      "detect_java":    "Detect",
      "create_profile": "Create",
      "add_offline":    "+ Offline",
      "add_microsoft":  "+ Microsoft"
    },
    "layout": {
      "sidebar_position":  "left",
      "play_button_align": "stretch"
    },
    "backgrounds": {
      "main": null
    },
    "css": {
      "main": "./css/main.css"
    }
  },
  "splash": {
    "texts": [
      "Mining some code…",
      "Summoning diamonds…",
      "Preparing the Nether…",
      "Loading chunks…",
      "Crafting your world…",
      "Consulting the villagers…",
      "Smelting ores…"
    ],
    "backgrounds": {
      "splash": null
    },
    "css": {
      "splash": "./css/splash.css"
    }
  }
}"#;

const LIGHT_MAIN_CSS: &str = r#"/* XLauchez — Светлая */
:root {
  --bg:      #f0f0f0;
  --surface: #e6e6e6;
  --line:    #cccccc;
  --accent:  #2d8a00;
  --ms:      #0078d4;
  --danger:  #c03030;
  --text:    #1a1a1a;
  --dim:     #888888;
}
"#;

const LIGHT_SPLASH_CSS: &str =
    "/* XLauchez — Светлая splash (extends main.css) */\n";

// ─── Public API ───────────────────────────────────────────────────────────────

/// Write the two built-in themes to disk if they don't already exist.
pub fn ensure_defaults(themes_dir: &Path) -> Result<()> {
    write_default(themes_dir, "dark",  DARK_JSON,  DARK_MAIN_CSS,  DARK_SPLASH_CSS)?;
    write_default(themes_dir, "light", LIGHT_JSON, LIGHT_MAIN_CSS, LIGHT_SPLASH_CSS)?;
    Ok(())
}

/// List every theme subdirectory that contains a valid `theme.json`.
pub fn list(themes_dir: &Path) -> Vec<ThemeEntry> {
    if !themes_dir.exists() { return Vec::new(); }
    std::fs::read_dir(themes_dir)
        .map(|entries| {
            let mut v: Vec<ThemeEntry> = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .filter_map(|e| {
                    let folder = e.file_name().to_string_lossy().to_string();
                    let meta = read_meta(&e.path()).ok()?;
                    Some(ThemeEntry {
                        folder_name:  folder,
                        display_name: meta.name,
                        author:       meta.author,
                    })
                })
                .collect();
            v.sort_by(|a, b| {
                let rank = |s: &str| match s { "dark" => 0, "light" => 1, _ => 2 };
                rank(&a.folder_name).cmp(&rank(&b.folder_name))
                    .then(a.folder_name.cmp(&b.folder_name))
            });
            v
        })
        .unwrap_or_default()
}

/// Load the full theme data (CSS + optional background images as data-URIs).
pub fn load(themes_dir: &Path, folder_name: &str) -> Result<ThemeData> {
    let theme_dir = themes_dir.join(folder_name);
    let meta = read_meta(&theme_dir)?;

    let main_css   = read_css(&theme_dir, meta.main.css.main.as_deref());
    let splash_css = read_css(&theme_dir, meta.splash.css.splash.as_deref());

    let main_bg_data_uri = meta.main.backgrounds.main.as_ref()
        .and_then(|p| image_to_data_uri(&resolve(&theme_dir, p)).ok());
    let splash_bg_data_uri = meta.splash.backgrounds.splash.as_ref()
        .and_then(|p| image_to_data_uri(&resolve(&theme_dir, p)).ok());

    Ok(ThemeData {
        folder_name: folder_name.to_string(),
        meta,
        main_css,
        splash_css,
        main_bg_data_uri,
        splash_bg_data_uri,
    })
}

// ─── Private helpers ──────────────────────────────────────────────────────────

fn write_default(
    themes_dir: &Path,
    name: &str,
    json: &str,
    main_css: &str,
    splash_css: &str,
) -> Result<()> {
    let dir     = themes_dir.join(name);
    let css_dir = dir.join("css");
    let bg_dir  = dir.join("backgrounds");
    std::fs::create_dir_all(&css_dir)?;
    std::fs::create_dir_all(&bg_dir)?;
    write_if_missing(&dir.join("theme.json"),        json.as_bytes())?;
    write_if_missing(&css_dir.join("main.css"),   main_css.as_bytes())?;
    write_if_missing(&css_dir.join("splash.css"), splash_css.as_bytes())?;
    Ok(())
}

fn write_if_missing(path: &Path, data: &[u8]) -> Result<()> {
    if !path.exists() { std::fs::write(path, data)?; }
    Ok(())
}

fn read_meta(theme_dir: &Path) -> Result<ThemeMeta> {
    let data = std::fs::read_to_string(theme_dir.join("theme.json"))?;
    Ok(serde_json::from_str(&data)?)
}

/// Resolve a theme-relative path (strips leading `"./"` if present).
fn resolve(theme_dir: &Path, relative: &str) -> PathBuf {
    theme_dir.join(relative.strip_prefix("./").unwrap_or(relative))
}

/// Read a CSS file referenced from `theme.json`; returns empty string on any error.
fn read_css(theme_dir: &Path, path: Option<&str>) -> String {
    path.map(|p| {
        let full = resolve(theme_dir, p);
        if full.exists() { std::fs::read_to_string(&full).unwrap_or_default() } else { String::new() }
    }).unwrap_or_default()
}

fn image_to_data_uri(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path)?;
    let mime = match path.extension().and_then(|e| e.to_str()) {
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("png")  => "image/png",
        Some("gif")  => "image/gif",
        Some("webp") => "image/webp",
        _            => "image/png",
    };
    Ok(format!("data:{};base64,{}", mime, b64_encode(&bytes)))
}

/// Minimal Base64 encoder — no external crate needed.
fn b64_encode(data: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let n  = (b0 << 16) | (b1 << 8) | b2;
        out.push(T[((n >> 18) & 63) as usize] as char);
        out.push(T[((n >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 { T[((n >> 6) & 63) as usize] as char } else { '=' });
        out.push(if chunk.len() > 2 { T[( n       & 63) as usize] as char } else { '=' });
    }
    out
}
