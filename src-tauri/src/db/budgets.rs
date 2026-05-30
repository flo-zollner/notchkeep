use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use super::{DbError, DbResult};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BudgetEntry {
    pub category_id: i64,
    pub year: i32,
    pub month: i32,
    pub amount_cents: i64,
    pub created_at: String,
}

pub async fn clear_budget(
    pool: &SqlitePool,
    category_id: i64,
    year: i32,
    month: i32,
) -> DbResult<bool> {
    let res = sqlx::query(
        "DELETE FROM category_budgets
         WHERE category_id = ?1 AND year = ?2 AND month = ?3",
    )
    .bind(category_id)
    .bind(year)
    .bind(month)
    .execute(pool)
    .await?;
    Ok(res.rows_affected() > 0)
}

/// Forward-fill: returns the latest override <= (year, month).
/// Returns None when no override exists.
pub async fn effective_budget(
    pool: &SqlitePool,
    category_id: i64,
    year: i32,
    month: i32,
) -> DbResult<Option<i64>> {
    let row: Option<(i64,)> = sqlx::query_as(
        "SELECT amount_cents
           FROM category_budgets
          WHERE category_id = ?1
            AND (year * 12 + month) <= (?2 * 12 + ?3)
          ORDER BY year DESC, month DESC
          LIMIT 1",
    )
    .bind(category_id)
    .bind(year)
    .bind(month)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|(v,)| v))
}

pub async fn set_budget(
    pool: &SqlitePool,
    category_id: i64,
    year: i32,
    month: i32,
    amount_cents: i64,
) -> DbResult<()> {
    if amount_cents < 0 {
        return Err(DbError::Decode("amount_cents must be >= 0".into()));
    }
    if !(1..=12).contains(&month) {
        return Err(DbError::Decode(format!(
            "month must be in 1..=12, got {month}",
        )));
    }
    sqlx::query(
        "INSERT INTO category_budgets (category_id, year, month, amount_cents)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT (category_id, year, month) DO UPDATE SET amount_cents = excluded.amount_cents",
    )
    .bind(category_id)
    .bind(year)
    .bind(month)
    .bind(amount_cents)
    .execute(pool)
    .await?;
    Ok(())
}

