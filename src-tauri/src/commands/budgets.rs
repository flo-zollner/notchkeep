use serde::Serialize;
use tauri::State;

use crate::commands::accounts::{CommandError, DbState};
use crate::db::aggregates as db_aggregates;
use crate::db::budgets::{self as db_budgets, BudgetEntry};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryMonthBudget {
    pub category_id: i64,
    pub category_name: String,
    pub budget_cents: Option<i64>,        // effective (forward-filled)
    pub override_cents: Option<i64>,      // explicit (for edit-state)
    pub spent_cents: i64,
    pub rollover_cents: i64,              // 0 in 6m1, populated in 6m2
    pub rollover_enabled: bool,
}

fn validate_year(y: i32) -> Result<(), CommandError> {
    if !(2000..=2100).contains(&y) {
        return Err(CommandError {
            message: format!("year must be in 2000..=2100, got {y}"),
        });
    }
    Ok(())
}

fn validate_month(m: i32) -> Result<(), CommandError> {
    if !(1..=12).contains(&m) {
        return Err(CommandError {
            message: format!("month must be in 1..=12, got {m}"),
        });
    }
    Ok(())
}

#[tauri::command]
pub async fn set_budget(
    state: State<'_, DbState>,
    category_id: i64,
    year: i32,
    month: i32,
    amount_cents: i64,
) -> Result<(), CommandError> {
    validate_year(year)?;
    validate_month(month)?;
    if amount_cents < 0 {
        return Err(CommandError { message: "amount_cents must be >= 0".into() });
    }
    Ok(db_budgets::set_budget(&state.pool(), category_id, year, month, amount_cents).await?)
}

#[tauri::command]
pub async fn clear_budget(
    state: State<'_, DbState>,
    category_id: i64,
    year: i32,
    month: i32,
) -> Result<bool, CommandError> {
    validate_year(year)?;
    validate_month(month)?;
    Ok(db_budgets::clear_budget(&state.pool(), category_id, year, month).await?)
}

#[tauri::command]
pub async fn list_budget_overrides(
    state: State<'_, DbState>,
    category_id: i64,
) -> Result<Vec<BudgetEntry>, CommandError> {
    Ok(db_budgets::list_budget_overrides(&state.pool(), category_id).await?)
}

#[tauri::command]
pub async fn month_overview(
    state: State<'_, DbState>,
    year: i32,
    month: i32,
) -> Result<Vec<CategoryMonthBudget>, CommandError> {
    validate_year(year)?;
    validate_month(month)?;
    let pool = state.pool();

    // 1. Category list (id, name, rollover_enabled).
    let cats: Vec<(i64, String, bool)> = sqlx::query_as(
        "SELECT id, name, rollover_enabled FROM categories ORDER BY name COLLATE NOCASE ASC",
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| CommandError { message: e.to_string() })?;

    // 2. Effective budgets per category (forward-fill).
    let eff = db_budgets::effective_budgets_for_month(&pool, year, month).await?;
    let eff_map: std::collections::HashMap<i64, i64> = eff.into_iter().collect();

    // 3. Explicit overrides for the target month (for showing the ×-icon).
    let overrides: Vec<(i64, i64)> = sqlx::query_as(
        "SELECT category_id, amount_cents FROM category_budgets
          WHERE year = ?1 AND month = ?2",
    )
    .bind(year)
    .bind(month)
    .fetch_all(&pool)
    .await
    .map_err(|e| CommandError { message: e.to_string() })?;
    let override_map: std::collections::HashMap<i64, i64> = overrides.into_iter().collect();

    // 4. Spent per category from monthly_spending. Note: monthly_spending
    // expects month: u32. After validate_month, month is in 1..=12, so the cast is safe.
    let spending = db_aggregates::monthly_spending(&pool, year, month as u32).await?;
    let spent_map: std::collections::HashMap<i64, i64> = spending
        .into_iter()
        .map(|s| (s.category_id, s.spent_cents))
        .collect();

    let mut result: Vec<CategoryMonthBudget> = Vec::with_capacity(cats.len());
    for (id, name, rollover_enabled) in cats {
        let rollover_cents = if rollover_enabled {
            db_budgets::rollover_for_month(&pool, id, year, month).await?
        } else {
            0
        };
        result.push(CategoryMonthBudget {
            category_id: id,
            category_name: name,
            budget_cents: eff_map.get(&id).copied(),
            override_cents: override_map.get(&id).copied(),
            spent_cents: spent_map.get(&id).copied().unwrap_or(0),
            rollover_cents,
            rollover_enabled,
        });
    }
    Ok(result)
}

