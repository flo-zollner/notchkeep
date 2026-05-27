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

/// Letzte FX-Rate ≤ `on_date` für die Foreign-Currency. EUR liefert immer 1_000_000.
///
/// Fallback: wenn keine Rate ≤ `on_date` existiert, wird die früheste Rate
/// DANACH genommen. Hintergrund: yahoo liefert Preise oft mit Markt-close-
/// Datum (z. B. Freitag), aber FX-Rates mit dem aktuellen Refresh-Datum
/// (z. B. Samstag). Ohne diesen Fallback würde der Caller auf 1.0 zurück-
/// fallen und einen HKD-Betrag fälschlich als EUR anzeigen.
pub async fn rate_on_date(
    pool: &SqlitePool,
    currency: &str,
    on_date: &str,
) -> DbResult<Option<i64>> {
    let cur = currency.to_uppercase();
    if cur == "EUR" {
        return Ok(Some(1_000_000));
    }
    // 1) Bevorzugt: jüngste Rate ≤ on_date
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
    // 2) Fallback: früheste Rate > on_date
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

/// Aktuellste FX-Rate. EUR liefert None (Caller behandelt).
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
        // Vor allen Raten: greift Fallback auf die nächste in der Zukunft.
        assert_eq!(rate_on_date(&pool, "USD", "2026-03-14").await.unwrap(), Some(900_000));
    }

    /// Realer Bug: yahoo liefert oft den Preis vom letzten Markttag (z.B.
    /// Freitag) und die FX-Rate vom heutigen Tag (z.B. Samstag). Wenn die
    /// FX-Rate also NACH dem Preis-Datum liegt, soll sie trotzdem genommen
    /// werden — sonst fällt der Caller auf 1.0 zurück und zeigt den
    /// HKD-Betrag fälschlich als EUR an.
    #[tokio::test]
    async fn rate_on_date_falls_back_to_earliest_future_when_no_past_rate() {
        let pool = connect_memory().await.unwrap();
        upsert_rate(&pool, "HKD", "2026-05-23", 110_009, "yahoo").await.unwrap();
        upsert_rate(&pool, "HKD", "2026-06-01", 111_000, "yahoo").await.unwrap();

        // Preis-Datum 22.5. — keine FX ≤ 22.5. existiert → nimm die früheste
        // Rate DANACH (23.5.), nicht die noch spätere (1.6.).
        let r = rate_on_date(&pool, "HKD", "2026-05-22").await.unwrap();
        assert_eq!(r, Some(110_009), "earliest-after fallback statt None");
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
