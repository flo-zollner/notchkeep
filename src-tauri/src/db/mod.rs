use std::path::{Path, PathBuf};

use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    SqlitePool,
};

pub mod accounts;
pub mod admin;
pub mod aggregates;
pub mod breakdowns;
pub mod bucket_allocations;
pub mod bucket_rules;
pub mod buckets;
pub mod budgets;
pub mod export;
pub mod fx;
pub mod integrity;
pub mod institutions;
pub mod lock;
pub mod networth;
pub mod portfolio;
pub mod prices;
pub mod recurring;
pub mod rules;
pub mod securities;
pub mod trades;
pub mod transactions;

pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error(transparent)]
    Migrate(#[from] sqlx::migrate::MigrateError),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("decode: {0}")]
    Decode(String),
}

pub type DbResult<T> = Result<T, DbError>;

/// Connects to the SQLite file at `db_path`, creating parent dirs and the file
/// if needed, then runs migrations.
pub async fn connect_file(db_path: &Path) -> DbResult<SqlitePool> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let opts = SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(true)
        .foreign_keys(true)
        // WAL allows concurrent reads during a write — important when the DB
        // lives in a Syncthing folder and multiple read operations run in
        // parallel (charts, lists, refresh).
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        // NORMAL is a good balance with WAL: durable on app-crash, fast.
        .synchronous(sqlx::sqlite::SqliteSynchronous::Normal);
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(opts)
        .await?;
    MIGRATOR.run(&pool).await?;
    Ok(pool)
}

/// In-memory pool for tests.
#[cfg(test)]
pub async fn connect_memory() -> DbResult<SqlitePool> {
    let opts = SqliteConnectOptions::new()
        .in_memory(true)
        .foreign_keys(true);
    // Single connection so all queries hit the same in-memory DB.
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await?;
    MIGRATOR.run(&pool).await?;
    Ok(pool)
}