/// Sum of all expenses without a category (category_id IS NULL) in the given
/// month. Trade sides (transfer, buy, sell, corporate_action) are excluded —
/// same filtering as monthly_spending. Positive cents.
#[tauri::command]
pub async fn uncategorized_monthly_spent(
    state: State<'_, DbState>,
    year: i32,
    month: i32,
) -> Result<i64, CommandError> {
    validate_year(year)?;
    validate_month(month)?;
    let pool = state.pool();
    let (from, to) = (
        format!("{year:04}-{month:02}-01"),
        // first day of next month
        if month == 12 {
            format!("{:04}-01-01", year + 1)
        } else {
            format!("{year:04}-{:02}-01", month + 1)
        },
    );
    let sql = format!(
        "SELECT COALESCE(SUM(-amount_cents), 0) FROM transactions
          WHERE category_id IS NULL
            AND amount_cents < 0
            AND {}
            AND booking_date >= ?1
            AND booking_date < ?2",
        crate::db::aggregates::EXCLUDED_KINDS_SQL,
    );
    let (sum,): (i64,) = sqlx::query_as(&sql)
        .bind(&from)
        .bind(&to)
        .fetch_one(&pool)
        .await
        .map_err(|e| CommandError { message: e.to_string() })?;
    Ok(sum)
}

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct InvestmentFlow {
    pub buys_cents: i64,       // positive (absolute purchases)
    pub sells_cents: i64,      // positive (absolute sale proceeds)
    pub dividends_cents: i64,  // positive (dividend income)
    pub net_invested_cents: i64, // buys − sells (positive = net cash into investments)
}

/// Investment flow for a month: separate totals for buy/sell/dividend
/// on cash accounts (kind in transactions, not securities_trades).
/// All values positive; net_invested = buys − sells.
#[tauri::command]
pub async fn investment_flow_for_month(
    state: State<'_, DbState>,
    year: i32,
    month: i32,
) -> Result<InvestmentFlow, CommandError> {
    validate_year(year)?;
    validate_month(month)?;
    let pool = state.pool();
    let (from, to) = (
        format!("{year:04}-{month:02}-01"),
        if month == 12 {
            format!("{:04}-01-01", year + 1)
        } else {
            format!("{year:04}-{:02}-01", month + 1)
        },
    );

    // buy: amount_cents is negative (cash out). We want the ABSOLUTE sum.
    let (buys,): (i64,) = sqlx::query_as(
        "SELECT COALESCE(SUM(-amount_cents), 0) FROM transactions
          WHERE kind = 'buy' AND booking_date >= ?1 AND booking_date < ?2"
    ).bind(&from).bind(&to).fetch_one(&pool).await
     .map_err(|e| CommandError { message: e.to_string() })?;

    // sell: amount_cents positive (cash in) — direct sum.
    let (sells,): (i64,) = sqlx::query_as(
        "SELECT COALESCE(SUM(amount_cents), 0) FROM transactions
          WHERE kind = 'sell' AND booking_date >= ?1 AND booking_date < ?2"
    ).bind(&from).bind(&to).fetch_one(&pool).await
     .map_err(|e| CommandError { message: e.to_string() })?;

    // dividend: amount_cents positive — direct sum.
    let (dividends,): (i64,) = sqlx::query_as(
        "SELECT COALESCE(SUM(amount_cents), 0) FROM transactions
          WHERE kind = 'dividend' AND booking_date >= ?1 AND booking_date < ?2"
    ).bind(&from).bind(&to).fetch_one(&pool).await
     .map_err(|e| CommandError { message: e.to_string() })?;

    Ok(InvestmentFlow {
        buys_cents: buys,
        sells_cents: sells,
        dividends_cents: dividends,
        net_invested_cents: buys - sells,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_year_range() {
        assert!(validate_year(2026).is_ok());
        assert!(validate_year(2000).is_ok());
        assert!(validate_year(2100).is_ok());
        assert!(validate_year(1999).is_err());
        assert!(validate_year(2101).is_err());
    }

    #[test]
    fn validate_month_range() {
        assert!(validate_month(1).is_ok());
        assert!(validate_month(12).is_ok());
        assert!(validate_month(0).is_err());
        assert!(validate_month(13).is_err());
    }
}
