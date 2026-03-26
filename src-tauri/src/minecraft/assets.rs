// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::path::Path;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::error::Result;
use super::version::download_file;

const RESOURCES_BASE: &str = "https://resources.download.minecraft.net";

#[derive(Debug, Serialize, Deserialize)]
pub struct AssetIndex {
    pub objects: HashMap<String, AssetObject>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AssetObject {
    pub hash: String,
    pub size: u64,
}

/// Download the asset index JSON and all referenced objects.
/// Already-present files are skipped (checked by path existence).
/// `on_progress(done, total)` is called after each downloaded file.
pub async fn download_assets(
    client: &reqwest::Client,
    game_dir: &Path,
    asset_index_url: &str,
    asset_index_id: &str,
    on_progress: impl Fn(usize, usize),
) -> Result<()> {
    let index: AssetIndex = client
        .get(asset_index_url)
        .send()
        .await?
        .json()
        .await?;

    // Persist the full index so the game can find it.
    // Must be {"objects": {...}} — Minecraft reads the "objects" key directly.
    let index_path = game_dir
        .join("assets")
        .join("indexes")
        .join(format!("{asset_index_id}.json"));
    if let Some(parent) = index_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(&index_path, serde_json::to_string(&index)?).await?;

    // Download each object (skip if already on disk).
    let objects: Vec<_> = index.objects.values().collect();
    let total = objects.len();

    for (i, obj) in objects.iter().enumerate() {
        let prefix = &obj.hash[..2];
        let dest = game_dir
            .join("assets")
            .join("objects")
            .join(prefix)
            .join(&obj.hash);

        if !dest.exists() {
            let url = format!("{RESOURCES_BASE}/{prefix}/{}", obj.hash);
            download_file(client, &url, &dest).await?;
        }

        on_progress(i + 1, total);
    }

    Ok(())
}
