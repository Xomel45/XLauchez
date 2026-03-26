// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Microsoft OAuth2 Device Code Flow → Xbox Live → XSTS → Minecraft auth.
//!
//! Flow:
//!  1. Request device code from Microsoft.
//!  2. Show `user_code` + `verification_uri` to the user.
//!  3. Poll `/token` until authorized.
//!  4. Exchange MS token for Xbox Live token.
//!  5. Exchange XBL token for XSTS token.
//!  6. Exchange XSTS token for Minecraft access token.
//!  7. Fetch Minecraft profile.

use serde::{Deserialize, Serialize};
use crate::config::{Account, AccountType};
use crate::error::{AppError, Result};

/// Public Azure client ID used by many open-source Minecraft launchers.
/// Replace with your own registered application for production use.
const CLIENT_ID: &str = "00000000402b5328";

// ─── Request/response types ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
    pub message: String,
}

#[derive(Debug, Deserialize)]
struct MsTokenResponse {
    access_token: String,
    expires_in: u64,
    refresh_token: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct XblResponse {
    #[serde(rename = "Token")]
    token: String,
    #[serde(rename = "DisplayClaims")]
    display_claims: XblDisplayClaims,
}

#[derive(Debug, Deserialize)]
struct XblDisplayClaims {
    xui: Vec<XblXui>,
}

#[derive(Debug, Deserialize)]
struct XblXui {
    uhs: String,
}

#[derive(Debug, Deserialize)]
struct McTokenResponse {
    access_token: String,
}

#[derive(Debug, Deserialize)]
struct McProfileResponse {
    id: String,
    name: String,
}

// ─── Public API ──────────────────────────────────────────────────────────────

/// Step 1 – request a device code from Microsoft.
pub async fn start_device_code_flow(client: &reqwest::Client) -> Result<DeviceCodeResponse> {
    let resp = client
        .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode")
        .form(&[
            ("client_id", CLIENT_ID),
            ("scope", "XboxLive.signin offline_access"),
        ])
        .send()
        .await?
        .json::<DeviceCodeResponse>()
        .await?;
    Ok(resp)
}

/// Step 2 – poll for the token. Returns `None` if still pending, `Some(Account)` on success.
pub async fn poll_device_code(
    client: &reqwest::Client,
    device_code: &str,
) -> Result<Option<Account>> {
    let resp = client
        .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/token")
        .form(&[
            ("client_id", CLIENT_ID),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ("device_code", device_code),
        ])
        .send()
        .await?
        .json::<MsTokenResponse>()
        .await?;

    if let Some(err) = &resp.error {
        return match err.as_str() {
            "authorization_pending" | "slow_down" => Ok(None),
            _ => Err(AppError::Auth(
                resp.error_description.unwrap_or_else(|| err.clone()),
            )),
        };
    }

    let account =
        authenticate_ms_token(client, &resp.access_token, resp.refresh_token).await?;
    Ok(Some(account))
}

/// Refresh a Microsoft account using the stored refresh token.
pub async fn refresh_account(client: &reqwest::Client, account: &Account) -> Result<Account> {
    let refresh_token = account
        .refresh_token
        .as_deref()
        .ok_or_else(|| AppError::Auth("No refresh token stored".into()))?;

    let resp = client
        .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/token")
        .form(&[
            ("client_id", CLIENT_ID),
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("scope", "XboxLive.signin offline_access"),
        ])
        .send()
        .await?
        .json::<MsTokenResponse>()
        .await?;

    if let Some(err) = resp.error {
        return Err(AppError::Auth(resp.error_description.unwrap_or(err)));
    }

    authenticate_ms_token(client, &resp.access_token, resp.refresh_token).await
}

// ─── Internal helpers ────────────────────────────────────────────────────────

async fn authenticate_ms_token(
    client: &reqwest::Client,
    ms_access_token: &str,
    refresh_token: Option<String>,
) -> Result<Account> {
    // Xbox Live
    let xbl: XblResponse = client
        .post("https://user.auth.xboxlive.com/user/authenticate")
        .json(&serde_json::json!({
            "Properties": {
                "AuthMethod": "RPS",
                "SiteName": "user.auth.xboxlive.com",
                "RpsTicket": format!("d={}", ms_access_token)
            },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT"
        }))
        .send()
        .await?
        .json()
        .await?;

    let xbl_token = xbl.token;
    let uhs = xbl
        .display_claims
        .xui
        .into_iter()
        .next()
        .ok_or_else(|| AppError::Auth("Missing UHS in XBL response".into()))?
        .uhs;

    // XSTS
    let xsts: XblResponse = client
        .post("https://xsts.auth.xboxlive.com/xsts/authorize")
        .json(&serde_json::json!({
            "Properties": {
                "SandboxId": "RETAIL",
                "UserTokens": [xbl_token]
            },
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType": "JWT"
        }))
        .send()
        .await?
        .json()
        .await?;

    let xsts_token = xsts.token;

    // Minecraft token
    let mc: McTokenResponse = client
        .post("https://api.minecraftservices.com/authentication/login_with_xbox")
        .json(&serde_json::json!({
            "identityToken": format!("XBL3.0 x={};{}", uhs, xsts_token)
        }))
        .send()
        .await?
        .json()
        .await?;

    // Minecraft profile
    let profile: McProfileResponse = client
        .get("https://api.minecraftservices.com/minecraft/profile")
        .bearer_auth(&mc.access_token)
        .send()
        .await?
        .json()
        .await?;

    Ok(Account {
        id: profile.id,
        username: profile.name,
        account_type: AccountType::Microsoft,
        access_token: Some(mc.access_token),
        refresh_token,
        xbox_uid: None,
    })
}
