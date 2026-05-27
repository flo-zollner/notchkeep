use serde::Serialize;
use sqlx::SqlitePool;

use super::aggregates::{month_buckets, step_month};
use super::{DbError, DbResult};

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NetWorthPoint {
    pub year: i32,
    pub month: u32,
    pub total_cents: i64,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NetWorthForecastPoint {
    pub year: i32,
    pub month: u32,
    pub mid_cents: i64,
    pub lo_cents: i64,
    pub hi_cents: i64,
}

/// Cumulative balance sum of all non-archived accounts at month-end.
/// `months = 0` returns the series from the month of the first transaction;
/// otherwise the last `months` buckets up to and including `(end_year, end_month)`.
pub async fn net_worth_history(
    pool: &SqlitePool,
    end_year: i32,
    end_month: u32,
    months: u32,
) -> DbResult<Vec<NetWorthPoint>> {
    // Find the first tx (for months=0 and for the "empty DB" short-circuit response).
    // SELECT MIN(...) always returns one row; the value is NULL when no transaction exists.
    let (first_opt,): (Option<String>,) = sqlx::query_as(
        "SELECT MIN(t.booking_date)
         FROM transactions t
         JOIN accounts a ON a.id = t.account_id
         WHERE a.archived = 0",
    )
    .fetch_one(pool)
    .await?;
    let Some(first_date) = first_opt else {
        return Ok(Vec::new());
    };

    // Determine the range.
    let buckets: Vec<(i32, u32)> = if months == 0 {
        let first_year: i32 = first_date[0..4]
            .parse()
            .map_err(|e: std::num::ParseIntError| DbError::Decode(format!("first year: {e}")))?;
        let first_month: u32 = first_date[5..7]
            .parse()
            .map_err(|e: std::num::ParseIntError| DbError::Decode(format!("first month: {e}")))?;
        // Number of months from (first_year, first_month) to (end_year, end_month) inclusive.
        let span = (end_year * 12 + end_month as i32) - (first_year * 12 + first_month as i32);
        if span < 0 {
            return Ok(Vec::new());
        }
        month_buckets(end_year, end_month, (span + 1) as u32)
    } else {
        month_buckets(end_year, end_month, months)
    };

    if buckets.is_empty() {
        return Ok(Vec::new());
    }

    // Monthly deltas from the beginning of time up to the end of the range.
    let (after_y, after_m) = step_month(end_year, end_month, 1);
    let upper_bound = format!("{after_y:04}-{after_m:02}-01");

    let rows: Vec<(String, String, i64)> = sqlx::query_as(
        "SELECT
            strftime('%Y', t.booking_date) AS year,
            strftime('%m', t.booking_date) AS month,
            CAST(SUM(t.amount_cents) AS INTEGER) AS delta_cents
         FROM transactions t
         JOIN accounts a ON a.id = t.account_id
         WHERE a.archived = 0
           AND t.kind != 'transfer'
           AND t.booking_date < ?1
         GROUP BY year, month
         ORDER BY year, month",
    )
    .bind(&upper_bound)
    .fetch_all(pool)
    .await?;

    // Accumulate and populate buckets. Buckets before the first tx get 0.
    let mut deltas: Vec<((i32, u32), i64)> = Vec::with_capacity(rows.len());
    for (y_str, m_str, delta_cents) in rows {
        let y: i32 = y_str
            .parse()
            .map_err(|e: std::num::ParseIntError| DbError::Decode(format!("year: {e}")))?;
        let m: u32 = m_str
            .parse()
            .map_err(|e: std::num::ParseIntError| DbError::Decode(format!("month: {e}")))?;
        deltas.push(((y, m), delta_cents));
    }

    let mut iter = deltas.into_iter().peekable();
    let mut running: i64 = 0;
    let mut out: Vec<NetWorthPoint> = Vec::with_capacity(buckets.len());
    for (y, m) in buckets {
        while let Some(&((dy, dm), _)) = iter.peek() {
            if (dy, dm) <= (y, m) {
                let (_, d) = iter.next().unwrap();
                running += d;
            } else {
                break;
            }
        }
        out.push(NetWorthPoint { year: y, month: m, total_cents: running });
    }

    // 6e: Add portfolio market value per bucket (last day of month).
    for point in &mut out {
        let (next_y, next_m) = step_month(point.year, point.month, 1);
        let last_day = chrono::NaiveDate::from_ymd_opt(next_y, next_m, 1)
            .and_then(|d| d.pred_opt())
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| format!("{:04}-{:02}-28", point.year, point.month));
        let portfolio_value = crate::db::portfolio::portfolio_value_on_date(pool, &last_day).await?;
        point.total_cents += portfolio_value;
    }

    Ok(out)
}

