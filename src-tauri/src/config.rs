// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use crate::error::{AppError, Result};

// ─── Config ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub game_dir: PathBuf,
    pub java_path: Option<PathBuf>,
    pub jvm_args: Vec<String>,
    pub max_memory_mb: u32,
    pub min_memory_mb: u32,
    pub accounts: Vec<Account>,
    pub active_account_id: Option<String>,
    #[serde(default)]
    pub profiles: Vec<Profile>,
    #[serde(default)]
    pub active_profile_id: Option<String>,
    #[serde(default = "default_theme")]
    pub active_theme: String,
}

// ─── Profile ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub version_id: String,
    /// Overrides global jvm_args when set.
    pub jvm_args_override: Option<Vec<String>>,
    /// Overrides global max_memory_mb when set.
    pub max_memory_mb_override: Option<u32>,
    /// Overrides global min_memory_mb when set.
    pub min_memory_mb_override: Option<u32>,
}

// ─── Account ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    /// UUID from Minecraft profile (Microsoft) or randomly generated (Offline).
    pub id: String,
    pub username: String,
    pub account_type: AccountType,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub xbox_uid: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AccountType {
    Microsoft,
    Offline,
}

// ─── Config impl ──────────────────────────────────────────────────────────────

impl Config {
    pub fn load() -> Result<Self> {
        let path = config_path()?;
        if path.exists() {
            let data = std::fs::read_to_string(&path)?;
            Ok(serde_json::from_str(&data)?)
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = config_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    // ── Directory helpers ─────────────────────────────────────────────────────

    /// Root directory that holds all profile game-dirs.
    pub fn profiles_root(&self) -> PathBuf {
        self.game_dir.join("profiles")
    }

    /// The per-profile `--gameDir` directory (mods, resourcepacks, shaderpacks, …).
    pub fn profile_game_dir(&self, profile_id: &str) -> PathBuf {
        self.profiles_root().join(profile_id)
    }

    /// Shared saves directory (worlds are common to all profiles).
    pub fn shared_saves_dir(&self) -> PathBuf {
        self.game_dir.join("shared").join("saves")
    }

    /// Directory that holds user themes: `{data_dir}/xlauchez/launcher/themes/`
    pub fn themes_dir(&self) -> PathBuf {
        self.game_dir
            .parent()                        // xlauchez/
            .unwrap_or(&self.game_dir)
            .join("launcher")
            .join("themes")
    }

    /// Scaffold a profile directory and symlink `saves/` to the shared location.
    pub fn ensure_profile_dirs(&self, profile_id: &str) -> Result<()> {
        let profile_dir = self.profile_game_dir(profile_id);
        let shared_saves = self.shared_saves_dir();

        // Create profile sub-directories.
        for sub in &["mods", "resourcepacks", "shaderpacks"] {
            std::fs::create_dir_all(profile_dir.join(sub))?;
        }

        // Create shared saves directory.
        std::fs::create_dir_all(&shared_saves)?;

        // Create saves symlink only if it doesn't already exist.
        let saves_link = profile_dir.join("saves");
        if !saves_link.exists() && saves_link.symlink_metadata().is_err() {
            symlink_dir(&shared_saves, &saves_link)?;
        }

        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            game_dir: default_game_dir(),
            java_path: None,
            jvm_args: vec![
                "-XX:+UseG1GC".into(),
                "-XX:+UnlockExperimentalVMOptions".into(),
                "-XX:G1NewSizePercent=20".into(),
                "-XX:MaxGCPauseMillis=50".into(),
            ],
            max_memory_mb: 2048,
            min_memory_mb: 512,
            accounts: Vec::new(),
            active_account_id: None,
            profiles: Vec::new(),
            active_profile_id: None,
            active_theme: "dark".into(),
        }
    }
}

// ─── Symlink helper ───────────────────────────────────────────────────────────

fn symlink_dir(target: &Path, link: &Path) -> Result<()> {
    #[cfg(unix)]
    std::os::unix::fs::symlink(target, link)?;

    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(target, link)?;

    Ok(())
}

// ─── Private helpers ──────────────────────────────────────────────────────────

fn default_theme() -> String { "dark".into() }

fn config_path() -> Result<PathBuf> {
    let dir = dirs::config_dir()
        .ok_or_else(|| AppError::Config("Cannot resolve config directory".into()))?;
    Ok(dir.join("xlauchez").join("config.json"))
}

fn default_game_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("xlauchez")
        .join("minecraft")
}
