//! APK over-the-air update plugin for Tauri Android applications.

use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};

/// Minisign public key for APK signature verification (base64-encoded, same as tauri.conf.json).
pub(crate) const PUBKEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDI2RUUyMTFGODk0OTgyMzEKUldReGdrbUpIeUh1SmdYWjJLcmJqcUtjaUZPaEYvWXFJK2xBUDYzeG5nYVN6NjJ2QUZPUEVnMzkK";

pub mod models;
pub mod verify;

mod commands;
mod error;

#[cfg(desktop)]
mod desktop;
#[cfg(mobile)]
mod mobile;

pub use error::{Error, Result};

#[cfg(desktop)]
use desktop::ApkUpdater;
#[cfg(mobile)]
use mobile::ApkUpdater;

/// Extensions to [`tauri::App`], [`tauri::AppHandle`], etc. to access the apk-updater APIs.
pub trait ApkUpdaterExt<R: Runtime> {
    fn apk_updater(&self) -> &ApkUpdater<R>;
}

impl<R: Runtime, T: Manager<R>> ApkUpdaterExt<R> for T {
    fn apk_updater(&self) -> &ApkUpdater<R> {
        self.state::<ApkUpdater<R>>().inner()
    }
}

/// Initializes the plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("apk-updater")
        .invoke_handler(tauri::generate_handler![
            commands::check,
            commands::download_and_install
        ])
        .setup(|app, api| {
            #[cfg(mobile)]
            let apk_updater = mobile::init(app, api)?;
            #[cfg(desktop)]
            let apk_updater = desktop::init(app, api)?;
            app.manage(apk_updater);
            Ok(())
        })
        .build()
}
