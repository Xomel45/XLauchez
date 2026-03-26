// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;
use crate::error::Result;

const VERSION_MANIFEST_URL: &str =
    "https://launchermeta.mojang.com/mc/game/version_manifest_v2.json";

// ─── Manifest ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionManifest {
    pub latest: LatestVersions,
    pub versions: Vec<VersionEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatestVersions {
    pub release: String,
    pub snapshot: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionEntry {
    pub id: String,
    #[serde(rename = "type")]
    pub version_type: String,
    pub url: String,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
}

// ─── Version meta (per-version JSON) ─────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionMeta {
    pub id: String,
    #[serde(rename = "mainClass")]
    pub main_class: String,
    /// New-style argument format (1.13+).
    pub arguments: Option<GameArguments>,
    /// Legacy argument string (pre-1.13).
    #[serde(rename = "minecraftArguments")]
    pub minecraft_arguments: Option<String>,
    pub libraries: Vec<Library>,
    pub downloads: VersionDownloads,
    #[serde(rename = "assetIndex")]
    pub asset_index: AssetIndexRef,
    pub assets: String,
    #[serde(rename = "javaVersion")]
    pub java_version: Option<JavaVersion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaVersion {
    pub component: String,
    #[serde(rename = "majorVersion")]
    pub major_version: u32,
}

/// Arguments can be plain strings or conditional objects – we keep them as raw
/// JSON values so we can safely ignore conditional entries for now.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameArguments {
    pub game: Vec<serde_json::Value>,
    pub jvm: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionDownloads {
    pub client: DownloadInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadInfo {
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetIndexRef {
    pub id: String,
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

// ─── Libraries ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Library {
    pub name: String,
    pub downloads: Option<LibraryDownloads>,
    pub rules: Option<Vec<LibraryRule>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryDownloads {
    pub artifact: Option<LibraryArtifact>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryArtifact {
    pub path: String,
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryRule {
    pub action: String,
    pub os: Option<OsRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OsRule {
    pub name: Option<String>,
}

// ─── Public helpers ───────────────────────────────────────────────────────────

pub async fn fetch_version_manifest(client: &reqwest::Client) -> Result<VersionManifest> {
    Ok(client
        .get(VERSION_MANIFEST_URL)
        .send()
        .await?
        .json::<VersionManifest>()
        .await?)
}

pub async fn fetch_version_meta(client: &reqwest::Client, url: &str) -> Result<VersionMeta> {
    Ok(client.get(url).send().await?.json::<VersionMeta>().await?)
}

/// Returns version IDs that have a client jar on disk.
pub fn installed_versions(game_dir: &Path) -> Vec<String> {
    let versions_dir = game_dir.join("versions");
    if !versions_dir.exists() {
        return Vec::new();
    }
    std::fs::read_dir(&versions_dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    if version_jar_path(game_dir, &name).exists() {
                        Some(name)
                    } else {
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn version_meta_path(game_dir: &Path, version_id: &str) -> PathBuf {
    game_dir
        .join("versions")
        .join(version_id)
        .join(format!("{version_id}.json"))
}

pub fn version_jar_path(game_dir: &Path, version_id: &str) -> PathBuf {
    game_dir
        .join("versions")
        .join(version_id)
        .join(format!("{version_id}.jar"))
}

pub async fn download_file(client: &reqwest::Client, url: &str, dest: &Path) -> Result<()> {
    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let bytes = client.get(url).send().await?.bytes().await?;
    let mut file = tokio::fs::File::create(dest).await?;
    file.write_all(&bytes).await?;
    Ok(())
}

/// Evaluate OS-based allow/deny rules for a library.
pub fn is_library_allowed(lib: &Library) -> bool {
    let os_name = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "osx"
    } else {
        "linux"
    };

    let rules = match &lib.rules {
        None => return true,
        Some(r) if r.is_empty() => return true,
        Some(r) => r,
    };

    let mut allowed = false;
    for rule in rules {
        let os_matches = match &rule.os {
            None => true,
            Some(os) => os.name.as_deref() == Some(os_name),
        };
        if os_matches {
            allowed = rule.action == "allow";
        }
    }
    allowed
}
