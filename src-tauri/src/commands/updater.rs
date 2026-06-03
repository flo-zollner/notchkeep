//! Desktop updater commands with a runtime-selectable manifest endpoint.
//!
//! The official `@tauri-apps/plugin-updater` JS `check()` can only use the
//! endpoint baked into `tauri.conf.json` — it cannot switch endpoints at
//! runtime. To support per-channel endpoints (stable vs beta) we drive the
//! official Rust updater directly via `updater_builder().endpoints(...)`. This
//! keeps the official download/verify/install machinery and the signature
//! pubkey from `tauri.conf.json`; we only override which manifest URL to read.
//!
//! Android uses the separate `apk-updater` plugin (different install flow), so
//! these commands are desktop-only.

use std::sync::Mutex;

use tauri::ipc::Channel;
use tauri_plugin_updater::{Update, UpdaterExt};

/// Holds the update returned by [`updater_check`] until [`updater_download_install`]
/// consumes it (the official `Update` is not serializable, so it can't round-trip
/// to JS and back — it lives in managed state instead).
#[derive(Default)]
pub struct PendingUpdate(pub Mutex<Option<Update>>);

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub version: String,
    pub notes: String,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub downloaded: u64,
    pub total: u64,
}

/// Checks `endpoint` (chosen per release channel by the frontend) for an update.
/// Returns `None` when up to date. Stores the pending `Update` for installation.
#[tauri::command]
pub async fn updater_check<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    state: tauri::State<'_, PendingUpdate>,
    endpoint: String,
) -> Result<Option<UpdateInfo>, String> {
    let url = endpoint
        .parse::<reqwest::Url>()
        .map_err(|e| format!("invalid updater endpoint: {e}"))?;
    let updater = app
        .updater_builder()
        .endpoints(vec![url])
        .map_err(|e| e.to_string())?
        .build()
        .map_err(|e| e.to_string())?;

    match updater.check().await {
        Ok(Some(update)) => {
            let info = UpdateInfo {
                version: update.version.clone(),
                notes: update.body.clone().unwrap_or_default(),
            };
            *state.0.lock().unwrap() = Some(update);
            Ok(Some(info))
        }
        Ok(None) => {
            *state.0.lock().unwrap() = None;
            Ok(None)
        }
        Err(e) => Err(e.to_string()),
    }
}

/// Downloads and installs the update stored by [`updater_check`], reporting byte
/// progress via `on_event`. Errors if no pending update exists.
#[tauri::command]
pub async fn updater_download_install(
    state: tauri::State<'_, PendingUpdate>,
    on_event: Channel<DownloadProgress>,
) -> Result<(), String> {
    // Take ownership out of the mutex so the lock guard is not held across the
    // await below (holding a std Mutex guard across .await is unsound).
    let update = state
        .0
        .lock()
        .unwrap()
        .take()
        .ok_or_else(|| "no pending update — call updater_check first".to_string())?;

    let mut downloaded: u64 = 0;
    update
        .download_and_install(
            move |chunk, total| {
                downloaded += chunk as u64;
                let _ = on_event.send(DownloadProgress {
                    downloaded,
                    total: total.unwrap_or(0),
                });
            },
            || {},
        )
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}