/// Batch variant for the /budgets monthly view: returns for every category with
/// at least one override <= (year, month) its effective value. Categories
/// without an override do NOT appear.
pub async fn effective_budgets_for_month(
    pool: &SqlitePool,
    year: i32,
    month: i32,
) -> DbResult<Vec<(i64, i64)>> {
    let rows: Vec<(i64, i64)> = sqlx::query_as(
        "SELECT cb.category_id, cb.amount_cents
           FROM category_budgets cb
          WHERE (cb.year * 12 + cb.month) = (
              SELECT MAX(cb2.year * 12 + cb2.month)
                FROM category_budgets cb2
               WHERE cb2.category_id = cb.category_id
                 AND (cb2.year * 12 + cb2.month) <= (?1 * 12 + ?2)
          )",
    )
    .bind(year)
    .bind(month)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn list_budget_overrides(
    pool: &SqlitePool,
    category_id: i64,
) -> DbResult<Vec<BudgetEntry>> {
    let rows = sqlx::query_as::<_, BudgetEntry>(
        "SELECT category_id, year, month, amount_cents, created_at
           FROM category_budgets
          WHERE category_id = ?1
          ORDER BY year ASC, month ASC",
    )
    .bind(category_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Pure rollover calculation: Σ(effective_budget_i − spent_i) over all months i
/// in the range [first_override, target − 1]. Returns 0 when the category has
/// no override yet or the target lies before the first override.
/// Can be negative (deficit).
pub async fn rollover_for_month(
    pool: &SqlitePool,
    category_id: i64,
    year: i32,
    month: i32,
) -> DbResult<i64> {
    // 1. All overrides for the category, ASC.
    let overrides: Vec<(i32, i32, i64)> = sqlx::query_as(
        "SELECT year, month, amount_cents FROM category_budgets
          WHERE category_id = ?1
          ORDER BY year ASC, month ASC",
    )
    .bind(category_id)
    .fetch_all(pool)
    .await?;

    if overrides.is_empty() {
        return Ok(0);
    }

    let target_key = year * 12 + month;
    let first = &overrides[0];
    let first_key = first.0 * 12 + first.1;
    if target_key <= first_key {
        // Target is at or before the first override → no rollover.
        return Ok(0);
    }

    // 2. Forward-fill per month in the range [first_key, target_key - 1].
    let mut current_budget = first.2;
    let mut idx = 1usize;
    let mut sum: i64 = 0;
    for key in first_key..target_key {
        // Apply override if present.
        while idx < overrides.len() && overrides[idx].0 * 12 + overrides[idx].1 == key {
            current_budget = overrides[idx].2;
            idx += 1;
        }
        // Decode key to (year, month): key = y*12 + m, m in 1..=12.
        // y_raw = key / 12, m_raw = key % 12. If m_raw == 0 → m=12, y=y_raw-1.
        let y_raw = key / 12;
        let m_raw = key % 12;
        let (y, m) = if m_raw == 0 {
            (y_raw - 1, 12)
        } else {
            (y_raw, m_raw)
        };

        let spent: i64 = monthly_spent_for_category(pool, category_id, y, m).await?;
        sum += current_budget - spent;
    }

    Ok(sum)
}

/// Helper: sums the negative transaction amounts for the category in the calendar month (cents as positive).
async fn monthly_spent_for_category(
    pool: &SqlitePool,
    category_id: i64,
    year: i32,
    month: i32,
) -> DbResult<i64> {
    let from = format!("{year:04}-{month:02}-01");
    let to_year = if month == 12 { year + 1 } else { year };
    let to_month = if month == 12 { 1 } else { month + 1 };
    let to = format!("{to_year:04}-{to_month:02}-01");

    let row: Option<(Option<i64>,)> = sqlx::query_as(
        "SELECT CAST(SUM(-amount_cents) AS INTEGER)
           FROM transactions
          WHERE category_id = ?1
            AND amount_cents < 0
            AND kind != 'transfer'
            AND booking_date >= ?2
            AND booking_date <  ?3",
    )
    .bind(category_id)
    .bind(&from)
    .bind(&to)
    .fetch_optional(pool)
    .await?;

    Ok(row.and_then(|(opt,)| opt).unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_memory;

    async fn seed_category(pool: &SqlitePool, name: &str) -> i64 {
        let (id,): (i64,) = sqlx::query_as(
            "INSERT INTO categories (parent_id, name, color, icon, rollover_enabled)
             VALUES (NULL, ?1, NULL, NULL, 0) RETURNING id",
        )
        .bind(name)
        .fetch_one(pool)
        .await
        .unwrap();
        id
    }

    async fn seed_spent(pool: &SqlitePool, category_id: i64, date: &str, amount_cents: i64) {
        // Idempotent test account
        sqlx::query(
            "INSERT INTO accounts (name, kind, currency) VALUES ('test', 'bank', 'EUR')
             ON CONFLICT DO NOTHING",
        )
        .execute(pool)
        .await
        .ok();
        let (acc_id,): (i64,) = sqlx::query_as("SELECT id FROM accounts LIMIT 1")
            .fetch_one(pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO transactions
             (account_id, booking_date, amount_cents, currency, category_id, source, imported_at)
             VALUES (?1, ?2, ?3, 'EUR', ?4, 'manual', '2026-05-19T00:00:00Z')",
        )
        .bind(acc_id)
        .bind(date)
        .bind(amount_cents)
        .bind(category_id)
        .execute(pool)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn set_budget_inserts_and_overwrites() {
        let pool = connect_memory().await.unwrap();
        let cat = seed_category(&pool, "TestCat").await;

        set_budget(&pool, cat, 2026, 5, 50000).await.unwrap();

        let count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM category_budgets WHERE category_id = ?1")
                .bind(cat)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(count.0, 1);

        set_budget(&pool, cat, 2026, 5, 60000).await.unwrap();

        let count2: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM category_budgets WHERE category_id = ?1")
                .bind(cat)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(count2.0, 1);

        let amount: (i64,) =
            sqlx::query_as("SELECT amount_cents FROM category_budgets WHERE category_id = ?1")
                .bind(cat)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(amount.0, 60000);
    }

    #[tokio::test]
    async fn clear_budget_removes_entry_and_returns_bool() {
        let pool = connect_memory().await.unwrap();
        let cat = seed_category(&pool, "ClearCat").await;

        set_budget(&pool, cat, 2026, 5, 50000).await.unwrap();

        let result = clear_budget(&pool, cat, 2026, 5).await.unwrap();
        assert!(result);

        let count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM category_budgets WHERE category_id = ?1")
                .bind(cat)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(count.0, 0);

        let result2 = clear_budget(&pool, cat, 2026, 5).await.unwrap();
        assert!(!result2);
    }

    #[tokio::test]
    async fn set_budget_rejects_negative_amount_and_bad_month() {
        let pool = connect_memory().await.unwrap();
        let cat = seed_category(&pool, "ValidationCat").await;

        assert!(set_budget(&pool, cat, 2026, 5, -1).await.is_err());
        assert!(set_budget(&pool, cat, 2026, 0, 1000).await.is_err());
        assert!(set_budget(&pool, cat, 2026, 13, 1000).await.is_err());
    }

    #[tokio::test]
    async fn effective_budget_forward_fills_from_earlier_month() {
        let pool = connect_memory().await.unwrap();
        let cat = seed_category(&pool, "FwdFillCat").await;

        set_budget(&pool, cat, 2026, 3, 50000).await.unwrap();

        // direct hit
        assert_eq!(
            effective_budget(&pool, cat, 2026, 3).await.unwrap(),
            Some(50000)
        );
        // forward-filled from March
        assert_eq!(
            effective_budget(&pool, cat, 2026, 4).await.unwrap(),
            Some(50000)
        );
        // before any override
        assert_eq!(effective_budget(&pool, cat, 2026, 2).await.unwrap(), None);
    }

    #[tokio::test]
    async fn effective_budget_crosses_year_boundary() {
        let pool = connect_memory().await.unwrap();
        let cat = seed_category(&pool, "YearBoundaryCat").await;

        set_budget(&pool, cat, 2025, 12, 50000).await.unwrap();

        assert_eq!(
            effective_budget(&pool, cat, 2026, 1).await.unwrap(),
            Some(50000)
        );
        assert_eq!(
            effective_budget(&pool, cat, 2026, 5).await.unwrap(),
            Some(50000)
        );
    }

    #[tokio::test]
    async fn effective_budget_picks_latest_le_target() {
        let pool = connect_memory().await.unwrap();
        let cat = seed_category(&pool, "MultiOverrideCat").await;

        set_budget(&pool, cat, 2026, 1, 30000).await.unwrap();
        set_budget(&pool, cat, 2026, 4, 50000).await.unwrap();
        set_budget(&pool, cat, 2026, 8, 70000).await.unwrap();

        assert_eq!(
            effective_budget(&pool, cat, 2026, 1).await.unwrap(),
            Some(30000)
        );
        assert_eq!(
            effective_budget(&pool, cat, 2026, 2).await.unwrap(),
            Some(30000)
        );
        assert_eq!(
            effective_budget(&pool, cat, 2026, 3).await.unwrap(),
            Some(30000)
        );
        assert_eq!(
            effective_budget(&pool, cat, 2026, 4).await.unwrap(),
            Some(50000)
        );
        assert_eq!(
            effective_budget(&pool, cat, 2026, 7).await.unwrap(),
            Some(50000)
        );
        assert_eq!(
            effective_budget(&pool, cat, 2026, 8).await.unwrap(),
            Some(70000)
        );
        assert_eq!(
            effective_budget(&pool, cat, 2026, 12).await.unwrap(),
            Some(70000)
        );
    }

    #[tokio::test]
    async fn effective_budget_empty_returns_none() {
        let pool = connect_memory().await.unwrap();
        let cat = seed_category(&pool, "EmptyCat").await;

        // no budgets seeded at all
        assert_eq!(effective_budget(&pool, cat, 2026, 5).await.unwrap(), None);
    }

    #[tokio::test]
    async fn effective_budgets_for_month_returns_all_with_overrides() {
        let pool = connect_memory().await.unwrap();
        let cat_a = seed_category(&pool, "BatchCatA").await;
        let cat_b = seed_category(&pool, "BatchCatB").await;
        let cat_c = seed_category(&pool, "BatchCatC").await;

        // A: override in Feb, forward-fills to May
        set_budget(&pool, cat_a, 2026, 2, 10000).await.unwrap();
        // B: direct override in May
        set_budget(&pool, cat_b, 2026, 5, 20000).await.unwrap();
        // C: no override at all

        let mut rows = effective_budgets_for_month(&pool, 2026, 5).await.unwrap();
        rows.sort_by_key(|(id, _)| *id);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], (cat_a, 10000));
        assert_eq!(rows[1], (cat_b, 20000));
        // C must NOT appear
        assert!(!rows.iter().any(|(id, _)| *id == cat_c));

        let rows_april = effective_budgets_for_month(&pool, 2026, 4).await.unwrap();
        assert_eq!(rows_april.len(), 1);
        assert_eq!(rows_april[0], (cat_a, 10000));
    }

    #[tokio::test]
    async fn list_budget_overrides_returns_chronological_entries() {
        let pool = connect_memory().await.unwrap();
        let cat = seed_category(&pool, "ChronoCat").await;

        // Insert in mixed order
        set_budget(&pool, cat, 2026, 3, 50000).await.unwrap();
        set_budget(&pool, cat, 2025, 12, 30000).await.unwrap();
        set_budget(&pool, cat, 2026, 1, 40000).await.unwrap();

        let rows = list_budget_overrides(&pool, cat).await.unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(
            (rows[0].year, rows[0].month, rows[0].amount_cents),
            (2025, 12, 30000)
        );
        assert_eq!(
            (rows[1].year, rows[1].month, rows[1].amount_cents),
            (2026, 1, 40000)
        );
        assert_eq!(
            (rows[2].year, rows[2].month, rows[2].amount_cents),
            (2026, 3, 50000)
        );
    }

    #[tokio::test]
    async fn migration_copies_existing_budget_cents_into_overrides() {
        let pool = connect_memory().await.unwrap();
        // Migration 0010 has already run on connect_memory.
        // Default categories from 0003 had NULL budget_cents → no overrides expected.
        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM category_budgets")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(
            count, 0,
            "Default categories had NULL budget_cents, no override expected"
        );
    }

    #[tokio::test]
    async fn rollover_positive_from_single_previous_month() {
        let pool = connect_memory().await.unwrap();
        let cat = seed_category(&pool, "Lebensmittel").await;

        // April 2026: budget 50000, spent 35000 → carry +15000
        set_budget(&pool, cat, 2026, 4, 50000).await.unwrap();
        seed_spent(&pool, cat, "2026-04-15", -35000).await;

        let r = rollover_for_month(&pool, cat, 2026, 5).await.unwrap();
        assert_eq!(r, 15000, "May should inherit 15000 rollover from April");
    }

    #[tokio::test]
    async fn rollover_negative_when_spent_exceeds_budget() {
        let pool = connect_memory().await.unwrap();
        let cat = seed_category(&pool, "Lebensmittel").await;

        set_budget(&pool, cat, 2026, 4, 50000).await.unwrap();
        seed_spent(&pool, cat, "2026-04-10", -58000).await;

        let r = rollover_for_month(&pool, cat, 2026, 5).await.unwrap();
        assert_eq!(r, -8000, "April deficit -8000 must carry into May rollover");
    }

    #[tokio::test]
    async fn rollover_accumulates_over_multiple_previous_months() {
        let pool = connect_memory().await.unwrap();
        let cat = seed_category(&pool, "Lebensmittel").await;

        set_budget(&pool, cat, 2026, 2, 50000).await.unwrap();
        seed_spent(&pool, cat, "2026-02-15", -40000).await; // +10000
                                                            // March: no new override → inherits 50000.
        seed_spent(&pool, cat, "2026-03-15", -45000).await; // +5000
        set_budget(&pool, cat, 2026, 4, 30000).await.unwrap();
        seed_spent(&pool, cat, "2026-04-15", -32000).await; // -2000

        let r = rollover_for_month(&pool, cat, 2026, 5).await.unwrap();
        // 10000 + 5000 + (-2000) = 13000
        assert_eq!(r, 13000);
    }

    #[tokio::test]
    async fn rollover_zero_when_target_at_or_before_first_override() {
        let pool = connect_memory().await.unwrap();
        let cat = seed_category(&pool, "Lebensmittel").await;

        set_budget(&pool, cat, 2026, 5, 50000).await.unwrap();
        seed_spent(&pool, cat, "2026-04-15", -10000).await;

        // May itself → no rollover.
        let r = rollover_for_month(&pool, cat, 2026, 5).await.unwrap();
        assert_eq!(r, 0);

        // April (before the override) → 0.
        let r2 = rollover_for_month(&pool, cat, 2026, 4).await.unwrap();
        assert_eq!(r2, 0);
    }

    #[tokio::test]
    async fn rollover_crosses_year_boundary() {
        let pool = connect_memory().await.unwrap();
        let cat = seed_category(&pool, "Lebensmittel").await;

        set_budget(&pool, cat, 2025, 12, 60000).await.unwrap();
        seed_spent(&pool, cat, "2025-12-20", -45000).await; // +15000

        let r = rollover_for_month(&pool, cat, 2026, 1).await.unwrap();
        assert_eq!(r, 15000);
    }

    #[tokio::test]
    async fn rollover_empty_overrides_returns_zero() {
        let pool = connect_memory().await.unwrap();
        let cat = seed_category(&pool, "Lebensmittel").await;
        seed_spent(&pool, cat, "2026-04-15", -10000).await;

        let r = rollover_for_month(&pool, cat, 2026, 5).await.unwrap();
        assert_eq!(r, 0);
    }

    /// Transfers (kind='transfer') must not be counted as spending.
    #[tokio::test]
    async fn monthly_spent_ignores_transfer_kind() {
        let pool = connect_memory().await.unwrap();
        let cat = seed_category(&pool, "TransferCat").await;

        // Normal expense — should count.
        seed_spent(&pool, cat, "2026-05-10", -30000).await;

        // Transfer — must NOT count.
        let (acc_id,): (i64,) = sqlx::query_as("SELECT id FROM accounts LIMIT 1")
            .fetch_one(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO transactions
             (account_id, booking_date, amount_cents, currency, category_id, kind, source, imported_at)
             VALUES (?1, '2026-05-15', -10000, 'EUR', ?2, 'transfer', 'manual', '2026-05-19T00:00:00Z')",
        )
        .bind(acc_id)
        .bind(cat)
        .execute(&pool)
        .await
        .unwrap();

        let spent = monthly_spent_for_category(&pool, cat, 2026, 5)
            .await
            .unwrap();
        // Only the -30000 expense should count; transfer (-10000) must be excluded.
        assert_eq!(spent, 30000);
    }
}
