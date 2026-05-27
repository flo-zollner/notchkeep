use std::path::Path;

use serde::Serialize;
use sqlx::SqlitePool;

use super::{connect_file, DbError, DbResult};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BackupRowCounts {
    pub transactions: i64,
    pub accounts: i64,
    pub categories: i64,
    pub securities: i64,
    pub recurring_payments: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupValidation {
    pub ok: bool,
    pub schema_version: Option<i64>,
    pub row_counts: BackupRowCounts,
    pub error: Option<String>,
}

/// Schreibt einen sauberen Snapshot der aktiven DB nach `target_path`
/// via `VACUUM INTO ?`. Funktioniert auch wenn die aktive DB im WAL-Modus ist.
pub async fn backup_to(pool: &SqlitePool, target_path: &Path) -> DbResult<u64> {
    let target_str = target_path.to_string_lossy().into_owned();
    sqlx::query("VACUUM INTO ?")
        .bind(&target_str)
        .execute(pool)
        .await?;
    let bytes = std::fs::metadata(target_path)
        .map_err(|e| DbError::Decode(format!("backup metadata: {e}")))?
        .len();
    Ok(bytes)
}

/// Öffnet `source_path` readonly, prüft Schema und liefert Row-Counts.
/// Returnt `ok = false` mit `error`-Message wenn keine valid Notchkeep-DB.
pub async fn validate_backup(source_path: &Path) -> BackupValidation {
    let zero_counts = BackupRowCounts {
        transactions: 0,
        accounts: 0,
        categories: 0,
        securities: 0,
        recurring_payments: 0,
    };

    if !source_path.exists() {
        return BackupValidation {
            ok: false,
            schema_version: None,
            row_counts: zero_counts,
            error: Some("Datei nicht gefunden".to_string()),
        };
    }

    let url = format!("sqlite://{}?mode=ro", source_path.to_string_lossy());
    let pool = match sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            return BackupValidation {
                ok: false,
                schema_version: None,
                row_counts: zero_counts,
                error: Some(format!("SQLite-Datei kann nicht geöffnet werden: {e}")),
            };
        }
    };

    let migrations_exists: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='_sqlx_migrations'",
    )
    .fetch_optional(&pool)
    .await
    .unwrap_or(None);

    if migrations_exists.is_none() {
        return BackupValidation {
            ok: false,
            schema_version: None,
            row_counts: zero_counts,
            error: Some("Keine Notchkeep-DB (Tabelle _sqlx_migrations fehlt)".to_string()),
        };
    }

    let schema_version: Option<i64> = sqlx::query_scalar(
        "SELECT COALESCE(MAX(version), 0) FROM _sqlx_migrations",
    )
    .fetch_one(&pool)
    .await
    .ok();

    async fn count(pool: &SqlitePool, table: &str) -> i64 {
        let q = format!("SELECT COUNT(*) FROM {table}");
        sqlx::query_scalar(&q).fetch_one(pool).await.unwrap_or(0)
    }

    let row_counts = BackupRowCounts {
        transactions: count(&pool, "transactions").await,
        accounts: count(&pool, "accounts").await,
        categories: count(&pool, "categories").await,
        securities: count(&pool, "securities").await,
        recurring_payments: count(&pool, "recurring_payments").await,
    };

    BackupValidation {
        ok: true,
        schema_version,
        row_counts,
        error: None,
    }
}

/// Erstellt eine leere DB an `path` (löscht vorhandene). Returnt einen
/// frischen Pool mit gelaufenen Migrations.
///
/// **Wichtig:** Der bisherige Pool (falls vorhanden) muss vor diesem
/// Aufruf geschlossen sein — sonst hält Windows das File-Handle und
/// `fs::remove_file` schlägt fehl.
pub async fn wipe_and_recreate(path: &Path) -> DbResult<SqlitePool> {
    if path.exists() {
        std::fs::remove_file(path)
            .map_err(|e| DbError::Decode(format!("remove old db: {e}")))?;
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| DbError::Decode(format!("create parent: {e}")))?;
    }
    connect_file(path).await
}

