use serde::de::DeserializeOwned;
use tauri::{
    plugin::{PluginApi, PluginHandle},
    AppHandle, Runtime,
};

#[cfg(target_os = "android")]
const PLUGIN_IDENTIFIER: &str = "app.tauri.apkupdater";

// initializes the Kotlin plugin class
pub fn init<R: Runtime, C: DeserializeOwned>(
    _app: &AppHandle<R>,
    api: PluginApi<R, C>,
) -> crate::Result<ApkUpdater<R>> {
    #[cfg(target_os = "android")]
    let handle = api
        .register_android_plugin(PLUGIN_IDENTIFIER, "ApkUpdaterPlugin")
        .map_err(|e| crate::Error::Tauri(tauri::Error::from(e)))?;
    #[cfg(not(target_os = "android"))]
    let handle = {
        let _ = api;
        unimplemented!("apk-updater is Android-only")
    };
    Ok(ApkUpdater(handle))
}

/// Access to the apk-updater APIs on mobile.
pub struct ApkUpdater<R: Runtime>(pub(crate) PluginHandle<R>);

#[derive(serde::Serialize)]
struct InstallPayload {
    path: String,
}

impl<R: Runtime> ApkUpdater<R> {
    pub fn app_handle(&self) -> &AppHandle<R> {
        self.0.app()
    }

    /// Launch the system APK installer for the given file path.
    /// Only meaningful on Android — the FileProvider exposes the cache dir.
    pub fn install_apk(&self, path: String) -> crate::Result<()> {
        self.0
            .run_mobile_plugin("installApk", InstallPayload { path })
            .map_err(|e| crate::Error::Tauri(tauri::Error::from(e)))
    }
}
