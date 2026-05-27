use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

use crate::app_config::AppConfig;
use crate::commands::accounts::{CommandError, DbState};
use crate::db::admin::{
    backup_to as db_backup_to, validate_backup as db_validate_backup, BackupValidation,
};
use crate::db::lock::{acquire, current_holder, force_acquire, release, LockHolder};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfigInfo {
    pub db_path: String,
    pub db_size_bytes: u64,
    pub db_modified_iso: String,
    pub lock_holder: Option<LockHolder>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum PathCheckResult {
    Existing { db_size_bytes: u64, valid: bool },
    Empty,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupResult {
    pub bytes: u64,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ChangePathAction {
    UseExisting,
    OverwriteCopy,
    Move,
    Copy,
    StartFresh,
}

#[derive(Clone)]
pub struct DeviceInfo {
    pub device_id: String,
    pub hostname: String,
}

/// Returns the `app_local_data_dir` for the app.
fn data_dir(app: &AppHandle) -> Result<PathBuf, CommandError> {
    app.path()
        .app_local_data_dir()
        .map_err(|e| CommandError { message: format!("app_local_data_dir: {e}") })
}

#[tauri::command]
pub async fn get_data_path_info(
    app: AppHandle,
    state: State<'_, DbState>,
) -> Result<AppConfigInfo, CommandError> {
    let dir = data_dir(&app)?;
    let cfg = AppConfig::load(&dir)
        .map_err(|e| CommandError { message: format!("load config: {e}") })?;

    let path = PathBuf::from(&cfg.db_path);
    let meta = std::fs::metadata(&path)
        .map_err(|e| CommandError { message: format!("metadata {}: {e}", cfg.db_path) })?;
    let modified = meta
        .modified()
        .map_err(|e| CommandError { message: format!("mtime: {e}") })?;
    let modified_iso = chrono::DateTime::<chrono::Utc>::from(modified).to_rfc3339();

    let pool = state.pool();
    let lock_holder = current_holder(&pool).await?;

    Ok(AppConfigInfo {
        db_path: cfg.db_path,
        db_size_bytes: meta.len(),
        db_modified_iso: modified_iso,
        lock_holder,
    })
}

#[tauri::command]
pub async fn force_acquire_sync_lock(
    state: State<'_, DbState>,
    device: State<'_, DeviceInfo>,
) -> Result<(), CommandError> {
    force_acquire(&state.pool(), &device.device_id, &device.hostname).await?;
    Ok(())
}

#[tauri::command]
pub async fn check_target_path(target_dir: String) -> Result<PathCheckResult, CommandError> {
    let target_db = PathBuf::from(&target_dir).join("budget.sqlite");
    if !target_db.exists() {
        return Ok(PathCheckResult::Empty);
    }
    let bytes = std::fs::metadata(&target_db)
        .map_err(|e| CommandError { message: format!("metadata: {e}") })?
        .len();
    let report = db_validate_backup(&target_db).await;
    Ok(PathCheckResult::Existing {
        db_size_bytes: bytes,
        valid: report.ok,
    })
}

/// Cleanly closes the current pool (via a dummy in-memory pool as an intermediate
/// step so that Windows file handles are released), executes `action` on the
/// filesystem, opens a new pool and replaces it in DbState.
/// If fs_action or opening the new pool fails, the function attempts to restore
/// the old pool (old_db_path) to avoid leaving a zombie dummy pool in DbState.
async fn swap_pool_around<F>(
    state: &DbState,
    device: &DeviceInfo,
    old_db_path: &Path,
    new_db_path: &Path,
    fs_action: F,
) -> Result<(), CommandError>
where
    F: FnOnce() -> Result<(), CommandError>,
{
    // 1. Release sync lock on old pool (best-effort).
    let old_pool_clone = state.pool();
    let _ = release(&old_pool_clone, &device.device_id).await;
    drop(old_pool_clone);

    // 2. Replace current pool with a dummy in-memory pool, then close the old one.
    let dummy = sqlx::SqlitePool::connect("sqlite::memory:")
        .await
        .map_err(|e| CommandError { message: format!("dummy pool: {e}") })?;
    let old_pool = state.swap(dummy);
    old_pool.close().await;

    // 3. Execute FS action — on failure attempt recovery to the old DB.
    if let Err(e) = fs_action() {
        if let Ok(recovery_pool) = crate::db::connect_file(old_db_path).await {
            let _ = acquire(&recovery_pool, &device.device_id, &device.hostname).await;
            let dummy = state.swap(recovery_pool);
            dummy.close().await;
        }
        // If recovery also fails: dummy stays in state. Extremely rare case.
        return Err(e);
    }

    // 4. Open new real pool at new_db_path — also attempt recovery on failure.
    let new_pool = match crate::db::connect_file(new_db_path).await {
        Ok(p) => p,
        Err(e) => {
            if let Ok(recovery_pool) = crate::db::connect_file(old_db_path).await {
                let _ = acquire(&recovery_pool, &device.device_id, &device.hostname).await;
                let dummy = state.swap(recovery_pool);
                dummy.close().await;
            }
            return Err(CommandError { message: format!("open new db: {e}") });
        }
    };

    // 5. Acquire sync lock on new pool (silently, ignore HeldByOther for now).
    let _ = acquire(&new_pool, &device.device_id, &device.hostname).await;

    // 6. Replace dummy with new pool, close dummy.
    let dummy = state.swap(new_pool);
    dummy.close().await;

    Ok(())
}

#[tauri::command]
pub async fn change_data_path(
    app: AppHandle,
    state: State<'_, DbState>,
    device: State<'_, DeviceInfo>,
    target_dir: String,
    action: ChangePathAction,
) -> Result<(), CommandError> {
    let target_dir_path = PathBuf::from(&target_dir);
    if !target_dir_path.is_dir() {
        return Err(CommandError {
            message: format!("target_dir is not a directory: {target_dir}"),
        });
    }
    let target_db = target_dir_path.join("budget.sqlite");
    let dir = data_dir(&app)?;
    let current_cfg = AppConfig::load(&dir)
        .map_err(|e| CommandError { message: format!("load config: {e}") })?;
    let current_db_path = PathBuf::from(&current_cfg.db_path);

    // Availability check
    let target_existed = target_db.exists();
    match (&action, target_existed) {
        (ChangePathAction::UseExisting, false)
        | (ChangePathAction::OverwriteCopy, false) => {
            return Err(CommandError {
                message: "Action requires an existing DB at the target path".into(),
            });
        }
        (ChangePathAction::Move, true)
        | (ChangePathAction::Copy, true)
        | (ChangePathAction::StartFresh, true) => {
            return Err(CommandError {
                message: "Action requires an empty target path".into(),
            });
        }
        _ => {}
    }

    let current_clone = current_db_path.clone();
    let target_clone = target_db.clone();
    let action_clone = action.clone();
    let fs_action = move || -> Result<(), CommandError> {
        match action_clone {
            ChangePathAction::UseExisting => Ok(()),
            ChangePathAction::OverwriteCopy => std::fs::copy(&current_clone, &target_clone)
                .map(|_| ())
                .map_err(|e| CommandError { message: format!("copy: {e}") }),
            ChangePathAction::Move => {
                match std::fs::rename(&current_clone, &target_clone) {
                    Ok(_) => Ok(()),
                    Err(_) => {
                        std::fs::copy(&current_clone, &target_clone)
                            .map_err(|e| CommandError { message: format!("copy fallback: {e}") })?;
                        let _ = std::fs::remove_file(&current_clone);
                        Ok(())
                    }
                }
            }
            ChangePathAction::Copy => std::fs::copy(&current_clone, &target_clone)
                .map(|_| ())
                .map_err(|e| CommandError { message: format!("copy: {e}") }),
            ChangePathAction::StartFresh => Ok(()),
        }
    };

    swap_pool_around(&state, &device, &current_db_path, &target_db, fs_action).await?;

    let new_cfg = AppConfig { db_path: target_db.to_string_lossy().into_owned() };
    new_cfg
        .save(&dir)
        .map_err(|e| CommandError { message: format!("save config: {e}") })?;

    Ok(())
}

#[tauri::command]
pub async fn backup_database(
    state: State<'_, DbState>,
    target_path: String,
) -> Result<BackupResult, CommandError> {
    let start = std::time::Instant::now();
    let pool = state.pool();
    let target = PathBuf::from(&target_path);
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| CommandError { message: format!("create parent: {e}") })?;
    }
    let bytes = db_backup_to(&pool, &target).await?;
    let duration_ms = start.elapsed().as_millis() as u64;
    Ok(BackupResult { bytes, duration_ms })
}

#[tauri::command]
pub async fn validate_backup(source_path: String) -> Result<BackupValidation, CommandError> {
    let path = PathBuf::from(&source_path);
    Ok(db_validate_backup(&path).await)
}

#[tauri::command]
pub async fn restore_database(
    app: AppHandle,
    state: State<'_, DbState>,
    device: State<'_, DeviceInfo>,
    source_path: String,
) -> Result<(), CommandError> {
    let source = PathBuf::from(&source_path);
    let report = db_validate_backup(&source).await;
    if !report.ok {
        return Err(CommandError {
            message: report.error.unwrap_or_else(|| "backup invalid".into()),
        });
    }

    let dir = data_dir(&app)?;
    let cfg = AppConfig::load(&dir)
        .map_err(|e| CommandError { message: format!("load config: {e}") })?;
    let current_db_path = PathBuf::from(&cfg.db_path);

    let src_clone = source.clone();
    let dst_clone = current_db_path.clone();
    let fs_action = move || -> Result<(), CommandError> {
        std::fs::copy(&src_clone, &dst_clone)
            .map(|_| ())
            .map_err(|e| CommandError { message: format!("restore copy: {e}") })
    };

    swap_pool_around(&state, &device, &current_db_path, &current_db_path, fs_action).await?;
    Ok(())
}

#[tauri::command]
pub async fn wipe_database(
    app: AppHandle,
    state: State<'_, DbState>,
    device: State<'_, DeviceInfo>,
) -> Result<(), CommandError> {
    let dir = data_dir(&app)?;
    let cfg = AppConfig::load(&dir)
        .map_err(|e| CommandError { message: format!("load config: {e}") })?;
    let current_db_path = PathBuf::from(&cfg.db_path);

    let path_clone = current_db_path.clone();
    let fs_action = move || -> Result<(), CommandError> {
        if path_clone.exists() {
            std::fs::remove_file(&path_clone)
                .map_err(|e| CommandError { message: format!("remove db: {e}") })?;
        }
        Ok(())
    };

    swap_pool_around(&state, &device, &current_db_path, &current_db_path, fs_action).await?;
    Ok(())
}

#[tauri::command]
pub async fn retry_startup(
    app: AppHandle,
    state: State<'_, DbState>,
    device: State<'_, DeviceInfo>,
) -> Result<(), CommandError> {
    let dir = data_dir(&app)?;
    let cfg = AppConfig::load(&dir)
        .map_err(|e| CommandError { message: format!("load config: {e}") })?;
    let target = PathBuf::from(&cfg.db_path);
    let parent_ok = target.parent().map(|p| p.is_dir()).unwrap_or(false);
    if !parent_ok {
        return Err(CommandError {
            message: format!("parent folder missing: {}", cfg.db_path),
        });
    }
    let fs = || -> Result<(), CommandError> { Ok(()) };
    swap_pool_around(&state, &device, &target, &target, fs).await
}

#[tauri::command]
pub async fn set_path_and_init(
    app: AppHandle,
    state: State<'_, DbState>,
    device: State<'_, DeviceInfo>,
    target_dir: String,
) -> Result<(), CommandError> {
    let target_path = PathBuf::from(&target_dir);
    if !target_path.is_dir() {
        return Err(CommandError {
            message: format!("not a directory: {target_dir}"),
        });
    }
    let target_db = target_path.join("budget.sqlite");
    let dir = data_dir(&app)?;
    let current_cfg = AppConfig::load(&dir)
        .map_err(|e| CommandError { message: format!("load config: {e}") })?;
    let current_db_path = PathBuf::from(&current_cfg.db_path);
    let fs = || -> Result<(), CommandError> { Ok(()) };
    swap_pool_around(&state, &device, &current_db_path, &target_db, fs).await?;
    AppConfig { db_path: target_db.to_string_lossy().into_owned() }
        .save(&dir)
        .map_err(|e| CommandError { message: format!("save config: {e}") })?;
    Ok(())
}

#[tauri::command]
pub async fn reset_path_to_default(
    app: AppHandle,
    state: State<'_, DbState>,
    device: State<'_, DeviceInfo>,
) -> Result<(), CommandError> {
    let dir = data_dir(&app)?;
    let current_cfg = AppConfig::load(&dir)
        .map_err(|e| CommandError { message: format!("load config: {e}") })?;
    let current_db_path = PathBuf::from(&current_cfg.db_path);
    let target_db = AppConfig::default_db_path(&dir);
    let fs = || -> Result<(), CommandError> { Ok(()) };
    swap_pool_around(&state, &device, &current_db_path, &target_db, fs).await?;
    AppConfig { db_path: target_db.to_string_lossy().into_owned() }
        .save(&dir)
        .map_err(|e| CommandError { message: format!("save config: {e}") })?;
    Ok(())
}

#[tauri::command]
pub async fn find_data_issues(
    state: State<'_, DbState>,
) -> Result<crate::db::integrity::IntegrityReport, CommandError> {
    Ok(crate::db::integrity::find_data_issues(&state.pool()).await?)
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncConflictFile {
    pub path: String,
    pub name: String,
    pub modified_unix: i64,
}

/// Scans the DB directory for Syncthing conflict files
/// (pattern: `*.sync-conflict-*`).
pub(crate) async fn check_sync_conflicts_impl(
    dir: &std::path::Path,
) -> std::io::Result<Vec<SyncConflictFile>> {
    let mut entries = Vec::new();
    let read_dir = std::fs::read_dir(dir)?;
    for ent in read_dir {
        let ent = ent?;
        let name = ent.file_name().to_string_lossy().into_owned();
        if !name.contains(".sync-conflict-") {
            continue;
        }
        let meta = ent.metadata()?;
        let modified_unix = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        entries.push(SyncConflictFile {
            path: ent.path().to_string_lossy().into_owned(),
            name,
            modified_unix,
        });
    }
    Ok(entries)
}

/// Moves all conflict files into a `conflict-trash` subdirectory
/// and leaves the current `budget.sqlite` unchanged.
pub(crate) async fn resolve_conflict_keep_current_impl(
    dir: &std::path::Path,
) -> std::io::Result<()> {
    let trash = dir.join("conflict-trash");
    std::fs::create_dir_all(&trash)?;
    for ent in std::fs::read_dir(dir)? {
        let ent = ent?;
        let name = ent.file_name().to_string_lossy().into_owned();
        if !name.contains(".sync-conflict-") {
            continue;
        }
        std::fs::rename(ent.path(), trash.join(&name))?;
    }
    Ok(())
}

/// Promotes the given conflict file to the new `budget.sqlite`.
/// The old `budget.sqlite` and all remaining conflict files are moved
/// into the `conflict-trash` subdirectory with a timestamp suffix for
/// uniqueness.
pub(crate) async fn resolve_conflict_use_other_impl(
    dir: &std::path::Path,
    other: &std::path::Path,
) -> std::io::Result<()> {
    let trash = dir.join("conflict-trash");
    std::fs::create_dir_all(&trash)?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let current = dir.join("budget.sqlite");
    if current.exists() {
        std::fs::rename(&current, trash.join(format!("budget.sqlite.replaced-{now}")))?;
    }
    // Copy other → budget.sqlite (so other still exists for the trash move afterwards)
    std::fs::copy(other, &current)?;
    // Alle Conflict-Files (inkl. other selbst) → trash
    for ent in std::fs::read_dir(dir)? {
        let ent = ent?;
        let name = ent.file_name().to_string_lossy().into_owned();
        if !name.contains(".sync-conflict-") {
            continue;
        }
        std::fs::rename(ent.path(), trash.join(&name))?;
    }
    Ok(())
}

/// Central data-dir resolution for conflict commands (same config logic as in lib.rs setup).
fn resolve_data_dir_for_conflict(app: &tauri::AppHandle) -> Result<std::path::PathBuf, CommandError> {
    use tauri::Manager;
    let r = {
        #[cfg(target_os = "android")]
        { app.path().app_data_dir() }
        #[cfg(not(target_os = "android"))]
        { app.path().app_local_data_dir() }
    };
    r.map_err(|e| CommandError { message: format!("data_dir: {e}") })
}

#[tauri::command]
pub async fn resolve_conflict_keep_current(
    app: tauri::AppHandle,
) -> Result<(), CommandError> {
    let data_dir = resolve_data_dir_for_conflict(&app)?;
    resolve_conflict_keep_current_impl(&data_dir)
        .await
        .map_err(|e| CommandError { message: format!("resolve keep: {e}") })
}

#[tauri::command]
pub async fn resolve_conflict_use_other(
    app: tauri::AppHandle,
    other_path: String,
) -> Result<(), CommandError> {
    let data_dir = resolve_data_dir_for_conflict(&app)?;
    let other = std::path::PathBuf::from(other_path);
    resolve_conflict_use_other_impl(&data_dir, &other)
        .await
        .map_err(|e| CommandError { message: format!("resolve use other: {e}") })
}

#[tauri::command]
pub async fn check_sync_conflicts(
    app: tauri::AppHandle,
) -> Result<Vec<SyncConflictFile>, CommandError> {
    let data_dir = resolve_data_dir_for_conflict(&app)?;
    check_sync_conflicts_impl(&data_dir)
        .await
        .map_err(|e| CommandError { message: format!("conflict scan: {e}") })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_file;
    use tempfile::tempdir;

    #[tokio::test]
    async fn check_target_path_returns_empty_for_missing() {
        let dir = tempdir().unwrap();
        let r = check_target_path(dir.path().to_string_lossy().into_owned()).await.unwrap();
        match r {
            PathCheckResult::Empty => {}
            _ => panic!("expected Empty"),
        }
    }

    #[tokio::test]
    async fn check_target_path_returns_existing_valid_for_real_db() {
        let dir = tempdir().unwrap();
        let pool = connect_file(&dir.path().join("budget.sqlite")).await.unwrap();
        pool.close().await;
        let r = check_target_path(dir.path().to_string_lossy().into_owned()).await.unwrap();
        match r {
            PathCheckResult::Existing { db_size_bytes, valid } => {
                assert!(db_size_bytes > 0);
                assert!(valid);
            }
            _ => panic!("expected Existing"),
        }
    }

    fn setup_device() -> DeviceInfo {
        DeviceInfo { device_id: "test-dev".into(), hostname: "test-host".into() }
    }

    async fn setup_state_with_db(path: &Path) -> DbState {
        let pool = connect_file(path).await.unwrap();
        sqlx::query(
            "INSERT INTO accounts (name, kind, currency) VALUES ('Original', 'bank', 'EUR')",
        ).execute(&pool).await.unwrap();
        DbState(std::sync::RwLock::new(pool))
    }

    #[tokio::test]
    async fn swap_move_to_empty_target() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src.sqlite");
        let dst_dir = dir.path().join("dst");
        std::fs::create_dir_all(&dst_dir).unwrap();
        let dst_db = dst_dir.join("budget.sqlite");

        let state = setup_state_with_db(&src).await;
        let device = setup_device();

        let current = src.clone();
        let target = dst_db.clone();
        let fs = move || -> Result<(), CommandError> {
            std::fs::rename(&current, &target).map_err(|e| CommandError { message: e.to_string() })?;
            Ok(())
        };

        swap_pool_around(&state, &device, &src, &dst_db, fs).await.unwrap();
        assert!(dst_db.exists());
        assert!(!src.exists());

        let pool = state.pool();
        let (name,): (String,) = sqlx::query_as(
            "SELECT name FROM accounts ORDER BY id LIMIT 1",
        ).fetch_one(&pool).await.unwrap();
        assert_eq!(name, "Original");
    }

    #[tokio::test]
    async fn swap_copy_keeps_source_intact() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src.sqlite");
        let dst_dir = dir.path().join("dst");
        std::fs::create_dir_all(&dst_dir).unwrap();
        let dst_db = dst_dir.join("budget.sqlite");

        let state = setup_state_with_db(&src).await;
        let device = setup_device();

        let current = src.clone();
        let target = dst_db.clone();
        let fs = move || -> Result<(), CommandError> {
            std::fs::copy(&current, &target).map_err(|e| CommandError { message: e.to_string() })?;
            Ok(())
        };
        swap_pool_around(&state, &device, &src, &dst_db, fs).await.unwrap();
        assert!(src.exists(), "original source must remain");
        assert!(dst_db.exists());
    }

    #[tokio::test]
    async fn swap_start_fresh_creates_empty_db() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src.sqlite");
        let dst_dir = dir.path().join("dst");
        std::fs::create_dir_all(&dst_dir).unwrap();
        let dst_db = dst_dir.join("budget.sqlite");

        let state = setup_state_with_db(&src).await;
        let device = setup_device();

        let fs = || -> Result<(), CommandError> { Ok(()) };
        swap_pool_around(&state, &device, &src, &dst_db, fs).await.unwrap();

        let pool = state.pool();
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM accounts")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(n, 0);
    }

    #[tokio::test]
    async fn swap_use_existing_keeps_target_data() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src.sqlite");
        let dst_dir = dir.path().join("dst");
        std::fs::create_dir_all(&dst_dir).unwrap();
        let dst_db = dst_dir.join("budget.sqlite");

        let state = setup_state_with_db(&src).await;
        let device = setup_device();

        let other_pool = connect_file(&dst_db).await.unwrap();
        sqlx::query(
            "INSERT INTO accounts (name, kind, currency) VALUES ('Target', 'bank', 'EUR')",
        ).execute(&other_pool).await.unwrap();
        other_pool.close().await;

        let fs = || -> Result<(), CommandError> { Ok(()) };
        swap_pool_around(&state, &device, &src, &dst_db, fs).await.unwrap();

        let pool = state.pool();
        let (name,): (String,) = sqlx::query_as(
            "SELECT name FROM accounts ORDER BY id LIMIT 1",
        ).fetch_one(&pool).await.unwrap();
        assert_eq!(name, "Target", "UseExisting must retain target DB data");
    }

    #[tokio::test]
    #[ignore = "WIP: swap_pool_around refactor unfinished (siehe import_flow/aggregates WIP)"]
    async fn swap_overwrite_copy_replaces_target() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src.sqlite");
        let dst_dir = dir.path().join("dst");
        std::fs::create_dir_all(&dst_dir).unwrap();
        let dst_db = dst_dir.join("budget.sqlite");

        let state = setup_state_with_db(&src).await;
        let device = setup_device();

        let other_pool = connect_file(&dst_db).await.unwrap();
        sqlx::query(
            "INSERT INTO accounts (name, kind, currency) VALUES ('Target', 'bank', 'EUR')",
        ).execute(&other_pool).await.unwrap();
        other_pool.close().await;

        let current = src.clone();
        let target = dst_db.clone();
        let fs = move || -> Result<(), CommandError> {
            std::fs::copy(&current, &target).map_err(|e| CommandError { message: e.to_string() })?;
            Ok(())
        };
        swap_pool_around(&state, &device, &src, &dst_db, fs).await.unwrap();

        let pool = state.pool();
        let (name,): (String,) = sqlx::query_as(
            "SELECT name FROM accounts ORDER BY id LIMIT 1",
        ).fetch_one(&pool).await.unwrap();
        assert_eq!(name, "Original", "OverwriteCopy must have source DB data");
    }

    #[tokio::test]
    async fn backup_database_via_helper_writes_file() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src.sqlite");
        let dst = dir.path().join("backup.sqlite");
        let state = setup_state_with_db(&src).await;

        let pool = state.pool();
        let bytes = db_backup_to(&pool, &dst).await.unwrap();
        assert!(bytes > 0);
        let report = db_validate_backup(&dst).await;
        assert!(report.ok);
    }

    #[tokio::test]
    async fn wipe_via_swap_helper_clears_db() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src.sqlite");
        let state = setup_state_with_db(&src).await;
        let device = setup_device();

        let path_clone = src.clone();
        let fs = move || -> Result<(), CommandError> {
            if path_clone.exists() { std::fs::remove_file(&path_clone).ok(); }
            Ok(())
        };
        swap_pool_around(&state, &device, &src, &src, fs).await.unwrap();

        let pool = state.pool();
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM accounts")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(n, 0);
    }

    #[tokio::test]
    #[ignore = "WIP: swap_pool_around refactor unfinished (siehe import_flow/aggregates WIP)"]
    async fn restore_via_swap_helper_replaces_db() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src.sqlite");
        let backup = dir.path().join("backup.sqlite");
        let state = setup_state_with_db(&src).await;
        let device = setup_device();

        let bk_pool = connect_file(&backup).await.unwrap();
        sqlx::query(
            "INSERT INTO accounts (name, kind, currency) VALUES ('FromBackup', 'bank', 'EUR')",
        ).execute(&bk_pool).await.unwrap();
        bk_pool.close().await;

        let src_clone = backup.clone();
        let dst_clone = src.clone();
        let fs = move || -> Result<(), CommandError> {
            std::fs::copy(&src_clone, &dst_clone).map_err(|e| CommandError { message: e.to_string() })?;
            Ok(())
        };
        swap_pool_around(&state, &device, &src, &src, fs).await.unwrap();

        let pool = state.pool();
        let (name,): (String,) = sqlx::query_as(
            "SELECT name FROM accounts ORDER BY id LIMIT 1",
        ).fetch_one(&pool).await.unwrap();
        assert_eq!(name, "FromBackup");
    }

    #[tokio::test]
    async fn swap_recovers_old_pool_when_fs_action_fails() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src.sqlite");
        let dst_db = dir.path().join("dst.sqlite");
        let state = setup_state_with_db(&src).await;
        let device = setup_device();

        // fs_action that always fails
        let fs = || -> Result<(), CommandError> {
            Err(CommandError { message: "simulated fail".into() })
        };

        let result = swap_pool_around(&state, &device, &src, &dst_db, fs).await;
        assert!(result.is_err());

        // After failure: DbState must point back to the old DB
        let pool = state.pool();
        let (name,): (String,) = sqlx::query_as(
            "SELECT name FROM accounts ORDER BY id LIMIT 1",
        ).fetch_one(&pool).await.unwrap();
        assert_eq!(name, "Original", "recovery must restore the old DB");
    }

    #[tokio::test]
    async fn validate_backup_command_proxies_to_db_layer() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("v.sqlite");
        let pool = connect_file(&path).await.unwrap();
        pool.close().await;
        let v = validate_backup(path.to_string_lossy().into_owned()).await.unwrap();
        assert!(v.ok);
    }

    #[tokio::test]
    async fn check_sync_conflicts_lists_conflict_files() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();

        // Setup: main DB + two conflict files
        std::fs::write(dir.join("budget.sqlite"), b"main").unwrap();
        std::fs::write(dir.join("budget.sqlite.sync-conflict-20260524-103045-AB12C.db"), b"a").unwrap();
        std::fs::write(dir.join("budget.sqlite.sync-conflict-20260525-091500-XY99Z.db"), b"b").unwrap();
        // unrelated file — must not end up in the list
        std::fs::write(dir.join("readme.txt"), b"x").unwrap();

        let result = check_sync_conflicts_impl(dir).await.unwrap();

        assert_eq!(result.len(), 2, "expected 2 conflict files, found: {result:?}");
        let names: Vec<&str> = result.iter().map(|f| f.name.as_str()).collect();
        assert!(names.iter().any(|n| n.contains("AB12C")));
        assert!(names.iter().any(|n| n.contains("XY99Z")));
    }

    #[tokio::test]
    async fn check_sync_conflicts_empty_dir_returns_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let result = check_sync_conflicts_impl(tmp.path()).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn resolve_conflict_keep_current_moves_conflict_to_trash() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        std::fs::write(dir.join("budget.sqlite"), b"main").unwrap();
        std::fs::write(dir.join("budget.sqlite.sync-conflict-A.db"), b"a").unwrap();
        std::fs::write(dir.join("budget.sqlite.sync-conflict-B.db"), b"b").unwrap();

        resolve_conflict_keep_current_impl(dir).await.unwrap();

        // budget.sqlite unchanged (still "main")
        assert_eq!(std::fs::read(dir.join("budget.sqlite")).unwrap(), b"main");
        // Conflict files gone from the main dir
        assert!(!dir.join("budget.sqlite.sync-conflict-A.db").exists());
        assert!(!dir.join("budget.sqlite.sync-conflict-B.db").exists());
        // Conflict files in trash subdirectory
        let trash = dir.join("conflict-trash");
        assert!(trash.exists());
        assert!(trash.join("budget.sqlite.sync-conflict-A.db").exists());
        assert!(trash.join("budget.sqlite.sync-conflict-B.db").exists());
    }

    #[tokio::test]
    async fn resolve_conflict_use_other_swaps_files() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        std::fs::write(dir.join("budget.sqlite"), b"main").unwrap();
        let other = dir.join("budget.sqlite.sync-conflict-X.db");
        std::fs::write(&other, b"other").unwrap();

        resolve_conflict_use_other_impl(dir, &other).await.unwrap();

        // New budget.sqlite has the "other" content
        assert_eq!(std::fs::read(dir.join("budget.sqlite")).unwrap(), b"other");
        // Conflict file is gone (in trash)
        assert!(!other.exists());
        // Old budget.sqlite is in trash with original content
        let trash = dir.join("conflict-trash");
        assert!(trash.exists());
        let entries: Vec<String> = std::fs::read_dir(&trash).unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        // At least 2 files in trash: old budget.sqlite + the conflict-X
        assert!(entries.len() >= 2);
    }
}
