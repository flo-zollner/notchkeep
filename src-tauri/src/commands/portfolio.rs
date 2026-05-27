use serde::{Deserialize, Serialize};
use tauri::State;

use crate::commands::accounts::{CommandError, DbState};
use crate::db::portfolio::{
    self as db_portfolio, AllocationSlice, BucketHoldingRow, CostBasisPoint, CostBasisPointDaily,
    DividendEntry, Holding, PortfolioKpis, SecurityBucketAllocation,
};

#[tauri::command]
pub async fn list_holdings(
    state: State<'_, DbState>,
) -> Result<Vec<Holding>, CommandError> {
    Ok(db_portfolio::current_holdings(&state.pool()).await?)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountMarketValue {
    pub account_id: i64,
    pub market_value_cents: i64,
}

#[tauri::command]
pub async fn portfolio_value_by_account_today(
    state: State<'_, DbState>,
) -> Result<Vec<AccountMarketValue>, CommandError> {
    let today = chrono::Utc::now().date_naive().format("%Y-%m-%d").to_string();
    let raw = db_portfolio::portfolio_value_by_account_on_date(&state.pool(), &today).await?;
    Ok(raw
        .into_iter()
        .map(|(account_id, market_value_cents)| AccountMarketValue {
            account_id,
            market_value_cents,
        })
        .collect())
}

#[tauri::command]
pub async fn realized_gains_summary(
    state: State<'_, DbState>,
    year: Option<i32>,
) -> Result<i64, CommandError> {
    Ok(db_portfolio::realized_gains_summary(&state.pool(), year).await?)
}

#[tauri::command]
pub async fn dividend_history(
    state: State<'_, DbState>,
) -> Result<Vec<DividendEntry>, CommandError> {
    Ok(db_portfolio::dividend_history(&state.pool()).await?)
}

#[tauri::command]
pub async fn cost_basis_history(
    state: State<'_, DbState>,
    end_year: i32,
    end_month: i32,
    months: u32,
) -> Result<Vec<CostBasisPoint>, CommandError> {
    if !(2000..=2100).contains(&end_year) {
        return Err(CommandError { message: format!("end_year out of range: {end_year}") });
    }
    if months > 600 {
        return Err(CommandError { message: format!("months out of range: {months}") });
    }
    Ok(db_portfolio::cost_basis_history(&state.pool(), end_year, end_month, months).await?)
}

#[tauri::command]
pub async fn cost_basis_history_daily(
    state: State<'_, DbState>,
    end_date: String,
    days: u32,
) -> Result<Vec<CostBasisPointDaily>, CommandError> {
    if days > 366 {
        return Err(CommandError {
            message: format!("days must be <= 366, got {days}"),
        });
    }
    Ok(db_portfolio::cost_basis_history_daily(&state.pool(), &end_date, days).await?)
}

#[tauri::command]
pub async fn asset_allocation(
    state: State<'_, DbState>,
    dimension: String,
) -> Result<Vec<AllocationSlice>, CommandError> {
    Ok(db_portfolio::asset_allocation(&state.pool(), &dimension).await?)
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AllocationItem {
    pub bucket_id: i64,
    pub shares_micro: i64,
}

#[tauri::command]
pub async fn list_security_allocations(
    state: State<'_, DbState>,
    security_id: i64,
) -> Result<Vec<SecurityBucketAllocation>, CommandError> {
    Ok(db_portfolio::list_allocations_for_security(&state.pool(), security_id).await?)
}

#[tauri::command]
pub async fn set_security_allocations(
    state: State<'_, DbState>,
    security_id: i64,
    items: Vec<AllocationItem>,
) -> Result<(), CommandError> {
    let pairs: Vec<(i64, i64)> = items.into_iter().map(|i| (i.bucket_id, i.shares_micro)).collect();
    Ok(db_portfolio::set_allocations_for_security(&state.pool(), security_id, &pairs).await?)
}

#[tauri::command]
pub async fn bucket_holdings(
    state: State<'_, DbState>,
    bucket_id: i64,
) -> Result<Vec<BucketHoldingRow>, CommandError> {
    Ok(db_portfolio::bucket_holdings(&state.pool(), bucket_id).await?)
}

#[tauri::command]
pub async fn portfolio_kpis(
    state: State<'_, DbState>,
    year: i32,
) -> Result<PortfolioKpis, CommandError> {
    let pool = state.pool();
    let holdings = db_portfolio::current_holdings(&pool).await?;
    let cost_basis: i64 = holdings.iter().map(|h| h.cost_basis_cents).sum();
    let market_value: i64 = holdings.iter().map(|h| h.market_value_cents).sum();
    let unrealized: i64 = holdings.iter().map(|h| h.unrealized_cents).sum();
    let realized_ytd = db_portfolio::realized_gains_summary(&pool, Some(year)).await?;
    Ok(PortfolioKpis {
        market_value_cents: market_value,
        cost_basis_cents: cost_basis,
        unrealized_cents: unrealized,
        realized_ytd_cents: realized_ytd,
    })
}
