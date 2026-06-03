use super::DbResult;
use serde::Serialize;
use sqlx::SqlitePool;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct OrphanTradeTx {
    pub id: i64,
    pub kind: String,
    pub booking_date: String,
    pub counterparty: Option<String>,
    pub amount_cents: i64,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct AllocToArchivedBucket {
    pub allocation_id: i64,
    pub security_id: i64,
    pub security_name: String,
    pub bucket_id: i64,
    pub bucket_name: String,
    pub shares_micro: i64,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ZombieSecurity {
    pub id: i64,
    pub isin: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrityReport {
    pub trade_kind_without_trade_row: Vec<OrphanTradeTx>,
    pub allocations_to_archived_buckets: Vec<AllocToArchivedBucket>,
    pub zombie_securities: Vec<ZombieSecurity>,
}

pub async fn find_data_issues(pool: &SqlitePool) -> DbResult<IntegrityReport> {
    let orphans: Vec<OrphanTradeTx> = sqlx::query_as(
        "SELECT t.id, t.kind, t.booking_date, t.counterparty, t.amount_cents
           FROM transactions t
          WHERE t.kind IN ('buy','sell','dividend','corporate_action','tax')
            AND NOT EXISTS (SELECT 1 FROM securities_trades st WHERE st.tx_id = t.id)
          ORDER BY t.booking_date DESC, t.id DESC",
    )
    .fetch_all(pool)
    .await?;

    let archived_allocs: Vec<AllocToArchivedBucket> = sqlx::query_as(
        "SELECT sba.id AS allocation_id, sba.security_id, s.name AS security_name,
                sba.bucket_id, b.name AS bucket_name, sba.shares_micro
           FROM security_bucket_allocations sba
           JOIN securities s ON s.id = sba.security_id
           JOIN buckets b ON b.id = sba.bucket_id
          WHERE b.archived = 1 AND b.deleted_at IS NULL
          ORDER BY s.name",
    )
    .fetch_all(pool)
    .await?;

    let zombies: Vec<ZombieSecurity> = sqlx::query_as(
        "SELECT s.id, s.isin, s.name
           FROM securities s
          WHERE NOT EXISTS (SELECT 1 FROM securities_trades st WHERE st.security_id = s.id)
            AND NOT EXISTS (SELECT 1 FROM security_prices sp WHERE sp.security_id = s.id)
          ORDER BY s.name",
    )
    .fetch_all(pool)
    .await?;

    Ok(IntegrityReport {
        trade_kind_without_trade_row: orphans,
        allocations_to_archived_buckets: archived_allocs,
        zombie_securities: zombies,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_memory;

    #[tokio::test]
    async fn empty_db_returns_empty_report() {
        let pool = connect_memory().await.unwrap();
        let r = find_data_issues(&pool).await.unwrap();
        assert!(r.trade_kind_without_trade_row.is_empty());
        assert!(r.allocations_to_archived_buckets.is_empty());
        assert!(r.zombie_securities.is_empty());
    }

    #[tokio::test]
    async fn detects_buy_tx_without_trade_row() {
        let pool = connect_memory().await.unwrap();
        sqlx::query(
            "INSERT INTO accounts (name, kind, currency) VALUES ('Broker', 'broker', 'EUR')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO transactions (account_id, booking_date, amount_cents, currency, source, imported_at, kind)
             VALUES (1, '2026-05-01', -10000, 'EUR', 'manual', '2026-05-01T00:00:00Z', 'buy')"
        ).execute(&pool).await.unwrap();
        let r = find_data_issues(&pool).await.unwrap();
        assert_eq!(r.trade_kind_without_trade_row.len(), 1);
    }

    #[tokio::test]
    async fn detects_zombie_security() {
        let pool = connect_memory().await.unwrap();
        sqlx::query(
            "INSERT INTO securities (isin, name, currency, asset_type) VALUES ('US0000000000', 'Zombie', 'USD', 'stock')"
        ).execute(&pool).await.unwrap();
        let r = find_data_issues(&pool).await.unwrap();
        assert_eq!(r.zombie_securities.len(), 1);
        assert_eq!(r.zombie_securities[0].name, "Zombie");
    }

    #[tokio::test]
    async fn detects_allocation_to_archived_bucket() {
        let pool = connect_memory().await.unwrap();
        sqlx::query(
            "INSERT INTO securities (isin, name, currency, asset_type) VALUES ('US0000000001', 'Sec', 'USD', 'stock')"
        ).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO buckets (name, archived) VALUES ('Old', 1)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO security_bucket_allocations (security_id, bucket_id, shares_micro) VALUES (1, 1, 1000000)"
        ).execute(&pool).await.unwrap();
        let r = find_data_issues(&pool).await.unwrap();
        assert_eq!(r.allocations_to_archived_buckets.len(), 1);
    }
}