/// Linear forecast from the mean/stddev of the monthly deltas
/// over the last `history_window` months. Per forecast month `i`:
/// `mid = last + μ·i`, `lo = mid − σ·√i`, `hi = mid + σ·√i`.
pub async fn net_worth_forecast(
    pool: &SqlitePool,
    end_year: i32,
    end_month: u32,
    history_window: u32,
    forecast_months: u32,
) -> DbResult<Vec<NetWorthForecastPoint>> {
    if forecast_months == 0 {
        return Ok(Vec::new());
    }
    let window = history_window.max(1);
    let history = net_worth_history(pool, end_year, end_month, window).await?;
    if history.is_empty() {
        return Ok(Vec::new());
    }

    let last = history.last().expect("history non-empty").total_cents;

    // Monthly deltas.
    let deltas: Vec<i64> = history
        .windows(2)
        .map(|w| w[1].total_cents - w[0].total_cents)
        .collect();

    let (mu, sigma) = if deltas.len() < 2 {
        let m = if deltas.is_empty() {
            0.0
        } else {
            deltas[0] as f64
        };
        (m, 0.0)
    } else {
        let n = deltas.len() as f64;
        let mean = deltas.iter().sum::<i64>() as f64 / n;
        let var = deltas
            .iter()
            .map(|d| {
                let x = *d as f64 - mean;
                x * x
            })
            .sum::<f64>()
            / (n - 1.0);
        (mean, var.sqrt())
    };

    let mut out: Vec<NetWorthForecastPoint> = Vec::with_capacity(forecast_months as usize);
    for i in 1..=forecast_months {
        let (y, m) = step_month(end_year, end_month, i as i32);
        let i_f = i as f64;
        let mid = last + (mu * i_f).round() as i64;
        let band = (sigma * i_f.sqrt()).round() as i64;
        out.push(NetWorthForecastPoint {
            year: y,
            month: m,
            mid_cents: mid,
            lo_cents: mid - band,
            hi_cents: mid + band,
        });
    }
    Ok(out)
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

    async fn seed_account_archived(pool: &SqlitePool, name: &str) -> i64 {
        let (id,): (i64,) = sqlx::query_as(
            "INSERT INTO accounts (name, kind, currency, archived)
             VALUES (?1, 'bank', 'EUR', 1) RETURNING id",
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
        booking_date: &str,
        amount_cents: i64,
    ) {
        sqlx::query(
            "INSERT INTO transactions
                (account_id, booking_date, amount_cents, currency, counterparty, source)
             VALUES (?1, ?2, ?3, 'EUR', ?4, 'manual')",
        )
        .bind(account_id)
        .bind(booking_date)
        .bind(amount_cents)
        .bind(format!("seed-{booking_date}-{amount_cents}"))
        .execute(pool)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn net_worth_history_empty_db_returns_empty() {
        let pool = connect_memory().await.unwrap();
        let result = net_worth_history(&pool, 2026, 5, 12).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn net_worth_history_cumulative_across_months() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool, "A").await;
        insert_tx(&pool, acc, "2026-03-15", 100_000).await; // +1000.00
        insert_tx(&pool, acc, "2026-04-10", 50_000).await;  // +500.00
        insert_tx(&pool, acc, "2026-05-05", -20_000).await; // -200.00

        let result = net_worth_history(&pool, 2026, 5, 3).await.unwrap();
        assert_eq!(
            result,
            vec![
                NetWorthPoint { year: 2026, month: 3, total_cents: 100_000 },
                NetWorthPoint { year: 2026, month: 4, total_cents: 150_000 },
                NetWorthPoint { year: 2026, month: 5, total_cents: 130_000 },
            ]
        );
    }

    #[tokio::test]
    async fn net_worth_history_ignores_archived_accounts() {
        let pool = connect_memory().await.unwrap();
        let active = seed_account(&pool, "Active").await;
        let archived = seed_account_archived(&pool, "Archived").await;
        insert_tx(&pool, active, "2026-05-01", 100_000).await;
        insert_tx(&pool, archived, "2026-05-01", 999_999).await;

        let result = net_worth_history(&pool, 2026, 5, 1).await.unwrap();
        assert_eq!(
            result,
            vec![NetWorthPoint { year: 2026, month: 5, total_cents: 100_000 }]
        );
    }

    #[tokio::test]
    async fn net_worth_history_zero_months_starts_at_first_tx() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool, "A").await;
        insert_tx(&pool, acc, "2026-03-01", 50_000).await;
        insert_tx(&pool, acc, "2026-05-01", 30_000).await;

        let result = net_worth_history(&pool, 2026, 5, 0).await.unwrap();
        assert_eq!(
            result,
            vec![
                NetWorthPoint { year: 2026, month: 3, total_cents: 50_000 },
                NetWorthPoint { year: 2026, month: 4, total_cents: 50_000 },
                NetWorthPoint { year: 2026, month: 5, total_cents: 80_000 },
            ]
        );
    }

    #[tokio::test]
    async fn net_worth_history_wraps_year_boundary() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool, "A").await;
        insert_tx(&pool, acc, "2026-12-15", 100_000).await;
        insert_tx(&pool, acc, "2027-01-10", 25_000).await;

        let result = net_worth_history(&pool, 2027, 1, 2).await.unwrap();
        assert_eq!(
            result,
            vec![
                NetWorthPoint { year: 2026, month: 12, total_cents: 100_000 },
                NetWorthPoint { year: 2027, month: 1, total_cents: 125_000 },
            ]
        );
    }

    #[tokio::test]
    async fn net_worth_history_fills_silent_months() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool, "A").await;
        insert_tx(&pool, acc, "2026-02-01", 100_000).await;
        // No tx in March/April, then again in May.
        insert_tx(&pool, acc, "2026-05-01", 50_000).await;

        let result = net_worth_history(&pool, 2026, 5, 4).await.unwrap();
        assert_eq!(
            result,
            vec![
                NetWorthPoint { year: 2026, month: 2, total_cents: 100_000 },
                NetWorthPoint { year: 2026, month: 3, total_cents: 100_000 },
                NetWorthPoint { year: 2026, month: 4, total_cents: 100_000 },
                NetWorthPoint { year: 2026, month: 5, total_cents: 150_000 },
            ]
        );
    }

    #[tokio::test]
    async fn net_worth_history_multiple_accounts_sum_correctly() {
        let pool = connect_memory().await.unwrap();
        let a = seed_account(&pool, "A").await;
        let b = seed_account(&pool, "B").await;
        insert_tx(&pool, a, "2026-05-01", 200_000).await;
        insert_tx(&pool, b, "2026-05-15", -50_000).await;

        let result = net_worth_history(&pool, 2026, 5, 1).await.unwrap();
        assert_eq!(
            result,
            vec![NetWorthPoint { year: 2026, month: 5, total_cents: 150_000 }]
        );
    }

    #[tokio::test]
    async fn net_worth_forecast_empty_history_returns_empty() {
        let pool = connect_memory().await.unwrap();
        let result = net_worth_forecast(&pool, 2026, 5, 6, 6).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn net_worth_forecast_zero_months_returns_empty() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool, "A").await;
        insert_tx(&pool, acc, "2026-05-01", 100_000).await;
        let result = net_worth_forecast(&pool, 2026, 5, 6, 0).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn net_worth_forecast_flat_history_zero_sigma() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool, "A").await;
        // History: delta 0 every month → flat balance at 100k.
        insert_tx(&pool, acc, "2025-12-01", 100_000).await;

        let result = net_worth_forecast(&pool, 2026, 5, 6, 3).await.unwrap();
        assert_eq!(result.len(), 3);
        let expected_months = [6_u32, 7, 8]; // Jun, Jul, Aug nach end_month=5
        for (i, p) in result.iter().enumerate() {
            assert_eq!(p.month, expected_months[i], "month {i}");
            assert_eq!(p.year, 2026);
            assert_eq!(p.mid_cents, 100_000, "mid {i}");
            assert_eq!(p.lo_cents, 100_000, "lo {i}");
            assert_eq!(p.hi_cents, 100_000, "hi {i}");
        }
    }

    #[tokio::test]
    async fn net_worth_forecast_linear_drift_zero_sigma() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool, "A").await;
        // History: uniform +10_000 per month over 6 months.
        insert_tx(&pool, acc, "2025-12-01", 100_000).await;
        insert_tx(&pool, acc, "2026-01-01", 10_000).await;
        insert_tx(&pool, acc, "2026-02-01", 10_000).await;
        insert_tx(&pool, acc, "2026-03-01", 10_000).await;
        insert_tx(&pool, acc, "2026-04-01", 10_000).await;
        insert_tx(&pool, acc, "2026-05-01", 10_000).await;

        let result = net_worth_forecast(&pool, 2026, 5, 6, 3).await.unwrap();
        assert_eq!(result.len(), 3);
        // History window 6 months → 5 Δ values, all 10_000 → μ=10_000, σ=0.
        // last = 150_000 (100k + 5×10k).
        assert_eq!(result[0], NetWorthForecastPoint { year: 2026, month: 6, mid_cents: 160_000, lo_cents: 160_000, hi_cents: 160_000 });
        assert_eq!(result[1], NetWorthForecastPoint { year: 2026, month: 7, mid_cents: 170_000, lo_cents: 170_000, hi_cents: 170_000 });
        assert_eq!(result[2], NetWorthForecastPoint { year: 2026, month: 8, mid_cents: 180_000, lo_cents: 180_000, hi_cents: 180_000 });
    }

    #[tokio::test]
    async fn net_worth_includes_portfolio_value() {
        let pool = connect_memory().await.unwrap();
        let (acc_id,): (i64,) = sqlx::query_as(
            "INSERT INTO accounts (name, kind, currency)
             VALUES ('Broker', 'broker', 'EUR') RETURNING id"
        ).fetch_one(&pool).await.unwrap();
        let (sec_id,): (i64,) = sqlx::query_as(
            "INSERT INTO securities (isin, name, currency, asset_type)
             VALUES ('US0378331005', 'Apple', 'EUR', 'stock') RETURNING id"
        ).fetch_one(&pool).await.unwrap();
        // Initial Cash-Tx +1000
        sqlx::query(
            "INSERT INTO transactions (account_id, booking_date, amount_cents, currency, source, kind, imported_at)
             VALUES (?1, '2026-01-15', 100000, 'EUR', 'manual', 'income', '2026-01-15T00:00:00Z')"
        ).bind(acc_id).execute(&pool).await.unwrap();
        // Buy 10 shares @ €50 = -500
        let (tx_id,): (i64,) = sqlx::query_as(
            "INSERT INTO transactions (account_id, booking_date, amount_cents, currency, source, kind, imported_at)
             VALUES (?1, '2026-02-15', -50000, 'EUR', 'manual', 'buy', '2026-02-15T00:00:00Z')
             RETURNING id"
        ).bind(acc_id).fetch_one(&pool).await.unwrap();
        sqlx::query(
            "INSERT INTO securities_trades (tx_id, security_id, side, shares_micro, unit_price_micro, fee_cents, tax_cents)
             VALUES (?1, ?2, 'buy', 10000000, 50000000, 0, 0)"
        ).bind(tx_id).bind(sec_id).execute(&pool).await.unwrap();
        // Price €70 to month-end February
        crate::db::prices::upsert_price(&pool, sec_id, "2026-02-28", 70_000_000, "yahoo")
            .await.unwrap();

        let hist = net_worth_history(&pool, 2026, 2, 2).await.unwrap();
        assert_eq!(hist.len(), 2);
        // Jan: 100_000 cash, no portfolio
        assert_eq!(hist[0].total_cents, 100_000);
        // Feb: 100_000 - 50_000 cash + portfolio (10 × €70 = 70_000) = 120_000
        assert_eq!(hist[1].total_cents, 120_000);
    }

    #[tokio::test]
    async fn net_worth_forecast_variance_widens_cone() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool, "A").await;
        // History: Δ values 0, 20_000, 0, 20_000, 0 → μ=8_000, σ_sample ≈ 10_954.5.
        insert_tx(&pool, acc, "2025-12-01", 100_000).await;
        // Jan: no tx → Δ=0
        insert_tx(&pool, acc, "2026-02-01", 20_000).await; // Δ=20_000
        // Mar: no tx → Δ=0
        insert_tx(&pool, acc, "2026-04-01", 20_000).await; // Δ=20_000
        // May: no tx → Δ=0

        let result = net_worth_forecast(&pool, 2026, 5, 6, 3).await.unwrap();
        assert_eq!(result.len(), 3);
        // last = 100k + 0 + 20k + 0 + 20k + 0 = 140_000
        // μ = (0+20k+0+20k+0)/5 = 8_000
        // σ (sample, n=5, n-1=4): variance = ((0-8k)² + (20k-8k)² + (0-8k)² + (20k-8k)² + (0-8k)²) / 4
        //                                  = (64M + 144M + 64M + 144M + 64M) / 4 = 480M / 4 = 120M
        //                                  → σ ≈ 10_954.45 → rounded 10_954
        // mid_i = 140_000 + 8_000·i; σ·√i.
        // i=1: mid=148_000, σ·1≈10_954 → lo=137_046, hi=158_954
        // i=2: mid=156_000, σ·√2≈15_491 → lo=140_509, hi=171_491
        // i=3: mid=164_000, σ·√3≈18_972 → lo=145_028, hi=182_972
        assert_eq!(result[0].mid_cents, 148_000);
        assert!(result[0].hi_cents > result[0].mid_cents);
        assert!(result[0].lo_cents < result[0].mid_cents);
        assert_eq!(result[0].hi_cents - result[0].mid_cents, result[0].mid_cents - result[0].lo_cents);

        // Cone widens from i=1 to i=3.
        let width1 = result[0].hi_cents - result[0].lo_cents;
        let width3 = result[2].hi_cents - result[2].lo_cents;
        assert!(width3 > width1, "cone should widen: {width1} → {width3}");

        // Konkrete Werte
        assert_eq!(result[0].hi_cents - result[0].lo_cents, 21_908); // 2·10_954
        assert_eq!(result[1].mid_cents, 156_000);
        assert_eq!(result[2].mid_cents, 164_000);
    }
}
