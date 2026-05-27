use std::path::PathBuf;

use serde::Serialize;
use tauri::State;

use crate::commands::accounts::{CommandError, DbState};
use crate::db::export::{fetch_export_rows, write_export_csv, ExportFilter};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    pub rows: usize,
    pub bytes: u64,
}

#[tauri::command]
pub async fn export_transactions_csv(
    state: State<'_, DbState>,
    filter: ExportFilter,
    target_path: String,
) -> Result<ExportResult, CommandError> {
    write_export_to_path(&state.pool(), &filter, PathBuf::from(target_path)).await
}

async fn write_export_to_path(
    pool: &sqlx::SqlitePool,
    filter: &ExportFilter,
    target_path: PathBuf,
) -> Result<ExportResult, CommandError> {
    if let (Some(from), Some(to)) = (filter.from, filter.to) {
        if from > to {
            return Err(CommandError {
                message: format!("from ({from}) must be <= to ({to})"),
            });
        }
    }
    let rows = fetch_export_rows(pool, filter).await?;
    let file = std::fs::File::create(&target_path).map_err(|e| CommandError {
        message: format!("could not open {}: {e}", target_path.display()),
    })?;
    write_export_csv(&rows, &file).map_err(|e| CommandError {
        message: format!("could not write csv: {e}"),
    })?;
    let bytes = std::fs::metadata(&target_path)
        .map_err(|e| CommandError {
            message: format!("could not stat {}: {e}", target_path.display()),
        })?
        .len();
    Ok(ExportResult {
        rows: rows.len(),
        bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_memory;
    use chrono::NaiveDate;
    use tempfile::tempdir;

    async fn seed_one(pool: &sqlx::SqlitePool) {
        let (acc_id,): (i64,) = sqlx::query_as(
            "INSERT INTO accounts (name, kind, currency) VALUES ('TR', 'bank', 'EUR') RETURNING id",
        )
        .fetch_one(pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO transactions
                (account_id, booking_date, amount_cents, currency,
                 counterparty, purpose, raw_ref, category_id, source, source_file_hash)
             VALUES (?1, '2026-05-17', -1299, 'EUR', 'REWE', 'Einkauf', NULL, NULL, 'manual', NULL)",
        )
        .bind(acc_id)
        .execute(pool)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn write_export_to_path_roundtrip() {
        let pool = connect_memory().await.unwrap();
        seed_one(&pool).await;

        let dir = tempdir().unwrap();
        let target = dir.path().join("out.csv");
        let result = write_export_to_path(&pool, &ExportFilter::default(), target.clone())
            .await
            .unwrap();

        assert_eq!(result.rows, 1);
        assert!(result.bytes > 0);
        let written = std::fs::read_to_string(&target).unwrap();
        assert!(written.starts_with("id,date,amount,"));
        assert!(written.contains(",-12.99,EUR,TR,"));
        let line_count = written.matches("\r\n").count();
        assert_eq!(line_count, 2); // header + one row
    }

    #[tokio::test]
    async fn write_export_to_path_rejects_from_after_to() {
        let pool = connect_memory().await.unwrap();
        let dir = tempdir().unwrap();
        let filter = ExportFilter {
            from: Some(NaiveDate::from_ymd_opt(2026, 6, 1).unwrap()),
            to: Some(NaiveDate::from_ymd_opt(2026, 5, 1).unwrap()),
            ..Default::default()
        };
        let err = write_export_to_path(&pool, &filter, dir.path().join("x.csv"))
            .await
            .unwrap_err();
        assert!(err.message.contains("must be <="), "got: {}", err.message);
    }

    #[tokio::test]
    async fn write_export_to_path_empty_db_writes_header_only() {
        let pool = connect_memory().await.unwrap();
        let dir = tempdir().unwrap();
        let target = dir.path().join("empty.csv");
        let result = write_export_to_path(&pool, &ExportFilter::default(), target.clone())
            .await
            .unwrap();
        assert_eq!(result.rows, 0);
        let written = std::fs::read_to_string(&target).unwrap();
        assert_eq!(written.matches("\r\n").count(), 1);
    }
}
