use serde::Serialize;
use sqlx::SqlitePool;

use super::DbResult;

#[derive(Debug, Clone, Serialize, sqlx::FromRow, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FxRow {
    pub currency: String,
    pub date: String,
    pub rate_micro: i64,
    pub source: String,
}

pub async fn upsert_rate(
    pool: &SqlitePool,
    currency: &str,
    date: &str,
    rate_micro: i64,
    source: &str,
) -> DbResult<()> {
    let cur = currency.to_uppercase();
    sqlx::query(
        "INSERT INTO fx_rates (currency, date, rate_micro, source)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT (currency, date) DO UPDATE SET
            rate_micro = excluded.rate_micro,
            source = excluded.source",
    )
    .bind(&cur)
    .bind(date)
    .bind(rate_micro)
    .bind(source)
    .execute(pool)
    .await?;
    Ok(())
}

/// Latest FX rate ≤ `on_date` for the foreign currency. EUR always returns 1_000_000.
///
/// Fallback: when no rate ≤ `on_date` exists, the earliest rate
/// AFTER `on_date` is used instead. Background: Yahoo often delivers prices with
/// the market-close date (e.g. Friday) but FX rates with the current refresh date
/// (e.g. Saturday). Without this fallback the caller would fall back to 1.0 and
/// display an HKD amount incorrectly as EUR.
pub async fn rate_on_date(
    pool: &SqlitePool,
    currency: &str,
    on_date: &str,
) -> DbResult<Option<i64>> {
    let cur = currency.to_uppercase();
    if cur == "EUR" {
        return Ok(Some(1_000_000));
    }
    // 1) Preferred: latest rate ≤ on_date
    let row: Option<(i64,)> = sqlx::query_as(
        "SELECT rate_micro FROM fx_rates
          WHERE currency = ?1 AND date <= ?2
          ORDER BY date DESC
          LIMIT 1",
    )
    .bind(&cur)
    .bind(on_date)
    .fetch_optional(pool)
    .await?;
    if let Some((r,)) = row {
        return Ok(Some(r));
    }
    // 2) Fallback: earliest rate > on_date
    let row2: Option<(i64,)> = sqlx::query_as(
        "SELECT rate_micro FROM fx_rates
          WHERE currency = ?1 AND date > ?2
          ORDER BY date ASC
          LIMIT 1",
    )
    .bind(&cur)
    .bind(on_date)
    .fetch_optional(pool)
    .await?;
    Ok(row2.map(|(r,)| r))
}

/// Most recent FX rate. EUR returns None (caller handles it).
pub async fn latest_rate(pool: &SqlitePool, currency: &str) -> DbResult<Option<(String, i64)>> {
    let cur = currency.to_uppercase();
    if cur == "EUR" {
        return Ok(None);
    }
    let row: Option<(String, i64)> = sqlx::query_as(
        "SELECT date, rate_micro FROM fx_rates
          WHERE currency = ?1
          ORDER BY date DESC
          LIMIT 1",
    )
    .bind(&cur)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_memory;

    #[tokio::test]
    async fn upsert_and_read_rate() {
        let pool = connect_memory().await.unwrap();
        upsert_rate(&pool, "USD", "2026-05-19", 909_100, "yahoo").await.unwrap();
        let r = rate_on_date(&pool, "usd", "2026-05-19").await.unwrap();
        assert_eq!(r, Some(909_100));
    }

    #[tokio::test]
    async fn rate_on_date_returns_latest_le() {
        let pool = connect_memory().await.unwrap();
        upsert_rate(&pool, "USD", "2026-03-15", 900_000, "yahoo").await.unwrap();
        upsert_rate(&pool, "USD", "2026-04-15", 910_000, "yahoo").await.unwrap();

        assert_eq!(rate_on_date(&pool, "USD", "2026-04-15").await.unwrap(), Some(910_000));
        assert_eq!(rate_on_date(&pool, "USD", "2026-04-01").await.unwrap(), Some(900_000));
        // Before all rates: falls back to the next one in the future.
        assert_eq!(rate_on_date(&pool, "USD", "2026-03-14").await.unwrap(), Some(900_000));
    }

    /// Real bug: Yahoo often delivers the price from the last market day (e.g.
    /// Friday) and the FX rate from the current day (e.g. Saturday). When the
    /// FX rate therefore lies AFTER the price date, it should still be used —
    /// otherwise the caller falls back to 1.0 and displays the HKD amount
    /// incorrectly as EUR.
    #[tokio::test]
    async fn rate_on_date_falls_back_to_earliest_future_when_no_past_rate() {
        let pool = connect_memory().await.unwrap();
        upsert_rate(&pool, "HKD", "2026-05-23", 110_009, "yahoo").await.unwrap();
        upsert_rate(&pool, "HKD", "2026-06-01", 111_000, "yahoo").await.unwrap();

        // Price date 22.5. — no FX ≤ 22.5. exists → take the earliest
        // rate AFTER (23.5.), not the even later one (1.6.).
        let r = rate_on_date(&pool, "HKD", "2026-05-22").await.unwrap();
        assert_eq!(r, Some(110_009), "earliest-after fallback instead of None");
    }

    #[tokio::test]
    async fn rate_on_date_returns_none_for_unknown_currency() {
        let pool = connect_memory().await.unwrap();
        assert_eq!(rate_on_date(&pool, "XYZ", "2026-05-22").await.unwrap(), None);
    }

    #[tokio::test]
    async fn eur_always_one_million() {
        let pool = connect_memory().await.unwrap();
        assert_eq!(rate_on_date(&pool, "EUR", "2026-05-19").await.unwrap(), Some(1_000_000));
        assert_eq!(rate_on_date(&pool, "eur", "2026-05-19").await.unwrap(), Some(1_000_000));
    }
}
