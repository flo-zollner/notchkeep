use tauri::State;

use crate::commands::accounts::{CommandError, DbState};
use crate::db::aggregates::{
    account_monthly_cashflow as db_account_monthly_cashflow,
    bucket_monthly_flow as db_bucket_monthly_flow,
    cashflow_breakdown as db_cashflow_breakdown,
    category_breakdown as db_category_breakdown, daily_spending as db_daily_spending,
    monthly_cashflow as db_monthly_cashflow,
    monthly_cashflow_excl_invest as db_monthly_cashflow_excl_invest,
    monthly_spending as db_monthly_spending,
    CashflowSlice, CategorySpending, MonthlyFlow,
};
use crate::db::networth::{
    net_worth_forecast as db_net_worth_forecast, net_worth_history as db_net_worth_history,
    NetWorthForecastPoint, NetWorthPoint,
};

#[tauri::command]
pub async fn monthly_spending(
    state: State<'_, DbState>,
    year: i32,
    month: u32,
) -> Result<Vec<CategorySpending>, CommandError> {
    if !(1..=12).contains(&month) {
        return Err(CommandError {
            message: format!("invalid month: {month}"),
        });
    }
    Ok(db_monthly_spending(&state.pool(), year, month).await?)
}

#[tauri::command]
pub async fn monthly_cashflow(
    state: State<'_, DbState>,
    end_year: i32,
    end_month: u32,
    months: u32,
    exclude_invest: Option<bool>,
) -> Result<Vec<MonthlyFlow>, CommandError> {
    if !(1..=12).contains(&end_month) {
        return Err(CommandError {
            message: format!("invalid month: {end_month}"),
        });
    }
    if !(1..=120).contains(&months) {
        return Err(CommandError {
            message: format!("invalid months count: {months}"),
        });
    }
    let pool = state.pool();
    let rows = if exclude_invest.unwrap_or(false) {
        db_monthly_cashflow_excl_invest(&pool, end_year, end_month, months).await?
    } else {
        db_monthly_cashflow(&pool, end_year, end_month, months).await?
    };
    Ok(rows)
}

#[tauri::command]
pub async fn account_monthly_cashflow(
    state: State<'_, DbState>,
    account_id: i64,
    end_year: i32,
    end_month: u32,
    months: u32,
) -> Result<Vec<MonthlyFlow>, CommandError> {
    if !(1..=12).contains(&end_month) {
        return Err(CommandError {
            message: format!("invalid month: {end_month}"),
        });
    }
    if !(1..=120).contains(&months) {
        return Err(CommandError {
            message: format!("invalid months count: {months}"),
        });
    }
    Ok(db_account_monthly_cashflow(&state.pool(), account_id, end_year, end_month, months).await?)
}

#[tauri::command]
pub async fn bucket_monthly_flow(
    state: State<'_, DbState>,
    bucket_id: i64,
    end_year: i32,
    end_month: u32,
    months: u32,
) -> Result<Vec<MonthlyFlow>, CommandError> {
    if !(1..=12).contains(&end_month) {
        return Err(CommandError { message: format!("invalid month: {end_month}") });
    }
    if !(1..=120).contains(&months) {
        return Err(CommandError { message: format!("invalid months count: {months}") });
    }
    Ok(db_bucket_monthly_flow(&state.pool(), bucket_id, end_year, end_month, months).await?)
}

#[tauri::command]
pub async fn daily_spending(
    state: State<'_, DbState>,
    year: i32,
    month: u32,
) -> Result<Vec<i64>, CommandError> {
    if !(1..=12).contains(&month) {
        return Err(CommandError {
            message: format!("invalid month: {month}"),
        });
    }
    Ok(db_daily_spending(&state.pool(), year, month).await?)
}

#[tauri::command]
pub async fn category_breakdown(
    state: State<'_, DbState>,
    from: String,
    to: String,
) -> Result<Vec<CategorySpending>, CommandError> {
    if !is_iso_date(&from) || !is_iso_date(&to) {
        return Err(CommandError {
            message: "from/to must be YYYY-MM-DD".into(),
        });
    }
    Ok(db_category_breakdown(&state.pool(), &from, &to).await?)
}

fn is_iso_date(s: &str) -> bool {
    let bytes = s.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes[..4].iter().all(|b| b.is_ascii_digit())
        && bytes[5..7].iter().all(|b| b.is_ascii_digit())
        && bytes[8..10].iter().all(|b| b.is_ascii_digit())
}

#[tauri::command]
pub async fn net_worth_history(
    state: State<'_, DbState>,
    end_year: i32,
    end_month: u32,
    months: u32,
) -> Result<Vec<NetWorthPoint>, CommandError> {
    if !(1..=12).contains(&end_month) {
        return Err(CommandError {
            message: format!("invalid month: {end_month}"),
        });
    }
    if months > 600 {
        return Err(CommandError {
            message: format!("invalid months count: {months}"),
        });
    }
    Ok(db_net_worth_history(&state.pool(), end_year, end_month, months).await?)
}

#[tauri::command]
pub async fn net_worth_forecast(
    state: State<'_, DbState>,
    end_year: i32,
    end_month: u32,
    history_window: u32,
    forecast_months: u32,
) -> Result<Vec<NetWorthForecastPoint>, CommandError> {
    if !(1..=12).contains(&end_month) {
        return Err(CommandError {
            message: format!("invalid month: {end_month}"),
        });
    }
    if !(1..=120).contains(&history_window) {
        return Err(CommandError {
            message: format!("invalid history_window: {history_window}"),
        });
    }
    if forecast_months > 24 {
        return Err(CommandError {
            message: format!("invalid forecast_months: {forecast_months}"),
        });
    }
    Ok(db_net_worth_forecast(&state.pool(), end_year, end_month, history_window, forecast_months).await?)
}

#[tauri::command]
pub async fn cashflow_breakdown(
    state: State<'_, DbState>,
    from: String,
    to: String,
) -> Result<Vec<CashflowSlice>, CommandError> {
    let from_d = chrono::NaiveDate::parse_from_str(&from, "%Y-%m-%d")
        .map_err(|e| CommandError { message: format!("invalid from {from:?}: {e}") })?;
    let to_d = chrono::NaiveDate::parse_from_str(&to, "%Y-%m-%d")
        .map_err(|e| CommandError { message: format!("invalid to {to:?}: {e}") })?;
    if from_d >= to_d {
        return Err(CommandError { message: format!("from ({from}) must be < to ({to})") });
    }
    Ok(db_cashflow_breakdown(&state.pool(), &from, &to).await?)
}
