use std::collections::HashMap;

use serde::Serialize;
use sqlx::SqlitePool;

use super::{DbError, DbResult};

/// SQL fragment for `WHERE` clauses that excludes transactions from cashflow aggregations:
/// - `transfer`: auto-paired mirror between own accounts, net-zero on both sides.
/// - `corporate_action`: splits/mergers with no cashflow (typically amount=0).
///   `buy`/`sell` are included as expense/income — the user treats
///   securities purchases like regular expenses on the cash account.
pub(crate) const EXCLUDED_KINDS_SQL: &str = "kind NOT IN ('transfer', 'corporate_action')";

#[derive(Debug, Clone, Serialize, sqlx::FromRow, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CategorySpending {
    pub category_id: i64,
    pub spent_cents: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MonthlyFlow {
    pub year: i32,
    pub month: u32,
    pub in_cents: i64,
    pub out_cents: i64,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CashflowSlice {
    pub category_id: Option<i64>,
    pub sign: i32,
    pub sum_abs_cents: i64,
}

/// Aggregates expenses (negative `amount_cents`) per category for the given month.
pub async fn monthly_spending(
    pool: &SqlitePool,
    year: i32,
    month: u32,
) -> DbResult<Vec<CategorySpending>> {
    let (from, to) = month_range(year, month);
    category_breakdown(pool, &from, &to).await
}

/// Aggregates expenses per category in the half-open interval `[from, to)`
/// (dates in `YYYY-MM-DD` format). Income and transactions without a `category_id`
/// are ignored; `spent_cents` is positive.
pub async fn category_breakdown(
    pool: &SqlitePool,
    from: &str,
    to: &str,
) -> DbResult<Vec<CategorySpending>> {
    let sql = format!(
        "SELECT category_id AS category_id,
                CAST(SUM(-amount_cents) AS INTEGER) AS spent_cents
         FROM transactions
         WHERE category_id IS NOT NULL
           AND amount_cents < 0
           AND {EXCLUDED_KINDS_SQL}
           AND booking_date >= ?1
           AND booking_date < ?2
         GROUP BY category_id
         ORDER BY category_id"
    );
    let rows = sqlx::query_as::<_, CategorySpending>(&sql)
        .bind(from)
        .bind(to)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

/// Income vs. expenses per month for the last `months` months
/// (inclusive of `end_year`/`end_month`). Missing months are zero-filled,
/// sorted chronologically.
pub async fn monthly_cashflow(
    pool: &SqlitePool,
    end_year: i32,
    end_month: u32,
    months: u32,
) -> DbResult<Vec<MonthlyFlow>> {
    monthly_cashflow_filtered(pool, None, end_year, end_month, months, false).await
}

/// Like `monthly_cashflow`, but buy/sell are additionally excluded from the
/// cashflow (for a "savings rate excluding investments" view).
pub async fn monthly_cashflow_excl_invest(
    pool: &SqlitePool,
    end_year: i32,
    end_month: u32,
    months: u32,
) -> DbResult<Vec<MonthlyFlow>> {
    monthly_cashflow_filtered(pool, None, end_year, end_month, months, true).await
}

/// Like `monthly_cashflow`, but restricted to transactions of the given account.
pub async fn account_monthly_cashflow(
    pool: &SqlitePool,
    account_id: i64,
    end_year: i32,
    end_month: u32,
    months: u32,
) -> DbResult<Vec<MonthlyFlow>> {
    monthly_cashflow_filtered(pool, Some(account_id), end_year, end_month, months, false).await
}

/// Monthly cashflow (income + expenses) filtered to a bucket.
/// Same bucket layout as monthly_cashflow_filtered, but WHERE bucket_id = ?.
pub async fn bucket_monthly_flow(
    pool: &SqlitePool,
    bucket_id: i64,
    end_year: i32,
    end_month: u32,
    months: u32,
) -> DbResult<Vec<MonthlyFlow>> {
    let buckets = month_buckets(end_year, end_month, months);
    let (start_y, start_m) = buckets[0];
    let from = format!("{start_y:04}-{start_m:02}-01");
    let (after_y, after_m) = step_month(end_year, end_month, 1);
    let to = format!("{after_y:04}-{after_m:02}-01");

    let bucket_sql = format!(
        "SELECT
            strftime('%Y', booking_date) AS year,
            strftime('%m', booking_date) AS month,
            CAST(SUM(CASE WHEN amount_cents > 0 THEN amount_cents ELSE 0 END) AS INTEGER) AS in_cents,
            CAST(SUM(CASE WHEN amount_cents < 0 THEN -amount_cents ELSE 0 END) AS INTEGER) AS out_cents
         FROM transactions
         WHERE booking_date >= ?1 AND booking_date < ?2
           AND {EXCLUDED_KINDS_SQL}
           AND bucket_id = ?3
         GROUP BY year, month"
    );
    let rows: Vec<(String, String, i64, i64)> = sqlx::query_as(&bucket_sql)
        .bind(&from)
        .bind(&to)
        .bind(bucket_id)
        .fetch_all(pool)
        .await?;

    let mut map: HashMap<(i32, u32), (i64, i64)> = HashMap::with_capacity(rows.len());
    for (y_str, m_str, in_c, out_c) in rows {
        let y: i32 = y_str
            .parse()
            .map_err(|e: std::num::ParseIntError| DbError::Decode(format!("year: {e}")))?;
        let m: u32 = m_str
            .parse()
            .map_err(|e: std::num::ParseIntError| DbError::Decode(format!("month: {e}")))?;
        map.insert((y, m), (in_c, out_c));
    }

    Ok(buckets
        .into_iter()
        .map(|(y, m)| {
            let (in_cents, out_cents) = map.get(&(y, m)).copied().unwrap_or((0, 0));
            MonthlyFlow {
                year: y,
                month: m,
                in_cents,
                out_cents,
            }
        })
        .collect())
}

async fn monthly_cashflow_filtered(
    pool: &SqlitePool,
    account_id: Option<i64>,
    end_year: i32,
    end_month: u32,
    months: u32,
    exclude_invest: bool,
) -> DbResult<Vec<MonthlyFlow>> {
    let buckets = month_buckets(end_year, end_month, months);
    let (start_y, start_m) = buckets[0];
    let from = format!("{start_y:04}-{start_m:02}-01");
    let (after_y, after_m) = step_month(end_year, end_month, 1);
    let to = format!("{after_y:04}-{after_m:02}-01");

    let extra_kind_filter = if exclude_invest {
        " AND kind NOT IN ('buy','sell')"
    } else {
        ""
    };
    let base_sql = format!(
        "SELECT
            strftime('%Y', booking_date) AS year,
            strftime('%m', booking_date) AS month,
            CAST(SUM(CASE WHEN amount_cents > 0 THEN amount_cents ELSE 0 END) AS INTEGER) AS in_cents,
            CAST(SUM(CASE WHEN amount_cents < 0 THEN -amount_cents ELSE 0 END) AS INTEGER) AS out_cents
         FROM transactions
         WHERE booking_date >= ?1 AND booking_date < ?2
           AND {EXCLUDED_KINDS_SQL}{extra_kind_filter}"
    );
    let sql = if account_id.is_some() {
        // Inline subtree-CTE — keep in sync with `collect_subtree` in db::accounts.
        format!(
            "{base_sql}
               AND account_id IN (
                 WITH RECURSIVE subtree(id) AS (
                     SELECT id FROM accounts WHERE id = ?3
                     UNION ALL
                     SELECT a.id FROM accounts a JOIN subtree s ON a.parent_id = s.id
                 )
                 SELECT id FROM subtree
               )
             GROUP BY year, month"
        )
    } else {
        format!("{base_sql} GROUP BY year, month")
    };

    let mut query = sqlx::query_as::<_, (String, String, i64, i64)>(&sql)
        .bind(&from)
        .bind(&to);
    if let Some(id) = account_id {
        query = query.bind(id);
    }
    let rows = query.fetch_all(pool).await?;

    let mut map: HashMap<(i32, u32), (i64, i64)> = HashMap::with_capacity(rows.len());
    for (y_str, m_str, in_c, out_c) in rows {
        let y: i32 = y_str
            .parse()
            .map_err(|e: std::num::ParseIntError| DbError::Decode(format!("year: {e}")))?;
        let m: u32 = m_str
            .parse()
            .map_err(|e: std::num::ParseIntError| DbError::Decode(format!("month: {e}")))?;
        map.insert((y, m), (in_c, out_c));
    }

    Ok(buckets
        .into_iter()
        .map(|(y, m)| {
            let (in_cents, out_cents) = map.get(&(y, m)).copied().unwrap_or((0, 0));
            MonthlyFlow {
                year: y,
                month: m,
                in_cents,
                out_cents,
            }
        })
        .collect())
}

/// Expenses (positive cents) per day of the month. Index 0 = day 1,
/// length = days in month. Income (positive `amount_cents`) is ignored;
/// uncategorized transactions are counted.
pub async fn daily_spending(pool: &SqlitePool, year: i32, month: u32) -> DbResult<Vec<i64>> {
    let (from, to) = month_range(year, month);
    let daily_sql = format!(
        "SELECT
            strftime('%d', booking_date) AS day,
            CAST(SUM(-amount_cents) AS INTEGER) AS spent
         FROM transactions
         WHERE amount_cents < 0
           AND {EXCLUDED_KINDS_SQL}
           AND booking_date >= ?1
           AND booking_date < ?2
         GROUP BY day"
    );
    let rows: Vec<(String, i64)> = sqlx::query_as(&daily_sql)
        .bind(&from)
        .bind(&to)
        .fetch_all(pool)
        .await?;

    let len = days_in_month(year, month) as usize;
    let mut out = vec![0_i64; len];
    for (day_str, spent) in rows {
        let day: u32 = day_str
            .parse()
            .map_err(|e: std::num::ParseIntError| DbError::Decode(format!("day: {e}")))?;
        if (1..=len as u32).contains(&day) {
            out[(day - 1) as usize] = spent;
        }
    }
    Ok(out)
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

fn month_range(year: i32, month: u32) -> (String, String) {
    let from = format!("{year:04}-{month:02}-01");
    let (next_y, next_m) = step_month(year, month, 1);
    let to = format!("{next_y:04}-{next_m:02}-01");
    (from, to)
}

pub(super) fn step_month(year: i32, month: u32, delta: i32) -> (i32, u32) {
    let total = year * 12 + (month as i32) - 1 + delta;
    let new_y = total.div_euclid(12);
    let new_m = (total.rem_euclid(12) + 1) as u32;
    (new_y, new_m)
}

pub(super) fn month_buckets(end_year: i32, end_month: u32, months: u32) -> Vec<(i32, u32)> {
    let n = months.max(1) as usize;
    let mut out = Vec::with_capacity(n);
    for i in (0..n).rev() {
        out.push(step_month(end_year, end_month, -(i as i32)));
    }
    out
}

/// Groups transactions in the half-open interval `[from, to)` by `(category_id, sign)`
/// and returns the sum of absolute amounts. A category can produce both
/// income rows (sign=+1) and expense rows (sign=-1).
/// Transactions with `amount_cents = 0` are filtered out (e.g. stock splits).
pub async fn cashflow_breakdown(
    pool: &SqlitePool,
    from: &str,
    to: &str,
) -> DbResult<Vec<CashflowSlice>> {
    let breakdown_sql = format!(
        "SELECT category_id,
                CASE WHEN amount_cents > 0 THEN 1 ELSE -1 END AS sign,
                CAST(SUM(ABS(amount_cents)) AS INTEGER) AS sum_abs_cents
           FROM transactions
          WHERE booking_date >= ?1
            AND booking_date <  ?2
            AND amount_cents != 0
            AND {EXCLUDED_KINDS_SQL}
          GROUP BY category_id, sign
          ORDER BY sign DESC, sum_abs_cents DESC"
    );
    let rows: Vec<CashflowSlice> = sqlx::query_as(&breakdown_sql)
        .bind(from)
        .bind(to)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_memory;

    async fn seed_account(pool: &SqlitePool, name: &str) -> i64 {
        let (id,): (i64,) = sqlx::query_as(
            "INSERT INTO accounts (name, kind, currency) VALUES (?1, 'bank', 'EUR') RETURNING id",
        )
        .bind(name)
        .fetch_one(pool)
        .await
        .unwrap();
        id
    }

    async fn seed_category(pool: &SqlitePool, name: &str) -> i64 {
        let (id,): (i64,) = sqlx::query_as(
            "INSERT INTO categories (parent_id, name) VALUES (NULL, ?1) RETURNING id",
        )
        .bind(name)
        .fetch_one(pool)
        .await
        .unwrap();
        id
    }

    async fn insert_tx(
        pool: &SqlitePool,
        account_id: i64,
        category_id: Option<i64>,
        booking_date: &str,
        amount_cents: i64,
    ) {
        insert_tx_with_kind(
            pool,
            account_id,
            category_id,
            booking_date,
            amount_cents,
            "expense",
        )
        .await;
    }

    async fn insert_tx_with_kind(
        pool: &SqlitePool,
        account_id: i64,
        category_id: Option<i64>,
        booking_date: &str,
        amount_cents: i64,
        kind: &str,
    ) {
        sqlx::query(
            "INSERT INTO transactions
                (account_id, booking_date, amount_cents, currency,
                 counterparty, source, kind)
             VALUES (?1, ?2, ?3, 'EUR', ?4, 'manual', ?5)",
        )
        .bind(account_id)
        .bind(booking_date)
        .bind(amount_cents)
        .bind(format!("seed-{booking_date}-{amount_cents}-{kind}"))
        .bind(kind)
        .execute(pool)
        .await
        .unwrap();
        if let Some(cid) = category_id {
            sqlx::query("UPDATE transactions SET category_id = ?1 WHERE id = last_insert_rowid()")
                .bind(cid)
                .execute(pool)
                .await
                .unwrap();
        }
    }

    #[tokio::test]
    async fn empty_table_returns_empty() {
        let pool = connect_memory().await.unwrap();
        let result = monthly_spending(&pool, 2026, 5).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn sums_expenses_per_category() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool, "TR").await;
        let groceries = seed_category(&pool, "TestGroceries").await;
        let rent = seed_category(&pool, "TestRent").await;

        insert_tx(&pool, acc, Some(groceries), "2026-05-03", -1200).await;
        insert_tx(&pool, acc, Some(groceries), "2026-05-15", -3450).await;
        insert_tx(&pool, acc, Some(rent), "2026-05-01", -128000).await;

        let result = monthly_spending(&pool, 2026, 5).await.unwrap();
        assert_eq!(
            result,
            vec![
                CategorySpending {
                    category_id: groceries,
                    spent_cents: 4650
                },
                CategorySpending {
                    category_id: rent,
                    spent_cents: 128000
                },
            ]
        );
    }

    #[tokio::test]
    async fn ignores_income_positive_amounts() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool, "TR").await;
        let income = seed_category(&pool, "TestIncome").await;

        insert_tx(&pool, acc, Some(income), "2026-05-30", 250000).await;
        let result = monthly_spending(&pool, 2026, 5).await.unwrap();
        assert!(
            result.is_empty(),
            "income should be filtered out: {result:?}"
        );
    }

    #[tokio::test]
    async fn ignores_other_months() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool, "TR").await;
        let cat = seed_category(&pool, "TestCat").await;

        insert_tx(&pool, acc, Some(cat), "2026-04-30", -1000).await;
        insert_tx(&pool, acc, Some(cat), "2026-05-15", -2000).await;
        insert_tx(&pool, acc, Some(cat), "2026-06-01", -3000).await;

        let result = monthly_spending(&pool, 2026, 5).await.unwrap();
        assert_eq!(
            result,
            vec![CategorySpending {
                category_id: cat,
                spent_cents: 2000
            }]
        );
    }

    #[tokio::test]
    async fn ignores_transactions_without_category() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool, "TR").await;
        let cat = seed_category(&pool, "TestCat").await;

        insert_tx(&pool, acc, None, "2026-05-10", -5000).await;
        insert_tx(&pool, acc, Some(cat), "2026-05-11", -2500).await;

        let result = monthly_spending(&pool, 2026, 5).await.unwrap();
        assert_eq!(
            result,
            vec![CategorySpending {
                category_id: cat,
                spent_cents: 2500
            }]
        );
    }

    #[tokio::test]
    async fn december_wraps_to_next_year() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool, "TR").await;
        let cat = seed_category(&pool, "TestCat").await;

        insert_tx(&pool, acc, Some(cat), "2026-12-15", -1500).await;
        insert_tx(&pool, acc, Some(cat), "2027-01-02", -9999).await;

        let result = monthly_spending(&pool, 2026, 12).await.unwrap();
        assert_eq!(
            result,
            vec![CategorySpending {
                category_id: cat,
                spent_cents: 1500
            }]
        );
    }

    #[tokio::test]
    async fn category_breakdown_arbitrary_range() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool, "TR").await;
        let groceries = seed_category(&pool, "G").await;
        let rent = seed_category(&pool, "R").await;

        insert_tx(&pool, acc, Some(groceries), "2026-04-01", -100).await;
        insert_tx(&pool, acc, Some(groceries), "2026-05-15", -200).await;
        insert_tx(&pool, acc, Some(groceries), "2026-06-01", -400).await; // boundary: excluded
        insert_tx(&pool, acc, Some(rent), "2026-05-01", -50_000).await;
        insert_tx(&pool, acc, Some(rent), "2026-05-20", 25_000).await; // income, ignored
        insert_tx(&pool, acc, None, "2026-05-10", -9_999).await; // no cat, ignored

        let result = category_breakdown(&pool, "2026-04-01", "2026-06-01")
            .await
            .unwrap();
        assert_eq!(
            result,
            vec![
                CategorySpending {
                    category_id: groceries,
                    spent_cents: 300
                },
                CategorySpending {
                    category_id: rent,
                    spent_cents: 50_000
                },
            ]
        );
    }

    #[tokio::test]
    async fn monthly_cashflow_empty_zero_filled() {
        let pool = connect_memory().await.unwrap();
        let result = monthly_cashflow(&pool, 2026, 5, 3).await.unwrap();
        assert_eq!(
            result,
            vec![
                MonthlyFlow {
                    year: 2026,
                    month: 3,
                    in_cents: 0,
                    out_cents: 0
                },
                MonthlyFlow {
                    year: 2026,
                    month: 4,
                    in_cents: 0,
                    out_cents: 0
                },
                MonthlyFlow {
                    year: 2026,
                    month: 5,
                    in_cents: 0,
                    out_cents: 0
                },
            ]
        );
    }

    #[tokio::test]
    async fn monthly_cashflow_splits_in_out() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool, "TR").await;
        let cat = seed_category(&pool, "C").await;

        insert_tx(&pool, acc, Some(cat), "2026-04-10", 300_000).await; // income
        insert_tx(&pool, acc, Some(cat), "2026-04-20", -120_000).await; // expense
        insert_tx(&pool, acc, None, "2026-04-25", -5_000).await; // uncat counts in cashflow
        insert_tx(&pool, acc, Some(cat), "2026-05-05", 280_000).await;
        insert_tx(&pool, acc, Some(cat), "2026-05-15", -90_000).await;

        let result = monthly_cashflow(&pool, 2026, 5, 2).await.unwrap();
        assert_eq!(
            result,
            vec![
                MonthlyFlow {
                    year: 2026,
                    month: 4,
                    in_cents: 300_000,
                    out_cents: 125_000
                },
                MonthlyFlow {
                    year: 2026,
                    month: 5,
                    in_cents: 280_000,
                    out_cents: 90_000
                },
            ]
        );
    }

    #[tokio::test]
    async fn monthly_cashflow_ignores_outside_range() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool, "TR").await;

        insert_tx(&pool, acc, None, "2026-01-15", 100_000).await;
        insert_tx(&pool, acc, None, "2026-05-15", -50_000).await;
        insert_tx(&pool, acc, None, "2026-06-01", -77).await; // after end month

        let result = monthly_cashflow(&pool, 2026, 5, 3).await.unwrap();
        assert_eq!(
            result,
            vec![
                MonthlyFlow {
                    year: 2026,
                    month: 3,
                    in_cents: 0,
                    out_cents: 0
                },
                MonthlyFlow {
                    year: 2026,
                    month: 4,
                    in_cents: 0,
                    out_cents: 0
                },
                MonthlyFlow {
                    year: 2026,
                    month: 5,
                    in_cents: 0,
                    out_cents: 50_000
                },
            ]
        );
    }

    #[tokio::test]
    async fn daily_spending_empty_zero_filled() {
        let pool = connect_memory().await.unwrap();
        let result = daily_spending(&pool, 2026, 5).await.unwrap();
        assert_eq!(result.len(), 31);
        assert!(result.iter().all(|&v| v == 0));
    }

    #[tokio::test]
    async fn daily_spending_sums_per_day() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool, "TR").await;
        let cat = seed_category(&pool, "C").await;

        insert_tx(&pool, acc, Some(cat), "2026-05-03", -1_200).await;
        insert_tx(&pool, acc, Some(cat), "2026-05-03", -800).await;
        insert_tx(&pool, acc, None, "2026-05-15", -5_000).await; // uncategorized counts
        insert_tx(&pool, acc, Some(cat), "2026-05-31", -99).await;

        let result = daily_spending(&pool, 2026, 5).await.unwrap();
        assert_eq!(result.len(), 31);
        assert_eq!(result[2], 2_000, "day 3 sum");
        assert_eq!(result[14], 5_000, "day 15 uncategorized");
        assert_eq!(result[30], 99, "day 31");
        let zero_days: usize = result.iter().filter(|&&v| v == 0).count();
        assert_eq!(zero_days, 28);
    }

    #[tokio::test]
    async fn daily_spending_ignores_income() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool, "TR").await;

        insert_tx(&pool, acc, None, "2026-05-05", 250_000).await; // income
        insert_tx(&pool, acc, None, "2026-05-05", -1_500).await;

        let result = daily_spending(&pool, 2026, 5).await.unwrap();
        assert_eq!(result[4], 1_500);
        assert_eq!(result.iter().sum::<i64>(), 1_500);
    }

    #[tokio::test]
    async fn daily_spending_ignores_other_months() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool, "TR").await;

        insert_tx(&pool, acc, None, "2026-04-30", -1_000).await;
        insert_tx(&pool, acc, None, "2026-05-01", -2_000).await;
        insert_tx(&pool, acc, None, "2026-06-01", -3_000).await;

        let result = daily_spending(&pool, 2026, 5).await.unwrap();
        assert_eq!(result[0], 2_000);
        assert_eq!(result.iter().sum::<i64>(), 2_000);
    }

    #[tokio::test]
    async fn daily_spending_length_per_month() {
        let pool = connect_memory().await.unwrap();
        // 31-day month
        assert_eq!(daily_spending(&pool, 2026, 1).await.unwrap().len(), 31);
        // 30-day month
        assert_eq!(daily_spending(&pool, 2026, 4).await.unwrap().len(), 30);
        // Non-leap February
        assert_eq!(daily_spending(&pool, 2026, 2).await.unwrap().len(), 28);
        // Leap February
        assert_eq!(daily_spending(&pool, 2024, 2).await.unwrap().len(), 29);
        // Century non-leap
        assert_eq!(daily_spending(&pool, 2100, 2).await.unwrap().len(), 28);
        // 400-year leap
        assert_eq!(daily_spending(&pool, 2000, 2).await.unwrap().len(), 29);
    }

    #[tokio::test]
    async fn daily_spending_december_wraps() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool, "TR").await;

        insert_tx(&pool, acc, None, "2026-12-31", -1_111).await;
        insert_tx(&pool, acc, None, "2027-01-01", -9_999).await;

        let result = daily_spending(&pool, 2026, 12).await.unwrap();
        assert_eq!(result.len(), 31);
        assert_eq!(result[30], 1_111);
        assert_eq!(result.iter().sum::<i64>(), 1_111);
    }

    #[tokio::test]
    async fn monthly_cashflow_wraps_year_boundary() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool, "TR").await;

        insert_tx(&pool, acc, None, "2026-12-15", -1_000).await;
        insert_tx(&pool, acc, None, "2027-01-10", 5_000).await;
        insert_tx(&pool, acc, None, "2027-01-20", -2_000).await;

        let result = monthly_cashflow(&pool, 2027, 1, 2).await.unwrap();
        assert_eq!(
            result,
            vec![
                MonthlyFlow {
                    year: 2026,
                    month: 12,
                    in_cents: 0,
                    out_cents: 1_000
                },
                MonthlyFlow {
                    year: 2027,
                    month: 1,
                    in_cents: 5_000,
                    out_cents: 2_000
                },
            ]
        );
    }

    #[tokio::test]
    async fn account_monthly_cashflow_isolates_target_account() {
        let pool = connect_memory().await.unwrap();
        let a = seed_account(&pool, "A").await;
        let b = seed_account(&pool, "B").await;

        insert_tx(&pool, a, None, "2026-05-05", 300_000).await;
        insert_tx(&pool, a, None, "2026-05-20", -120_000).await;
        insert_tx(&pool, b, None, "2026-05-05", 999_999).await;
        insert_tx(&pool, b, None, "2026-05-20", -50_000).await;

        let only_a = account_monthly_cashflow(&pool, a, 2026, 5, 1)
            .await
            .unwrap();
        assert_eq!(
            only_a,
            vec![MonthlyFlow {
                year: 2026,
                month: 5,
                in_cents: 300_000,
                out_cents: 120_000
            }]
        );

        let only_b = account_monthly_cashflow(&pool, b, 2026, 5, 1)
            .await
            .unwrap();
        assert_eq!(
            only_b,
            vec![MonthlyFlow {
                year: 2026,
                month: 5,
                in_cents: 999_999,
                out_cents: 50_000
            }]
        );

        // Unfiltered monthly_cashflow sees both.
        let combined = monthly_cashflow(&pool, 2026, 5, 1).await.unwrap();
        assert_eq!(
            combined,
            vec![MonthlyFlow {
                year: 2026,
                month: 5,
                in_cents: 1_299_999,
                out_cents: 170_000
            }]
        );
    }

    #[tokio::test]
    async fn account_monthly_cashflow_includes_subtree() {
        use crate::db::accounts::create_account;
        let pool = connect_memory().await.unwrap();
        let parent = create_account(&pool, "p", "bank", "EUR", None, None, None)
            .await
            .unwrap();
        let child = create_account(&pool, "c", "cash", "EUR", Some(parent.id), None, None)
            .await
            .unwrap();

        sqlx::query(
            "INSERT INTO transactions
                (account_id, booking_date, amount_cents, currency, counterparty, source)
             VALUES
                (?1, '2026-05-15',  500, 'EUR', 'a', 'manual'),
                (?2, '2026-05-20', -200, 'EUR', 'b', 'manual')",
        )
        .bind(parent.id)
        .bind(child.id)
        .execute(&pool)
        .await
        .unwrap();

        let rows = account_monthly_cashflow(&pool, parent.id, 2026, 5, 1)
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].in_cents, 500);
        assert_eq!(rows[0].out_cents, 200);
    }

    #[tokio::test]
    async fn account_monthly_cashflow_zero_filled_for_unknown_account() {
        let pool = connect_memory().await.unwrap();
        let result = account_monthly_cashflow(&pool, 12345, 2026, 5, 3)
            .await
            .unwrap();
        assert_eq!(
            result,
            vec![
                MonthlyFlow {
                    year: 2026,
                    month: 3,
                    in_cents: 0,
                    out_cents: 0
                },
                MonthlyFlow {
                    year: 2026,
                    month: 4,
                    in_cents: 0,
                    out_cents: 0
                },
                MonthlyFlow {
                    year: 2026,
                    month: 5,
                    in_cents: 0,
                    out_cents: 0
                },
            ]
        );
    }

    #[tokio::test]
    async fn cashflow_breakdown_empty_db_returns_empty() {
        let pool = connect_memory().await.unwrap();
        let result = cashflow_breakdown(&pool, "2026-01-01", "2026-06-01")
            .await
            .unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn cashflow_breakdown_groups_by_category_and_sign() {
        let pool = connect_memory().await.unwrap();
        let (acc,): (i64,) = sqlx::query_as(
            "INSERT INTO accounts (name, kind, currency)
             VALUES ('Giro', 'bank', 'EUR') RETURNING id",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        let (cat_income,): (i64,) = sqlx::query_as(
            "INSERT INTO categories (parent_id, name) VALUES (NULL, 'Gehalt') RETURNING id",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        let (cat_food,): (i64,) = sqlx::query_as(
            "INSERT INTO categories (parent_id, name) VALUES (NULL, 'Lebensmittel') RETURNING id",
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        for &(date, amt, cat) in &[
            ("2026-05-01", 300_000_i64, Some(cat_income)),
            ("2026-05-15", 300_000, Some(cat_income)),
            ("2026-05-05", -20_000, Some(cat_food)),
            ("2026-05-10", -15_000, Some(cat_food)),
            ("2026-05-20", -25_000, Some(cat_food)),
            ("2026-05-12", -10_000, None::<i64>),
        ] {
            sqlx::query(
                "INSERT INTO transactions
                    (account_id, booking_date, amount_cents, currency,
                     category_id, source, kind, imported_at)
                 VALUES (?1, ?2, ?3, 'EUR', ?4, 'manual', 'expense', '2026-05-20T00:00:00Z')",
            )
            .bind(acc)
            .bind(date)
            .bind(amt)
            .bind(cat)
            .execute(&pool)
            .await
            .unwrap();
        }

        let result = cashflow_breakdown(&pool, "2026-05-01", "2026-06-01")
            .await
            .unwrap();
        assert_eq!(result.len(), 3);

        let income = result.iter().find(|s| s.sign == 1).unwrap();
        assert_eq!(income.category_id, Some(cat_income));
        assert_eq!(income.sum_abs_cents, 600_000);

        let food = result
            .iter()
            .find(|s| s.category_id == Some(cat_food))
            .unwrap();
        assert_eq!(food.sign, -1);
        assert_eq!(food.sum_abs_cents, 60_000);

        let uncat = result.iter().find(|s| s.category_id.is_none()).unwrap();
        assert_eq!(uncat.sign, -1);
        assert_eq!(uncat.sum_abs_cents, 10_000);
    }

    #[tokio::test]
    async fn cashflow_breakdown_ignores_zero_amount() {
        let pool = connect_memory().await.unwrap();
        let (acc,): (i64,) = sqlx::query_as(
            "INSERT INTO accounts (name, kind, currency)
             VALUES ('Giro', 'bank', 'EUR') RETURNING id",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO transactions
                (account_id, booking_date, amount_cents, currency,
                 source, kind, imported_at)
             VALUES (?1, '2026-05-15', 0, 'EUR', 'manual', 'corporate_action', '2026-05-20T00:00:00Z')"
        )
        .bind(acc).execute(&pool).await.unwrap();

        let result = cashflow_breakdown(&pool, "2026-05-01", "2026-06-01")
            .await
            .unwrap();
        assert!(result.is_empty(), "amount=0 tx must be filtered out");
    }

    // --- Transfer-exclusion tests ---

    #[tokio::test]
    async fn monthly_spending_excludes_transfers() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool, "Giro").await;
        let cat = seed_category(&pool, "Cat").await;

        // Regular expense: -100
        insert_tx_with_kind(&pool, acc, Some(cat), "2026-05-10", -10_000, "expense").await;
        // Transfer: -500 — must NOT appear in spending
        insert_tx_with_kind(&pool, acc, Some(cat), "2026-05-15", -50_000, "transfer").await;

        let result = monthly_spending(&pool, 2026, 5).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0].spent_cents, 10_000,
            "transfer must be excluded from monthly_spending"
        );
    }

    #[tokio::test]
    async fn category_breakdown_excludes_transfers() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool, "Giro").await;
        let cat = seed_category(&pool, "Cat").await;

        insert_tx_with_kind(&pool, acc, Some(cat), "2026-05-10", -10_000, "expense").await;
        insert_tx_with_kind(&pool, acc, Some(cat), "2026-05-15", -50_000, "transfer").await;

        let result = category_breakdown(&pool, "2026-05-01", "2026-06-01")
            .await
            .unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0].spent_cents, 10_000,
            "transfer must be excluded from category_breakdown"
        );
    }

    #[tokio::test]
    async fn daily_spending_excludes_transfers() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool, "Giro").await;

        insert_tx_with_kind(&pool, acc, None, "2026-05-10", -10_000, "expense").await;
        insert_tx_with_kind(&pool, acc, None, "2026-05-10", -50_000, "transfer").await;

        let result = daily_spending(&pool, 2026, 5).await.unwrap();
        assert_eq!(
            result[9], 10_000,
            "transfer must be excluded from daily_spending"
        );
        assert_eq!(result.iter().sum::<i64>(), 10_000);
    }

    #[tokio::test]
    async fn cashflow_breakdown_excludes_transfers() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool, "Giro").await;

        insert_tx_with_kind(&pool, acc, None, "2026-05-10", -10_000, "expense").await;
        insert_tx_with_kind(&pool, acc, None, "2026-05-10", -50_000, "transfer").await;

        let result = cashflow_breakdown(&pool, "2026-05-01", "2026-06-01")
            .await
            .unwrap();
        // Only the expense row should appear
        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0].sum_abs_cents, 10_000,
            "transfer must be excluded from cashflow_breakdown"
        );
    }

    #[tokio::test]
    async fn monthly_cashflow_filtered_excludes_transfers() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool, "Giro").await;

        insert_tx_with_kind(&pool, acc, None, "2026-05-10", -10_000, "expense").await;
        insert_tx_with_kind(&pool, acc, None, "2026-05-10", -50_000, "transfer").await;

        let result = monthly_cashflow(&pool, 2026, 5, 1).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0].out_cents, 10_000,
            "transfer must be excluded from monthly_cashflow_filtered"
        );
        assert_eq!(result[0].in_cents, 0);
    }

    #[tokio::test]
    async fn bucket_monthly_flow_excludes_transfers() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool, "Giro").await;
        let (bucket_id,): (i64,) =
            sqlx::query_as("INSERT INTO buckets (name) VALUES ('TestBucket') RETURNING id")
                .fetch_one(&pool)
                .await
                .unwrap();

        // Income expense with bucket
        sqlx::query(
            "INSERT INTO transactions
                (account_id, booking_date, amount_cents, currency, counterparty,
                 source, kind, bucket_id)
             VALUES (?1, '2026-05-10', 10_000, 'EUR', 'income-cp', 'manual', 'income', ?2)",
        )
        .bind(acc)
        .bind(bucket_id)
        .execute(&pool)
        .await
        .unwrap();

        // Transfer with same bucket — should be excluded
        sqlx::query(
            "INSERT INTO transactions
                (account_id, booking_date, amount_cents, currency, counterparty,
                 source, kind, bucket_id)
             VALUES (?1, '2026-05-10', 50_000, 'EUR', 'transfer-cp', 'manual', 'transfer', ?2)",
        )
        .bind(acc)
        .bind(bucket_id)
        .execute(&pool)
        .await
        .unwrap();

        let result = bucket_monthly_flow(&pool, bucket_id, 2026, 5, 1)
            .await
            .unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0].in_cents, 10_000,
            "transfer must be excluded from bucket_monthly_flow"
        );
        assert_eq!(result[0].out_cents, 0);
    }

    #[tokio::test]
    async fn monthly_cashflow_includes_trade_kinds() {
        let pool = connect_memory().await.unwrap();
        let acc: i64 = sqlx::query_scalar(
            "INSERT INTO accounts (name, kind, currency) VALUES ('TR', 'broker', 'EUR') RETURNING id"
        ).fetch_one(&pool).await.unwrap();
        // expense + buy + fee + transfer (excluded)
        sqlx::query(
            "INSERT INTO transactions (account_id, booking_date, amount_cents, currency, kind, source)
             VALUES (?1, '2026-05-15', -1000, 'EUR', 'expense', 'manual'),
                    (?1, '2026-05-15', -100000, 'EUR', 'buy', 'manual'),
                    (?1, '2026-05-15', -100, 'EUR', 'fee', 'manual'),
                    (?1, '2026-05-15', 50000, 'EUR', 'sell', 'manual'),
                    (?1, '2026-05-15', -200, 'EUR', 'transfer', 'manual')"
        ).bind(acc).execute(&pool).await.unwrap();

        let flows = monthly_cashflow(&pool, 2026, 5, 1).await.unwrap();
        assert_eq!(flows.len(), 1);
        // buy now counts as expense, sell as income; transfer stays excluded.
        assert_eq!(flows[0].in_cents, 50000, "sell should count as income");
        assert_eq!(
            flows[0].out_cents, 101100,
            "expense+buy+fee = 1000+100000+100"
        );
    }
}
