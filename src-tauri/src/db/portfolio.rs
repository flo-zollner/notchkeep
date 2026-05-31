use std::collections::HashMap;

use serde::Serialize;
use sqlx::SqlitePool;

use super::{DbError, DbResult};

/// Input for the FIFO solver. A lighter struct than TradeWithTx —
/// only the fields FIFO needs.
#[derive(Debug, Clone)]
pub struct FifoTradeInput {
    pub booking_date: String,
    pub side: String,
    pub shares_micro: i64,
    pub amount_cents: i64,
    pub fee_cents: i64,
    pub tax_cents: i64,
    /// Pair identifier for `fusion_out`/`fusion_in` trades. Identical on
    /// both sides of the fusion. `None` for all other sides.
    pub fusion_group: Option<String>,
}

/// Cost-basis transfer from a fusion source side to the target side.
/// Built in a first pass over all securities and consumed in the second
/// pass at the `fusion_in` event as the acquisition value for the new lot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FusionTransfer {
    pub cost_cents: i64,
    pub acquired_date: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lot {
    pub acquired_date: String,
    pub shares_remaining_micro: i64,
    pub cost_remaining_cents: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Realized {
    pub sell_date: String,
    pub shares_consumed_micro: i64,
    pub proceeds_cents: i64,
    pub cost_basis_cents: i64,
    pub gain_cents: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Holding {
    pub security_id: i64,
    pub isin: String,
    pub symbol: Option<String>,
    pub name: String,
    pub currency: String,
    pub shares_micro: i64,
    pub cost_basis_cents: i64,
    pub avg_cost_per_share_micro: i64,
    pub market_value_cents: i64,
    pub unrealized_cents: i64,
    pub last_price_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct DividendEntry {
    pub tx_id: i64,
    pub booking_date: String,
    pub security_id: i64,
    pub security_name: String,
    pub amount_cents: i64,
    pub tax_cents: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CostBasisPoint {
    pub year: i32,
    pub month: i32,
    pub cost_basis_cents: i64,
    pub market_value_cents: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CostBasisPointDaily {
    pub date: String, // YYYY-MM-DD
    pub cost_basis_cents: i64,
    pub market_value_cents: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AllocationSlice {
    pub key: String,
    pub value_cents: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioKpis {
    pub market_value_cents: i64,
    pub cost_basis_cents: i64,
    pub unrealized_cents: i64,
    pub realized_ytd_cents: i64,
}

/// Allocation: how many micro-shares of a security are held in a bucket.
#[derive(Debug, Clone, Serialize, sqlx::FromRow, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SecurityBucketAllocation {
    pub id: i64,
    pub security_id: i64,
    pub bucket_id: i64,
    pub shares_micro: i64,
}

/// Per-row aggregate for the /buckets UI: which securities with their current
/// value are allocated to a bucket.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BucketHoldingRow {
    pub security_id: i64,
    pub security_name: String,
    pub isin: String,
    pub shares_micro: i64,
    pub value_cents: i64,
}

/// Pure FIFO solver. Expects chronologically ASC-sorted trades for exactly one
/// security. Consumes lots on sells, scales them proportionally on
/// `corporate_action` (splits), empties them on `fusion_out`, creates a new
/// lot on `fusion_in` with the cost basis supplied from `fusion_carry`.
/// Returns (remaining lots, all realized gains).
///
/// `fusion_carry`: map `fusion_group → FusionTransfer` from a prior pass over
/// ALL securities. Pass empty when no fusions exist (e.g. in tests).
pub fn fifo_apply(
    trades: &[FifoTradeInput],
    fusion_carry: &HashMap<String, FusionTransfer>,
) -> (Vec<Lot>, Vec<Realized>) {
    let mut lots: Vec<Lot> = Vec::new();
    let mut realized: Vec<Realized> = Vec::new();

    for t in trades {
        match t.side.as_str() {
            "buy" => {
                let cost = t.amount_cents.abs();
                lots.push(Lot {
                    acquired_date: t.booking_date.clone(),
                    shares_remaining_micro: t.shares_micro.abs(),
                    cost_remaining_cents: cost,
                });
            }
            "sell" => {
                let mut shares_to_consume = t.shares_micro.abs();
                let proceeds = t.amount_cents.abs();
                let mut cost_basis_acc: i64 = 0;
                let mut consumed_micro: i64 = 0;

                while shares_to_consume > 0 && !lots.is_empty() {
                    let lot = &mut lots[0];
                    if lot.shares_remaining_micro <= shares_to_consume {
                        cost_basis_acc += lot.cost_remaining_cents;
                        consumed_micro += lot.shares_remaining_micro;
                        shares_to_consume -= lot.shares_remaining_micro;
                        lots.remove(0);
                    } else {
                        let part_cost: i64 = (lot.cost_remaining_cents as i128
                            * shares_to_consume as i128
                            / lot.shares_remaining_micro as i128)
                            as i64;
                        cost_basis_acc += part_cost;
                        consumed_micro += shares_to_consume;
                        lot.shares_remaining_micro -= shares_to_consume;
                        lot.cost_remaining_cents -= part_cost;
                        shares_to_consume = 0;
                    }
                }

                realized.push(Realized {
                    sell_date: t.booking_date.clone(),
                    shares_consumed_micro: consumed_micro,
                    proceeds_cents: proceeds,
                    cost_basis_cents: cost_basis_acc,
                    gain_cents: proceeds - cost_basis_acc,
                });
            }
            "corporate_action" => {
                let total_before: i64 = lots.iter().map(|l| l.shares_remaining_micro).sum();
                if total_before == 0 {
                    continue;
                }
                let delta = t.shares_micro;
                for lot in lots.iter_mut() {
                    let lot_delta: i64 = (lot.shares_remaining_micro as i128 * delta as i128
                        / total_before as i128) as i64;
                    lot.shares_remaining_micro += lot_delta;
                }
            }
            "fusion_out" => {
                // Full write-off: all lots are cleared. The cost basis was
                // already collected in the pre-pass (`collect_fusion_transfers`)
                // and is available in `fusion_carry` — here we just discard the
                // lots. shares_micro from the record is not evaluated because a
                // fusion always covers the ENTIRE position.
                lots.clear();
            }
            "fusion_in" => {
                // Book in the new shares with the cost basis transferred from
                // the source side. Without a carry entry (data error, orphaned
                // tx) the cost basis falls back to 0 — the lot is still created
                // so the new position remains visible.
                let shares = t.shares_micro.abs();
                if shares == 0 {
                    continue;
                }
                let (cost, acquired_date) =
                    match t.fusion_group.as_ref().and_then(|g| fusion_carry.get(g)) {
                        Some(transfer) => (transfer.cost_cents, transfer.acquired_date.clone()),
                        None => (0, t.booking_date.clone()),
                    };
                lots.push(Lot {
                    acquired_date,
                    shares_remaining_micro: shares,
                    cost_remaining_cents: cost,
                });
            }
            "dividend" => {
                // Lots and realized gains are NOT touched.
            }
            _ => {
                // Unknown side → silently ignore. Schema-CHECK enforces this.
            }
        }
    }

    (lots, realized)
}

/// First pass over all trade groups (one per security): collects for each
/// `fusion_out` event the accumulated cost basis and the earliest acquisition
/// date of the lots being cleared into a map
/// `fusion_group → FusionTransfer`. The second pass (`fifo_apply`) consumes
/// this map to create a new lot with the correct acquisition value on the
/// `fusion_in` side.
///
/// Runs over ALL securities, not per security — otherwise `fifo_apply` would
/// not see the cost basis from the source security at `fusion_in`.
pub fn collect_fusion_transfers(groups: &[Vec<FifoTradeInput>]) -> HashMap<String, FusionTransfer> {
    let mut out: HashMap<String, FusionTransfer> = HashMap::new();

    for trades in groups {
        // Local lot simulation, only as far as needed to know the cost basis
        // at the `fusion_out` event. `fusion_in` without prior lots (target
        // security did not exist before) contributes nothing here — which is
        // correct, since its cost basis comes from the source security.
        let mut lots: Vec<Lot> = Vec::new();
        for t in trades {
            match t.side.as_str() {
                "buy" => {
                    lots.push(Lot {
                        acquired_date: t.booking_date.clone(),
                        shares_remaining_micro: t.shares_micro.abs(),
                        cost_remaining_cents: t.amount_cents.abs(),
                    });
                }
                "sell" => {
                    let mut to_consume = t.shares_micro.abs();
                    while to_consume > 0 && !lots.is_empty() {
                        let lot = &mut lots[0];
                        if lot.shares_remaining_micro <= to_consume {
                            to_consume -= lot.shares_remaining_micro;
                            lots.remove(0);
                        } else {
                            let part_cost = (lot.cost_remaining_cents as i128 * to_consume as i128
                                / lot.shares_remaining_micro as i128)
                                as i64;
                            lot.shares_remaining_micro -= to_consume;
                            lot.cost_remaining_cents -= part_cost;
                            to_consume = 0;
                        }
                    }
                }
                "corporate_action" => {
                    let total_before: i64 = lots.iter().map(|l| l.shares_remaining_micro).sum();
                    if total_before == 0 {
                        continue;
                    }
                    let delta = t.shares_micro;
                    for lot in lots.iter_mut() {
                        let lot_delta = (lot.shares_remaining_micro as i128 * delta as i128
                            / total_before as i128) as i64;
                        lot.shares_remaining_micro += lot_delta;
                    }
                }
                "fusion_out" => {
                    if let Some(group) = &t.fusion_group {
                        let cost_cents: i64 = lots.iter().map(|l| l.cost_remaining_cents).sum();
                        // Earliest acquisition date of the collected lots —
                        // important for German/Austrian tax rules
                        // (FIFO order is preserved).
                        let acquired_date = lots
                            .iter()
                            .map(|l| l.acquired_date.clone())
                            .min()
                            .unwrap_or_else(|| t.booking_date.clone());
                        out.insert(
                            group.clone(),
                            FusionTransfer {
                                cost_cents,
                                acquired_date,
                            },
                        );
                    }
                    lots.clear();
                }
                _ => {
                    // fusion_in / dividend / unknown: irrelevant for carry construction.
                }
            }
        }
    }

    out
}

pub async fn current_holdings(pool: &SqlitePool) -> DbResult<Vec<Holding>> {
    #[derive(sqlx::FromRow)]
    struct Row {
        security_id: i64,
        isin: String,
        symbol: Option<String>,
        name: String,
        currency: String,
        booking_date: String,
        side: String,
        shares_micro: i64,
        amount_cents: i64,
        fee_cents: i64,
        tax_cents: i64,
        fusion_group: Option<String>,
    }

    // Deliberately NO `WHERE s.archived = 0`: a source security archived after
    // a fusion carries the `fusion_out` trade plus its buy history, which
    // `collect_fusion_transfers` needs for the cost-basis carry to the target
    // security. Archived securities with zero remaining shares are cleanly
    // filtered out below via `shares_total == 0`.
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT s.id AS security_id, s.isin, s.symbol, s.name, s.currency,
                tx.booking_date, st.side, st.shares_micro, tx.amount_cents,
                st.fee_cents, st.tax_cents, st.fusion_group
           FROM securities_trades st
           JOIN transactions tx ON tx.id = st.tx_id
           JOIN securities s ON s.id = st.security_id
          ORDER BY s.id ASC, tx.booking_date ASC, tx.id ASC",
    )
    .fetch_all(pool)
    .await?;

    // 6e: Latest prices per security (Map).
    let prices_vec = crate::db::prices::latest_per_security(pool).await?;
    let price_map: std::collections::HashMap<i64, (String, i64)> = prices_vec
        .into_iter()
        .map(|(id, d, c)| (id, (d, c)))
        .collect();

    // Pass 1: group trades by security and build the fusion cost carry.
    let groups = group_trades_by_security(
        &rows
            .iter()
            .map(|r| TradeRowRef {
                security_id: r.security_id,
                booking_date: &r.booking_date,
                side: &r.side,
                shares_micro: r.shares_micro,
                amount_cents: r.amount_cents,
                fee_cents: r.fee_cents,
                tax_cents: r.tax_cents,
                fusion_group: r.fusion_group.as_deref(),
            })
            .collect::<Vec<_>>(),
    );
    let fusion_carry = collect_fusion_transfers(&groups);

    // Parallel metadata list (security headers). `group_trades_by_security`
    // produces one group per security in the same order as the rows.
    let mut meta: Vec<(i64, String, Option<String>, String, String)> = Vec::new();
    {
        let mut current: Option<i64> = None;
        for r in &rows {
            if Some(r.security_id) != current {
                meta.push((
                    r.security_id,
                    r.isin.clone(),
                    r.symbol.clone(),
                    r.name.clone(),
                    r.currency.clone(),
                ));
                current = Some(r.security_id);
            }
        }
    }

    // Pass 2: run FIFO with carry per security.
    let mut holdings: Vec<Holding> = Vec::new();
    for (group_idx, trades) in groups.iter().enumerate() {
        let (sec_id, isin, symbol, name, currency) = {
            let m = &meta[group_idx];
            (m.0, m.1.clone(), m.2.clone(), m.3.clone(), m.4.clone())
        };

        let (lots, _) = fifo_apply(trades, &fusion_carry);
        let shares_total: i64 = lots.iter().map(|l| l.shares_remaining_micro).sum();
        if shares_total == 0 {
            continue;
        }
        let cost_total: i64 = lots.iter().map(|l| l.cost_remaining_cents).sum();
        let avg_cost_per_share_micro: i64 = if shares_total > 0 {
            (cost_total as i128 * 10_000_000_000_i128 / shares_total as i128) as i64
        } else {
            0
        };

        // 6e: Market value with cost fallback when price data is missing.
        let (last_price_date, market_value_cents, unrealized_cents) =
            if let Some((date, close_micro)) = price_map.get(&sec_id) {
                let fx_rate = crate::db::fx::rate_on_date(pool, &currency, date)
                    .await?
                    .unwrap_or(1_000_000);
                let mv = compute_position_value_cents(shares_total, *close_micro, fx_rate);
                (Some(date.clone()), mv, mv - cost_total)
            } else {
                (None, cost_total, 0)
            };

        holdings.push(Holding {
            security_id: sec_id,
            isin,
            symbol,
            name,
            currency,
            shares_micro: shares_total,
            cost_basis_cents: cost_total,
            avg_cost_per_share_micro,
            market_value_cents,
            unrealized_cents,
            last_price_date,
        });
    }

    Ok(holdings)
}

/// Summiert realisierte Gewinne in Cents. `year = None` = all-time.
/// `year = Some(y)` filtert Sells im Kalenderjahr `y`.
pub async fn realized_gains_summary(pool: &SqlitePool, year: Option<i32>) -> DbResult<i64> {
    #[derive(sqlx::FromRow)]
    struct Row {
        security_id: i64,
        booking_date: String,
        side: String,
        shares_micro: i64,
        amount_cents: i64,
        fee_cents: i64,
        tax_cents: i64,
        fusion_group: Option<String>,
    }

    let rows: Vec<Row> = sqlx::query_as(
        "SELECT s.id AS security_id, tx.booking_date, st.side,
                st.shares_micro, tx.amount_cents, st.fee_cents, st.tax_cents,
                st.fusion_group
           FROM securities_trades st
           JOIN transactions tx ON tx.id = st.tx_id
           JOIN securities s ON s.id = st.security_id
          ORDER BY s.id ASC, tx.booking_date ASC, tx.id ASC",
    )
    .fetch_all(pool)
    .await?;

    let groups = group_trades_by_security(
        &rows
            .iter()
            .map(|r| TradeRowRef {
                security_id: r.security_id,
                booking_date: &r.booking_date,
                side: &r.side,
                shares_micro: r.shares_micro,
                amount_cents: r.amount_cents,
                fee_cents: r.fee_cents,
                tax_cents: r.tax_cents,
                fusion_group: r.fusion_group.as_deref(),
            })
            .collect::<Vec<_>>(),
    );
    let carry = collect_fusion_transfers(&groups);

    let mut sum: i64 = 0;
    for trades in &groups {
        let (_lots, realized) = fifo_apply(trades, &carry);
        for r in realized {
            if let Some(y) = year {
                let year_str = format!("{y:04}");
                if !r.sell_date.starts_with(&year_str) {
                    continue;
                }
            }
            sum += r.gain_cents;
        }
    }

    Ok(sum)
}

/// Lightweight row reference for `group_trades_by_security` — avoids requiring
/// each caller struct to have its own conversion function.
struct TradeRowRef<'a> {
    security_id: i64,
    booking_date: &'a str,
    side: &'a str,
    shares_micro: i64,
    amount_cents: i64,
    fee_cents: i64,
    tax_cents: i64,
    fusion_group: Option<&'a str>,
}

/// Groups a trade list sorted by `security_id` into a Vec-per-security.
fn group_trades_by_security(rows: &[TradeRowRef<'_>]) -> Vec<Vec<FifoTradeInput>> {
    let mut groups: Vec<Vec<FifoTradeInput>> = Vec::new();
    let mut current: Option<i64> = None;
    for r in rows {
        if Some(r.security_id) != current {
            groups.push(Vec::new());
            current = Some(r.security_id);
        }
        groups.last_mut().unwrap().push(FifoTradeInput {
            booking_date: r.booking_date.to_string(),
            side: r.side.to_string(),
            shares_micro: r.shares_micro,
            amount_cents: r.amount_cents,
            fee_cents: r.fee_cents,
            tax_cents: r.tax_cents,
            fusion_group: r.fusion_group.map(str::to_string),
        });
    }
    groups
}

/// Returns the total cost basis of all lots per sample month (going back from
/// end_year/end_month, `months` values) based on trades up to month-end.
/// Sorted chronologically ASC.
pub async fn cost_basis_history(
    pool: &SqlitePool,
    end_year: i32,
    end_month: i32,
    months: u32,
) -> DbResult<Vec<CostBasisPoint>> {
    if months == 0 {
        return Ok(Vec::new());
    }
    if !(1..=12).contains(&end_month) {
        return Err(DbError::Decode(format!(
            "end_month must be in 1..=12, got {end_month}",
        )));
    }

    // Sample points: (year, month) descending from (end_year, end_month), `months` entries.
    // Then reversed for ASC output.
    let mut buckets: Vec<(i32, i32)> = Vec::with_capacity(months as usize);
    let mut y = end_year;
    let mut m = end_month;
    for _ in 0..months {
        buckets.push((y, m));
        m -= 1;
        if m == 0 {
            m = 12;
            y -= 1;
        }
    }
    buckets.reverse();

    #[derive(sqlx::FromRow)]
    struct Row {
        security_id: i64,
        booking_date: String,
        side: String,
        shares_micro: i64,
        amount_cents: i64,
        fee_cents: i64,
        tax_cents: i64,
        fusion_group: Option<String>,
    }

    let rows: Vec<Row> = sqlx::query_as(
        "SELECT s.id AS security_id, tx.booking_date, st.side,
                st.shares_micro, tx.amount_cents, st.fee_cents, st.tax_cents,
                st.fusion_group
           FROM securities_trades st
           JOIN transactions tx ON tx.id = st.tx_id
           JOIN securities s ON s.id = st.security_id
          ORDER BY s.id ASC, tx.booking_date ASC, tx.id ASC",
    )
    .fetch_all(pool)
    .await?;

    let mut out: Vec<CostBasisPoint> = Vec::with_capacity(buckets.len());
    for (by, bm) in buckets {
        // Exclusive Cutoff = erster Tag des Folgemonats.
        let next_y = if bm == 12 { by + 1 } else { by };
        let next_m = if bm == 12 { 1 } else { bm + 1 };
        let cutoff = format!("{next_y:04}-{next_m:02}-01");

        let groups = group_trades_by_security(
            &rows
                .iter()
                .filter(|r| r.booking_date < cutoff)
                .map(|r| TradeRowRef {
                    security_id: r.security_id,
                    booking_date: &r.booking_date,
                    side: &r.side,
                    shares_micro: r.shares_micro,
                    amount_cents: r.amount_cents,
                    fee_cents: r.fee_cents,
                    tax_cents: r.tax_cents,
                    fusion_group: r.fusion_group.as_deref(),
                })
                .collect::<Vec<_>>(),
        );
        let carry = collect_fusion_transfers(&groups);

        let mut total_cost: i64 = 0;
        for trades in &groups {
            let (lots, _) = fifo_apply(trades, &carry);
            total_cost += lots.iter().map(|l| l.cost_remaining_cents).sum::<i64>();
        }

        // 6f: additionally compute market value per bucket
        let next_y = if bm == 12 { by + 1 } else { by };
        let next_m = if bm == 12 { 1 } else { bm + 1 };
        let last_day = chrono::NaiveDate::from_ymd_opt(next_y, next_m as u32, 1)
            .and_then(|d| d.pred_opt())
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| format!("{:04}-{:02}-28", by, bm));
        let market_value_cents = portfolio_value_on_date(pool, &last_day).await?;

        out.push(CostBasisPoint {
            year: by,
            month: bm,
            cost_basis_cents: total_cost,
            market_value_cents,
        });
    }
    Ok(out)
}

/// Returns the total cost basis of all lots per day (going back from end_date,
/// `days` values) based on trades up to that day. Sorted chronologically ASC.
pub async fn cost_basis_history_daily(
    pool: &SqlitePool,
    end_date: &str,
    days: u32,
) -> DbResult<Vec<CostBasisPointDaily>> {
    if days == 0 {
        return Ok(Vec::new());
    }
    let end = chrono::NaiveDate::parse_from_str(end_date, "%Y-%m-%d")
        .map_err(|e| DbError::Decode(format!("bad end_date '{end_date}': {e}")))?;

    // Generate dates in ASC order, ending at end_date.
    let dates: Vec<chrono::NaiveDate> = (0..days)
        .map(|i| end - chrono::Duration::days((days - 1 - i) as i64))
        .collect();

    #[derive(sqlx::FromRow)]
    struct Row {
        security_id: i64,
        booking_date: String,
        side: String,
        shares_micro: i64,
        amount_cents: i64,
        fee_cents: i64,
        tax_cents: i64,
        fusion_group: Option<String>,
    }

    let rows: Vec<Row> = sqlx::query_as(
        "SELECT s.id AS security_id, tx.booking_date, st.side,
                st.shares_micro, tx.amount_cents, st.fee_cents, st.tax_cents,
                st.fusion_group
           FROM securities_trades st
           JOIN transactions tx ON tx.id = st.tx_id
           JOIN securities s ON s.id = st.security_id
          ORDER BY s.id ASC, tx.booking_date ASC, tx.id ASC",
    )
    .fetch_all(pool)
    .await?;

    let mut out = Vec::with_capacity(dates.len());
    for d in dates {
        // Cutoff = next day (exclusive): trades with booking_date < cutoff are included.
        let cutoff = (d + chrono::Duration::days(1))
            .format("%Y-%m-%d")
            .to_string();

        let groups = group_trades_by_security(
            &rows
                .iter()
                .filter(|r| r.booking_date < cutoff)
                .map(|r| TradeRowRef {
                    security_id: r.security_id,
                    booking_date: &r.booking_date,
                    side: &r.side,
                    shares_micro: r.shares_micro,
                    amount_cents: r.amount_cents,
                    fee_cents: r.fee_cents,
                    tax_cents: r.tax_cents,
                    fusion_group: r.fusion_group.as_deref(),
                })
                .collect::<Vec<_>>(),
        );
        let carry = collect_fusion_transfers(&groups);

        let mut total_cost: i64 = 0;
        for trades in &groups {
            let (lots, _) = fifo_apply(trades, &carry);
            total_cost += lots.iter().map(|l| l.cost_remaining_cents).sum::<i64>();
        }

        let d_str = d.format("%Y-%m-%d").to_string();
        let market_value_cents = portfolio_value_on_date(pool, &d_str).await?;

        out.push(CostBasisPointDaily {
            date: d_str,
            cost_basis_cents: total_cost,
            market_value_cents,
        });
    }
    Ok(out)
}

/// Portfolio market value (EUR cents) as of the given date (e.g. last day of month).
/// Falls back to cost basis when prices are missing. Returns 0 when there are no
/// positions or all lots are empty.
pub async fn portfolio_value_on_date(pool: &SqlitePool, on_date: &str) -> DbResult<i64> {
    #[derive(sqlx::FromRow)]
    struct Row {
        security_id: i64,
        currency: String,
        booking_date: String,
        side: String,
        shares_micro: i64,
        amount_cents: i64,
        fee_cents: i64,
        tax_cents: i64,
        fusion_group: Option<String>,
    }

    let rows: Vec<Row> = sqlx::query_as(
        "SELECT s.id AS security_id, s.currency, tx.booking_date, st.side,
                st.shares_micro, tx.amount_cents, st.fee_cents, st.tax_cents,
                st.fusion_group
           FROM securities_trades st
           JOIN transactions tx ON tx.id = st.tx_id
           JOIN securities s ON s.id = st.security_id
          WHERE tx.booking_date <= ?1
          ORDER BY s.id ASC, tx.booking_date ASC, tx.id ASC",
    )
    .bind(on_date)
    .fetch_all(pool)
    .await?;

    let groups = group_trades_by_security(
        &rows
            .iter()
            .map(|r| TradeRowRef {
                security_id: r.security_id,
                booking_date: &r.booking_date,
                side: &r.side,
                shares_micro: r.shares_micro,
                amount_cents: r.amount_cents,
                fee_cents: r.fee_cents,
                tax_cents: r.tax_cents,
                fusion_group: r.fusion_group.as_deref(),
            })
            .collect::<Vec<_>>(),
    );
    let carry = collect_fusion_transfers(&groups);

    // Parallel security headers (sec_id, currency) in the same order as groups.
    let mut headers: Vec<(i64, String)> = Vec::new();
    {
        let mut current: Option<i64> = None;
        for r in &rows {
            if Some(r.security_id) != current {
                headers.push((r.security_id, r.currency.clone()));
                current = Some(r.security_id);
            }
        }
    }

    let mut total: i64 = 0;
    for (group_idx, trades) in groups.iter().enumerate() {
        let (sec_id, currency) = (headers[group_idx].0, headers[group_idx].1.clone());
        let (lots, _) = fifo_apply(trades, &carry);
        let shares_total: i64 = lots.iter().map(|l| l.shares_remaining_micro).sum();
        if shares_total == 0 {
            continue;
        }
        let cost_total: i64 = lots.iter().map(|l| l.cost_remaining_cents).sum();

        let price = crate::db::prices::price_on_date(pool, sec_id, on_date).await?;
        let value = match price {
            Some(close_micro) => {
                let fx = crate::db::fx::rate_on_date(pool, &currency, on_date)
                    .await?
                    .unwrap_or(1_000_000);
                compute_position_value_cents(shares_total, close_micro, fx)
            }
            None => cost_total,
        };
        total += value;
    }
    Ok(total)
}

/// Market value (EUR cents) per account as of the given date. Account-scoped FIFO:
/// buys and sells are processed separately per `(account_id, security_id)`.
/// Accounts with no open position do not appear in the result.
/// Sorted by `account_id` ascending.
pub async fn portfolio_value_by_account_on_date(
    pool: &SqlitePool,
    on_date: &str,
) -> DbResult<Vec<(i64, i64)>> {
    #[derive(sqlx::FromRow)]
    struct Row {
        account_id: i64,
        security_id: i64,
        currency: String,
        booking_date: String,
        side: String,
        shares_micro: i64,
        amount_cents: i64,
        fee_cents: i64,
        tax_cents: i64,
        fusion_group: Option<String>,
    }

    let rows: Vec<Row> = sqlx::query_as(
        "SELECT COALESCE(st.account_id, tx.account_id) AS account_id,
                s.id AS security_id, s.currency, tx.booking_date,
                st.side, st.shares_micro, tx.amount_cents, st.fee_cents, st.tax_cents,
                st.fusion_group
           FROM securities_trades st
           JOIN transactions tx ON tx.id = st.tx_id
           JOIN securities s ON s.id = st.security_id
          WHERE tx.booking_date <= ?1
          ORDER BY COALESCE(st.account_id, tx.account_id) ASC, s.id ASC, tx.booking_date ASC, tx.id ASC",
    )
    .bind(on_date)
    .fetch_all(pool)
    .await?;

    // Build fusion carry over ALL securities (independent of account bucketing)
    // so fusion_in can find the cost basis from the source security.
    let groups_for_carry = group_trades_by_security(
        &rows
            .iter()
            .map(|r| TradeRowRef {
                security_id: r.security_id,
                booking_date: &r.booking_date,
                side: &r.side,
                shares_micro: r.shares_micro,
                amount_cents: r.amount_cents,
                fee_cents: r.fee_cents,
                tax_cents: r.tax_cents,
                fusion_group: r.fusion_group.as_deref(),
            })
            .collect::<Vec<_>>(),
    );
    let carry = collect_fusion_transfers(&groups_for_carry);

    let mut totals: std::collections::BTreeMap<i64, i64> = std::collections::BTreeMap::new();
    let mut i = 0;
    while i < rows.len() {
        let acc_id = rows[i].account_id;
        let sec_id = rows[i].security_id;
        let currency = rows[i].currency.clone();
        let mut trades: Vec<FifoTradeInput> = Vec::new();
        while i < rows.len() && rows[i].account_id == acc_id && rows[i].security_id == sec_id {
            trades.push(FifoTradeInput {
                booking_date: rows[i].booking_date.clone(),
                side: rows[i].side.clone(),
                shares_micro: rows[i].shares_micro,
                amount_cents: rows[i].amount_cents,
                fee_cents: rows[i].fee_cents,
                tax_cents: rows[i].tax_cents,
                fusion_group: rows[i].fusion_group.clone(),
            });
            i += 1;
        }
        let (lots, _) = fifo_apply(&trades, &carry);
        let shares_total: i64 = lots.iter().map(|l| l.shares_remaining_micro).sum();
        if shares_total == 0 {
            continue;
        }
        let cost_total: i64 = lots.iter().map(|l| l.cost_remaining_cents).sum();

        let price = crate::db::prices::price_on_date(pool, sec_id, on_date).await?;
        let value = match price {
            Some(close_micro) => {
                let fx = crate::db::fx::rate_on_date(pool, &currency, on_date)
                    .await?
                    .unwrap_or(1_000_000);
                compute_position_value_cents(shares_total, close_micro, fx)
            }
            None => cost_total,
        };
        *totals.entry(acc_id).or_insert(0) += value;
    }

    Ok(totals.into_iter().collect())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshReport {
    pub securities_total: usize,
    pub prices_updated: usize,
    pub prices_failed: usize,
    pub fx_updated: usize,
    pub fx_failed: usize,
}

use crate::pricing_provider::{FxProvider, PriceProvider};

/// Updates `securities.currency` when the provider returns a currency that
/// differs from the current DB value. Idempotent: no UPDATE when the currency
/// is identical. When the provider returns no currency (None) or the value is
/// empty, the DB is left unchanged.
async fn update_currency_if_changed(
    pool: &SqlitePool,
    security_id: i64,
    current_currency: &str,
    provider_currency: Option<&str>,
) -> DbResult<()> {
    let Some(new_cur) = provider_currency.map(|s| s.trim().to_uppercase()) else {
        return Ok(());
    };
    if new_cur.is_empty() || new_cur == current_currency.to_uppercase() {
        return Ok(());
    }
    sqlx::query("UPDATE securities SET currency = ?1 WHERE id = ?2")
        .bind(&new_cur)
        .bind(security_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn refresh_all_prices<P>(pool: &SqlitePool, provider: &P) -> DbResult<RefreshReport>
where
    P: PriceProvider + FxProvider + ?Sized + Send + Sync,
{
    let holdings = current_holdings(pool).await?;
    let mut report = RefreshReport {
        securities_total: holdings.len(),
        prices_updated: 0,
        prices_failed: 0,
        fx_updated: 0,
        fx_failed: 0,
    };
    let today = chrono::Utc::now()
        .date_naive()
        .format("%Y-%m-%d")
        .to_string();

    let mut foreign_currencies: std::collections::HashSet<String> =
        std::collections::HashSet::new();

    for h in &holdings {
        // Resolve symbol (cached in securities.symbol, otherwise via provider).
        let (existing,): (Option<String>,) =
            sqlx::query_as("SELECT symbol FROM securities WHERE id = ?1")
                .bind(h.security_id)
                .fetch_one(pool)
                .await?;

        let symbol: Option<String> = match existing {
            Some(s) if !s.is_empty() => Some(s),
            _ => match provider.resolve_symbol(&h.isin).await {
                Ok(Some(sym)) => {
                    sqlx::query("UPDATE securities SET symbol = ?1 WHERE id = ?2")
                        .bind(&sym)
                        .bind(h.security_id)
                        .execute(pool)
                        .await?;
                    Some(sym)
                }
                _ => None,
            },
        };

        if let Some(sym) = symbol {
            // Decide: history-fetch (no existing prices) or single-quote (already has prices)
            let (existing_count,): (i64,) =
                sqlx::query_as("SELECT COUNT(*) FROM security_prices WHERE security_id = ?1")
                    .bind(h.security_id)
                    .fetch_one(pool)
                    .await?;

            if existing_count == 0 {
                // Newly-added security → fetch 5 years of history
                let today_date = chrono::Utc::now().date_naive();
                let from = today_date - chrono::Duration::days(5 * 365);
                match provider.fetch_history(&sym, from, today_date).await {
                    Ok(history) if !history.is_empty() => {
                        for p in &history {
                            let d = p.date.format("%Y-%m-%d").to_string();
                            let _ = crate::db::prices::upsert_price(
                                pool,
                                h.security_id,
                                &d,
                                p.close_micro,
                                "yahoo",
                            )
                            .await;
                        }
                        report.prices_updated += history.len();
                    }
                    _ => {
                        // History fetch failed or empty → fall back to single quote
                        match provider.fetch_quote(&sym).await {
                            Ok(q) => {
                                let date_str = q.as_of.format("%Y-%m-%d").to_string();
                                crate::db::prices::upsert_price(
                                    pool,
                                    h.security_id,
                                    &date_str,
                                    q.price_micro,
                                    "yahoo",
                                )
                                .await?;
                                update_currency_if_changed(
                                    pool,
                                    h.security_id,
                                    &h.currency,
                                    q.currency.as_deref(),
                                )
                                .await?;
                                report.prices_updated += 1;
                            }
                            Err(_) => report.prices_failed += 1,
                        }
                    }
                }
            } else {
                // Existing security → single quote update
                match provider.fetch_quote(&sym).await {
                    Ok(q) => {
                        let date_str = q.as_of.format("%Y-%m-%d").to_string();
                        crate::db::prices::upsert_price(
                            pool,
                            h.security_id,
                            &date_str,
                            q.price_micro,
                            "yahoo",
                        )
                        .await?;
                        update_currency_if_changed(
                            pool,
                            h.security_id,
                            &h.currency,
                            q.currency.as_deref(),
                        )
                        .await?;
                        report.prices_updated += 1;
                    }
                    Err(_) => {
                        report.prices_failed += 1;
                    }
                }
            }
        } else {
            report.prices_failed += 1;
        }

        if h.currency.to_uppercase() != "EUR" {
            foreign_currencies.insert(h.currency.to_uppercase());
        }
    }

    for cur in foreign_currencies {
        match provider.fetch_eur_rate(&cur).await {
            Ok(rate_micro) => {
                crate::db::fx::upsert_rate(pool, &cur, &today, rate_micro, "yahoo").await?;
                report.fx_updated += 1;
            }
            Err(_) => {
                report.fx_failed += 1;
            }
        }
    }

    Ok(report)
}

/// Returns only genuine dividend transactions (kind='dividend'). Accumulation
/// withholding tax and fund tax charges (kind='tax') have side='tax' and are
/// excluded here via the tx.kind filter.
pub async fn dividend_history(pool: &SqlitePool) -> DbResult<Vec<DividendEntry>> {
    let entries: Vec<DividendEntry> = sqlx::query_as(
        "SELECT tx.id AS tx_id,
                tx.booking_date,
                s.id AS security_id,
                s.name AS security_name,
                tx.amount_cents,
                st.tax_cents
           FROM securities_trades st
           JOIN transactions tx ON tx.id = st.tx_id
           JOIN securities s ON s.id = st.security_id
          WHERE tx.kind = 'dividend'
          ORDER BY tx.booking_date DESC, tx.id DESC",
    )
    .fetch_all(pool)
    .await?;
    Ok(entries)
}

/// Converts a position (shares + price in foreign currency + FX rate) to
/// EUR cents. Uses i128 arithmetic to avoid overflow.
///
/// Formula: shares_micro × close_micro × fx_rate_micro / 1e16
pub fn compute_position_value_cents(
    shares_micro: i64,
    close_micro: i64,
    fx_rate_micro: i64,
) -> i64 {
    let n = (shares_micro as i128)
        .checked_mul(close_micro as i128)
        .and_then(|p| p.checked_mul(fx_rate_micro as i128));
    match n {
        Some(prod) => (prod / 10_000_000_000_000_000_i128) as i64,
        None => 0,
    }
}

/// Aggregator for AllocationDonut. Dimension ∈ {"asset_type","country","sector"}.
///
/// - asset_type: groups per-security by securities.asset_type (no breakdown).
/// - country / sector: uses security_breakdowns when available, otherwise
///   securities.{country|sector}, otherwise "Unbekannt".
///
/// Returns an empty list for an empty portfolio. value_cents sums may differ
/// slightly from the holdings total due to integer rounding in the breakdown case.
pub async fn asset_allocation(
    pool: &SqlitePool,
    dimension: &str,
) -> DbResult<Vec<AllocationSlice>> {
    let allowed = ["asset_type", "country", "sector"];
    if !allowed.contains(&dimension) {
        return Err(DbError::Decode(format!(
            "dimension must be one of {allowed:?}, got {dimension:?}"
        )));
    }

    let holdings = current_holdings(pool).await?;
    let mut alloc: std::collections::HashMap<String, i64> = std::collections::HashMap::new();

    for h in &holdings {
        if h.market_value_cents <= 0 {
            continue;
        }

        match dimension {
            "asset_type" => {
                let (asset_type,): (String,) =
                    sqlx::query_as("SELECT asset_type FROM securities WHERE id = ?1")
                        .bind(h.security_id)
                        .fetch_one(pool)
                        .await?;
                *alloc.entry(asset_type).or_insert(0) += h.market_value_cents;
            }
            "country" | "sector" => {
                let breakdowns: Vec<(String, i64)> = sqlx::query_as(
                    "SELECT key, weight_bps FROM security_breakdowns
                      WHERE security_id = ?1 AND dimension = ?2",
                )
                .bind(h.security_id)
                .bind(dimension)
                .fetch_all(pool)
                .await?;

                if !breakdowns.is_empty() {
                    for (key, weight_bps) in breakdowns {
                        let weighted = (h.market_value_cents as i128 * weight_bps as i128
                            / 10_000_i128) as i64;
                        *alloc.entry(key).or_insert(0) += weighted;
                    }
                } else {
                    let column = dimension;
                    let sql = format!("SELECT {column} FROM securities WHERE id = ?1");
                    let (raw,): (Option<String>,) = sqlx::query_as(&sql)
                        .bind(h.security_id)
                        .fetch_one(pool)
                        .await?;
                    let key = raw
                        .filter(|s| !s.is_empty())
                        .unwrap_or_else(|| "Unbekannt".to_string());
                    *alloc.entry(key).or_insert(0) += h.market_value_cents;
                }
            }
            _ => unreachable!(),
        }
    }

    let mut slices: Vec<AllocationSlice> = alloc
        .into_iter()
        .map(|(key, value_cents)| AllocationSlice { key, value_cents })
        .collect();
    slices.sort_by(|a, b| b.value_cents.cmp(&a.value_cents));
    Ok(slices)
}

/// Returns all allocations of a security, ascending by id.
pub async fn list_allocations_for_security(
    pool: &SqlitePool,
    security_id: i64,
) -> DbResult<Vec<SecurityBucketAllocation>> {
    let rows: Vec<SecurityBucketAllocation> = sqlx::query_as(
        "SELECT id, security_id, bucket_id, shares_micro
           FROM security_bucket_allocations
          WHERE security_id = ?1
          ORDER BY id ASC",
    )
    .bind(security_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Atomically sets the allocations of a security:
/// deletes all existing rows, inserts the new ones.
/// Hard block: sum must not exceed held shares.
/// `items`: Vec<(bucket_id, shares_micro)> – shares_micro must be > 0.
pub async fn set_allocations_for_security(
    pool: &SqlitePool,
    security_id: i64,
    items: &[(i64, i64)],
) -> DbResult<()> {
    // Determine total held shares via FIFO.
    let holdings = current_holdings(pool).await?;
    let held = holdings
        .iter()
        .find(|h| h.security_id == security_id)
        .map(|h| h.shares_micro)
        .unwrap_or(0);

    let mut sum: i64 = 0;
    for &(_, shares) in items {
        if shares <= 0 {
            return Err(DbError::Decode(format!(
                "shares_micro must be > 0 (got {shares})"
            )));
        }
        sum = sum
            .checked_add(shares)
            .ok_or_else(|| DbError::Decode("shares_micro overflow".into()))?;
    }
    if sum > held {
        return Err(DbError::Decode(format!(
            "Allocation ({sum} µ) exceeds held shares ({held} µ)"
        )));
    }

    // Reject duplicate bucket_ids.
    let mut seen: std::collections::HashSet<i64> = std::collections::HashSet::new();
    for &(bid, _) in items {
        if !seen.insert(bid) {
            return Err(DbError::Decode(format!("Duplicate bucket_id {bid}")));
        }
    }

    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM security_bucket_allocations WHERE security_id = ?1")
        .bind(security_id)
        .execute(&mut *tx)
        .await?;
    for &(bid, shares) in items {
        sqlx::query(
            "INSERT INTO security_bucket_allocations (security_id, bucket_id, shares_micro)
             VALUES (?1, ?2, ?3)",
        )
        .bind(security_id)
        .bind(bid)
        .bind(shares)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

/// For the /buckets UI: returns securities allocated to this bucket
/// with their current market value (pro-rata via current_holdings).
pub async fn bucket_holdings(pool: &SqlitePool, bucket_id: i64) -> DbResult<Vec<BucketHoldingRow>> {
    let alloc_rows: Vec<(i64, i64, String, String)> = sqlx::query_as(
        "SELECT sba.security_id, sba.shares_micro, s.name, s.isin
           FROM security_bucket_allocations sba
           JOIN securities s ON s.id = sba.security_id
          WHERE sba.bucket_id = ?1
          ORDER BY s.name ASC",
    )
    .bind(bucket_id)
    .fetch_all(pool)
    .await?;

    if alloc_rows.is_empty() {
        return Ok(Vec::new());
    }

    let holdings = current_holdings(pool).await?;
    let by_sec: std::collections::HashMap<i64, &Holding> =
        holdings.iter().map(|h| (h.security_id, h)).collect();

    let mut out = Vec::with_capacity(alloc_rows.len());
    for (sid, shares, name, isin) in alloc_rows {
        let value_cents = match by_sec.get(&sid) {
            Some(h) if h.shares_micro > 0 => {
                let value = (h.market_value_cents as i128).saturating_mul(shares as i128)
                    / (h.shares_micro as i128);
                value as i64
            }
            _ => 0,
        };
        out.push(BucketHoldingRow {
            security_id: sid,
            security_name: name,
            isin,
            shares_micro: shares,
            value_cents,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_memory;

    #[test]
    fn fifo_empty_input_returns_empty() {
        let (lots, realized) = fifo_apply(&[], &std::collections::HashMap::new());
        assert!(lots.is_empty());
        assert!(realized.is_empty());
    }

    #[test]
    fn fifo_single_buy_creates_one_lot() {
        let trades = vec![FifoTradeInput {
            booking_date: "2026-01-15".into(),
            side: "buy".into(),
            shares_micro: 10_000_000,
            amount_cents: -100_000,
            fee_cents: 0,
            tax_cents: 0,
            fusion_group: None,
        }];
        let (lots, realized) = fifo_apply(&trades, &std::collections::HashMap::new());
        assert_eq!(lots.len(), 1);
        assert_eq!(lots[0].shares_remaining_micro, 10_000_000);
        assert_eq!(lots[0].cost_remaining_cents, 100_000);
        assert_eq!(lots[0].acquired_date, "2026-01-15");
        assert!(realized.is_empty());
    }

    #[test]
    fn fifo_full_sell_realizes_gain() {
        let trades = vec![
            FifoTradeInput {
                booking_date: "2026-01-15".into(),
                side: "buy".into(),
                shares_micro: 10_000_000,
                amount_cents: -100_000,
                fee_cents: 0,
                tax_cents: 0,
                fusion_group: None,
            },
            FifoTradeInput {
                booking_date: "2026-03-10".into(),
                side: "sell".into(),
                shares_micro: 10_000_000,
                amount_cents: 120_000,
                fee_cents: 0,
                tax_cents: 0,
                fusion_group: None,
            },
        ];
        let (lots, realized) = fifo_apply(&trades, &std::collections::HashMap::new());
        assert!(lots.is_empty());
        assert_eq!(realized.len(), 1);
        assert_eq!(realized[0].sell_date, "2026-03-10");
        assert_eq!(realized[0].shares_consumed_micro, 10_000_000);
        assert_eq!(realized[0].proceeds_cents, 120_000);
        assert_eq!(realized[0].cost_basis_cents, 100_000);
        assert_eq!(realized[0].gain_cents, 20_000);
    }

    #[test]
    fn fifo_full_sell_realizes_loss() {
        let trades = vec![
            FifoTradeInput {
                booking_date: "2026-01-15".into(),
                side: "buy".into(),
                shares_micro: 10_000_000,
                amount_cents: -100_000,
                fee_cents: 0,
                tax_cents: 0,
                fusion_group: None,
            },
            FifoTradeInput {
                booking_date: "2026-03-10".into(),
                side: "sell".into(),
                shares_micro: 10_000_000,
                amount_cents: 80_000,
                fee_cents: 0,
                tax_cents: 0,
                fusion_group: None,
            },
        ];
        let (_, realized) = fifo_apply(&trades, &std::collections::HashMap::new());
        assert_eq!(realized[0].gain_cents, -20_000);
    }

    #[test]
    fn fifo_multi_lot_consumes_oldest_first() {
        let trades = vec![
            FifoTradeInput {
                booking_date: "2026-01-01".into(),
                side: "buy".into(),
                shares_micro: 10_000_000,
                amount_cents: -100_000,
                fee_cents: 0,
                tax_cents: 0,
                fusion_group: None,
            },
            FifoTradeInput {
                booking_date: "2026-02-01".into(),
                side: "buy".into(),
                shares_micro: 10_000_000,
                amount_cents: -150_000,
                fee_cents: 0,
                tax_cents: 0,
                fusion_group: None,
            },
            FifoTradeInput {
                booking_date: "2026-03-01".into(),
                side: "sell".into(),
                shares_micro: 15_000_000,
                amount_cents: 270_000,
                fee_cents: 0,
                tax_cents: 0,
                fusion_group: None,
            },
        ];
        let (lots, realized) = fifo_apply(&trades, &std::collections::HashMap::new());
        assert_eq!(lots.len(), 1);
        assert_eq!(lots[0].shares_remaining_micro, 5_000_000);
        assert_eq!(lots[0].cost_remaining_cents, 75_000);
        assert_eq!(realized.len(), 1);
        assert_eq!(realized[0].cost_basis_cents, 175_000);
        assert_eq!(realized[0].gain_cents, 95_000);
    }

    #[test]
    fn fifo_stock_split_doubles_shares_keeps_cost() {
        let trades = vec![
            FifoTradeInput {
                booking_date: "2026-01-01".into(),
                side: "buy".into(),
                shares_micro: 10_000_000,
                amount_cents: -100_000,
                fee_cents: 0,
                tax_cents: 0,
                fusion_group: None,
            },
            FifoTradeInput {
                booking_date: "2026-02-15".into(),
                side: "corporate_action".into(),
                shares_micro: 10_000_000,
                amount_cents: 0,
                fee_cents: 0,
                tax_cents: 0,
                fusion_group: None,
            },
        ];
        let (lots, realized) = fifo_apply(&trades, &std::collections::HashMap::new());
        assert!(realized.is_empty());
        assert_eq!(lots.len(), 1);
        assert_eq!(lots[0].shares_remaining_micro, 20_000_000);
        assert_eq!(lots[0].cost_remaining_cents, 100_000);
    }

    #[test]
    fn fifo_reverse_split_halves_shares() {
        let trades = vec![
            FifoTradeInput {
                booking_date: "2026-01-01".into(),
                side: "buy".into(),
                shares_micro: 20_000_000,
                amount_cents: -200_000,
                fee_cents: 0,
                tax_cents: 0,
                fusion_group: None,
            },
            FifoTradeInput {
                booking_date: "2026-02-15".into(),
                side: "corporate_action".into(),
                shares_micro: -10_000_000,
                amount_cents: 0,
                fee_cents: 0,
                tax_cents: 0,
                fusion_group: None,
            },
        ];
        let (lots, _) = fifo_apply(&trades, &std::collections::HashMap::new());
        assert_eq!(lots[0].shares_remaining_micro, 10_000_000);
        assert_eq!(lots[0].cost_remaining_cents, 200_000);
    }

    #[test]
    fn fifo_split_distributes_across_lots_proportionally() {
        let trades = vec![
            FifoTradeInput {
                booking_date: "2026-01-01".into(),
                side: "buy".into(),
                shares_micro: 5_000_000,
                amount_cents: -50_000,
                fee_cents: 0,
                tax_cents: 0,
                fusion_group: None,
            },
            FifoTradeInput {
                booking_date: "2026-02-01".into(),
                side: "buy".into(),
                shares_micro: 15_000_000,
                amount_cents: -300_000,
                fee_cents: 0,
                tax_cents: 0,
                fusion_group: None,
            },
            FifoTradeInput {
                booking_date: "2026-03-01".into(),
                side: "corporate_action".into(),
                shares_micro: 20_000_000,
                amount_cents: 0,
                fee_cents: 0,
                tax_cents: 0,
                fusion_group: None,
            },
        ];
        let (lots, _) = fifo_apply(&trades, &std::collections::HashMap::new());
        assert_eq!(lots.len(), 2);
        assert_eq!(lots[0].shares_remaining_micro, 10_000_000);
        assert_eq!(lots[0].cost_remaining_cents, 50_000);
        assert_eq!(lots[1].shares_remaining_micro, 30_000_000);
        assert_eq!(lots[1].cost_remaining_cents, 300_000);
    }

    #[test]
    fn fifo_dividend_does_not_affect_lots() {
        let trades = vec![
            FifoTradeInput {
                booking_date: "2026-01-01".into(),
                side: "buy".into(),
                shares_micro: 10_000_000,
                amount_cents: -100_000,
                fee_cents: 0,
                tax_cents: 0,
                fusion_group: None,
            },
            FifoTradeInput {
                booking_date: "2026-03-15".into(),
                side: "dividend".into(),
                shares_micro: 0,
                amount_cents: 5_000,
                fee_cents: 0,
                tax_cents: 750,
                fusion_group: None,
            },
        ];
        let (lots, realized) = fifo_apply(&trades, &std::collections::HashMap::new());
        assert_eq!(lots.len(), 1);
        assert_eq!(lots[0].shares_remaining_micro, 10_000_000);
        assert_eq!(lots[0].cost_remaining_cents, 100_000);
        assert!(realized.is_empty());
    }

    // ── Fusion semantics ────────────────────────────────────────────────────────

    /// Source side: lots are emptied regardless of whether the PDF holding
    /// exactly matches the DB holding (robustness against missing buys).
    #[test]
    fn fifo_fusion_out_empties_all_lots_even_when_share_count_diverges() {
        // User holds 192.34 shares (3 buys), fusion reports 212.57 — savings-plan gap.
        let source_trades = vec![
            FifoTradeInput {
                booking_date: "2022-06-01".into(),
                side: "buy".into(),
                shares_micro: 60_000_000,
                amount_cents: -60_000,
                fee_cents: 0,
                tax_cents: 0,
                fusion_group: None,
            },
            FifoTradeInput {
                booking_date: "2022-07-01".into(),
                side: "buy".into(),
                shares_micro: 100_000_000,
                amount_cents: -100_000,
                fee_cents: 0,
                tax_cents: 0,
                fusion_group: None,
            },
            FifoTradeInput {
                booking_date: "2022-08-01".into(),
                side: "buy".into(),
                shares_micro: 32_342_348,
                amount_cents: -32_500,
                fee_cents: 0,
                tax_cents: 0,
                fusion_group: None,
            },
            FifoTradeInput {
                booking_date: "2025-02-21".into(),
                side: "fusion_out".into(),
                shares_micro: -212_571_745,
                amount_cents: 0,
                fee_cents: 0,
                tax_cents: 0,
                fusion_group: Some("FUSION-X".into()),
            },
        ];
        let (lots, realized) = fifo_apply(&source_trades, &std::collections::HashMap::new());
        assert!(
            lots.is_empty(),
            "fusion_out must clear all lots, had {} remaining",
            lots.len()
        );
        assert!(
            realized.is_empty(),
            "fusion_out must not produce any realized gain"
        );
    }

    /// `collect_fusion_transfers` must collect the cost basis of all lots before
    /// `fusion_out` and store it under the fusion_group key.
    #[test]
    fn collect_fusion_transfers_captures_total_cost_basis_per_group() {
        let source_trades = vec![
            FifoTradeInput {
                booking_date: "2022-06-01".into(),
                side: "buy".into(),
                shares_micro: 60_000_000,
                amount_cents: -60_000,
                fee_cents: 0,
                tax_cents: 0,
                fusion_group: None,
            },
            FifoTradeInput {
                booking_date: "2022-07-01".into(),
                side: "buy".into(),
                shares_micro: 100_000_000,
                amount_cents: -100_000,
                fee_cents: 0,
                tax_cents: 0,
                fusion_group: None,
            },
            FifoTradeInput {
                booking_date: "2025-02-21".into(),
                side: "fusion_out".into(),
                shares_micro: -160_000_000,
                amount_cents: 0,
                fee_cents: 0,
                tax_cents: 0,
                fusion_group: Some("FUSION-X".into()),
            },
        ];
        let carry = collect_fusion_transfers(&[source_trades]);
        let t = carry
            .get("FUSION-X")
            .expect("fusion_group missing in carry");
        assert_eq!(t.cost_cents, 160_000, "Cost basis = sum of all buy amounts");
        assert_eq!(t.acquired_date, "2022-06-01", "Earliest acquisition date");
    }

    /// Target side: `fusion_in` creates a new lot with the cost basis transferred
    /// from the carry (instead of 0 as before).
    #[test]
    fn fifo_fusion_in_creates_lot_from_carry() {
        let mut carry = std::collections::HashMap::new();
        carry.insert(
            "FUSION-X".to_string(),
            FusionTransfer {
                cost_cents: 160_000,
                acquired_date: "2022-06-01".into(),
            },
        );
        let target_trades = vec![FifoTradeInput {
            booking_date: "2025-02-21".into(),
            side: "fusion_in".into(),
            shares_micro: 30_000_000,
            amount_cents: 0,
            fee_cents: 0,
            tax_cents: 0,
            fusion_group: Some("FUSION-X".into()),
        }];
        let (lots, _) = fifo_apply(&target_trades, &carry);
        assert_eq!(lots.len(), 1, "fusion_in must create a new lot");
        assert_eq!(lots[0].shares_remaining_micro, 30_000_000);
        assert_eq!(
            lots[0].cost_remaining_cents, 160_000,
            "Cost basis transferred"
        );
        assert_eq!(
            lots[0].acquired_date, "2022-06-01",
            "Acquisition date from source lot — relevant for FIFO tax logic"
        );
    }

    /// Defensive: without a carry entry (e.g. orphaned fusion tx) the lot is
    /// still created — with cost basis 0 and the tx date as acquisition date.
    /// This ensures the new position is at least visible.
    #[test]
    fn fifo_fusion_in_without_carry_creates_zero_cost_lot() {
        let target_trades = vec![FifoTradeInput {
            booking_date: "2025-02-21".into(),
            side: "fusion_in".into(),
            shares_micro: 30_000_000,
            amount_cents: 0,
            fee_cents: 0,
            tax_cents: 0,
            fusion_group: Some("FUSION-MISSING".into()),
        }];
        let (lots, _) = fifo_apply(&target_trades, &std::collections::HashMap::new());
        assert_eq!(lots.len(), 1);
        assert_eq!(lots[0].shares_remaining_micro, 30_000_000);
        assert_eq!(lots[0].cost_remaining_cents, 0);
        assert_eq!(lots[0].acquired_date, "2025-02-21");
    }

    /// Sell after fusion: realized gain uses the TRANSFERRED cost basis
    /// from the source security.
    #[test]
    fn fifo_sell_after_fusion_in_uses_transferred_cost_basis() {
        let mut carry = std::collections::HashMap::new();
        carry.insert(
            "FUSION-X".to_string(),
            FusionTransfer {
                cost_cents: 100_000,
                acquired_date: "2022-06-01".into(),
            },
        );
        let target_trades = vec![
            FifoTradeInput {
                booking_date: "2025-02-21".into(),
                side: "fusion_in".into(),
                shares_micro: 10_000_000,
                amount_cents: 0,
                fee_cents: 0,
                tax_cents: 0,
                fusion_group: Some("FUSION-X".into()),
            },
            FifoTradeInput {
                booking_date: "2025-06-01".into(),
                side: "sell".into(),
                shares_micro: 10_000_000,
                amount_cents: 130_000,
                fee_cents: 0,
                tax_cents: 0,
                fusion_group: None,
            },
        ];
        let (lots, realized) = fifo_apply(&target_trades, &carry);
        assert!(lots.is_empty(), "All shares sold");
        assert_eq!(realized.len(), 1);
        // Gain = proceeds - transferred cost basis = 130,000 - 100,000 = 30,000
        assert_eq!(realized[0].cost_basis_cents, 100_000);
        assert_eq!(realized[0].gain_cents, 30_000);
    }

    // ── seed helpers ────────────────────────────────────────────────────────────

    async fn seed_account(pool: &sqlx::SqlitePool) -> i64 {
        let (id,): (i64,) = sqlx::query_as(
            "INSERT INTO accounts (name, kind, currency)
             VALUES ('Broker', 'broker', 'EUR') RETURNING id",
        )
        .fetch_one(pool)
        .await
        .unwrap();
        id
    }

    async fn seed_security(pool: &sqlx::SqlitePool, isin: &str, name: &str) -> i64 {
        let (id,): (i64,) = sqlx::query_as(
            "INSERT INTO securities (isin, name, currency, asset_type)
             VALUES (?1, ?2, 'EUR', 'stock') RETURNING id",
        )
        .bind(isin)
        .bind(name)
        .fetch_one(pool)
        .await
        .unwrap();
        id
    }

    async fn seed_trade(
        pool: &sqlx::SqlitePool,
        acc_id: i64,
        sec_id: i64,
        date: &str,
        side: &str,
        shares_micro: i64,
        amount_cents: i64,
    ) {
        let (tx_id,): (i64,) = sqlx::query_as(
            "INSERT INTO transactions
                (account_id, booking_date, amount_cents, currency, source, kind, imported_at)
             VALUES (?1, ?2, ?3, 'EUR', 'manual', ?4, '2026-05-19T00:00:00Z')
             RETURNING id",
        )
        .bind(acc_id)
        .bind(date)
        .bind(amount_cents)
        .bind(side)
        .fetch_one(pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO securities_trades
                (tx_id, security_id, side, shares_micro, unit_price_micro, fee_cents, tax_cents)
             VALUES (?1, ?2, ?3, ?4, NULL, 0, 0)",
        )
        .bind(tx_id)
        .bind(sec_id)
        .bind(side)
        .bind(shares_micro)
        .execute(pool)
        .await
        .unwrap();
    }

    async fn seed_buy(
        pool: &sqlx::SqlitePool,
        acc_id: i64,
        sec_id: i64,
        date: &str,
        shares_micro: i64,
        amount_cents: i64,
    ) {
        seed_trade(
            pool,
            acc_id,
            sec_id,
            date,
            "buy",
            shares_micro,
            -amount_cents.abs(),
        )
        .await;
    }

    async fn seed_fusion(
        pool: &sqlx::SqlitePool,
        acc_id: i64,
        sec_id: i64,
        date: &str,
        side: &str,
        shares_micro: i64,
        fusion_group: &str,
    ) {
        // transactions.kind is constrained by the schema to a fixed list
        // ('fusion_out'/'fusion_in' are not allowed kinds there). The importer
        // sets kind='corporate_action' for both fusion sides;
        // securities_trades.side carries the distinction. counterparty must
        // differ per side, otherwise the dedup index collides
        // (account_id, booking_date, amount_cents=0, counterparty, hash).
        let (tx_id,): (i64,) = sqlx::query_as(
            "INSERT INTO transactions
                (account_id, booking_date, amount_cents, currency, counterparty,
                 source, kind, imported_at)
             VALUES (?1, ?2, 0, 'EUR', ?3, 'manual', 'corporate_action',
                     '2026-05-19T00:00:00Z')
             RETURNING id",
        )
        .bind(acc_id)
        .bind(date)
        .bind(format!("Fusion {side}"))
        .fetch_one(pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO securities_trades
                (tx_id, security_id, side, shares_micro, unit_price_micro,
                 fee_cents, tax_cents, fusion_group)
             VALUES (?1, ?2, ?3, ?4, NULL, 0, 0, ?5)",
        )
        .bind(tx_id)
        .bind(sec_id)
        .bind(side)
        .bind(shares_micro)
        .bind(fusion_group)
        .execute(pool)
        .await
        .unwrap();
    }

    // ── current_holdings tests ───────────────────────────────────────────────

    #[tokio::test]
    async fn current_holdings_empty_db_returns_empty() {
        let pool = connect_memory().await.unwrap();
        let holdings = current_holdings(&pool).await.unwrap();
        assert!(holdings.is_empty());
    }

    #[tokio::test]
    async fn current_holdings_single_buy_shows_one_position() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool).await;
        let sec = seed_security(&pool, "DE000A0F5UH1", "iShares Core MSCI World").await;
        seed_trade(&pool, acc, sec, "2026-01-15", "buy", 10_000_000, -100_000).await;

        let holdings = current_holdings(&pool).await.unwrap();
        assert_eq!(holdings.len(), 1);
        assert_eq!(holdings[0].security_id, sec);
        assert_eq!(holdings[0].shares_micro, 10_000_000);
        assert_eq!(holdings[0].cost_basis_cents, 100_000);
    }

    #[tokio::test]
    async fn current_holdings_filters_fully_sold() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool).await;
        let sec = seed_security(&pool, "US0378331005", "Apple").await;
        seed_trade(&pool, acc, sec, "2026-01-15", "buy", 10_000_000, -100_000).await;
        seed_trade(&pool, acc, sec, "2026-03-10", "sell", 10_000_000, 120_000).await;
        let holdings = current_holdings(&pool).await.unwrap();
        assert!(holdings.is_empty(), "fully sold security must not appear");
    }

    #[tokio::test]
    async fn current_holdings_multiple_securities() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool).await;
        let s1 = seed_security(&pool, "US0378331005", "Apple").await;
        let s2 = seed_security(&pool, "DE000A0F5UH1", "iShares").await;
        seed_trade(&pool, acc, s1, "2026-01-01", "buy", 5_000_000, -50_000).await;
        seed_trade(&pool, acc, s2, "2026-01-01", "buy", 3_000_000, -90_000).await;

        let holdings = current_holdings(&pool).await.unwrap();
        assert_eq!(holdings.len(), 2);
        let mut ids: Vec<i64> = holdings.iter().map(|h| h.security_id).collect();
        ids.sort();
        assert_eq!(ids, vec![s1, s2]);
    }

    /// After a fusion the source security is typically archived (position = 0).
    /// The cost-basis carry logic must still see its `fusion_out` trade and
    /// previous buys — otherwise the new security falls back to cost_basis_cents = 0.
    #[tokio::test]
    async fn current_holdings_preserves_fusion_carry_when_source_security_archived() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool).await;
        let src = seed_security(&pool, "LU1781541179", "LYXOR (Quelle)").await;
        let dst = seed_security(&pool, "IE000BI8OT95", "AMUNDI (Ziel)").await;

        seed_buy(&pool, acc, src, "2022-06-01", 100_000_000, 160_000).await;
        seed_fusion(
            &pool,
            acc,
            src,
            "2025-02-21",
            "fusion_out",
            -100_000_000,
            "FUSION-1",
        )
        .await;
        seed_fusion(
            &pool,
            acc,
            dst,
            "2025-02-21",
            "fusion_in",
            30_000_000,
            "FUSION-1",
        )
        .await;

        // Archive the source security (position is already 0 after fusion_out).
        sqlx::query("UPDATE securities SET archived = 1 WHERE id = ?1")
            .bind(src)
            .execute(&pool)
            .await
            .unwrap();

        let holdings = current_holdings(&pool).await.unwrap();
        assert_eq!(
            holdings.len(),
            1,
            "Only the target security (AMUNDI) has a position"
        );
        assert_eq!(holdings[0].security_id, dst);
        assert_eq!(
            holdings[0].cost_basis_cents, 160_000,
            "Cost basis must have been transferred from the archived source"
        );
    }

    // ── realized_gains_summary tests ─────────────────────────────────────────

    #[tokio::test]
    async fn realized_gains_empty_returns_zero() {
        let pool = connect_memory().await.unwrap();
        let sum = realized_gains_summary(&pool, Some(2026)).await.unwrap();
        assert_eq!(sum, 0);
        let all = realized_gains_summary(&pool, None).await.unwrap();
        assert_eq!(all, 0);
    }

    #[tokio::test]
    async fn realized_gains_aggregates_sells() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool).await;
        let sec = seed_security(&pool, "US0378331005", "Apple").await;
        seed_trade(&pool, acc, sec, "2026-01-15", "buy", 10_000_000, -100_000).await;
        seed_trade(&pool, acc, sec, "2026-03-10", "sell", 10_000_000, 120_000).await;
        seed_trade(&pool, acc, sec, "2026-04-01", "buy", 5_000_000, -75_000).await;
        seed_trade(&pool, acc, sec, "2026-06-01", "sell", 5_000_000, 60_000).await;

        let sum_26 = realized_gains_summary(&pool, Some(2026)).await.unwrap();
        // (+20_000) + (-15_000) = 5_000
        assert_eq!(sum_26, 5_000);

        let sum_25 = realized_gains_summary(&pool, Some(2025)).await.unwrap();
        assert_eq!(sum_25, 0);

        let all = realized_gains_summary(&pool, None).await.unwrap();
        assert_eq!(all, 5_000);
    }

    // ── dividend_history tests ───────────────────────────────────────────────

    #[tokio::test]
    async fn dividend_history_excludes_thesaurierung_tax() {
        // Accumulation withholding tax tx: side='tax' + kind='tax'. Must NOT
        // appear in the dividend report.
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool).await;
        let sec = seed_security(&pool, "LU1781541179", "LYXOR").await;
        seed_trade(&pool, acc, sec, "2026-06-15", "dividend", 0, 5_000).await;
        let (tx_id,): (i64,) = sqlx::query_as(
            "INSERT INTO transactions
                (account_id, booking_date, amount_cents, currency, source, kind, imported_at)
             VALUES (?1, '2026-05-01', -16500, 'EUR', 'flatex_pdf', 'tax', '2026-05-19T00:00:00Z')
             RETURNING id",
        )
        .bind(acc)
        .fetch_one(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO securities_trades (tx_id, security_id, side, shares_micro,
                unit_price_micro, fee_cents, tax_cents)
             VALUES (?1, ?2, 'tax', 0, NULL, 0, 16500)",
        )
        .bind(tx_id)
        .bind(sec)
        .execute(&pool)
        .await
        .unwrap();

        let entries = dividend_history(&pool).await.unwrap();
        assert_eq!(
            entries.len(),
            1,
            "only the genuine dividend, withholding tax tx filtered out"
        );
        assert_eq!(entries[0].amount_cents, 5_000);
    }

    #[tokio::test]
    async fn dividend_history_returns_entries_desc() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool).await;
        let sec = seed_security(&pool, "US0378331005", "Apple").await;
        seed_trade(&pool, acc, sec, "2026-03-15", "dividend", 0, 5_000).await;
        seed_trade(&pool, acc, sec, "2026-06-15", "dividend", 0, 7_500).await;

        let entries = dividend_history(&pool).await.unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].booking_date, "2026-06-15");
        assert_eq!(entries[0].amount_cents, 7_500);
        assert_eq!(entries[1].booking_date, "2026-03-15");
        assert_eq!(entries[1].amount_cents, 5_000);
    }

    // ── compute_position_value_cents tests ──────────────────────────────────

    #[test]
    fn compute_position_value_basic_eur() {
        let v = compute_position_value_cents(10_000_000, 180_500_000, 909_100);
        assert!((164_080..=164_100).contains(&v), "got {v}");
    }

    #[test]
    fn compute_position_value_eur_identity() {
        let v = compute_position_value_cents(10_000_000, 50_000_000, 1_000_000);
        assert_eq!(v, 50_000);
    }

    #[test]
    fn compute_position_value_zero_shares() {
        let v = compute_position_value_cents(0, 180_500_000, 909_100);
        assert_eq!(v, 0);
    }

    #[tokio::test]
    async fn current_holdings_uses_market_value_when_price_exists() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool).await;
        let sec = seed_security(&pool, "US0378331005", "Apple").await;
        seed_trade(&pool, acc, sec, "2026-01-15", "buy", 10_000_000, -50_000).await;
        crate::db::prices::upsert_price(&pool, sec, "2026-05-19", 70_000_000, "yahoo")
            .await
            .unwrap();

        let h = &current_holdings(&pool).await.unwrap()[0];
        assert_eq!(h.cost_basis_cents, 50_000);
        assert_eq!(h.market_value_cents, 70_000);
        assert_eq!(h.unrealized_cents, 20_000);
        assert_eq!(h.last_price_date.as_deref(), Some("2026-05-19"));
    }

    #[tokio::test]
    async fn current_holdings_falls_back_to_cost_basis_when_no_price() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool).await;
        let sec = seed_security(&pool, "US0378331005", "Apple").await;
        seed_trade(&pool, acc, sec, "2026-01-15", "buy", 10_000_000, -50_000).await;

        let h = &current_holdings(&pool).await.unwrap()[0];
        assert_eq!(h.market_value_cents, 50_000);
        assert_eq!(h.unrealized_cents, 0);
        assert!(h.last_price_date.is_none());
    }

    // ── cost_basis_history tests ─────────────────────────────────────────────

    #[tokio::test]
    async fn cost_basis_history_empty_db_returns_zero_points() {
        let pool = connect_memory().await.unwrap();
        let points = cost_basis_history(&pool, 2026, 5, 3).await.unwrap();
        assert_eq!(points.len(), 3);
        for p in &points {
            assert_eq!(p.cost_basis_cents, 0);
        }
        // Sorted ASC: (2026,3), (2026,4), (2026,5)
        assert_eq!(points[0].month, 3);
        assert_eq!(points[1].month, 4);
        assert_eq!(points[2].month, 5);
    }

    #[tokio::test]
    async fn cost_basis_history_accumulates_over_time() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool).await;
        let sec = seed_security(&pool, "US0378331005", "Apple").await;
        seed_trade(&pool, acc, sec, "2026-01-15", "buy", 10_000_000, -100_000).await;
        seed_trade(&pool, acc, sec, "2026-04-15", "buy", 5_000_000, -50_000).await;

        // 6 months back from 2026-06 = Jan, Feb, Mar, Apr, May, Jun
        let points = cost_basis_history(&pool, 2026, 6, 6).await.unwrap();
        assert_eq!(points.len(), 6);
        assert_eq!(points[0].month, 1);
        assert_eq!(points[5].month, 6);
        assert_eq!(points[0].cost_basis_cents, 100_000);
        assert_eq!(points[1].cost_basis_cents, 100_000);
        assert_eq!(points[2].cost_basis_cents, 100_000);
        assert_eq!(points[3].cost_basis_cents, 150_000);
        assert_eq!(points[5].cost_basis_cents, 150_000);
    }

    #[tokio::test]
    async fn cost_basis_history_includes_market_value_when_prices_exist() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool).await;
        let sec = seed_security(&pool, "US0378331005", "Apple").await;
        seed_trade(&pool, acc, sec, "2026-01-15", "buy", 10_000_000, -50_000).await;
        // Price €70 at end of February
        crate::db::prices::upsert_price(&pool, sec, "2026-02-28", 70_000_000, "yahoo")
            .await
            .unwrap();

        let points = cost_basis_history(&pool, 2026, 2, 2).await.unwrap();
        assert_eq!(points.len(), 2);
        // Jan: cost 50_000, market = 50_000 (no price at end of Jan, fallback)
        assert_eq!(points[0].cost_basis_cents, 50_000);
        assert_eq!(points[0].market_value_cents, 50_000);
        // Feb: cost 50_000, market 70_000
        assert_eq!(points[1].cost_basis_cents, 50_000);
        assert_eq!(points[1].market_value_cents, 70_000);
    }

    #[tokio::test]
    async fn cost_basis_history_market_falls_back_to_cost_when_no_prices() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool).await;
        let sec = seed_security(&pool, "US0378331005", "Apple").await;
        seed_trade(&pool, acc, sec, "2026-01-15", "buy", 10_000_000, -50_000).await;

        let points = cost_basis_history(&pool, 2026, 3, 3).await.unwrap();
        for p in &points {
            assert_eq!(p.market_value_cents, p.cost_basis_cents);
        }
    }

    // ── portfolio_value_on_date tests ────────────────────────────────────────

    #[tokio::test]
    async fn portfolio_value_on_date_uses_prices_and_falls_back() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool).await;
        let s1 = seed_security(&pool, "US0378331005", "Apple").await;
        let s2 = seed_security(&pool, "DE000A0F5UH1", "iShares").await;
        seed_trade(&pool, acc, s1, "2026-01-15", "buy", 10_000_000, -50_000).await;
        seed_trade(&pool, acc, s2, "2026-01-15", "buy", 5_000_000, -100_000).await;
        crate::db::prices::upsert_price(&pool, s1, "2026-04-30", 70_000_000, "yahoo")
            .await
            .unwrap();

        let v = portfolio_value_on_date(&pool, "2026-04-30").await.unwrap();
        // s1: 10 × €70 = 70_000 cents (market)
        // s2: fallback to cost = 100_000 cents
        assert_eq!(v, 70_000 + 100_000);
    }

    // ── portfolio_value_by_account_on_date tests ─────────────────────────────

    async fn seed_account_named(pool: &sqlx::SqlitePool, name: &str) -> i64 {
        let (id,): (i64,) = sqlx::query_as(
            "INSERT INTO accounts (name, kind, currency)
             VALUES (?1, 'broker', 'EUR') RETURNING id",
        )
        .bind(name)
        .fetch_one(pool)
        .await
        .unwrap();
        id
    }

    #[tokio::test]
    async fn portfolio_value_by_account_on_date_splits_per_account() {
        let pool = connect_memory().await.unwrap();
        let acc_a = seed_account_named(&pool, "TR").await;
        let acc_b = seed_account_named(&pool, "SC").await;
        let s = seed_security(&pool, "US0378331005", "Apple").await;
        // TR: 10 Aktien
        seed_trade(&pool, acc_a, s, "2026-01-15", "buy", 10_000_000, -50_000).await;
        // SC: 5 Aktien
        seed_trade(&pool, acc_b, s, "2026-01-15", "buy", 5_000_000, -25_000).await;
        crate::db::prices::upsert_price(&pool, s, "2026-04-30", 70_000_000, "yahoo")
            .await
            .unwrap();

        let v = portfolio_value_by_account_on_date(&pool, "2026-04-30")
            .await
            .unwrap();
        assert_eq!(
            v,
            vec![
                (acc_a, 70_000), // 10 × €70
                (acc_b, 35_000), // 5  × €70
            ]
        );
    }

    #[tokio::test]
    async fn portfolio_value_by_account_on_date_uses_account_scoped_fifo() {
        // A sell on B must NOT consume FIFO lots from A.
        let pool = connect_memory().await.unwrap();
        let acc_a = seed_account_named(&pool, "TR").await;
        let acc_b = seed_account_named(&pool, "SC").await;
        let s = seed_security(&pool, "US0378331005", "Apple").await;
        // A: 10 buy
        seed_trade(&pool, acc_a, s, "2026-01-15", "buy", 10_000_000, -50_000).await;
        // B: 5 buy
        seed_trade(&pool, acc_b, s, "2026-01-15", "buy", 5_000_000, -25_000).await;
        // B: 3 sell (proceeds arbitrary)
        seed_trade(&pool, acc_b, s, "2026-02-20", "sell", 3_000_000, 18_000).await;
        crate::db::prices::upsert_price(&pool, s, "2026-04-30", 70_000_000, "yahoo")
            .await
            .unwrap();

        let v = portfolio_value_by_account_on_date(&pool, "2026-04-30")
            .await
            .unwrap();
        assert_eq!(
            v,
            vec![
                (acc_a, 70_000), // 10 × €70 = €700
                (acc_b, 14_000), // (5-3) × €70 = €140
            ]
        );
    }

    #[tokio::test]
    async fn portfolio_value_by_account_on_date_excludes_fully_sold_account() {
        let pool = connect_memory().await.unwrap();
        let acc_a = seed_account_named(&pool, "TR").await;
        let acc_b = seed_account_named(&pool, "SC").await;
        let s = seed_security(&pool, "US0378331005", "Apple").await;
        seed_trade(&pool, acc_a, s, "2026-01-15", "buy", 10_000_000, -50_000).await;
        seed_trade(&pool, acc_b, s, "2026-01-15", "buy", 5_000_000, -25_000).await;
        // B sells everything
        seed_trade(&pool, acc_b, s, "2026-02-20", "sell", 5_000_000, 30_000).await;
        crate::db::prices::upsert_price(&pool, s, "2026-04-30", 70_000_000, "yahoo")
            .await
            .unwrap();

        let v = portfolio_value_by_account_on_date(&pool, "2026-04-30")
            .await
            .unwrap();
        assert_eq!(v, vec![(acc_a, 70_000)]); // B is omitted
    }

    #[tokio::test]
    async fn portfolio_value_by_account_on_date_falls_back_to_cost_basis() {
        // No price → cost basis as market value (analogous to portfolio_value_on_date).
        let pool = connect_memory().await.unwrap();
        let acc = seed_account_named(&pool, "TR").await;
        let s = seed_security(&pool, "US0378331005", "Apple").await;
        seed_trade(&pool, acc, s, "2026-01-15", "buy", 10_000_000, -50_000).await;

        let v = portfolio_value_by_account_on_date(&pool, "2026-04-30")
            .await
            .unwrap();
        assert_eq!(v, vec![(acc, 50_000)]);
    }

    #[tokio::test]
    async fn portfolio_value_by_account_uses_securities_trades_account_id_when_set() {
        // Variant B: tx is linked to settlement account, securities_trades.account_id points to depot.
        // Holdings should land on the depot, NOT on the settlement account.
        let pool = connect_memory().await.unwrap();
        sqlx::query("INSERT INTO institutions (name) VALUES ('TR')")
            .execute(&pool)
            .await
            .unwrap();
        let (inst_id,): (i64,) = sqlx::query_as("SELECT id FROM institutions WHERE name='TR'")
            .fetch_one(&pool)
            .await
            .unwrap();
        let verrechnung = crate::db::accounts::create_account(
            &pool,
            "V",
            "bank",
            "EUR",
            None,
            None,
            Some(inst_id),
        )
        .await
        .unwrap();
        let depot = crate::db::accounts::create_account(
            &pool,
            "D",
            "broker",
            "EUR",
            None,
            None,
            Some(inst_id),
        )
        .await
        .unwrap();

        let sec = crate::db::securities::create_security(
            &pool,
            crate::db::securities::NewSecurityPayload {
                isin: "LU0000000003".into(),
                symbol: None,
                name: "X".into(),
                currency: None,
                asset_type: "etf_equity".into(),
                country: None,
                sector: None,
                note: None,
            },
        )
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO transactions (account_id, booking_date, amount_cents, currency, counterparty, source, kind)
             VALUES (?1, '2026-04-01', -50000, 'EUR', 'X', 'tr_csv', 'buy')"
        ).bind(verrechnung.id).execute(&pool).await.unwrap();
        let (tx_id,): (i64,) = sqlx::query_as("SELECT id FROM transactions WHERE account_id=?1")
            .bind(verrechnung.id)
            .fetch_one(&pool)
            .await
            .unwrap();

        // Trade row with account_id = depot (Variant B!)
        crate::db::trades::insert_trade_row(
            &pool,
            tx_id,
            sec.id,
            "buy",
            5_000_000,
            Some(10_000_000),
            0,
            0,
            0,
            None,
            Some(depot.id),
            None,
        )
        .await
        .unwrap();

        let result = portfolio_value_by_account_on_date(&pool, "2026-04-30")
            .await
            .unwrap();
        // Holdings should appear on the DEPOT, not on the settlement account
        let depot_value = result.iter().find(|(acc_id, _)| *acc_id == depot.id);
        let verrechnung_value = result.iter().find(|(acc_id, _)| *acc_id == verrechnung.id);
        assert!(
            depot_value.is_some(),
            "Holdings should bucket to depot when st.account_id set"
        );
        assert!(
            verrechnung_value.is_none(),
            "Holdings should NOT bucket to verrechnung when st.account_id set"
        );
    }

    #[tokio::test]
    async fn portfolio_value_by_account_falls_back_to_tx_account_when_st_account_null() {
        // When securities_trades.account_id is NULL, it falls back to tx.account_id.
        let pool = connect_memory().await.unwrap();
        let acc =
            crate::db::accounts::create_account(&pool, "A", "broker", "EUR", None, None, None)
                .await
                .unwrap();
        let sec = crate::db::securities::create_security(
            &pool,
            crate::db::securities::NewSecurityPayload {
                isin: "LU0000000004".into(),
                symbol: None,
                name: "Y".into(),
                currency: None,
                asset_type: "etf_equity".into(),
                country: None,
                sector: None,
                note: None,
            },
        )
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO transactions (account_id, booking_date, amount_cents, currency, counterparty, source, kind)
             VALUES (?1, '2026-04-01', -50000, 'EUR', 'Y', 'tr_csv', 'buy')"
        ).bind(acc.id).execute(&pool).await.unwrap();
        let (tx_id,): (i64,) = sqlx::query_as("SELECT id FROM transactions WHERE account_id=?1")
            .bind(acc.id)
            .fetch_one(&pool)
            .await
            .unwrap();

        // st.account_id = None → fallback
        crate::db::trades::insert_trade_row(
            &pool,
            tx_id,
            sec.id,
            "buy",
            5_000_000,
            Some(10_000_000),
            0,
            0,
            0,
            None,
            None,
            None,
        )
        .await
        .unwrap();

        let result = portfolio_value_by_account_on_date(&pool, "2026-04-30")
            .await
            .unwrap();
        let value = result.iter().find(|(acc_id, _)| *acc_id == acc.id);
        assert!(
            value.is_some(),
            "Holdings should bucket to tx.account_id when st.account_id is NULL"
        );
    }

    // ── asset_allocation tests ───────────────────────────────────────────────

    #[tokio::test]
    async fn asset_allocation_empty_db_returns_empty() {
        let pool = connect_memory().await.unwrap();
        let result = asset_allocation(&pool, "asset_type").await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn asset_allocation_groups_by_asset_type() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool).await;
        let s1 = seed_security(&pool, "US0378331005", "Apple").await;
        // Apple is 'stock' (default from seed_security helper)
        let s2 = seed_security(&pool, "DE000A0F5UH1", "iShares MSCI World").await;
        sqlx::query("UPDATE securities SET asset_type = 'etf_equity' WHERE id = ?")
            .bind(s2)
            .execute(&pool)
            .await
            .unwrap();

        seed_trade(&pool, acc, s1, "2026-01-15", "buy", 5_000_000, -50_000).await;
        seed_trade(&pool, acc, s2, "2026-01-15", "buy", 10_000_000, -100_000).await;

        let mut result = asset_allocation(&pool, "asset_type").await.unwrap();
        result.sort_by(|a, b| a.key.cmp(&b.key));
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].key, "etf_equity");
        assert_eq!(result[0].value_cents, 100_000);
        assert_eq!(result[1].key, "stock");
        assert_eq!(result[1].value_cents, 50_000);
    }

    #[tokio::test]
    async fn asset_allocation_country_uses_security_column_when_no_breakdown() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool).await;
        let s = seed_security(&pool, "US0378331005", "Apple").await;
        sqlx::query("UPDATE securities SET country = 'US' WHERE id = ?")
            .bind(s)
            .execute(&pool)
            .await
            .unwrap();
        seed_trade(&pool, acc, s, "2026-01-15", "buy", 10_000_000, -100_000).await;

        let result = asset_allocation(&pool, "country").await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].key, "US");
        assert_eq!(result[0].value_cents, 100_000);
    }

    #[tokio::test]
    async fn asset_allocation_country_uses_breakdown_weights() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool).await;
        let etf = seed_security(&pool, "IE00BK5BQT80", "VWCE").await;
        sqlx::query("UPDATE securities SET asset_type = 'etf_equity' WHERE id = ?")
            .bind(etf)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO security_breakdowns (security_id, dimension, key, weight_bps) VALUES (?1, 'country', 'US', 7000)")
            .bind(etf).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO security_breakdowns (security_id, dimension, key, weight_bps) VALUES (?1, 'country', 'JP', 2000)")
            .bind(etf).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO security_breakdowns (security_id, dimension, key, weight_bps) VALUES (?1, 'country', 'EU', 1000)")
            .bind(etf).execute(&pool).await.unwrap();
        seed_trade(&pool, acc, etf, "2026-01-15", "buy", 10_000_000, -100_000).await;

        let mut result = asset_allocation(&pool, "country").await.unwrap();
        result.sort_by(|a, b| a.key.cmp(&b.key));
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].key, "EU");
        assert_eq!(result[0].value_cents, 10_000);
        assert_eq!(result[1].key, "JP");
        assert_eq!(result[1].value_cents, 20_000);
        assert_eq!(result[2].key, "US");
        assert_eq!(result[2].value_cents, 70_000);
    }

    // ── cost_basis_history_daily tests ──────────────────────────────────────

    #[tokio::test]
    async fn cost_basis_history_daily_empty_db_returns_zeros() {
        let pool = connect_memory().await.unwrap();
        let points = cost_basis_history_daily(&pool, "2026-05-21", 3)
            .await
            .unwrap();
        assert_eq!(points.len(), 3);
        for p in &points {
            assert_eq!(p.cost_basis_cents, 0);
            assert_eq!(p.market_value_cents, 0);
        }
        assert_eq!(points[0].date, "2026-05-19");
        assert_eq!(points[1].date, "2026-05-20");
        assert_eq!(points[2].date, "2026-05-21");
    }

    #[tokio::test]
    async fn cost_basis_history_daily_with_buy_shows_cost_only_on_or_after_buy() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool).await;
        let sec = seed_security(&pool, "DE0001234567", "Test").await;
        // Buy on 2026-05-20
        seed_buy(&pool, acc, sec, "2026-05-20", 100_000, 10).await;

        let points = cost_basis_history_daily(&pool, "2026-05-21", 3)
            .await
            .unwrap();
        // 2026-05-19: nothing yet
        assert_eq!(points[0].cost_basis_cents, 0);
        // 2026-05-20: buy happened, cost should be > 0
        assert!(points[1].cost_basis_cents > 0);
        // 2026-05-21: still has the buy
        assert_eq!(points[2].cost_basis_cents, points[1].cost_basis_cents);
    }

    #[tokio::test]
    async fn cost_basis_history_daily_days_zero_returns_empty() {
        let pool = connect_memory().await.unwrap();
        let points = cost_basis_history_daily(&pool, "2026-05-21", 0)
            .await
            .unwrap();
        assert!(points.is_empty());
    }

    #[tokio::test]
    async fn refresh_all_prices_with_mock_writes_prices() {
        use crate::pricing_provider::mock::MockProvider;
        use chrono::NaiveDate;

        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool).await;
        let sec = seed_security(&pool, "US0378331005", "Apple").await;
        seed_trade(&pool, acc, sec, "2026-01-15", "buy", 10_000_000, -50_000).await;

        let mock = MockProvider::new()
            .with_symbol("US0378331005", "AAPL")
            .with_quote(
                "AAPL",
                180_500_000,
                NaiveDate::from_ymd_opt(2026, 5, 19).unwrap(),
            );

        let report = refresh_all_prices(&pool, &mock).await.unwrap();
        assert_eq!(report.securities_total, 1);
        assert_eq!(report.prices_updated, 1);
        assert_eq!(report.prices_failed, 0);

        // Symbol stored
        let (sym,): (Option<String>,) =
            sqlx::query_as("SELECT symbol FROM securities WHERE id = ?")
                .bind(sec)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(sym.as_deref(), Some("AAPL"));

        // Price in security_prices
        let (count,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM security_prices WHERE security_id = ?")
                .bind(sec)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn refresh_all_prices_fetches_history_for_new_security() {
        use crate::pricing_provider::{mock::MockProvider, PricePoint};
        use chrono::NaiveDate;

        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool).await;
        let sec = seed_security(&pool, "DE0007236101", "Test AG").await;
        seed_buy(&pool, acc, sec, "2026-05-15", 100_000, 10).await;

        // No prices in DB yet → refresh should fetch history.
        let provider = MockProvider::new()
            .with_symbol("DE0007236101", "TEST.DE")
            .with_history(
                "TEST.DE",
                vec![
                    PricePoint {
                        date: NaiveDate::from_ymd_opt(2026, 5, 18).unwrap(),
                        close_micro: 100_000_000,
                    },
                    PricePoint {
                        date: NaiveDate::from_ymd_opt(2026, 5, 19).unwrap(),
                        close_micro: 101_000_000,
                    },
                    PricePoint {
                        date: NaiveDate::from_ymd_opt(2026, 5, 20).unwrap(),
                        close_micro: 102_000_000,
                    },
                ],
            );

        let report = refresh_all_prices(&pool, &provider).await.unwrap();
        assert_eq!(report.prices_updated, 3, "should fetch 3 history points");

        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM security_prices WHERE security_id = ?")
                .bind(sec)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(count, 3);
    }

    #[tokio::test]
    async fn refresh_all_prices_uses_quote_for_existing_security() {
        use crate::pricing_provider::mock::MockProvider;
        use chrono::NaiveDate;

        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool).await;
        let sec = seed_security(&pool, "DE0007236101", "Test AG").await;
        seed_buy(&pool, acc, sec, "2026-05-15", 100_000, 10).await;

        // Pre-seed a price so security counts as "existing".
        crate::db::prices::upsert_price(&pool, sec, "2026-05-19", 100_000_000, "manual")
            .await
            .unwrap();

        let provider = MockProvider::new()
            .with_symbol("DE0007236101", "TEST.DE")
            .with_quote(
                "TEST.DE",
                103_000_000,
                NaiveDate::from_ymd_opt(2026, 5, 20).unwrap(),
            );

        let report = refresh_all_prices(&pool, &provider).await.unwrap();
        assert_eq!(
            report.prices_updated, 1,
            "should fetch single quote, not history"
        );

        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM security_prices WHERE security_id = ?")
                .bind(sec)
                .fetch_one(&pool)
                .await
                .unwrap();
        // 1 pre-seeded + 1 from quote = 2 total
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn refresh_all_prices_updates_currency_from_quote() {
        use crate::pricing_provider::mock::MockProvider;
        use chrono::NaiveDate;

        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool).await;
        // Security with default currency EUR, but actually USD-denominated
        let sec = seed_security(&pool, "DE0007236101", "USD ETF").await;
        seed_buy(&pool, acc, sec, "2026-05-15", 100_000, 10).await;
        crate::db::prices::upsert_price(&pool, sec, "2026-05-19", 100_000_000, "manual")
            .await
            .unwrap();

        let provider = MockProvider::new()
            .with_symbol("DE0007236101", "TEST.DE")
            .with_quote_currency(
                "TEST.DE",
                103_000_000,
                NaiveDate::from_ymd_opt(2026, 5, 20).unwrap(),
                "USD",
            );

        refresh_all_prices(&pool, &provider).await.unwrap();

        let cur: String = sqlx::query_scalar("SELECT currency FROM securities WHERE id = ?1")
            .bind(sec)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(cur, "USD");
    }

    #[tokio::test]
    async fn refresh_all_prices_keeps_currency_when_quote_has_none() {
        // When the provider returns no currency, the DB remains unchanged.
        use crate::pricing_provider::mock::MockProvider;
        use chrono::NaiveDate;

        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool).await;
        let sec = seed_security(&pool, "DE0007236102", "EUR ETF").await;
        seed_buy(&pool, acc, sec, "2026-05-15", 100_000, 10).await;
        crate::db::prices::upsert_price(&pool, sec, "2026-05-19", 100_000_000, "manual")
            .await
            .unwrap();

        let provider = MockProvider::new()
            .with_symbol("DE0007236102", "EUR.DE")
            // with_quote without _currency suffix → currency=None
            .with_quote(
                "EUR.DE",
                100_000_000,
                NaiveDate::from_ymd_opt(2026, 5, 20).unwrap(),
            );

        refresh_all_prices(&pool, &provider).await.unwrap();
        let cur: String = sqlx::query_scalar("SELECT currency FROM securities WHERE id = ?1")
            .bind(sec)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(cur, "EUR", "currency must remain unchanged");
    }

    // ── seed helpers for bucket tests ────────────────────────────────────────

    async fn seed_bucket(pool: &sqlx::SqlitePool, name: &str) -> i64 {
        let (id,): (i64,) = sqlx::query_as("INSERT INTO buckets (name) VALUES (?1) RETURNING id")
            .bind(name)
            .fetch_one(pool)
            .await
            .unwrap();
        id
    }

    // ── security_bucket_allocations tests ────────────────────────────────────

    #[tokio::test]
    async fn set_allocations_writes_rows_and_idempotent_replace() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool).await;
        let sec = seed_security(&pool, "US0378331005", "Apple").await;
        seed_buy(&pool, acc, sec, "2026-01-15", 10_000_000, 100_000).await;
        let b1 = seed_bucket(&pool, "Altersvorsorge").await;
        let b2 = seed_bucket(&pool, "Urlaub").await;

        // First call: 6 + 4 = 10 shares
        set_allocations_for_security(&pool, sec, &[(b1, 6_000_000), (b2, 4_000_000)])
            .await
            .unwrap();

        let rows = list_allocations_for_security(&pool, sec).await.unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows.iter()
                .find(|r| r.bucket_id == b1)
                .unwrap()
                .shares_micro,
            6_000_000
        );
        assert_eq!(
            rows.iter()
                .find(|r| r.bucket_id == b2)
                .unwrap()
                .shares_micro,
            4_000_000
        );

        // Second call: replaces completely
        set_allocations_for_security(&pool, sec, &[(b1, 3_000_000)])
            .await
            .unwrap();

        let rows2 = list_allocations_for_security(&pool, sec).await.unwrap();
        assert_eq!(rows2.len(), 1);
        assert_eq!(rows2[0].bucket_id, b1);
        assert_eq!(rows2[0].shares_micro, 3_000_000);
    }

    #[tokio::test]
    async fn set_allocations_rejects_over_allocation() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool).await;
        let sec = seed_security(&pool, "US0378331005", "Apple").await;
        // Only 10 shares held
        seed_buy(&pool, acc, sec, "2026-01-15", 10_000_000, 100_000).await;
        let b = seed_bucket(&pool, "Test").await;

        // 11 > 10 → Err
        let result = set_allocations_for_security(&pool, sec, &[(b, 11_000_000)]).await;
        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("exceeds"), "Error message: {msg}");
    }

    #[tokio::test]
    async fn set_allocations_rejects_duplicate_bucket_ids() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool).await;
        let sec = seed_security(&pool, "US0378331005", "Apple").await;
        seed_buy(&pool, acc, sec, "2026-01-15", 10_000_000, 100_000).await;
        let b = seed_bucket(&pool, "Test").await;

        let result =
            set_allocations_for_security(&pool, sec, &[(b, 3_000_000), (b, 2_000_000)]).await;
        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("Duplicate"), "Error message: {msg}");
    }

    #[tokio::test]
    async fn set_allocations_rejects_zero_or_negative_shares() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool).await;
        let sec = seed_security(&pool, "US0378331005", "Apple").await;
        seed_buy(&pool, acc, sec, "2026-01-15", 10_000_000, 100_000).await;
        let b = seed_bucket(&pool, "Test").await;

        let zero = set_allocations_for_security(&pool, sec, &[(b, 0)]).await;
        assert!(zero.is_err(), "shares_micro = 0 must be rejected");

        let neg = set_allocations_for_security(&pool, sec, &[(b, -1)]).await;
        assert!(neg.is_err(), "shares_micro < 0 must be rejected");
    }

    #[tokio::test]
    async fn list_allocations_for_security_returns_empty_when_none() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool).await;
        let sec = seed_security(&pool, "US0378331005", "Apple").await;
        seed_buy(&pool, acc, sec, "2026-01-15", 10_000_000, 100_000).await;

        let rows = list_allocations_for_security(&pool, sec).await.unwrap();
        assert!(rows.is_empty());
    }

    #[tokio::test]
    async fn bucket_holdings_returns_value_based_on_current_price() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool).await;
        let sec = seed_security(&pool, "US0378331005", "Apple").await;
        // 10 shares at €10 cost; price €20 → market_value = 20000 cents
        seed_buy(&pool, acc, sec, "2026-01-15", 10_000_000, 1_000_000).await;
        crate::db::prices::upsert_price(&pool, sec, "2026-05-20", 20_000_000, "yahoo")
            .await
            .unwrap();
        let b = seed_bucket(&pool, "Depot").await;
        // 4 shares allocated → value = 20000 * 4/10 = 8000 cents
        set_allocations_for_security(&pool, sec, &[(b, 4_000_000)])
            .await
            .unwrap();

        let rows = bucket_holdings(&pool, b).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].security_id, sec);
        assert_eq!(rows[0].shares_micro, 4_000_000);
        // market_value 20000 cents × 4/10 = 8000
        assert_eq!(rows[0].value_cents, 8_000);
    }
}