/// Default location of the production DB file under the OS-specific
/// app-local data directory.
pub fn default_db_path(app_local_data_dir: &Path) -> PathBuf {
    app_local_data_dir.join("budget.sqlite")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn migrations_run_clean_on_memory_db() {
        let pool = connect_memory().await.expect("connect");
        let tables: Vec<(String,)> = sqlx::query_as(
            "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name",
        )
        .fetch_all(&pool)
        .await
        .expect("select tables");
        let names: Vec<&str> = tables.iter().map(|(s,)| s.as_str()).collect();
        for expected in [
            "accounts",
            "buckets",
            "categories",
            "rules",
            "sync_lock",
            "transactions",
        ] {
            assert!(names.contains(&expected), "missing table {expected}: {names:?}");
        }
    }

    #[tokio::test]
    async fn migration_0003_adds_allocations_and_drops_goals() {
        let pool = connect_memory().await.unwrap();

        // bucket_allocations exists
        let (alloc_tbl,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='bucket_allocations'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(alloc_tbl, 1, "bucket_allocations table must exist");

        // goals is gone
        let (goals_tbl,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='goals'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(goals_tbl, 0, "goals table must be dropped");
    }

    #[tokio::test]
    async fn migration_0015_creates_institutions_table_and_account_column() {
        let pool = connect_memory().await.expect("connect");
        let (cnt,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='institutions'",
        )
        .fetch_one(&pool).await.unwrap();
        assert_eq!(cnt, 1);
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT name FROM pragma_table_info('accounts') WHERE name='institution_id'",
        )
        .fetch_all(&pool).await.unwrap();
        assert_eq!(rows.len(), 1);
        // Conditional: empty DB → no TR institution.
        let (inst_cnt,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM institutions")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(inst_cnt, 0);
    }

    #[tokio::test]
    async fn migration_0015_auto_assigns_tr_institution_when_tr_tx_exist() {
        let pool = connect_memory().await.unwrap();
        let acc = crate::db::accounts::create_account(&pool, "TR Verrechnung", "bank", "EUR", None, None, None)
            .await.unwrap();
        sqlx::query(
            "INSERT INTO transactions (account_id, booking_date, amount_cents, currency, counterparty, source)
             VALUES (?1, '2026-05-01', 100, 'EUR', 'x', 'tr_csv')",
        )
        .bind(acc.id).execute(&pool).await.unwrap();

        // Replay the auto-migration SQL (idempotent):
        sqlx::query(
            "INSERT INTO institutions (name, icon, color, country)
             SELECT 'Trade Republic', 'bank', 'oklch(0.55 0.13 230)', 'DE'
             WHERE EXISTS (SELECT 1 FROM transactions WHERE source = 'tr_csv')",
        ).execute(&pool).await.unwrap();
        sqlx::query(
            "UPDATE accounts SET institution_id = (SELECT id FROM institutions WHERE LOWER(name)='trade republic')
             WHERE id IN (SELECT DISTINCT account_id FROM transactions WHERE source='tr_csv')",
        ).execute(&pool).await.unwrap();

        let reloaded = crate::db::accounts::get_account(&pool, acc.id).await.unwrap();
        assert!(reloaded.institution_id.is_some(), "TR account not assigned");
    }

    #[tokio::test]
    async fn migration_0016_adds_securities_trades_account_id() {
        let pool = connect_memory().await.unwrap();
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT name FROM pragma_table_info('securities_trades') WHERE name='account_id'",
        ).fetch_all(&pool).await.unwrap();
        assert_eq!(rows.len(), 1, "securities_trades.account_id missing");
    }

    #[tokio::test]
    async fn migration_0016_backfill_routes_trade_to_broker_in_same_institution() {
        // Setup: institution with settlement account + depot, trade linked to settlement account.
        let pool = connect_memory().await.unwrap();
        sqlx::query("INSERT INTO institutions (name) VALUES ('TR')")
            .execute(&pool).await.unwrap();
        let (inst_id,): (i64,) = sqlx::query_as("SELECT id FROM institutions WHERE name='TR'")
            .fetch_one(&pool).await.unwrap();
        let verrechnung = crate::db::accounts::create_account(&pool, "Verrechnung", "bank", "EUR", None, None, Some(inst_id))
            .await.unwrap();
        let depot = crate::db::accounts::create_account(&pool, "Depot", "broker", "EUR", None, None, Some(inst_id))
            .await.unwrap();
        // Insert security + tx + trade manually (current insert_trade_row does not know account_id)
        sqlx::query(
            "INSERT INTO securities (isin, name, asset_type) VALUES ('LU0000000001', 'TestETF', 'etf_equity')"
        ).execute(&pool).await.unwrap();
        let (sec_id,): (i64,) = sqlx::query_as("SELECT id FROM securities WHERE isin='LU0000000001'")
            .fetch_one(&pool).await.unwrap();
        sqlx::query(
            "INSERT INTO transactions (account_id, booking_date, amount_cents, currency, counterparty, source, kind)
             VALUES (?1, '2026-05-15', -50000, 'EUR', 'TestETF', 'tr_csv', 'buy')"
        ).bind(verrechnung.id).execute(&pool).await.unwrap();
        let (tx_id,): (i64,) = sqlx::query_as("SELECT id FROM transactions WHERE account_id=?1 LIMIT 1")
            .bind(verrechnung.id).fetch_one(&pool).await.unwrap();
        sqlx::query(
            "INSERT INTO securities_trades (tx_id, security_id, side, shares_micro)
             VALUES (?1, ?2, 'buy', 5000000)"
        ).bind(tx_id).bind(sec_id).execute(&pool).await.unwrap();

        // Replay the backfill SQL (idempotent — the migration runs only once on connect)
        sqlx::query(
            "UPDATE securities_trades AS st SET account_id = (
                SELECT broker.id
                  FROM transactions tx
                  JOIN accounts curr   ON curr.id = tx.account_id
                  JOIN accounts broker ON broker.institution_id = curr.institution_id
                                      AND broker.kind = 'broker'
                                      AND broker.id != curr.id
                 WHERE tx.id = st.tx_id
                   AND curr.kind != 'broker'
                   AND curr.institution_id IS NOT NULL
                 LIMIT 1
             )
             WHERE st.account_id IS NULL"
        ).execute(&pool).await.unwrap();

        // Verify: securities_trades.account_id now points to the depot
        let (st_acc,): (Option<i64>,) = sqlx::query_as("SELECT account_id FROM securities_trades WHERE tx_id=?1")
            .bind(tx_id).fetch_one(&pool).await.unwrap();
        assert_eq!(st_acc, Some(depot.id));
    }

    #[tokio::test]
    async fn migration_0016_backfill_skips_when_already_broker() {
        // When the tx is already linked to the broker account, account_id should remain NULL.
        let pool = connect_memory().await.unwrap();
        sqlx::query("INSERT INTO institutions (name) VALUES ('TR')")
            .execute(&pool).await.unwrap();
        let (inst_id,): (i64,) = sqlx::query_as("SELECT id FROM institutions WHERE name='TR'")
            .fetch_one(&pool).await.unwrap();
        let depot = crate::db::accounts::create_account(&pool, "Depot", "broker", "EUR", None, None, Some(inst_id))
            .await.unwrap();
        sqlx::query(
            "INSERT INTO securities (isin, name, asset_type) VALUES ('LU0000000002', 'X', 'etf_equity')"
        ).execute(&pool).await.unwrap();
        let (sec_id,): (i64,) = sqlx::query_as("SELECT id FROM securities WHERE isin='LU0000000002'")
            .fetch_one(&pool).await.unwrap();
        sqlx::query(
            "INSERT INTO transactions (account_id, booking_date, amount_cents, currency, counterparty, source, kind)
             VALUES (?1, '2026-05-15', -50000, 'EUR', 'X', 'tr_csv', 'buy')"
        ).bind(depot.id).execute(&pool).await.unwrap();
        let (tx_id,): (i64,) = sqlx::query_as("SELECT id FROM transactions WHERE account_id=?1 LIMIT 1")
            .bind(depot.id).fetch_one(&pool).await.unwrap();
        sqlx::query(
            "INSERT INTO securities_trades (tx_id, security_id, side, shares_micro)
             VALUES (?1, ?2, 'buy', 5000000)"
        ).bind(tx_id).bind(sec_id).execute(&pool).await.unwrap();

        sqlx::query(
            "UPDATE securities_trades AS st SET account_id = (
                SELECT broker.id
                  FROM transactions tx
                  JOIN accounts curr   ON curr.id = tx.account_id
                  JOIN accounts broker ON broker.institution_id = curr.institution_id
                                      AND broker.kind = 'broker'
                                      AND broker.id != curr.id
                 WHERE tx.id = st.tx_id
                   AND curr.kind != 'broker'
                   AND curr.institution_id IS NOT NULL
                 LIMIT 1
             )
             WHERE st.account_id IS NULL"
        ).execute(&pool).await.unwrap();

        let (st_acc,): (Option<i64>,) = sqlx::query_as("SELECT account_id FROM securities_trades WHERE tx_id=?1")
            .bind(tx_id).fetch_one(&pool).await.unwrap();
        assert_eq!(st_acc, None, "Backfill must not fire when tx is already on the broker account");
    }

    #[tokio::test]
    async fn migration_0017_adds_kest_and_withholding_columns() {
        let pool = connect_memory().await.unwrap();
        for col in ["kest_cents", "withholding_tax_cents"] {
            let rows: Vec<(String,)> = sqlx::query_as(&format!(
                "SELECT name FROM pragma_table_info('securities_trades') WHERE name='{col}'",
            )).fetch_all(&pool).await.unwrap();
            assert_eq!(rows.len(), 1, "column {col} missing");
        }
    }

    #[tokio::test]
    async fn migration_0017_backfills_legacy_tax_cents_into_kest() {
        let pool = connect_memory().await.unwrap();
        let acc = crate::db::accounts::create_account(&pool, "A", "broker", "EUR", None, None, None).await.unwrap();
        sqlx::query("INSERT INTO securities (isin, name, asset_type, currency)
                     VALUES ('IE00BK5BQT80', 'X', 'etf_equity', 'EUR')")
            .execute(&pool).await.unwrap();
        let (sec_id,): (i64,) = sqlx::query_as("SELECT id FROM securities WHERE isin='IE00BK5BQT80'")
            .fetch_one(&pool).await.unwrap();
        sqlx::query(
            "INSERT INTO transactions (account_id, booking_date, amount_cents, currency, counterparty, source, kind)
             VALUES (?1, '2026-05-01', -1000, 'EUR', 'x', 'manual', 'buy')"
        ).bind(acc.id).execute(&pool).await.unwrap();
        let (tx_id,): (i64,) = sqlx::query_as("SELECT id FROM transactions LIMIT 1")
            .fetch_one(&pool).await.unwrap();
        sqlx::query("INSERT INTO securities_trades
                     (tx_id, security_id, side, shares_micro, tax_cents, kest_cents, withholding_tax_cents)
                     VALUES (?1, ?2, 'buy', 1000, 500, 0, 0)")
            .bind(tx_id).bind(sec_id).execute(&pool).await.unwrap();

        sqlx::query("UPDATE securities_trades SET kest_cents = tax_cents WHERE tax_cents > 0 AND kest_cents = 0")
            .execute(&pool).await.unwrap();

        let (kest,): (i64,) = sqlx::query_as("SELECT kest_cents FROM securities_trades WHERE tx_id=?1")
            .bind(tx_id).fetch_one(&pool).await.unwrap();
        assert_eq!(kest, 500);
    }
}
