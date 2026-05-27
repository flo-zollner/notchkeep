use serde::Serialize;
use sqlx::SqlitePool;

use super::DbResult;

#[derive(Debug, Clone, Serialize, sqlx::FromRow, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PriceRow {
    pub security_id: i64,
    pub date: String,
    pub close_micro: i64,
    pub source: String,
}

/// Upsert eines Tagespreises. `(security_id, date)` ist PK, Konflikt → update.
pub async fn upsert_price(
    pool: &SqlitePool,
    security_id: i64,
    date: &str,
    close_micro: i64,
    source: &str,
) -> DbResult<()> {
    sqlx::query(
        "INSERT INTO security_prices (security_id, date, close_micro, source)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT (security_id, date) DO UPDATE SET
            close_micro = excluded.close_micro,
            source = excluded.source",
    )
    .bind(security_id)
    .bind(date)
    .bind(close_micro)
    .bind(source)
    .execute(pool)
    .await?;
    Ok(())
}

/// Letzter bekannter Preis ≤ `on_date`. None falls keiner existiert.
pub async fn price_on_date(
    pool: &SqlitePool,
    security_id: i64,
    on_date: &str,
) -> DbResult<Option<i64>> {
    let row: Option<(i64,)> = sqlx::query_as(
        "SELECT close_micro FROM security_prices
          WHERE security_id = ?1 AND date <= ?2
          ORDER BY date DESC
          LIMIT 1",
    )
    .bind(security_id)
    .bind(on_date)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|(c,)| c))
}

/// Aktuellster Preis. None falls noch keiner gespeichert.
pub async fn latest_price(
    pool: &SqlitePool,
    security_id: i64,
) -> DbResult<Option<(String, i64)>> {
    let row: Option<(String, i64)> = sqlx::query_as(
        "SELECT date, close_micro FROM security_prices
          WHERE security_id = ?1
          ORDER BY date DESC
          LIMIT 1",
    )
    .bind(security_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Eine Liste (security_id, date, close_micro) für alle Securities mit
/// mindestens einem Eintrag. Für `current_holdings`-Aggregation.
pub async fn latest_per_security(pool: &SqlitePool) -> DbResult<Vec<(i64, String, i64)>> {
    let rows: Vec<(i64, String, i64)> = sqlx::query_as(
        "SELECT sp.security_id, sp.date, sp.close_micro
           FROM security_prices sp
          INNER JOIN (
              SELECT security_id, MAX(date) AS d
                FROM security_prices
               GROUP BY security_id
          ) latest ON latest.security_id = sp.security_id AND latest.d = sp.date",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_memory;

    async fn seed_sec(pool: &SqlitePool, isin: &str) -> i64 {
        let (id,): (i64,) = sqlx::query_as(
            "INSERT INTO securities (isin, name, currency, asset_type)
             VALUES (?1, 'name', 'EUR', 'stock') RETURNING id",
        )
        .bind(isin)
        .fetch_one(pool)
        .await
        .unwrap();
        id
    }

    #[tokio::test]
    async fn upsert_inserts_then_updates() {
        let pool = connect_memory().await.unwrap();
        let sec = seed_sec(&pool, "US0378331005").await;
        upsert_price(&pool, sec, "2026-05-19", 180_500_000, "yahoo").await.unwrap();
        upsert_price(&pool, sec, "2026-05-19", 181_000_000, "yahoo").await.unwrap();

        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM security_prices")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(count, 1, "second upsert updates, not inserts");

        let (close,): (i64,) = sqlx::query_as(
            "SELECT close_micro FROM security_prices WHERE security_id = ? AND date = ?"
        ).bind(sec).bind("2026-05-19").fetch_one(&pool).await.unwrap();
        assert_eq!(close, 181_000_000);
    }

    #[tokio::test]
    async fn price_on_date_finds_latest_le_target() {
        let pool = connect_memory().await.unwrap();
        let sec = seed_sec(&pool, "US0378331005").await;
        upsert_price(&pool, sec, "2026-03-15", 150_000_000, "yahoo").await.unwrap();
        upsert_price(&pool, sec, "2026-04-15", 160_000_000, "yahoo").await.unwrap();

        assert_eq!(price_on_date(&pool, sec, "2026-04-15").await.unwrap(), Some(160_000_000));
        assert_eq!(price_on_date(&pool, sec, "2026-04-01").await.unwrap(), Some(150_000_000));
        assert_eq!(price_on_date(&pool, sec, "2026-03-15").await.unwrap(), Some(150_000_000));
        assert_eq!(price_on_date(&pool, sec, "2026-03-14").await.unwrap(), None);
    }

    #[tokio::test]
    async fn latest_price_returns_newest() {
        let pool = connect_memory().await.unwrap();
        let sec = seed_sec(&pool, "US0378331005").await;
        upsert_price(&pool, sec, "2026-03-15", 150_000_000, "yahoo").await.unwrap();
        upsert_price(&pool, sec, "2026-04-15", 160_000_000, "yahoo").await.unwrap();

        let (date, close) = latest_price(&pool, sec).await.unwrap().unwrap();
        assert_eq!(date, "2026-04-15");
        assert_eq!(close, 160_000_000);
    }

    #[tokio::test]
    async fn latest_per_security_returns_one_row_per_sec() {
        let pool = connect_memory().await.unwrap();
        let s1 = seed_sec(&pool, "US0378331005").await;
        let s2 = seed_sec(&pool, "US5949181045").await;
        upsert_price(&pool, s1, "2026-03-15", 150_000_000, "yahoo").await.unwrap();
        upsert_price(&pool, s1, "2026-04-15", 160_000_000, "yahoo").await.unwrap();
        upsert_price(&pool, s2, "2026-04-10", 300_000_000, "yahoo").await.unwrap();

        let mut all = latest_per_security(&pool).await.unwrap();
        all.sort_by_key(|(id, _, _)| *id);
        assert_eq!(all.len(), 2);
        assert_eq!(all[0], (s1, "2026-04-15".to_string(), 160_000_000));
        assert_eq!(all[1], (s2, "2026-04-10".to_string(), 300_000_000));
    }
}