/// Kopiert `source_path` über `target_path` und öffnet das Ergebnis als Pool
/// (mit Migrations, falls Backup ältere Version war).
///
/// **Wichtig:** Wie wipe_and_recreate — vorheriger Pool muss geschlossen sein.
pub async fn restore_from(source_path: &Path, target_path: &Path) -> DbResult<SqlitePool> {
    if let Some(parent) = target_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| DbError::Decode(format!("create parent: {e}")))?;
    }
    std::fs::copy(source_path, target_path)
        .map_err(|e| DbError::Decode(format!("copy backup: {e}")))?;
    connect_file(target_path).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_file;
    use tempfile::tempdir;

    #[tokio::test]
    async fn backup_to_writes_a_non_empty_file() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("source.sqlite");
        let dst = dir.path().join("backup.sqlite");
        let pool = connect_file(&src).await.unwrap();
        sqlx::query(
            "INSERT INTO accounts (name, kind, currency) VALUES ('Test', 'bank', 'EUR')",
        )
        .execute(&pool)
        .await
        .unwrap();
        pool.close().await;

        let pool2 = connect_file(&src).await.unwrap();
        let bytes = backup_to(&pool2, &dst).await.unwrap();
        assert!(bytes > 0, "Backup-File muss > 0 bytes haben");
        assert!(dst.exists());
    }

    #[tokio::test]
    async fn validate_backup_ok_for_real_db() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("real.sqlite");
        let _pool = connect_file(&src).await.unwrap();
        _pool.close().await;

        let report = validate_backup(&src).await;
        assert!(report.ok, "valid DB: {:?}", report.error);
        assert_eq!(report.row_counts.transactions, 0);
        assert!(report.schema_version.unwrap_or(-1) >= 1);
    }

    #[tokio::test]
    async fn validate_backup_rejects_missing_file() {
        let dir = tempdir().unwrap();
        let report = validate_backup(&dir.path().join("nope.sqlite")).await;
        assert!(!report.ok);
        assert!(report.error.unwrap().contains("nicht gefunden"));
    }

    #[tokio::test]
    async fn validate_backup_rejects_non_sqlite_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("garbage.sqlite");
        std::fs::write(&path, "this is not a sqlite file").unwrap();
        let report = validate_backup(&path).await;
        assert!(!report.ok);
    }

    #[tokio::test]
    async fn validate_backup_rejects_empty_sqlite_without_migrations_table() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("empty.sqlite");
        let url = format!("sqlite://{}?mode=rwc", path.to_string_lossy());
        let pool = sqlx::SqlitePool::connect(&url).await.unwrap();
        pool.close().await;
        let report = validate_backup(&path).await;
        assert!(!report.ok);
        assert!(report.error.unwrap().contains("_sqlx_migrations"));
    }

    #[tokio::test]
    async fn validate_backup_reports_actual_row_counts() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("seeded.sqlite");
        let pool = connect_file(&src).await.unwrap();
        for i in 0..3 {
            sqlx::query(
                "INSERT INTO accounts (name, kind, currency) VALUES (?1, 'bank', 'EUR')",
            )
            .bind(format!("acc-{i}"))
            .execute(&pool)
            .await
            .unwrap();
        }
        pool.close().await;
        let report = validate_backup(&src).await;
        assert!(report.ok);
        assert_eq!(report.row_counts.accounts, 3);
    }

    #[tokio::test]
    async fn backup_to_then_validate_roundtrips() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("orig.sqlite");
        let dst = dir.path().join("backup.sqlite");
        let pool = connect_file(&src).await.unwrap();
        // Count seeded categories from migrations before inserting
        let base_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM categories")
            .fetch_one(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO categories (parent_id, name) VALUES (NULL, 'Test')",
        )
        .execute(&pool)
        .await
        .unwrap();
        backup_to(&pool, &dst).await.unwrap();
        pool.close().await;
        let report = validate_backup(&dst).await;
        assert!(report.ok);
        assert_eq!(report.row_counts.categories, base_count + 1);
    }

    #[tokio::test]
    async fn wipe_and_recreate_creates_empty_db_when_none_exists() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("new.sqlite");
        let pool = wipe_and_recreate(&path).await.unwrap();
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM accounts")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(count, 0);
        assert!(path.exists());
        pool.close().await;
    }

    #[tokio::test]
    async fn wipe_and_recreate_deletes_existing_data() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("ex.sqlite");
        let pool = connect_file(&path).await.unwrap();
        sqlx::query(
            "INSERT INTO accounts (name, kind, currency) VALUES ('A', 'bank', 'EUR')",
        ).execute(&pool).await.unwrap();
        pool.close().await;

        let pool = wipe_and_recreate(&path).await.unwrap();
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM accounts")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(count, 0);
        pool.close().await;
    }

    #[tokio::test]
    async fn restore_from_overwrites_target() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("backup.sqlite");
        let dst = dir.path().join("target.sqlite");
        let pool = connect_file(&src).await.unwrap();
        sqlx::query(
            "INSERT INTO accounts (name, kind, currency) VALUES ('Backup', 'bank', 'EUR')",
        ).execute(&pool).await.unwrap();
        pool.close().await;

        let pool = connect_file(&dst).await.unwrap();
        sqlx::query(
            "INSERT INTO accounts (name, kind, currency) VALUES ('Old', 'bank', 'EUR')",
        ).execute(&pool).await.unwrap();
        pool.close().await;

        let new_pool = restore_from(&src, &dst).await.unwrap();
        let (name,): (String,) = sqlx::query_as(
            "SELECT name FROM accounts ORDER BY id LIMIT 1",
        ).fetch_one(&new_pool).await.unwrap();
        assert_eq!(name, "Backup");
        new_pool.close().await;
    }

    #[tokio::test]
    async fn restore_from_creates_parent_dir() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src.sqlite");
        let dst = dir.path().join("sub").join("nested").join("dst.sqlite");
        let p = connect_file(&src).await.unwrap();
        p.close().await;
        let new_pool = restore_from(&src, &dst).await.unwrap();
        assert!(dst.exists());
        new_pool.close().await;
    }
}
