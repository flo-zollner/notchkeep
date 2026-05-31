use tauri::Manager as _;

use crate::models::AndroidManifest;
use crate::Result;

#[tauri::command]
pub(crate) async fn check(endpoint: String) -> Result<AndroidManifest> {
    let body = reqwest::get(&endpoint)
        .await?
        .error_for_status()?
        .text()
        .await?;
    let manifest: AndroidManifest = serde_json::from_str(&body)?;
    Ok(manifest)
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub downloaded: u64,
    pub total: u64,
}

#[tauri::command]
pub(crate) async fn download_and_install<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    manifest: AndroidManifest,
    on_event: tauri::ipc::Channel<DownloadProgress>,
) -> Result<()> {
    use futures_util::StreamExt;

    let resp = reqwest::get(&manifest.url).await?.error_for_status()?;
    let total = resp.content_length().unwrap_or(0);
    let mut downloaded = 0u64;
    let mut buf = Vec::new();
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        downloaded += chunk.len() as u64;
        buf.extend_from_slice(&chunk);
        let _ = on_event.send(DownloadProgress { downloaded, total });
    }

    if !crate::verify::verify_sha256(&buf, &manifest.sha256) {
        return Err(crate::Error::Verify("sha256 mismatch".into()));
    }
    crate::verify::verify_minisign(&buf, &manifest.signature, crate::PUBKEY)?;

    let dir = app
        .path()
        .app_cache_dir()
        .map_err(|_| crate::Error::Verify("no cache dir".into()))?;
    std::fs::create_dir_all(&dir).ok();
    let path = dir.join("update.apk");
    std::fs::write(&path, &buf).map_err(|_| crate::Error::Verify("write failed".into()))?;

    #[cfg(target_os = "android")]
    {
        use crate::ApkUpdaterExt as _;
        app.apk_updater()
            .install_apk(path.to_string_lossy().to_string())?;
    }

    Ok(())
}
