use tauri::State;

use crate::commands::accounts::{CommandError, DbState};
use crate::db::portfolio::{self as db_portfolio, RefreshReport};
use crate::db::prices as db_prices;
use crate::pricing_provider::CombinedProvider;

pub struct ProviderState(pub Box<dyn CombinedProvider + Send + Sync>);

#[tauri::command]
pub async fn refresh_prices(
    db_state: State<'_, DbState>,
    provider_state: State<'_, ProviderState>,
) -> Result<RefreshReport, CommandError> {
    Ok(db_portfolio::refresh_all_prices(&db_state.pool(), &*provider_state.0).await?)
}

#[tauri::command]
pub async fn set_manual_price(
    state: State<'_, DbState>,
    security_id: i64,
    date: String,
    price_micro: i64,
) -> Result<(), CommandError> {
    if price_micro < 0 {
        return Err(CommandError { message: "price_micro must be >= 0".into() });
    }
    Ok(db_prices::upsert_price(&state.pool(), security_id, &date, price_micro, "manual").await?)
}

#[tauri::command]
pub async fn get_price_history(
    state: State<'_, DbState>,
    security_id: i64,
) -> Result<Vec<db_prices::PriceRow>, CommandError> {
    let rows: Vec<db_prices::PriceRow> = sqlx::query_as(
        "SELECT security_id, date, close_micro, source
           FROM security_prices
          WHERE security_id = ?1
          ORDER BY date ASC"
    )
    .bind(security_id)
    .fetch_all(&state.pool())
    .await
    .map_err(|e| CommandError { message: e.to_string() })?;
    Ok(rows)
}

#[tauri::command]
pub async fn fetch_security_history(
    db_state: State<'_, DbState>,
    provider_state: State<'_, ProviderState>,
    security_id: i64,
    years: u32,
) -> Result<usize, CommandError> {
    if years == 0 || years > 20 {
        return Err(CommandError {
            message: format!("years must be 1..=20, got {years}"),
        });
    }

    let pool = db_state.pool();

    // Load security to get ISIN + cached symbol.
    let (isin, cached_symbol): (String, Option<String>) = sqlx::query_as(
        "SELECT isin, symbol FROM securities WHERE id = ?1"
    )
    .bind(security_id)
    .fetch_optional(&pool)
    .await
    .map_err(|e| CommandError { message: e.to_string() })?
    .ok_or_else(|| CommandError {
        message: format!("security {security_id} not found"),
    })?;

    // Resolve symbol: use cache if available, else ask provider.
    let provider = &*provider_state.0;
    let symbol = match cached_symbol.filter(|s| !s.is_empty()) {
        Some(s) => s,
        None => match provider.resolve_symbol(&isin).await {
            Ok(Some(sym)) => {
                // Cache it.
                sqlx::query("UPDATE securities SET symbol = ?1 WHERE id = ?2")
                    .bind(&sym)
                    .bind(security_id)
                    .execute(&pool)
                    .await
                    .map_err(|e| CommandError { message: e.to_string() })?;
                sym
            }
            Ok(None) => {
                return Err(CommandError {
                    message: format!("Symbol für ISIN {isin} bei Yahoo nicht gefunden"),
                });
            }
            Err(e) => {
                return Err(CommandError {
                    message: format!("resolve_symbol failed: {e}"),
                });
            }
        },
    };

    let today = chrono::Utc::now().date_naive();
    let from = today - chrono::Duration::days((years as i64) * 365);

    let points = provider.fetch_history(&symbol, from, today).await
        .map_err(|e| CommandError { message: format!("fetch_history: {e}") })?;

    let mut inserted = 0usize;
    for p in &points {
        let d = p.date.format("%Y-%m-%d").to_string();
        if db_prices::upsert_price(&pool, security_id, &d, p.close_micro, "yahoo").await.is_ok() {
            inserted += 1;
        }
    }

    Ok(inserted)
}
