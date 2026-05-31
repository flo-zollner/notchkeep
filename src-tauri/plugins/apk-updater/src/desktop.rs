use serde::de::DeserializeOwned;
use tauri::{plugin::PluginApi, AppHandle, Runtime};

pub fn init<R: Runtime, C: DeserializeOwned>(
    app: &AppHandle<R>,
    _api: PluginApi<R, C>,
) -> crate::Result<ApkUpdater<R>> {
    Ok(ApkUpdater(app.clone()))
}

/// Access to the apk-updater APIs on desktop (stub — plugin is Android-only).
pub struct ApkUpdater<R: Runtime>(AppHandle<R>);

impl<R: Runtime> ApkUpdater<R> {
    pub fn app_handle(&self) -> &AppHandle<R> {
        &self.0
    }

    /// No-op stub: APK installation is not supported on desktop.
    pub fn install_apk(&self, _path: String) -> crate::Result<()> {
        Ok(())
    }
}
