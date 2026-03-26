// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;
use crate::config::{Account, AccountType};
use crate::error::Result;
use super::version::{VersionMeta, is_library_allowed};

pub struct LaunchOptions<'a> {
    /// Shared install dir: versions/, assets/, libraries/.
    pub game_dir: &'a Path,
    /// Per-profile dir passed as `--gameDir`: mods/, resourcepacks/, saves/ symlink, …
    pub profile_game_dir: &'a Path,
    pub version_id: &'a str,
    pub meta: &'a VersionMeta,
    pub account: &'a Account,
    pub java_path: &'a str,
    pub extra_jvm_args: &'a [String],
    pub max_memory_mb: u32,
    pub min_memory_mb: u32,
}

/// Build a `tokio::process::Command` ready to spawn Minecraft.
pub fn build_command(opts: &LaunchOptions<'_>) -> Result<Command> {
    let game_dir = opts.game_dir;
    let meta = opts.meta;
    let version_id = opts.version_id;

    let classpath = build_classpath(game_dir, version_id, meta);

    let mut args: Vec<String> = Vec::new();

    // Memory
    args.push(format!("-Xmx{}m", opts.max_memory_mb));
    args.push(format!("-Xms{}m", opts.min_memory_mb));

    // Extra JVM args from config
    args.extend_from_slice(opts.extra_jvm_args);

    // JVM args from version meta
    if let Some(arguments) = &meta.arguments {
        for arg in &arguments.jvm {
            if let Some(s) = arg.as_str() {
                args.push(substitute_jvm(s, game_dir, version_id, &classpath));
            }
            // Conditional objects (rules) are skipped for simplicity.
        }
    } else {
        // Legacy: manually add the required JVM args.
        args.push(format!(
            "-Djava.library.path={}",
            natives_dir(game_dir, version_id).display()
        ));
        args.push("-cp".into());
        args.push(classpath.clone());
    }

    // Main class
    args.push(meta.main_class.clone());

    // Game args
    args.extend(game_args(opts, &classpath));

    let mut cmd = Command::new(opts.java_path);
    cmd.args(&args)
        .current_dir(game_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    Ok(cmd)
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn build_classpath(game_dir: &Path, version_id: &str, meta: &VersionMeta) -> String {
    let sep = if cfg!(windows) { ";" } else { ":" };
    let mut entries: Vec<String> = Vec::new();

    for lib in &meta.libraries {
        if !is_library_allowed(lib) {
            continue;
        }
        if let Some(dl) = &lib.downloads {
            if let Some(artifact) = &dl.artifact {
                let path = game_dir.join("libraries").join(&artifact.path);
                if path.exists() {
                    entries.push(path.to_string_lossy().into_owned());
                }
            }
        }
    }

    let jar = game_dir
        .join("versions")
        .join(version_id)
        .join(format!("{version_id}.jar"));
    entries.push(jar.to_string_lossy().into_owned());

    entries.join(sep)
}

fn natives_dir(game_dir: &Path, version_id: &str) -> std::path::PathBuf {
    game_dir.join("versions").join(version_id).join("natives")
}

fn substitute_jvm(arg: &str, game_dir: &Path, version_id: &str, classpath: &str) -> String {
    let sep = if cfg!(windows) { ";" } else { ":" };
    arg.replace("${classpath}", classpath)
        .replace(
            "${natives_directory}",
            &natives_dir(game_dir, version_id).to_string_lossy(),
        )
        .replace("${launcher_name}", "XLauchez")
        .replace("${launcher_version}", env!("CARGO_PKG_VERSION"))
        .replace("${classpath_separator}", sep)
}

fn game_args(opts: &LaunchOptions<'_>, _classpath: &str) -> Vec<String> {
    // --gameDir points at the profile directory; assets live in the shared install dir.
    let game_dir_str = opts.profile_game_dir.to_string_lossy();
    let assets_dir = opts.game_dir.join("assets").to_string_lossy().into_owned();
    let asset_index = &opts.meta.asset_index.id;
    let username = &opts.account.username;
    let uuid = &opts.account.id;
    let access_token = opts
        .account
        .access_token
        .as_deref()
        .unwrap_or("offline");
    let user_type = if opts.account.account_type == AccountType::Microsoft {
        "msa"
    } else {
        "legacy"
    };

    let substitute = |s: &str| {
        s.replace("${auth_player_name}", username)
            .replace("${version_name}", opts.version_id)
            .replace("${game_directory}", &game_dir_str)
            .replace("${assets_root}", &assets_dir)
            .replace("${assets_index_name}", asset_index)
            .replace("${auth_uuid}", uuid)
            .replace("${auth_access_token}", access_token)
            .replace("${user_type}", user_type)
            .replace("${version_type}", "release")
            // Modern extras (safe to leave empty for basic use)
            .replace("${clientid}", "")
            .replace("${auth_xuid}", "")
            .replace("${user_properties}", "{}")
    };

    let mut out = Vec::new();

    if let Some(arguments) = &opts.meta.arguments {
        for arg in &arguments.game {
            if let Some(s) = arg.as_str() {
                out.push(substitute(s));
            }
        }
    } else if let Some(mc_args) = &opts.meta.minecraft_arguments {
        out.extend(mc_args.split_whitespace().map(|s| substitute(s)));
    }

    out
}
