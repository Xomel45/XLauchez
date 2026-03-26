// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Theme loading, listing, and default-theme scaffolding.
//!
//! Each theme lives in its own subdirectory of `themes_dir`:
//!   `{themes_dir}/{folder_name}/theme.json`
//!   `{themes_dir}/{folder_name}/{stylesheet}`
//!   `{themes_dir}/{folder_name}/{main_background}`   (optional)
//!   `{themes_dir}/{folder_name}/{splash_background}` (optional)
//!
//! `theme.json` is the manifest; `stylesheet` is a CSS file that overrides
//! `:root` variables.  Background images (if any) are returned as data-URIs.

use std::collections::HashMap;
use std::path::Path;
use serde::{Deserialize, Serialize};
use crate::error::Result;

// ─── Public types ─────────────────────────────────────────────────────────────

/// Manifest file (`theme.json`) for a theme.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeMeta {
    /// Display name shown in the launcher UI.
    pub name: String,
    #[serde(default)]
    pub author: String,
    #[serde(default = "ver_default")]
    pub version: String,
    /// Relative path to the CSS file (from the theme folder).
    pub stylesheet: String,
    /// Relative path to the main window background image, or null.
    #[serde(default)]
    pub main_background: Option<String>,
    /// Relative path to the splash screen background image, or null.
    #[serde(default)]
    pub splash_background: Option<String>,
    /// Random texts displayed on the splash screen.
    #[serde(default)]
    pub splash_texts: Vec<String>,
    /// Overrides for button/navigation labels.
    /// Keys: "play", "install", "nav_play", "nav_settings", "save_settings",
    ///       "detect_java", "create_profile", "add_offline", "add_microsoft".
    #[serde(default)]
    pub labels: HashMap<String, String>,
    /// Layout hints for the UI.
    #[serde(default)]
    pub layout: ThemeLayout,
}

fn ver_default() -> String { "1.0".into() }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThemeLayout {
    /// "left" (default) or "right" — which side the sidebar appears on.
    #[serde(default)]
    pub sidebar_position: Option<String>,
    /// "stretch" (default), "left", "center", "right".
    #[serde(default)]
    pub play_button_align: Option<String>,
}

/// Full theme payload sent to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeData {
    pub folder_name: String,
    pub meta: ThemeMeta,
    /// Raw CSS text to inject into the page.
    pub css: String,
    /// Optional data-URI for the main background.
    pub main_bg_data_uri: Option<String>,
    /// Optional data-URI for the splash background.
    pub splash_bg_data_uri: Option<String>,
}

/// Lightweight entry for the theme list.
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
  "version": "1.0",
  "stylesheet": "theme.css",
  "main_background": null,
  "splash_background": null,
  "splash_texts": [
    "Mining some code…",
    "Summoning diamonds…",
    "Preparing the Nether…",
    "Loading chunks…",
    "Crafting your world…",
    "Consulting the villagers…",
    "Smelting ores…"
  ],
  "labels": {
    "play": "Play",
    "install": "Install",
    "nav_play": "Play",
    "nav_settings": "Settings",
    "save_settings": "Save",
    "detect_java": "Detect",
    "create_profile": "Create",
    "add_offline": "+ Offline",
    "add_microsoft": "+ Microsoft"
  },
  "layout": {}
}"#;

const DARK_CSS: &str = r#"/* XLauchez — Тёмная (built-in) */
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

const LIGHT_JSON: &str = r#"{
  "name": "Светлая",
  "author": "XLauchez",
  "version": "1.0",
  "stylesheet": "theme.css",
  "main_background": null,
  "splash_background": null,
  "splash_texts": [
    "Mining some code…",
    "Summoning diamonds…",
    "Preparing the Nether…",
    "Loading chunks…",
    "Crafting your world…",
    "Consulting the villagers…",
    "Smelting ores…"
  ],
  "labels": {
    "play": "Play",
    "install": "Install",
    "nav_play": "Play",
    "nav_settings": "Settings",
    "save_settings": "Save",
    "detect_java": "Detect",
    "create_profile": "Create",
    "add_offline": "+ Offline",
    "add_microsoft": "+ Microsoft"
  },
  "layout": {}
}"#;

const LIGHT_CSS: &str = r#"/* XLauchez — Светлая (built-in) */
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

// ─── Public API ───────────────────────────────────────────────────────────────

/// Write the two built-in themes to disk if they don't already exist.
pub fn ensure_defaults(themes_dir: &Path) -> Result<()> {
    write_default(themes_dir, "dark",  DARK_JSON,  DARK_CSS)?;
    write_default(themes_dir, "light", LIGHT_JSON, LIGHT_CSS)?;
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
                        folder_name: folder,
                        display_name: meta.name,
                        author: meta.author,
                    })
                })
                .collect();
            // Built-in themes first, then alphabetical.
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

    let css_path = theme_dir.join(&meta.stylesheet);
    let css = if css_path.exists() {
        std::fs::read_to_string(&css_path)?
    } else {
        String::new()
    };

    let main_bg_data_uri = meta.main_background.as_ref()
        .and_then(|p| image_to_data_uri(&theme_dir.join(p)).ok());
    let splash_bg_data_uri = meta.splash_background.as_ref()
        .and_then(|p| image_to_data_uri(&theme_dir.join(p)).ok());

    Ok(ThemeData { folder_name: folder_name.to_string(), meta, css, main_bg_data_uri, splash_bg_data_uri })
}

// ─── Private helpers ──────────────────────────────────────────────────────────

fn write_default(themes_dir: &Path, name: &str, json: &str, css: &str) -> Result<()> {
    let dir = themes_dir.join(name);
    std::fs::create_dir_all(&dir)?;
    write_if_missing(&dir.join("theme.json"), json.as_bytes())?;
    write_if_missing(&dir.join("theme.css"),  css.as_bytes())?;
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
