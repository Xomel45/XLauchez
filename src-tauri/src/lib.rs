// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod auth;
mod config;
mod error;
mod minecraft;
mod theme;

use std::sync::Mutex;
use tauri::{AppHandle, Emitter, State};

use config::{Account, Config, Profile};

pub struct AppState {
    config: Mutex<Config>,
    http: reqwest::Client,
}

// ─── Config commands ──────────────────────────────────────────────────────────

#[tauri::command]
async fn config_get(state: State<'_, AppState>) -> std::result::Result<Config, String> {
    Ok(state.config.lock().unwrap().clone())
}

#[tauri::command]
async fn config_update(
    state: State<'_, AppState>,
    new_config: Config,
) -> std::result::Result<(), String> {
    let mut cfg = state.config.lock().unwrap();
    *cfg = new_config;
    cfg.save().map_err(|e| e.to_string())
}

// ─── Profile commands ─────────────────────────────────────────────────────────

#[tauri::command]
async fn profile_list(state: State<'_, AppState>) -> std::result::Result<Vec<Profile>, String> {
    Ok(state.config.lock().unwrap().profiles.clone())
}

#[tauri::command]
async fn profile_create(
    state: State<'_, AppState>,
    name: String,
    version_id: String,
) -> std::result::Result<Profile, String> {
    let profile = Profile {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        version_id,
        jvm_args_override: None,
        max_memory_mb_override: None,
        min_memory_mb_override: None,
    };

    let mut cfg = state.config.lock().unwrap();
    cfg.ensure_profile_dirs(&profile.id).map_err(|e| e.to_string())?;
    if cfg.active_profile_id.is_none() {
        cfg.active_profile_id = Some(profile.id.clone());
    }
    cfg.profiles.push(profile.clone());
    cfg.save().map_err(|e| e.to_string())?;
    Ok(profile)
}

#[tauri::command]
async fn profile_delete(
    state: State<'_, AppState>,
    profile_id: String,
) -> std::result::Result<(), String> {
    let mut cfg = state.config.lock().unwrap();
    cfg.profiles.retain(|p| p.id != profile_id);
    if cfg.active_profile_id.as_deref() == Some(&profile_id) {
        cfg.active_profile_id = cfg.profiles.first().map(|p| p.id.clone());
    }
    cfg.save().map_err(|e| e.to_string())
}

#[tauri::command]
async fn profile_set_active(
    state: State<'_, AppState>,
    profile_id: String,
) -> std::result::Result<(), String> {
    let mut cfg = state.config.lock().unwrap();
    if cfg.profiles.iter().any(|p| p.id == profile_id) {
        cfg.active_profile_id = Some(profile_id);
        cfg.save().map_err(|e| e.to_string())
    } else {
        Err("Profile not found".into())
    }
}

#[tauri::command]
async fn profile_update(
    state: State<'_, AppState>,
    profile: Profile,
) -> std::result::Result<(), String> {
    let mut cfg = state.config.lock().unwrap();
    if let Some(existing) = cfg.profiles.iter_mut().find(|p| p.id == profile.id) {
        *existing = profile;
        cfg.save().map_err(|e| e.to_string())
    } else {
        Err("Profile not found".into())
    }
}

// ─── Account commands ─────────────────────────────────────────────────────────

#[tauri::command]
async fn auth_add_offline(
    state: State<'_, AppState>,
    username: String,
) -> std::result::Result<Account, String> {
    let account = auth::offline::create_offline_account(username);
    let mut cfg = state.config.lock().unwrap();
    if cfg.active_account_id.is_none() {
        cfg.active_account_id = Some(account.id.clone());
    }
    cfg.accounts.push(account.clone());
    cfg.save().map_err(|e| e.to_string())?;
    Ok(account)
}

#[tauri::command]
async fn auth_remove(
    state: State<'_, AppState>,
    account_id: String,
) -> std::result::Result<(), String> {
    let mut cfg = state.config.lock().unwrap();
    cfg.accounts.retain(|a| a.id != account_id);
    if cfg.active_account_id.as_deref() == Some(&account_id) {
        cfg.active_account_id = cfg.accounts.first().map(|a| a.id.clone());
    }
    cfg.save().map_err(|e| e.to_string())
}

#[tauri::command]
async fn auth_set_active(
    state: State<'_, AppState>,
    account_id: String,
) -> std::result::Result<(), String> {
    let mut cfg = state.config.lock().unwrap();
    if cfg.accounts.iter().any(|a| a.id == account_id) {
        cfg.active_account_id = Some(account_id);
        cfg.save().map_err(|e| e.to_string())
    } else {
        Err("Account not found".into())
    }
}

