use serde::Serialize;
use sqlx::SqlitePool;
use tauri::State;

use crate::commands::accounts::{CommandError, DbState};
use crate::commands::prices::ProviderState;
use crate::db::{fx as db_fx, DbResult};
use crate::pricing_provider::FxProvider;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrencyStatus {
    pub code: String,
    pub rate_micro: Option<i64>,
    pub date: Option<String>,
    pub source: Option<String>,
    pub in_use: bool,
}

/// Returns for each currency (from securities.currency ∪ fx_rates) the most
/// recent known rate + date + source. EUR is filtered out (always 1.0,
/// not relevant for user display).
pub(crate) async fn list_currencies_impl(pool: &SqlitePool) -> DbResult<Vec<CurrencyStatus>> {
    let rows: Vec<(String, Option<i64>, Option<String>, Option<String>, i64)> = sqlx::query_as(
        "WITH used AS (
            SELECT DISTINCT UPPER(currency) AS code FROM securities WHERE currency != 'EUR'
         ),
         all_codes AS (
            SELECT code FROM used
            UNION
            SELECT DISTINCT UPPER(currency) AS code FROM fx_rates
         ),
         latest AS (
            SELECT currency, MAX(date) AS max_date FROM fx_rates GROUP BY currency
         )
         SELECT
             a.code,
             fx.rate_micro,
             fx.date,
             fx.source,
             CASE WHEN EXISTS (
                 SELECT 1 FROM securities s WHERE UPPER(s.currency) = a.code
             ) THEN 1 ELSE 0 END AS in_use
         FROM all_codes a
         LEFT JOIN latest l ON l.currency = a.code
         LEFT JOIN fx_rates fx ON fx.currency = a.code AND fx.date = l.max_date
         ORDER BY a.code"
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|(code, rate, date, source, in_use)| CurrencyStatus {
        code,
        rate_micro: rate,
        date,
        source,
        in_use: in_use == 1,
    }).collect())
}

/// Validates a currency code as 3 uppercase letters (ISO 4217 form).
fn validate_code(code: &str) -> Result<String, CommandError> {
    let upper = code.to_uppercase();
    if upper.len() != 3 || !upper.chars().all(|c| c.is_ascii_uppercase()) {
        return Err(CommandError {
            message: format!("currency code must match [A-Z]{{3}}, got {code:?}"),
        });
    }
    Ok(upper)
}

pub(crate) async fn update_currency_rate_impl(
    pool: &SqlitePool,
    code: &str,
    rate_micro: i64,
) -> Result<CurrencyStatus, CommandError> {
    let upper = validate_code(code)?;
    if rate_micro <= 0 {
        return Err(CommandError {
            message: "rate_micro must be > 0".into(),
        });
    }
    let today = chrono::Utc::now().date_naive().format("%Y-%m-%d").to_string();
    db_fx::upsert_rate(pool, &upper, &today, rate_micro, "manual")
        .await
        .map_err(CommandError::from)?;
    let all = list_currencies_impl(pool).await.map_err(CommandError::from)?;
    all.into_iter()
        .find(|c| c.code == upper)
        .ok_or_else(|| CommandError {
            message: format!("currency {upper} not found after update"),
        })
}

#[tauri::command]
pub async fn update_currency_rate(
    state: State<'_, DbState>,
    currency: String,
    rate_micro: i64,
) -> Result<CurrencyStatus, CommandError> {
    update_currency_rate_impl(&state.pool(), &currency, rate_micro).await
}

pub(crate) async fn refresh_currency_rate_impl<P: FxProvider + ?Sized>(
    pool: &SqlitePool,
    provider: &P,
    code: &str,
) -> Result<CurrencyStatus, CommandError> {
    let upper = validate_code(code)?;
    let rate = provider.fetch_eur_rate(&upper).await.map_err(|e| CommandError {
        message: format!("yahoo fx fetch failed for {upper}: {e}"),
    })?;
    let today = chrono::Utc::now().date_naive().format("%Y-%m-%d").to_string();
    db_fx::upsert_rate(pool, &upper, &today, rate, "yahoo")
        .await
        .map_err(CommandError::from)?;
    let all = list_currencies_impl(pool).await.map_err(CommandError::from)?;
    all.into_iter()
        .find(|c| c.code == upper)
        .ok_or_else(|| CommandError {
            message: format!("currency {upper} not found after refresh"),
        })
}

#[tauri::command]
pub async fn refresh_currency_rate(
    state: State<'_, DbState>,
    provider_state: State<'_, ProviderState>,
    currency: String,
) -> Result<CurrencyStatus, CommandError> {
    refresh_currency_rate_impl(&state.pool(), &*provider_state.0, &currency).await
}

