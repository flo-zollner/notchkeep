use chrono::NaiveDate;
use tauri::State;

use crate::commands::accounts::{CommandError, DbState};
use crate::db::trades::{
    self as db_trades, NewTradePayload, TradeWithTx, UpdateTradePayload,
};
use crate::model::SecurityTrade;

const ALLOWED_SIDES: &[&str] = &[
    "buy", "sell", "dividend", "corporate_action", "fusion_out", "fusion_in", "tax",
];

fn validate_side(s: &str) -> Result<(), CommandError> {
    if ALLOWED_SIDES.contains(&s) {
        Ok(())
    } else {
        Err(CommandError {
            message: format!("side must be one of {ALLOWED_SIDES:?}, got {s:?}"),
        })
    }
}

fn validate_booking_date(d: &str) -> Result<(), CommandError> {
    NaiveDate::parse_from_str(d, "%Y-%m-%d").map_err(|e| CommandError {
        message: format!("booking_date must be valid YYYY-MM-DD, got {d:?}: {e}"),
    })?;
    Ok(())
}

#[tauri::command]
pub async fn list_trades(
    state: State<'_, DbState>,
    security_id: Option<i64>,
) -> Result<Vec<TradeWithTx>, CommandError> {
    Ok(db_trades::list_trades(&state.pool(), security_id).await?)
}

#[tauri::command]
pub async fn get_trade(
    state: State<'_, DbState>,
    tx_id: i64,
) -> Result<TradeWithTx, CommandError> {
    Ok(db_trades::get_trade(&state.pool(), tx_id).await?)
}

#[tauri::command]
pub async fn create_trade(
    state: State<'_, DbState>,
    payload: NewTradePayload,
) -> Result<TradeWithTx, CommandError> {
    validate_side(&payload.side)?;
    validate_booking_date(&payload.booking_date)?;
    if payload.fee_cents < 0 {
        return Err(CommandError { message: "fee_cents must be >= 0".into() });
    }
    if payload.kest_cents < 0 || payload.withholding_tax_cents < 0 {
        return Err(CommandError { message: "tax fields must be >= 0".into() });
    }
    match payload.side.as_str() {
        "buy" | "sell" if payload.unit_price_micro.is_none() => {
            return Err(CommandError {
                message: "unit_price_micro required for buy/sell".into(),
            });
        }
        _ => {}
    }
    Ok(db_trades::create_trade(&state.pool(), payload).await?)
}

#[tauri::command]
pub async fn update_trade(
    state: State<'_, DbState>,
    tx_id: i64,
    payload: UpdateTradePayload,
) -> Result<SecurityTrade, CommandError> {
    Ok(db_trades::update_trade(&state.pool(), tx_id, payload).await?)
}

#[tauri::command]
pub async fn delete_trade(
    state: State<'_, DbState>,
    tx_id: i64,
) -> Result<bool, CommandError> {
    Ok(db_trades::delete_trade(&state.pool(), tx_id).await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_side_whitelist() {
        for s in ALLOWED_SIDES { assert!(validate_side(s).is_ok()); }
        assert!(validate_side("transfer").is_err());
        assert!(validate_side("").is_err());
    }

    #[test]
    fn validate_booking_date_iso() {
        assert!(validate_booking_date("2026-05-18").is_ok());
        assert!(validate_booking_date("2026/05/18").is_err());
        assert!(validate_booking_date("18.05.2026").is_err());
    }

    #[test]
    fn validate_booking_date_rejects_invalid_calendar_dates() {
        assert!(validate_booking_date("2026-13-01").is_err()); // month 13
        assert!(validate_booking_date("2026-02-30").is_err()); // Feb 30
        assert!(validate_booking_date("2026-04-31").is_err()); // April 31
        assert!(validate_booking_date("2026-13-99").is_err()); // both bad
        assert!(validate_booking_date("2025-02-29").is_err()); // not leap
        assert!(validate_booking_date("2024-02-29").is_ok());  // leap year
    }
}