#[tauri::command]
async fn auth_start_microsoft(
    state: State<'_, AppState>,
) -> std::result::Result<auth::microsoft::DeviceCodeResponse, String> {
    auth::microsoft::start_device_code_flow(&state.http)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn auth_poll_microsoft(
    state: State<'_, AppState>,
    device_code: String,
) -> std::result::Result<Option<Account>, String> {
    let result = auth::microsoft::poll_device_code(&state.http, &device_code)
        .await
        .map_err(|e| e.to_string())?;

    if let Some(account) = result {
        let mut cfg = state.config.lock().unwrap();
        if let Some(existing) = cfg.accounts.iter_mut().find(|a| a.id == account.id) {
            *existing = account.clone();
        } else {
            cfg.accounts.push(account.clone());
        }
        if cfg.active_account_id.is_none() {
            cfg.active_account_id = Some(account.id.clone());
        }
        cfg.save().map_err(|e| e.to_string())?;
        Ok(Some(account))
    } else {
        Ok(None)
    }
}

// ─── Version / install commands ───────────────────────────────────────────────

#[tauri::command]
async fn versions_get_manifest(
    state: State<'_, AppState>,
) -> std::result::Result<minecraft::version::VersionManifest, String> {
    minecraft::version::fetch_version_manifest(&state.http)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn versions_get_installed(
    state: State<'_, AppState>,
) -> std::result::Result<Vec<String>, String> {
    let cfg = state.config.lock().unwrap();
    Ok(minecraft::version::installed_versions(&cfg.game_dir))
}

#[tauri::command]
async fn game_install_version(
    app: AppHandle,
    state: State<'_, AppState>,
    version_id: String,
    version_url: String,
) -> std::result::Result<(), String> {
    let game_dir = state.config.lock().unwrap().game_dir.clone();
    let client = state.http.clone();

    emit_progress(&app, "Fetching version metadata…", 0.0);

    let meta = minecraft::version::fetch_version_meta(&client, &version_url)
        .await
        .map_err(|e| e.to_string())?;

    let meta_path = minecraft::version::version_meta_path(&game_dir, &version_id);
    if let Some(parent) = meta_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&meta_path, serde_json::to_string_pretty(&meta).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;

    emit_progress(&app, "Downloading client jar…", 0.05);
    let jar_path = minecraft::version::version_jar_path(&game_dir, &version_id);
    if !jar_path.exists() {
        minecraft::version::download_file(&client, &meta.downloads.client.url, &jar_path)
            .await
            .map_err(|e| e.to_string())?;
    }

    let lib_count = meta.libraries.len() as f32;
    for (i, lib) in meta.libraries.iter().enumerate() {
        if !minecraft::version::is_library_allowed(lib) {
            continue;
        }
        if let Some(dl) = &lib.downloads {
            if let Some(artifact) = &dl.artifact {
                let dest = game_dir.join("libraries").join(&artifact.path);
                if !dest.exists() {
                    minecraft::version::download_file(&client, &artifact.url, &dest)
                        .await
                        .map_err(|e| e.to_string())?;
                }
            }
        }
        emit_progress(
            &app,
            &format!("Libraries ({}/{})", i + 1, lib_count as usize),
            0.1 + 0.5 * (i as f32 / lib_count),
        );
    }

    emit_progress(&app, "Downloading assets…", 0.6);
    minecraft::assets::download_assets(
        &client,
        &game_dir,
        &meta.asset_index.url,
        &meta.asset_index.id,
        |done, total| {
            emit_progress(
                &app,
                &format!("Assets ({done}/{total})"),
                0.6 + 0.38 * (done as f32 / total as f32),
            );
        },
    )
    .await
    .map_err(|e| e.to_string())?;

    emit_progress(&app, "Done!", 1.0);
    Ok(())
}

/// Launch the game using the given profile.
/// Emits: `game_started`, `game_stopped(exit_code)`, `game_error(message)`.
#[tauri::command]
async fn game_launch(
    app: AppHandle,
    state: State<'_, AppState>,
    profile_id: String,
) -> std::result::Result<(), String> {
    let cfg = state.config.lock().unwrap().clone();

    let profile = cfg.profiles
        .iter()
        .find(|p| p.id == profile_id)
        .ok_or("Profile not found")?
        .clone();

    let account = {
        let active_id = cfg.active_account_id.as_deref().ok_or("No active account")?;
        cfg.accounts.iter().find(|a| a.id == active_id)
            .ok_or("Active account not found")?
            .clone()
    };

    let java_path = cfg.java_path
        .as_ref()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| "java".into());

    let version_id = &profile.version_id;
    let meta_path = minecraft::version::version_meta_path(&cfg.game_dir, version_id);
    if !meta_path.exists() {
        return Err(format!("Version {version_id} is not installed."));
    }
    let meta: minecraft::version::VersionMeta =
        serde_json::from_str(&std::fs::read_to_string(&meta_path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;

    // Ensure profile directories + saves symlink exist.
    cfg.ensure_profile_dirs(&profile.id).map_err(|e| e.to_string())?;
    let profile_game_dir = cfg.profile_game_dir(&profile.id);

    let jvm_args = profile.jvm_args_override.as_deref().unwrap_or(&cfg.jvm_args);
    let max_mem = profile.max_memory_mb_override.unwrap_or(cfg.max_memory_mb);
    let min_mem = profile.min_memory_mb_override.unwrap_or(cfg.min_memory_mb);

    let opts = minecraft::launch::LaunchOptions {
        game_dir: &cfg.game_dir,
        profile_game_dir: &profile_game_dir,
        version_id,
        meta: &meta,
        account: &account,
        java_path: &java_path,
        extra_jvm_args: jvm_args,
        max_memory_mb: max_mem,
        min_memory_mb: min_mem,
    };

    let mut cmd = minecraft::launch::build_command(&opts).map_err(|e| e.to_string())?;

    let app_clone = app.clone();
    tokio::spawn(async move {
        match cmd.spawn() {
            Ok(mut child) => {
                let _ = app_clone.emit("game_started", ());
                let exit_code = child.wait().await.map(|s| s.code()).unwrap_or(None);
                let _ = app_clone.emit("game_stopped", exit_code);
            }
            Err(e) => {
                let _ = app_clone.emit("game_error", e.to_string());
            }
        }
    });

    Ok(())
}

// ─── Theme commands ───────────────────────────────────────────────────────────

#[tauri::command]
async fn theme_list(state: State<'_, AppState>) -> std::result::Result<Vec<theme::ThemeEntry>, String> {
    let cfg = state.config.lock().unwrap();
    Ok(theme::list(&cfg.themes_dir()))
}

#[tauri::command]
async fn theme_get_active(state: State<'_, AppState>) -> std::result::Result<theme::ThemeData, String> {
    let cfg = state.config.lock().unwrap();
    theme::load(&cfg.themes_dir(), &cfg.active_theme.clone())
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn theme_set_active(
    state: State<'_, AppState>,
    folder_name: String,
) -> std::result::Result<(), String> {
    let mut cfg = state.config.lock().unwrap();
    cfg.active_theme = folder_name;
    cfg.save().map_err(|e| e.to_string())
}

#[tauri::command]
async fn theme_open_dir(state: State<'_, AppState>) -> std::result::Result<(), String> {
    let cfg = state.config.lock().unwrap();
    let dir = cfg.themes_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    drop(cfg);
    opener::open(dir).map_err(|e| e.to_string())
}

// ─── Java detection ───────────────────────────────────────────────────────────

#[tauri::command]
async fn java_detect() -> std::result::Result<Vec<String>, String> {
    let candidates: Vec<&str> = if cfg!(target_os = "windows") {
        vec![
            "java",
            r"C:\Program Files\Eclipse Adoptium\jdk-21\bin\java.exe",
            r"C:\Program Files\Java\jdk-21\bin\java.exe",
        ]
    } else if cfg!(target_os = "macos") {
        vec![
            "java",
            "/usr/bin/java",
            "/Library/Java/JavaVirtualMachines/temurin-21.jdk/Contents/Home/bin/java",
        ]
    } else {
        vec![
            "java",
            "/usr/bin/java",
            "/usr/lib/jvm/java-21-openjdk/bin/java",
            "/usr/lib/jvm/java-21-openjdk-amd64/bin/java",
            "/usr/lib/jvm/java-17-openjdk/bin/java",
        ]
    };

    let mut found = Vec::new();
    for candidate in candidates {
        if let Ok(out) = std::process::Command::new(candidate).arg("-version").output() {
            if out.status.success() || !out.stderr.is_empty() {
                found.push(candidate.to_string());
            }
        }
    }
    Ok(found)
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn emit_progress(app: &AppHandle, message: &str, progress: f32) {
    let _ = app.emit(
        "install_progress",
        serde_json::json!({ "message": message, "progress": progress }),
    );
}

// ─── Entry point ─────────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let config = Config::load().unwrap_or_default();
    let _ = theme::ensure_defaults(&config.themes_dir());

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {
            config: Mutex::new(config),
            http: reqwest::Client::builder()
                .user_agent(concat!("XLauchez/", env!("CARGO_PKG_VERSION")))
                .build()
                .expect("Failed to build HTTP client"),
        })
        .invoke_handler(tauri::generate_handler![
            config_get,
            config_update,
            profile_list,
            profile_create,
            profile_delete,
            profile_set_active,
            profile_update,
            auth_add_offline,
            auth_remove,
            auth_set_active,
            auth_start_microsoft,
            auth_poll_microsoft,
            versions_get_manifest,
            versions_get_installed,
            game_install_version,
            game_launch,
            java_detect,
            theme_list,
            theme_get_active,
            theme_set_active,
            theme_open_dir,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