#[tauri::command]
pub async fn list_currencies(
    state: State<'_, DbState>,
) -> Result<Vec<CurrencyStatus>, CommandError> {
    Ok(list_currencies_impl(&state.pool()).await?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_memory;

    #[tokio::test]
    async fn list_currencies_returns_active_with_latest_rate() {
        let pool = connect_memory().await.unwrap();
        sqlx::query(
            "INSERT INTO securities (isin, name, currency, asset_type)
             VALUES ('XX0000000001', 'A', 'HKD', 'stock'),
                    ('XX0000000002', 'B', 'USD', 'stock')"
        ).execute(&pool).await.unwrap();
        db_fx::upsert_rate(&pool, "HKD", "2026-05-23", 110_009, "yahoo").await.unwrap();

        let result = list_currencies_impl(&pool).await.unwrap();
        assert_eq!(result.len(), 2);

        let hkd = result.iter().find(|c| c.code == "HKD").unwrap();
        assert_eq!(hkd.rate_micro, Some(110_009));
        assert_eq!(hkd.date.as_deref(), Some("2026-05-23"));
        assert_eq!(hkd.source.as_deref(), Some("yahoo"));
        assert!(hkd.in_use);

        let usd = result.iter().find(|c| c.code == "USD").unwrap();
        assert_eq!(usd.rate_micro, None);
        assert!(usd.in_use);
    }

    #[tokio::test]
    async fn list_currencies_includes_orphan_rate_marked_not_in_use() {
        let pool = connect_memory().await.unwrap();
        db_fx::upsert_rate(&pool, "JPY", "2026-05-22", 6_100, "manual").await.unwrap();

        let result = list_currencies_impl(&pool).await.unwrap();
        let jpy = result.iter().find(|c| c.code == "JPY").unwrap();
        assert_eq!(jpy.rate_micro, Some(6_100));
        assert!(!jpy.in_use);
    }

    #[tokio::test]
    async fn list_currencies_excludes_eur() {
        let pool = connect_memory().await.unwrap();
        sqlx::query(
            "INSERT INTO securities (isin, name, currency, asset_type)
             VALUES ('XX0000000003', 'EUR-Asset', 'EUR', 'etf_equity')"
        ).execute(&pool).await.unwrap();
        let result = list_currencies_impl(&pool).await.unwrap();
        assert!(!result.iter().any(|c| c.code == "EUR"));
    }

    #[tokio::test]
    async fn update_currency_rate_inserts_manual_today() {
        let pool = connect_memory().await.unwrap();
        let result = update_currency_rate_impl(&pool, "HKD", 111_000).await.unwrap();
        assert_eq!(result.code, "HKD");
        assert_eq!(result.rate_micro, Some(111_000));
        assert_eq!(result.source.as_deref(), Some("manual"));
        let date = result.date.expect("date set");
        assert!(date.len() == 10 && date.chars().nth(4) == Some('-'),
            "expected YYYY-MM-DD, got {date}");
    }

    #[tokio::test]
    async fn update_currency_rate_rejects_negative_rate() {
        let pool = connect_memory().await.unwrap();
        let err = update_currency_rate_impl(&pool, "USD", -1).await;
        assert!(err.is_err());
    }

    #[tokio::test]
    async fn update_currency_rate_rejects_zero_rate() {
        let pool = connect_memory().await.unwrap();
        let err = update_currency_rate_impl(&pool, "USD", 0).await;
        assert!(err.is_err());
    }

    #[tokio::test]
    async fn update_currency_rate_rejects_malformed_code() {
        let pool = connect_memory().await.unwrap();
        let cases = ["", "EU", "EURO", "12X", "us"];
        for c in cases {
            let err = update_currency_rate_impl(&pool, c, 1_000_000).await;
            assert!(err.is_err(), "code {c:?} should be rejected");
        }
    }

    #[tokio::test]
    async fn update_currency_rate_normalizes_lowercase() {
        let pool = connect_memory().await.unwrap();
        let result = update_currency_rate_impl(&pool, "hkd", 100_000).await.unwrap();
        assert_eq!(result.code, "HKD", "code should be uppercase-normalized");
    }

    #[tokio::test]
    async fn refresh_currency_rate_with_mock_writes_yahoo_source() {
        use crate::pricing_provider::mock::MockProvider;
        let pool = connect_memory().await.unwrap();
        let mock = MockProvider::new().with_fx("HKD", 110_009);

        let result = refresh_currency_rate_impl(&pool, &mock, "HKD").await.unwrap();
        assert_eq!(result.code, "HKD");
        assert_eq!(result.rate_micro, Some(110_009));
        assert_eq!(result.source.as_deref(), Some("yahoo"));
    }

    #[tokio::test]
    async fn refresh_currency_rate_adds_new_currency() {
        use crate::pricing_provider::mock::MockProvider;
        let pool = connect_memory().await.unwrap();
        let mock = MockProvider::new().with_fx("JPY", 6_100);

        let before = list_currencies_impl(&pool).await.unwrap();
        assert!(!before.iter().any(|c| c.code == "JPY"));

        let result = refresh_currency_rate_impl(&pool, &mock, "JPY").await.unwrap();
        assert_eq!(result.code, "JPY");
        assert_eq!(result.rate_micro, Some(6_100));
    }

    #[tokio::test]
    async fn refresh_currency_rate_propagates_provider_error() {
        use crate::pricing_provider::mock::MockProvider;
        let pool = connect_memory().await.unwrap();
        let mock = MockProvider::new();
        let err = refresh_currency_rate_impl(&pool, &mock, "USD").await;
        assert!(err.is_err());
    }

    #[tokio::test]
    async fn refresh_currency_rate_rejects_malformed_code() {
        use crate::pricing_provider::mock::MockProvider;
        let pool = connect_memory().await.unwrap();
        let mock = MockProvider::new();
        let err = refresh_currency_rate_impl(&pool, &mock, "us").await;
        assert!(err.is_err());
    }
}
