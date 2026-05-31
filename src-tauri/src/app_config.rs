use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub db_path: String,
}

impl AppConfig {
    /// Returns the default DB path: `<app_local_data_dir>/budget.sqlite`.
    pub fn default_db_path(data_dir: &Path) -> PathBuf {
        data_dir.join("budget.sqlite")
    }

    /// Loads config from `<data_dir>/app-config.json`. If the file is missing,
    /// returns the default built from `default_db_path`.
    pub fn load(data_dir: &Path) -> std::io::Result<Self> {
        let path = data_dir.join("app-config.json");
        if !path.exists() {
            return Ok(Self::default(data_dir));
        }
        let raw = std::fs::read_to_string(&path)?;
        let cfg: Self = serde_json::from_str(&raw)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(cfg)
    }

    /// Saves config to `<data_dir>/app-config.json` (atomic via tmp+rename).
    pub fn save(&self, data_dir: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(data_dir)?;
        let target = data_dir.join("app-config.json");
        let tmp = data_dir.join("app-config.json.tmp");
        let raw = serde_json::to_string_pretty(self)
            .map_err(std::io::Error::other)?;
        std::fs::write(&tmp, raw)?;
        std::fs::rename(&tmp, &target)?;
        Ok(())
    }

    pub fn default(data_dir: &Path) -> Self {
        Self {
            db_path: Self::default_db_path(data_dir)
                .to_string_lossy()
                .into_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn load_returns_default_when_file_missing() {
        let dir = tempdir().unwrap();
        let cfg = AppConfig::load(dir.path()).unwrap();
        assert_eq!(
            cfg.db_path,
            dir.path().join("budget.sqlite").to_string_lossy()
        );
    }

    #[test]
    fn save_then_load_roundtrips() {
        let dir = tempdir().unwrap();
        let cfg = AppConfig {
            db_path: "/custom/path/budget.sqlite".into(),
        };
        cfg.save(dir.path()).unwrap();
        let loaded = AppConfig::load(dir.path()).unwrap();
        assert_eq!(loaded, cfg);
    }

    #[test]
    fn save_overwrites_existing_file() {
        let dir = tempdir().unwrap();
        AppConfig {
            db_path: "/first".into(),
        }
        .save(dir.path())
        .unwrap();
        AppConfig {
            db_path: "/second".into(),
        }
        .save(dir.path())
        .unwrap();
        let loaded = AppConfig::load(dir.path()).unwrap();
        assert_eq!(loaded.db_path, "/second");
    }

    #[test]
    fn load_invalid_json_returns_err() {
        let dir = tempdir().unwrap();
        let bad = dir.path().join("app-config.json");
        std::fs::write(&bad, "not json").unwrap();
        assert!(AppConfig::load(dir.path()).is_err());
    }
}
