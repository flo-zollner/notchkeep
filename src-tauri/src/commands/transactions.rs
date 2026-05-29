use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tauri::State;

use crate::categorization::fuzzy::{suggest_category_from_history_scored, HistoryEntry};
use crate::commands::accounts::{CommandError, DbState};
use crate::db::transactions::{insert_raw_transaction, InsertOutcome};
use crate::importers::RawTransaction;
use crate::model::Transaction;

const FUZZY_THRESHOLD: f64 = 0.85;
/// Trade kinds that must NOT be touched via `update_transaction`/`delete_transaction`
/// (cash editor) — they carry trade-detail fields that can only be changed
/// consistently through the trade commands.
///
/// `fee` is intentionally excluded: a pure fee transaction is cash-editable
/// (e.g. a custody fee without a trade row).
const TRADE_KINDS: &[&str] = &["buy", "sell", "dividend", "corporate_action", "tax"];

/// Trims and uppercases an IBAN. Empty → None. Format check
/// (`^[A-Z]{2}\d{2}[A-Z0-9]{11,30}$`).
fn normalize_iban(raw: Option<&str>) -> Result<Option<String>, CommandError> {
    let Some(s) = raw else { return Ok(None) };
    let cleaned: String = s.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_uppercase();
    if cleaned.is_empty() {
        return Ok(None);
    }
    if cleaned.len() < 15 || cleaned.len() > 34 {
        return Err(CommandError {
            message: format!("iban length must be 15-34, got {} for {cleaned:?}", cleaned.len()),
        });
    }
    let mut chars = cleaned.chars();
    let c1 = chars.next().unwrap();
    let c2 = chars.next().unwrap();
    let d1 = chars.next().unwrap();
    let d2 = chars.next().unwrap();
    if !c1.is_ascii_uppercase() || !c2.is_ascii_uppercase()
        || !d1.is_ascii_digit() || !d2.is_ascii_digit()
        || !chars.all(|c| c.is_ascii_alphanumeric() && (c.is_ascii_uppercase() || c.is_ascii_digit()))
    {
        return Err(CommandError {
            message: format!("iban format invalid: {cleaned:?}"),
        });
    }
    Ok(Some(cleaned))
}

/// Returns Err if the given tx is a securities-trade (kind ∈ trades).
/// Trade rows must be mutated through the trade commands so that the
/// `securities_trades` side stays consistent. Missing tx → Ok (the caller
/// will produce its own "not found" error).
async fn assert_not_trade_tx(pool: &SqlitePool, tx_id: i64) -> Result<(), CommandError> {
    let row: Option<(String,)> = sqlx::query_as("SELECT kind FROM transactions WHERE id = ?")
        .bind(tx_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| CommandError { message: e.to_string() })?;
    if let Some((kind,)) = row {
        if TRADE_KINDS.contains(&kind.as_str()) {
            return Err(CommandError {
                message: format!(
                    "transaction {tx_id} is a {kind} trade — use the trade commands instead"
                ),
            });
        }
    }
    Ok(())
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TxFilter {
    pub account_id: Option<i64>,
    pub category_id: Option<i64>,
    pub bucket_id: Option<i64>,
    pub institution_id: Option<i64>,
    pub search: Option<String>,
    /// Inclusive ('YYYY-MM-DD'). Tx with booking_date >= from.
    pub from: Option<String>,
    /// Inclusive ('YYYY-MM-DD'). Tx with booking_date <= to.
    pub to: Option<String>,
    /// If true: only Tx without a category (category_id IS NULL).
    pub uncategorized: Option<bool>,
    /// abs(amount_cents) >= min_amount_cents.
    pub min_amount_cents: Option<i64>,
    /// Page size; default 200, clamped to [1, 5000].
    pub limit: Option<i64>,
    /// Opaque cursor 'YYYY-MM-DD|<id>' — fetch Tx before this point (in DESC order).
    pub cursor: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListTransactionsPage {
    pub rows: Vec<Transaction>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

/// Aggregate totals across all Tx matching the filter (without LIMIT/cursor).
/// Excludes transfer/buy/sell/corporate_action (= not real cash flows).
#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct TxAggregate {
    pub in_cents: i64,
    pub out_cents: i64,
    pub count: i64,
}

fn parse_cursor(s: &str) -> Option<(String, i64)> {
    let (d, idstr) = s.split_once('|')?;
    let id = idstr.parse::<i64>().ok()?;
    Some((d.to_string(), id))
}

/// Builds the WHERE clauses for a TxFilter (without ORDER/LIMIT) and returns
/// the SQL fragment together with the bind order. Used by both
/// `list_transactions_inner` and `aggregate_transactions_inner`.
fn build_tx_where(f: &TxFilter) -> (String, Vec<TxBind>) {
    let mut sql = String::new();
    let mut binds: Vec<TxBind> = Vec::new();

    if let Some(aid) = f.account_id {
        sql.push_str(
            " AND (account_id = ? OR id IN (\
                SELECT tx_id FROM securities_trades \
                 WHERE account_id = ? \
                   AND side IN ('buy','sell','corporate_action','fusion_out','fusion_in')\
             ))"
        );
        binds.push(TxBind::I64(aid));
        binds.push(TxBind::I64(aid));
    }
    if let Some(cid) = f.category_id {
        sql.push_str(" AND category_id = ?");
        binds.push(TxBind::I64(cid));
    }
    if let Some(bid) = f.bucket_id {
        sql.push_str(" AND bucket_id = ?");
        binds.push(TxBind::I64(bid));
    }
    if let Some(iid) = f.institution_id {
        sql.push_str(" AND account_id IN (SELECT id FROM accounts WHERE institution_id = ?)");
        binds.push(TxBind::I64(iid));
    }
    if let Some(s) = f.search.as_deref().filter(|s| !s.is_empty()) {
        sql.push_str(" AND (counterparty LIKE ? OR purpose LIKE ? OR manual_note LIKE ?)");
        let needle = format!("%{s}%");
        binds.push(TxBind::Str(needle.clone()));
        binds.push(TxBind::Str(needle.clone()));
        binds.push(TxBind::Str(needle));
    }
    if let Some(from) = f.from.as_deref().filter(|s| !s.is_empty()) {
        sql.push_str(" AND booking_date >= ?");
        binds.push(TxBind::Str(from.to_string()));
    }
    if let Some(to) = f.to.as_deref().filter(|s| !s.is_empty()) {
        sql.push_str(" AND booking_date <= ?");
        binds.push(TxBind::Str(to.to_string()));
    }
    if f.uncategorized == Some(true) {
        sql.push_str(" AND category_id IS NULL");
    }
    if let Some(min) = f.min_amount_cents.filter(|n| *n > 0) {
        sql.push_str(" AND ABS(amount_cents) >= ?");
        binds.push(TxBind::I64(min));
    }
    (sql, binds)
}

enum TxBind {
    I64(i64),
    Str(String),
}

fn bind_all<'q, O>(
    mut q: sqlx::query::QueryAs<'q, sqlx::Sqlite, O, sqlx::sqlite::SqliteArguments<'q>>,
    binds: Vec<TxBind>,
) -> sqlx::query::QueryAs<'q, sqlx::Sqlite, O, sqlx::sqlite::SqliteArguments<'q>>
where
    O: for<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow> + Send + Unpin,
{
    for b in binds {
        q = match b {
            TxBind::I64(v) => q.bind(v),
            TxBind::Str(v) => q.bind(v),
        };
    }
    q
}

pub(crate) async fn list_transactions_inner(
    pool: &sqlx::SqlitePool,
    f: &TxFilter,
) -> Result<ListTransactionsPage, sqlx::Error> {
    let limit = f.limit.unwrap_or(200).clamp(1, 5000);

    let mut sql = String::from(
        "SELECT id, account_id, booking_date, value_date, amount_cents, currency,
                counterparty, purpose, raw_ref, category_id, source, source_file_hash,
                imported_at, manual_note, bucket_id, kind, counterparty_iban,
                paired_tx_id,
                (SELECT account_id FROM securities_trades
                  WHERE tx_id = transactions.id
                    AND side IN ('buy','sell','corporate_action','fusion_out','fusion_in')
                  LIMIT 1) AS holding_account_id,
                (SELECT side FROM securities_trades
                  WHERE tx_id = transactions.id
                  LIMIT 1) AS trade_side
         FROM transactions
         WHERE 1=1",
    );

    let (where_sql, mut binds) = build_tx_where(f);
    sql.push_str(&where_sql);

    if let Some(cur) = f.cursor.as_deref().filter(|s| !s.is_empty()) {
        if let Some((date, id)) = parse_cursor(cur) {
            sql.push_str(" AND (booking_date < ? OR (booking_date = ? AND id < ?))");
            binds.push(TxBind::Str(date.clone()));
            binds.push(TxBind::Str(date));
            binds.push(TxBind::I64(id));
        }
    }
    sql.push_str(" ORDER BY booking_date DESC, id DESC LIMIT ?");
    binds.push(TxBind::I64(limit + 1)); // +1 for has_more detection

    let q = sqlx::query_as::<_, Transaction>(&sql);
    let mut rows: Vec<Transaction> = bind_all(q, binds).fetch_all(pool).await?;

    let has_more = rows.len() as i64 > limit;
    if has_more {
        rows.truncate(limit as usize);
    }
    let next_cursor = if has_more {
        rows.last().map(|t| format!("{}|{}", t.booking_date, t.id))
    } else {
        None
    };

    Ok(ListTransactionsPage { rows, next_cursor, has_more })
}

#[tauri::command]
pub async fn list_transactions(
    state: State<'_, DbState>,
    filter: Option<TxFilter>,
) -> Result<ListTransactionsPage, CommandError> {
    let f = filter.unwrap_or_default();
    Ok(list_transactions_inner(&state.pool(), &f).await?)
}

/// Aggregate totals across all filtered Tx without LIMIT/cursor.
/// `in_cents` = SUM(amount > 0), `out_cents` = SUM(-amount) for amount < 0.
/// Excludes transfer/buy/sell/corporate_action (= not real cash flows).
pub(crate) async fn aggregate_transactions_inner(
    pool: &sqlx::SqlitePool,
    f: &TxFilter,
) -> Result<TxAggregate, sqlx::Error> {
    let mut sql = String::from(
        "SELECT
            COALESCE(SUM(CASE WHEN amount_cents > 0 THEN amount_cents ELSE 0 END), 0) AS in_cents,
            COALESCE(SUM(CASE WHEN amount_cents < 0 THEN -amount_cents ELSE 0 END), 0) AS out_cents,
            COUNT(*) AS count
         FROM transactions
         WHERE kind NOT IN ('transfer','corporate_action')"
    );
    let (where_sql, binds) = build_tx_where(f);
    sql.push_str(&where_sql);

    let q = sqlx::query_as::<_, TxAggregate>(&sql);
    bind_all(q, binds).fetch_one(pool).await
}

#[tauri::command]
pub async fn aggregate_transactions(
    state: State<'_, DbState>,
    filter: Option<TxFilter>,
) -> Result<TxAggregate, CommandError> {
    let f = filter.unwrap_or_default();
    Ok(aggregate_transactions_inner(&state.pool(), &f).await?)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewTransaction {
    pub account_id: i64,
    pub booking_date: String,
    pub amount_cents: i64,
    #[serde(default = "default_currency")]
    pub currency: String,
    pub counterparty: Option<String>,
    pub purpose: Option<String>,
    pub category_id: Option<i64>,
    pub bucket_id: Option<i64>,
    pub manual_note: Option<String>,
    pub counterparty_iban: Option<String>,
    /// Optional kind override. None = auto from amount sign
    /// (positive → income, negative → expense). Allowed values: income,
    /// expense, transfer, fee, tax_general. Securities tax (kind='tax')
    /// and other trade kinds (buy/sell/...) go through the trade commands.
    #[serde(default)]
    pub kind: Option<String>,
}

fn default_currency() -> String {
    "EUR".to_string()
}

#[tauri::command]
pub async fn create_transaction(
    state: State<'_, DbState>,
    tx: NewTransaction,
) -> Result<Transaction, CommandError> {
    let booking_date = NaiveDate::parse_from_str(&tx.booking_date, "%Y-%m-%d")
        .map_err(|e| CommandError { message: format!("bad date: {e}") })?;
    let normalized_iban = normalize_iban(tx.counterparty_iban.as_deref())?;
    // Whitelist: only cash kinds are permitted from the UI. Trade kinds must
    // go through the trade commands, otherwise an orphan Tx is created
    // without a trade-detail row. 'tax_general' = general tax expense without
    // a securities reference (income tax, property tax, …); 'tax' remains
    // reserved for securities tax with a securities_trades row.
    let allowed_kinds = ["income", "expense", "transfer", "fee", "tax_general"];
    let kind = match tx.kind.as_deref() {
        Some(k) if !allowed_kinds.contains(&k) => {
            return Err(CommandError {
                message: format!(
                    "kind {k:?} not allowed for create_transaction; use trade commands for buy/sell/dividend/corporate_action/tax"
                ),
            });
        }
        other => other.map(String::from),
    };
    let raw = RawTransaction {
        booking_date,
        amount_cents: tx.amount_cents,
        currency: tx.currency,
        counterparty: tx.counterparty,
        purpose: tx.purpose,
        raw_ref: None,
        kind,
        trade: None,
        counterparty_iban: normalized_iban,
    };

    let pool = state.pool();
    let outcome = insert_raw_transaction(&pool, tx.account_id, "manual", None, &raw, tx.category_id).await?;
    let id = match outcome {
        InsertOutcome::Inserted(id) => id,
        InsertOutcome::Skipped => {
            return Err(CommandError {
                message: "duplicate transaction (account/date/amount/counterparty already exists)".into(),
            })
        }
    };

    if tx.manual_note.as_deref().map(|s| !s.is_empty()).unwrap_or(false) {
        sqlx::query("UPDATE transactions SET manual_note = ? WHERE id = ?")
            .bind(tx.manual_note.as_deref())
            .bind(id)
            .execute(&pool)
            .await?;
    }

    if tx.bucket_id.is_some() {
        sqlx::query("UPDATE transactions SET bucket_id = ? WHERE id = ?")
            .bind(tx.bucket_id)
            .bind(id)
            .execute(&pool)
            .await?;
    }

    let row: Transaction = sqlx::query_as(
        "SELECT id, account_id, booking_date, value_date, amount_cents, currency,
                counterparty, purpose, raw_ref, category_id, source, source_file_hash,
                imported_at, manual_note, bucket_id, kind, counterparty_iban,
                paired_tx_id
         FROM transactions WHERE id = ?",
    )
    .bind(id)
    .fetch_one(&pool)
    .await?;
    Ok(row)
}

#[tauri::command]
pub async fn assign_category(
    state: State<'_, DbState>,
    transaction_id: i64,
    category_id: Option<i64>,
) -> Result<(), CommandError> {
    sqlx::query("UPDATE transactions SET category_id = ? WHERE id = ?")
        .bind(category_id)
        .bind(transaction_id)
        .execute(&state.pool())
        .await?;
    Ok(())
}

#[tauri::command]
pub async fn assign_bucket(
    state: State<'_, DbState>,
    transaction_id: i64,
    bucket_id: Option<i64>,
) -> Result<(), CommandError> {
    crate::db::transactions::set_transaction_bucket(&state.pool(), transaction_id, bucket_id)
        .await?;
    Ok(())
}

/// Moves a Tx to a different account. Also works for trade Tx
/// (kind ∈ buy/sell/dividend/corporate_action) that `update_transaction`
/// would otherwise reject. Only changes `transactions.account_id`;
/// `securities_trades.account_id` is left untouched (variant B): if NULL
/// there, holdings automatically follow the new tx.account_id; if explicitly
/// set, they stay where they are. If the user also wants to move holdings,
/// they must update the trade separately through the trade-edit paths.
#[tauri::command]
pub async fn assign_account(
    state: State<'_, DbState>,
    transaction_id: i64,
    account_id: i64,
) -> Result<(), CommandError> {
    let res = sqlx::query("UPDATE transactions SET account_id = ?1 WHERE id = ?2")
        .bind(account_id)
        .bind(transaction_id)
        .execute(&state.pool())
        .await?;
    if res.rows_affected() == 0 {
        return Err(CommandError {
            message: format!("transaction {transaction_id} not found"),
        });
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTransaction {
    pub id: i64,
    pub account_id: i64,
    pub booking_date: String,
    pub amount_cents: i64,
    pub currency: String,
    pub counterparty: Option<String>,
    pub purpose: Option<String>,
    pub category_id: Option<i64>,
    pub bucket_id: Option<i64>,
    pub manual_note: Option<String>,
    pub counterparty_iban: Option<String>,
    /// Optional kind override. None = leave unchanged. Allowed values:
    /// income, expense, transfer, fee, tax_general — trade kinds would be
    /// inconsistent here (Tx without a trade-detail row).
    #[serde(default)]
    pub kind: Option<String>,
}

#[tauri::command]
pub async fn update_transaction(
    state: State<'_, DbState>,
    tx: UpdateTransaction,
) -> Result<Transaction, CommandError> {
    NaiveDate::parse_from_str(&tx.booking_date, "%Y-%m-%d")
        .map_err(|e| CommandError { message: format!("bad date: {e}") })?;
    let normalized_iban = normalize_iban(tx.counterparty_iban.as_deref())?;
    let pool = state.pool();
    assert_not_trade_tx(&pool, tx.id).await?;

    let allowed_kinds = ["income", "expense", "transfer", "fee", "tax_general"];
    if let Some(k) = tx.kind.as_deref() {
        if !allowed_kinds.contains(&k) {
            return Err(CommandError {
                message: format!(
                    "kind {k:?} not allowed for update_transaction; use trade commands for buy/sell/dividend/corporate_action/tax"
                ),
            });
        }
    }

    // SQL: only write kind if explicitly set (Option-aware).
    let rows = sqlx::query(
        "UPDATE transactions SET
            account_id        = ?1,
            booking_date      = ?2,
            amount_cents      = ?3,
            currency          = ?4,
            counterparty      = ?5,
            purpose           = ?6,
            category_id       = ?7,
            bucket_id         = ?8,
            manual_note       = ?9,
            counterparty_iban = ?10,
            kind              = COALESCE(?11, kind)
         WHERE id = ?12",
    )
    .bind(tx.account_id)
    .bind(&tx.booking_date)
    .bind(tx.amount_cents)
    .bind(&tx.currency)
    .bind(tx.counterparty.as_deref())
    .bind(tx.purpose.as_deref())
    .bind(tx.category_id)
    .bind(tx.bucket_id)
    .bind(tx.manual_note.as_deref().filter(|s| !s.is_empty()))
    .bind(normalized_iban.as_deref())
    .bind(tx.kind.as_deref())
    .bind(tx.id)
    .execute(&pool)
    .await?
    .rows_affected();

    if rows == 0 {
        return Err(CommandError { message: format!("transaction {} not found", tx.id) });
    }

    let row: Transaction = sqlx::query_as(
        "SELECT id, account_id, booking_date, value_date, amount_cents, currency,
                counterparty, purpose, raw_ref, category_id, source, source_file_hash,
                imported_at, manual_note, bucket_id, kind, counterparty_iban,
                paired_tx_id
         FROM transactions WHERE id = ?",
    )
    .bind(tx.id)
    .fetch_one(&pool)
    .await?;
    Ok(row)
}

#[tauri::command]
pub async fn delete_transaction(
    state: State<'_, DbState>,
    id: i64,
) -> Result<(), CommandError> {
    let pool = state.pool();
    assert_not_trade_tx(&pool, id).await?;
    let rows = sqlx::query("DELETE FROM transactions WHERE id = ?")
        .bind(id)
        .execute(&pool)
        .await?
        .rows_affected();
    if rows == 0 {
        return Err(CommandError { message: format!("transaction {id} not found") });
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategorySuggestion {
    pub category_id: i64,
    pub category_name: String,
    pub score: f64,
}

#[tauri::command]
pub async fn suggest_category(
    state: State<'_, DbState>,
    name: String,
    account_id: Option<i64>,
) -> Result<Option<CategorySuggestion>, CommandError> {
    if name.trim().is_empty() {
        return Ok(None);
    }

    let pool = state.pool();
    let history = load_history(&pool, account_id).await?;
    let Some((category_id, score)) =
        suggest_category_from_history_scored(&name, &history, FUZZY_THRESHOLD)
    else {
        return Ok(None);
    };

    let (category_name,): (String,) =
        sqlx::query_as("SELECT name FROM categories WHERE id = ?")
            .bind(category_id)
            .fetch_one(&pool)
            .await?;

    Ok(Some(CategorySuggestion { category_id, category_name, score }))
}

async fn load_history(
    pool: &sqlx::SqlitePool,
    account_id: Option<i64>,
) -> Result<Vec<HistoryEntry>, CommandError> {
    let mut sql = String::from(
        "SELECT counterparty, category_id FROM transactions t1
         WHERE counterparty IS NOT NULL
           AND category_id IS NOT NULL",
    );
    if account_id.is_some() {
        sql.push_str(" AND account_id = ?");
    }
    sql.push_str(
        "   AND booking_date = (
               SELECT MAX(booking_date) FROM transactions t2
               WHERE t2.counterparty = t1.counterparty
                 AND t2.category_id IS NOT NULL
           )
         GROUP BY counterparty",
    );

    let mut q = sqlx::query_as::<_, (String, i64)>(&sql);
    if let Some(aid) = account_id {
        q = q.bind(aid);
    }
    let rows = q.fetch_all(pool).await?;
    Ok(rows
        .into_iter()
        .map(|(counterparty, category_id)| HistoryEntry { counterparty, category_id })
        .collect())
}

/// Detects transactions where `counterparty_iban` matches one of the user's
/// own account IBANs and sets `kind = 'transfer'`. Useful for existing data
/// that was imported before the P25 update.
#[tauri::command]
pub async fn detect_transfers(
    state: State<'_, DbState>,
) -> Result<usize, CommandError> {
    let pool = state.pool();
    let detected = crate::import_flow::detect_inter_account_transfers(&pool).await?;
    let _ = cleanup_phantom_mirrors_inner(&pool).await;
    Ok(detected)
}

/// Finds auto-pair mirror Tx for which a real matching Tx exists on the same
/// account within +/- 3 days. Two scenarios:
///
/// 1. Real Tx is still UNPAIRED → delete the mirror and link the real Tx to
///    the original.
/// 2. Real Tx is paired with ANOTHER auto-pair mirror (double-sided detection,
///    both sides carry counterparty_iban) → delete BOTH mirrors and link the
///    two REAL Tx directly.
///
/// Returns the number of fixed mirror pairs.
pub(crate) async fn cleanup_phantom_mirrors_inner(pool: &SqlitePool) -> Result<usize, sqlx::Error> {
    // Fetch all auto_pair mirrors (with their originals)
    let mirrors: Vec<(i64, i64, String, i64, String, Option<i64>)> = sqlx::query_as(
        "SELECT id, account_id, booking_date, amount_cents, currency, paired_tx_id
           FROM transactions WHERE source = 'auto_pair'"
    ).fetch_all(pool).await?;

    let mut cleaned = 0usize;
    for (mirror_id, account_id, booking_date, amount_cents, currency, original_paired) in mirrors {
        // Skip if the mirror no longer exists (e.g. cascade-deleted in a previous iteration)
        let still_exists: Option<(i64,)> = sqlx::query_as(
            "SELECT id FROM transactions WHERE id = ?1"
        ).bind(mirror_id).fetch_optional(pool).await?;
        if still_exists.is_none() {
            continue;
        }

        // Search for a real matching Tx on the mirror's account within the ±3-day window
        let real: Option<(i64, Option<i64>)> = sqlx::query_as(
            "SELECT id, paired_tx_id FROM transactions
              WHERE account_id = ?1
                AND amount_cents = ?2
                AND currency = ?3
                AND id != ?4
                AND source != 'auto_pair'
                AND booking_date BETWEEN date(?5, '-3 days') AND date(?5, '+3 days')
              ORDER BY ABS(julianday(booking_date) - julianday(?5)), id
              LIMIT 1"
        )
        .bind(account_id).bind(amount_cents).bind(&currency).bind(mirror_id).bind(&booking_date)
        .fetch_optional(pool).await?;

        let Some((real_id, real_paired)) = real else {
            continue;
        };

        let orig_id = match original_paired {
            Some(o) => o,
            None => continue, // Mirror without an original — should not happen, skip
        };

        match real_paired {
            // CASE 1: real_R is still unpaired
            None => {
                // Re-point original.paired_tx_id to real_R
                sqlx::query("UPDATE transactions SET paired_tx_id = ?1 WHERE id = ?2")
                    .bind(real_id).bind(orig_id)
                    .execute(pool).await?;
                // real_R gets kind=transfer + paired_tx_id = orig
                sqlx::query("UPDATE transactions SET kind = 'transfer', paired_tx_id = ?1 WHERE id = ?2")
                    .bind(orig_id).bind(real_id)
                    .execute(pool).await?;
                // Delete the mirror
                sqlx::query("DELETE FROM transactions WHERE id = ?1")
                    .bind(mirror_id)
                    .execute(pool).await?;
                cleaned += 1;
            }
            // CASE 2: real_R is paired with another mirror (double-sided phantom detection)
            Some(other_mirror_id) => {
                // Verify that other_mirror_id is really an auto_pair mirror
                let other_check: Option<(String, i64)> = sqlx::query_as(
                    "SELECT source, account_id FROM transactions WHERE id = ?1"
                ).bind(other_mirror_id)
                .fetch_optional(pool).await?;
                let Some((other_source, other_account_id)) = other_check else { continue };
                if other_source != "auto_pair" {
                    // real_R is paired with a real Tx — we must not interfere, skip
                    continue;
                }
                // Verify: other_mirror_id should be on orig_id's account
                let orig_account: Option<(i64,)> = sqlx::query_as(
                    "SELECT account_id FROM transactions WHERE id = ?1"
                ).bind(orig_id)
                .fetch_optional(pool).await?;
                let Some((orig_account_id,)) = orig_account else { continue };
                if other_account_id != orig_account_id {
                    // Mirror M2 is on a third account — atypical, skip
                    continue;
                }

                // Re-link both REAL Tx to each other
                sqlx::query("UPDATE transactions SET paired_tx_id = ?1 WHERE id = ?2")
                    .bind(real_id).bind(orig_id)
                    .execute(pool).await?;
                sqlx::query("UPDATE transactions SET kind = 'transfer', paired_tx_id = ?1 WHERE id = ?2")
                    .bind(orig_id).bind(real_id)
                    .execute(pool).await?;
                // Delete BOTH mirrors
                sqlx::query("DELETE FROM transactions WHERE id = ?1")
                    .bind(mirror_id)
                    .execute(pool).await?;
                sqlx::query("DELETE FROM transactions WHERE id = ?1")
                    .bind(other_mirror_id)
                    .execute(pool).await?;
                cleaned += 1;
            }
        }
    }
    Ok(cleaned)
}

#[tauri::command]
pub async fn cleanup_phantom_mirrors(state: State<'_, DbState>) -> Result<usize, CommandError> {
    Ok(cleanup_phantom_mirrors_inner(&state.pool()).await
        .map_err(|e| CommandError { message: e.to_string() })?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_memory;

    async fn seed_account(pool: &sqlx::SqlitePool, name: &str) -> i64 {
        let (id,): (i64,) = sqlx::query_as(
            "INSERT INTO accounts (name, kind, currency) VALUES (?1, 'bank', 'EUR') RETURNING id",
        )
        .bind(name)
        .fetch_one(pool)
        .await
        .unwrap();
        id
    }

    async fn seed_category(pool: &sqlx::SqlitePool, name: &str) -> i64 {
        let (id,): (i64,) =
            sqlx::query_as("INSERT INTO categories (name) VALUES (?1) RETURNING id")
                .bind(name)
                .fetch_one(pool)
                .await
                .unwrap();
        id
    }

    #[tokio::test]
    async fn list_transactions_returns_sorted_desc() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool, "TR").await;
        for (date, amount, cp) in [
            ("2026-05-10", -1299_i64, "REWE"),
            ("2026-05-12", -500, "EDEKA"),
            ("2026-05-08", -2200, "LIDL"),
        ] {
            sqlx::query(
                "INSERT INTO transactions (account_id, booking_date, amount_cents, currency, counterparty, source)
                 VALUES (?1, ?2, ?3, 'EUR', ?4, 'manual')",
            )
            .bind(acc).bind(date).bind(amount).bind(cp)
            .execute(&pool).await.unwrap();
        }

        let sql = "SELECT id, account_id, booking_date, value_date, amount_cents, currency,
                          counterparty, purpose, raw_ref, category_id, source, source_file_hash,
                          imported_at, manual_note, bucket_id, kind, counterparty_iban,
                          paired_tx_id
                   FROM transactions ORDER BY booking_date DESC, id DESC LIMIT 100";
        let rows: Vec<Transaction> = sqlx::query_as(sql).fetch_all(&pool).await.unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].counterparty.as_deref(), Some("EDEKA"));
        assert_eq!(rows[2].counterparty.as_deref(), Some("LIDL"));
    }

    #[tokio::test]
    async fn assign_account_moves_trade_tx_to_other_account() {
        // Trade Tx (kind=buy) must be movable — assert_not_trade_tx
        // in update_transaction would normally reject this.
        let pool = connect_memory().await.unwrap();
        let from_acc = seed_account(&pool, "TR Depot").await;
        let to_acc = seed_account(&pool, "Flatex Depot").await;
        let (tx_id,): (i64,) = sqlx::query_as(
            "INSERT INTO transactions (account_id, booking_date, amount_cents, currency, source, kind)
             VALUES (?1, '2026-05-10', -10000, 'EUR', 'flatex_pdf', 'buy') RETURNING id",
        ).bind(from_acc).fetch_one(&pool).await.unwrap();

        // Inner UPDATE (directly, because State<DbState> is not testable)
        sqlx::query("UPDATE transactions SET account_id = ?1 WHERE id = ?2")
            .bind(to_acc).bind(tx_id).execute(&pool).await.unwrap();

        let (stored,): (i64,) =
            sqlx::query_as("SELECT account_id FROM transactions WHERE id = ?1")
                .bind(tx_id).fetch_one(&pool).await.unwrap();
        assert_eq!(stored, to_acc);
    }

    #[tokio::test]
    async fn assign_category_updates_row() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool, "TR").await;
        let cat = seed_category(&pool, "Lebensmittel").await;
        let (tx_id,): (i64,) = sqlx::query_as(
            "INSERT INTO transactions (account_id, booking_date, amount_cents, currency, source)
             VALUES (?1, '2026-05-10', -1299, 'EUR', 'manual') RETURNING id",
        )
        .bind(acc)
        .fetch_one(&pool)
        .await
        .unwrap();

        sqlx::query("UPDATE transactions SET category_id = ? WHERE id = ?")
            .bind(cat)
            .bind(tx_id)
            .execute(&pool)
            .await
            .unwrap();

        let (stored,): (Option<i64>,) =
            sqlx::query_as("SELECT category_id FROM transactions WHERE id = ?")
                .bind(tx_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(stored, Some(cat));
    }

    async fn insert_tx(
        pool: &sqlx::SqlitePool,
        account_id: i64,
        date: &str,
        amount: i64,
        cp: Option<&str>,
        category_id: Option<i64>,
    ) -> i64 {
        let (id,): (i64,) = sqlx::query_as(
            "INSERT INTO transactions
                (account_id, booking_date, amount_cents, currency, counterparty,
                 category_id, source)
             VALUES (?1, ?2, ?3, 'EUR', ?4, ?5, 'manual') RETURNING id",
        )
        .bind(account_id)
        .bind(date)
        .bind(amount)
        .bind(cp)
        .bind(category_id)
        .fetch_one(pool)
        .await
        .unwrap();
        id
    }

    #[tokio::test]
    async fn update_transaction_changes_fields() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool, "TR").await;
        let cat = seed_category(&pool, "Lebensmittel").await;
        let id = insert_tx(&pool, acc, "2026-05-10", -1299, Some("REWE"), None).await;

        let upd = UpdateTransaction {
            id,
            account_id: acc,
            booking_date: "2026-05-11".into(),
            amount_cents: -1500,
            currency: "EUR".into(),
            counterparty: Some("REWE Markt".into()),
            purpose: Some("Wocheneinkauf".into()),
            category_id: Some(cat),
            bucket_id: None,
            manual_note: Some("Notiz".into()),
            counterparty_iban: None,
            kind: None,
        };

        let row: Transaction = sqlx::query_as(
            "SELECT id, account_id, booking_date, value_date, amount_cents, currency,
                    counterparty, purpose, raw_ref, category_id, source, source_file_hash,
                    imported_at, manual_note, bucket_id, kind, counterparty_iban,
                    paired_tx_id
             FROM transactions WHERE id = ?",
        )
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(row.amount_cents, -1299);

        sqlx::query(
            "UPDATE transactions SET
                account_id = ?1, booking_date = ?2, amount_cents = ?3, currency = ?4,
                counterparty = ?5, purpose = ?6, category_id = ?7, manual_note = ?8
             WHERE id = ?9",
        )
        .bind(upd.account_id)
        .bind(&upd.booking_date)
        .bind(upd.amount_cents)
        .bind(&upd.currency)
        .bind(upd.counterparty.as_deref())
        .bind(upd.purpose.as_deref())
        .bind(upd.category_id)
        .bind(upd.manual_note.as_deref())
        .bind(upd.id)
        .execute(&pool)
        .await
        .unwrap();

        let row: Transaction = sqlx::query_as(
            "SELECT id, account_id, booking_date, value_date, amount_cents, currency,
                    counterparty, purpose, raw_ref, category_id, source, source_file_hash,
                    imported_at, manual_note, bucket_id, kind, counterparty_iban,
                    paired_tx_id
             FROM transactions WHERE id = ?",
        )
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(row.amount_cents, -1500);
        assert_eq!(row.booking_date, "2026-05-11");
        assert_eq!(row.counterparty.as_deref(), Some("REWE Markt"));
        assert_eq!(row.purpose.as_deref(), Some("Wocheneinkauf"));
        assert_eq!(row.category_id, Some(cat));
        assert_eq!(row.manual_note.as_deref(), Some("Notiz"));
    }

    #[tokio::test]
    async fn delete_transaction_removes_row() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool, "TR").await;
        let id = insert_tx(&pool, acc, "2026-05-10", -1299, Some("REWE"), None).await;

        let rows = sqlx::query("DELETE FROM transactions WHERE id = ?")
            .bind(id)
            .execute(&pool)
            .await
            .unwrap()
            .rows_affected();
        assert_eq!(rows, 1);

        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM transactions")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn suggest_category_returns_match_above_threshold() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool, "TR").await;
        let cat = seed_category(&pool, "Lebensmittel").await;
        insert_tx(&pool, acc, "2026-04-01", -1000, Some("REWE Markt Berlin"), Some(cat))
            .await;

        let history = load_history(&pool, None).await.unwrap();
        let hit = suggest_category_from_history_scored(
            "REWE Markt Hamburg",
            &history,
            FUZZY_THRESHOLD,
        );
        let (category_id, score) = hit.expect("expected fuzzy match");
        assert_eq!(category_id, cat);
        assert!(score >= FUZZY_THRESHOLD);
    }

    #[tokio::test]
    async fn suggest_category_returns_none_for_empty_history() {
        let pool = connect_memory().await.unwrap();
        let _ = seed_account(&pool, "TR").await;
        let history = load_history(&pool, None).await.unwrap();
        assert!(history.is_empty());
        let hit =
            suggest_category_from_history_scored("REWE", &history, FUZZY_THRESHOLD);
        assert!(hit.is_none());
    }

    #[tokio::test]
    async fn list_transactions_search_finds_manual_note() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool, "TR").await;
        sqlx::query(
            "INSERT INTO transactions
                (account_id, booking_date, amount_cents, currency, counterparty,
                 manual_note, source)
             VALUES (?1, '2026-05-10', -1299, 'EUR', 'REWE', 'Geburtstagsgeschenk Anna', 'manual')",
        )
        .bind(acc)
        .execute(&pool)
        .await
        .unwrap();

        let sql = "SELECT id, account_id, booking_date, value_date, amount_cents, currency,
                          counterparty, purpose, raw_ref, category_id, source, source_file_hash,
                          imported_at, manual_note, bucket_id, kind, counterparty_iban,
                          paired_tx_id
                   FROM transactions
                   WHERE counterparty LIKE ? OR purpose LIKE ? OR manual_note LIKE ?
                   ORDER BY booking_date DESC, id DESC LIMIT 100";
        let needle = "%Anna%".to_string();
        let rows: Vec<Transaction> = sqlx::query_as(sql)
            .bind(&needle)
            .bind(&needle)
            .bind(&needle)
            .fetch_all(&pool)
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
    }

    #[tokio::test]
    async fn assign_bucket_updates_only_target_tx() {
        let pool = connect_memory().await.unwrap();
        let acc = crate::db::accounts::create_account(
            &pool, "A", "bank", "EUR", None, None, None,
        ).await.unwrap();
        let b = crate::db::buckets::create_bucket(&pool, crate::db::buckets::NewBucketPayload {
            name: "x".into(), icon: None, color: None, note: None,
            target_cents: None, start_date: None, target_date: None,
        }).await.unwrap();
        let (t1,): (i64,) = sqlx::query_as(
            "INSERT INTO transactions (account_id, booking_date, amount_cents, currency, counterparty, source)
             VALUES (?1, '2026-05-01', 100, 'EUR', 'a', 'manual') RETURNING id",
        ).bind(acc.id).fetch_one(&pool).await.unwrap();
        let (t2,): (i64,) = sqlx::query_as(
            "INSERT INTO transactions (account_id, booking_date, amount_cents, currency, counterparty, source)
             VALUES (?1, '2026-05-02', 200, 'EUR', 'b', 'manual') RETURNING id",
        ).bind(acc.id).fetch_one(&pool).await.unwrap();

        sqlx::query("UPDATE transactions SET bucket_id = ?1 WHERE id = ?2")
            .bind(b.id).bind(t1).execute(&pool).await.unwrap();

        let (cnt,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM transactions WHERE bucket_id = ?1"
        ).bind(b.id).fetch_one(&pool).await.unwrap();
        assert_eq!(cnt, 1);
        let (t2_bucket,): (Option<i64>,) = sqlx::query_as(
            "SELECT bucket_id FROM transactions WHERE id = ?1"
        ).bind(t2).fetch_one(&pool).await.unwrap();
        assert!(t2_bucket.is_none());
    }

    #[tokio::test]
    async fn list_transactions_filter_by_bucket_id() {
        let pool = connect_memory().await.unwrap();
        let acc = crate::db::accounts::create_account(
            &pool, "A", "bank", "EUR", None, None, None,
        ).await.unwrap();
        let b = crate::db::buckets::create_bucket(&pool, crate::db::buckets::NewBucketPayload {
            name: "x".into(), icon: None, color: None, note: None,
            target_cents: None, start_date: None, target_date: None,
        }).await.unwrap();
        sqlx::query(
            "INSERT INTO transactions (account_id, booking_date, amount_cents, currency, counterparty, source, bucket_id)
             VALUES
                (?1, '2026-05-01', 100, 'EUR', 'in', 'manual', ?2),
                (?1, '2026-05-02', 200, 'EUR', 'no', 'manual', NULL)",
        ).bind(acc.id).bind(b.id).execute(&pool).await.unwrap();

        let sql = "SELECT id, account_id, booking_date, value_date, amount_cents, currency,
                          counterparty, purpose, raw_ref, category_id, source, source_file_hash,
                          imported_at, manual_note, bucket_id, kind, counterparty_iban,
                          paired_tx_id
                     FROM transactions WHERE bucket_id = ?1";
        let rows: Vec<Transaction> = sqlx::query_as(sql).bind(b.id).fetch_all(&pool).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].bucket_id, Some(b.id));
    }

    async fn insert_tx_with_kind(pool: &sqlx::SqlitePool, account_id: i64, kind: &str) -> i64 {
        // counterparty includes kind to avoid dedup-index collisions across kinds.
        let (id,): (i64,) = sqlx::query_as(
            "INSERT INTO transactions
                (account_id, booking_date, amount_cents, currency, counterparty,
                 source, kind)
             VALUES (?1, '2026-05-18', -10000, 'EUR', ?2, 'manual', ?2) RETURNING id",
        )
        .bind(account_id)
        .bind(kind)
        .fetch_one(pool)
        .await
        .unwrap();
        id
    }

    #[tokio::test]
    async fn assert_not_trade_tx_rejects_each_trade_kind() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool, "Broker").await;
        for kind in ["buy", "sell", "dividend", "corporate_action", "tax"] {
            let id = insert_tx_with_kind(&pool, acc, kind).await;
            let res = assert_not_trade_tx(&pool, id).await;
            assert!(res.is_err(), "kind={kind} should be rejected");
            assert!(
                res.unwrap_err().message.contains(kind),
                "error should mention the kind",
            );
        }
    }

    #[tokio::test]
    async fn assert_not_trade_tx_allows_normal_kinds() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool, "Giro").await;
        for kind in ["income", "expense", "transfer", "fee", "tax_general"] {
            let id = insert_tx_with_kind(&pool, acc, kind).await;
            assert!(assert_not_trade_tx(&pool, id).await.is_ok(), "kind={kind}");
        }
    }

    #[tokio::test]
    async fn assert_not_trade_tx_passes_unknown_id() {
        // Non-existing tx is not the guard's job to flag — caller produces "not found".
        let pool = connect_memory().await.unwrap();
        assert!(assert_not_trade_tx(&pool, 999_999).await.is_ok());
    }

    async fn seed_inst_and_account(
        pool: &sqlx::SqlitePool,
        inst_name: &str,
        acc_name: &str,
    ) -> (i64, i64) {
        sqlx::query("INSERT INTO institutions (name) VALUES (?1)")
            .bind(inst_name)
            .execute(pool)
            .await
            .unwrap();
        let (iid,): (i64,) =
            sqlx::query_as("SELECT id FROM institutions WHERE name = ?1")
                .bind(inst_name)
                .fetch_one(pool)
                .await
                .unwrap();
        let acc = crate::db::accounts::create_account(
            pool, acc_name, "bank", "EUR", None, None, Some(iid),
        )
        .await
        .unwrap();
        (iid, acc.id)
    }

    async fn insert_simple_tx(pool: &sqlx::SqlitePool, account_id: i64, amount: i64, counterparty: &str) {
        sqlx::query(
            "INSERT INTO transactions (account_id, booking_date, amount_cents, currency, counterparty, source)
             VALUES (?1, '2026-05-01', ?2, 'EUR', ?3, 'manual')",
        )
        .bind(account_id)
        .bind(amount)
        .bind(counterparty)
        .execute(pool)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn list_transactions_filters_by_institution() {
        let pool = connect_memory().await.unwrap();
        let (iid_a, acc_a) = seed_inst_and_account(&pool, "BankA", "GiroA").await;
        let (_iid_b, acc_b) = seed_inst_and_account(&pool, "BankB", "GiroB").await;
        insert_simple_tx(&pool, acc_a, 100, "x").await;
        insert_simple_tx(&pool, acc_b, 200, "y").await;

        let f = TxFilter { institution_id: Some(iid_a), ..Default::default() };
        let rows = list_transactions_inner(&pool, &f).await.unwrap().rows;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].counterparty.as_deref(), Some("x"));
    }

    #[tokio::test]
    async fn list_transactions_paginates_via_cursor() {
        let pool = connect_memory().await.unwrap();
        let acc = crate::db::accounts::create_account(&pool, "A", "bank", "EUR", None, None, None).await.unwrap();
        // 5 Tx on different days (DESC: 5, 4, 3, 2, 1).
        for d in 1..=5 {
            sqlx::query(
                "INSERT INTO transactions (account_id, booking_date, amount_cents, currency, counterparty, source, kind)
                 VALUES (?1, ?2, -1000, 'EUR', ?3, 't', 'expense')",
            )
            .bind(acc.id)
            .bind(format!("2026-05-{:02}", d))
            .bind(format!("c{d}"))
            .execute(&pool).await.unwrap();
        }

        let f = TxFilter { limit: Some(2), ..Default::default() };
        let p1 = list_transactions_inner(&pool, &f).await.unwrap();
        assert_eq!(p1.rows.len(), 2);
        assert!(p1.has_more);
        assert_eq!(p1.rows[0].counterparty.as_deref(), Some("c5"));
        assert_eq!(p1.rows[1].counterparty.as_deref(), Some("c4"));
        let cur = p1.next_cursor.expect("next_cursor expected");

        let f2 = TxFilter { limit: Some(2), cursor: Some(cur), ..Default::default() };
        let p2 = list_transactions_inner(&pool, &f2).await.unwrap();
        assert_eq!(p2.rows.len(), 2);
        assert_eq!(p2.rows[0].counterparty.as_deref(), Some("c3"));
        assert_eq!(p2.rows[1].counterparty.as_deref(), Some("c2"));
        assert!(p2.has_more);

        let f3 = TxFilter { limit: Some(2), cursor: p2.next_cursor.clone(), ..Default::default() };
        let p3 = list_transactions_inner(&pool, &f3).await.unwrap();
        assert_eq!(p3.rows.len(), 1);
        assert!(!p3.has_more);
        assert!(p3.next_cursor.is_none());
    }

    #[tokio::test]
    async fn list_transactions_filters_date_range_uncategorized_min_amount() {
        let pool = connect_memory().await.unwrap();
        let acc = crate::db::accounts::create_account(&pool, "A", "bank", "EUR", None, None, None).await.unwrap();
        let cat = sqlx::query_scalar::<_, i64>(
            "INSERT INTO categories (name) VALUES ('Food') RETURNING id"
        ).fetch_one(&pool).await.unwrap();

        // tx1: 2026-04-01, -500, cat=Food
        sqlx::query("INSERT INTO transactions (account_id, booking_date, amount_cents, currency, source, kind, category_id) VALUES (?1, '2026-04-01', -500, 'EUR', 't', 'expense', ?2)")
            .bind(acc.id).bind(cat).execute(&pool).await.unwrap();
        // tx2: 2026-05-01, -5000, cat=NULL
        sqlx::query("INSERT INTO transactions (account_id, booking_date, amount_cents, currency, source, kind) VALUES (?1, '2026-05-01', -5000, 'EUR', 't', 'expense')")
            .bind(acc.id).execute(&pool).await.unwrap();
        // tx3: 2026-06-01, -100, cat=NULL
        sqlx::query("INSERT INTO transactions (account_id, booking_date, amount_cents, currency, source, kind) VALUES (?1, '2026-06-01', -100, 'EUR', 't', 'expense')")
            .bind(acc.id).execute(&pool).await.unwrap();

        let f = TxFilter {
            from: Some("2026-05-01".into()),
            to: Some("2026-05-31".into()),
            ..Default::default()
        };
        let rows = list_transactions_inner(&pool, &f).await.unwrap().rows;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].booking_date, "2026-05-01");

        let f2 = TxFilter { uncategorized: Some(true), ..Default::default() };
        let rows = list_transactions_inner(&pool, &f2).await.unwrap().rows;
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|t| t.category_id.is_none()));

        let f3 = TxFilter { min_amount_cents: Some(1000), ..Default::default() };
        let rows = list_transactions_inner(&pool, &f3).await.unwrap().rows;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].booking_date, "2026-05-01");
    }

    #[tokio::test]
    async fn aggregate_transactions_sums_filtered_in_out() {
        let pool = connect_memory().await.unwrap();
        let acc = crate::db::accounts::create_account(&pool, "A", "bank", "EUR", None, None, None).await.unwrap();
        for (date, amt, kind) in [
            ("2026-05-01", 10000_i64, "income"),
            ("2026-05-02", -3000_i64, "expense"),
            ("2026-05-03", -2000_i64, "expense"),
            ("2026-05-04", 5000_i64, "transfer"), // excluded (aggregate filters by kind)
        ] {
            sqlx::query("INSERT INTO transactions (account_id, booking_date, amount_cents, currency, source, kind) VALUES (?1, ?2, ?3, 'EUR', 't', ?4)")
                .bind(acc.id).bind(date).bind(amt).bind(kind).execute(&pool).await.unwrap();
        }

        let agg = aggregate_transactions_inner(&pool, &TxFilter::default()).await.unwrap();
        assert_eq!(agg.in_cents, 10000);
        assert_eq!(agg.out_cents, 5000);
        assert_eq!(agg.count, 3); // transfer is excluded
    }

    #[tokio::test]
    async fn list_transactions_depot_view_shows_only_holding_changes() {
        // Double-entry semantics: all cash movements (including dividends and
        // withholding tax) recorded at the settlement account. The depot view
        // shows only what changes the holdings: buy, sell, corporate_action,
        // fusion_out, fusion_in. Dividends and accumulation tax are pure cash
        // events and do NOT belong in the depot view.
        let pool = connect_memory().await.unwrap();
        let cash = crate::db::accounts::create_account(
            &pool, "Cash", "savings", "EUR", None, None, None,
        ).await.unwrap();
        let depot = crate::db::accounts::create_account(
            &pool, "Depot", "broker", "EUR", None, None, None,
        ).await.unwrap();
        let (sec_id,): (i64,) = sqlx::query_as(
            "INSERT INTO securities (isin, name, currency, asset_type)
             VALUES ('LU1781541179', 'LYXOR', 'EUR', 'etf_equity') RETURNING id"
        ).fetch_one(&pool).await.unwrap();

        // Buy: Tx on Cash, trade-detail side='buy' with account_id=Depot
        let (buy_tx_id,): (i64,) = sqlx::query_as(
            "INSERT INTO transactions
                (account_id, booking_date, amount_cents, currency, counterparty, source, kind, imported_at)
             VALUES (?1, '2026-05-15', -10000, 'EUR', 'broker', 'flatex_pdf', 'buy', '2026-05-15T00:00:00Z')
             RETURNING id"
        ).bind(cash.id).fetch_one(&pool).await.unwrap();
        sqlx::query(
            "INSERT INTO securities_trades (tx_id, security_id, side, shares_micro, account_id)
             VALUES (?1, ?2, 'buy', 1000000, ?3)"
        ).bind(buy_tx_id).bind(sec_id).bind(depot.id).execute(&pool).await.unwrap();

        // Dividend: Tx on Cash, trade-detail side='dividend' (shares=0), account_id=Depot
        let (div_tx_id,): (i64,) = sqlx::query_as(
            "INSERT INTO transactions
                (account_id, booking_date, amount_cents, currency, counterparty, source, kind, imported_at)
             VALUES (?1, '2026-06-15', 125, 'EUR', 'broker', 'flatex_pdf', 'dividend', '2026-06-15T00:00:00Z')
             RETURNING id"
        ).bind(cash.id).fetch_one(&pool).await.unwrap();
        sqlx::query(
            "INSERT INTO securities_trades (tx_id, security_id, side, shares_micro, account_id)
             VALUES (?1, ?2, 'dividend', 0, ?3)"
        ).bind(div_tx_id).bind(sec_id).bind(depot.id).execute(&pool).await.unwrap();

        // Cash view: sees both (cash-flow perspective)
        let f_cash = TxFilter { account_id: Some(cash.id), ..Default::default() };
        let cash_rows = list_transactions_inner(&pool, &f_cash).await.unwrap().rows;
        assert_eq!(cash_rows.len(), 2, "cash view shows buy + dividend");

        // Depot view: only buy, NOT dividend
        let f_depot = TxFilter { account_id: Some(depot.id), ..Default::default() };
        let depot_rows = list_transactions_inner(&pool, &f_depot).await.unwrap().rows;
        assert_eq!(depot_rows.len(), 1, "depot view shows only holding-changing Tx");
        assert_eq!(depot_rows[0].id, buy_tx_id, "it is the buy Tx, not the dividend");
    }

    #[tokio::test]
    async fn list_transactions_combines_institution_and_account_filter() {
        let pool = connect_memory().await.unwrap();
        let (iid, acc_a) = seed_inst_and_account(&pool, "BankA", "GiroA").await;
        let acc_b = crate::db::accounts::create_account(
            &pool, "DepotA", "broker", "EUR", None, None, Some(iid),
        )
        .await
        .unwrap()
        .id;
        insert_simple_tx(&pool, acc_a, 100, "giro-tx").await;
        insert_simple_tx(&pool, acc_b, 200, "depot-tx").await;

        // Institution filter alone → both
        let f1 = TxFilter { institution_id: Some(iid), ..Default::default() };
        assert_eq!(list_transactions_inner(&pool, &f1).await.unwrap().rows.len(), 2);

        // Institution + account filter → only one account
        let f2 = TxFilter {
            institution_id: Some(iid),
            account_id: Some(acc_a),
            ..Default::default()
        };
        let rows = list_transactions_inner(&pool, &f2).await.unwrap().rows;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].counterparty.as_deref(), Some("giro-tx"));
    }

    #[tokio::test]
    async fn update_transaction_persists_counterparty_iban() {
        use crate::db::connect_memory;
        let pool = connect_memory().await.unwrap();
        let acc = crate::db::accounts::create_account(&pool, "A", "bank", "EUR", None, None, None).await.unwrap();
        // Insert Tx, IBAN initially empty
        sqlx::query(
            "INSERT INTO transactions (account_id, booking_date, amount_cents, currency, counterparty, source, kind)
             VALUES (?1, '2026-05-15', -10000, 'EUR', 'X', 'manual', 'expense')"
        ).bind(acc.id).execute(&pool).await.unwrap();
        let (tx_id,): (i64,) = sqlx::query_as("SELECT id FROM transactions LIMIT 1").fetch_one(&pool).await.unwrap();

        // Test normalize_iban + DB update separately
        let normalized = normalize_iban(Some(" de89 3704 0044 0532 0130 00 ")).unwrap();
        assert_eq!(normalized.as_deref(), Some("DE89370400440532013000"));

        sqlx::query("UPDATE transactions SET counterparty_iban = ?1 WHERE id = ?2")
            .bind(normalized.as_deref()).bind(tx_id)
            .execute(&pool).await.unwrap();

        let (iban,): (Option<String>,) = sqlx::query_as("SELECT counterparty_iban FROM transactions WHERE id = ?1")
            .bind(tx_id).fetch_one(&pool).await.unwrap();
        assert_eq!(iban.as_deref(), Some("DE89370400440532013000"));
    }

    #[test]
    fn normalize_iban_handles_typical_inputs() {
        assert_eq!(normalize_iban(Some(" de89 3704 0044 0532 0130 00 ")).unwrap().as_deref(),
                   Some("DE89370400440532013000"));
        assert!(normalize_iban(Some("")).unwrap().is_none());
        assert!(normalize_iban(Some("   ")).unwrap().is_none());
        assert!(normalize_iban(None).unwrap().is_none());
    }

    #[test]
    fn normalize_iban_rejects_too_short() {
        let err = normalize_iban(Some("DE89")).unwrap_err();
        assert!(err.message.contains("length"));
    }

    #[test]
    fn normalize_iban_rejects_bad_country_code() {
        let err = normalize_iban(Some("12893704004405320130001")).unwrap_err();
        assert!(err.message.contains("format"));
    }

    #[tokio::test]
    async fn cleanup_resolves_double_sided_phantom_mirrors() {
        let pool = connect_memory().await.unwrap();

        // 2 accounts with IBAN
        let src: i64 = sqlx::query_scalar(
            "INSERT INTO accounts (name, kind, currency, iban) VALUES ('Main', 'bank', 'EUR', 'AT000000000000000010') RETURNING id"
        ).fetch_one(&pool).await.unwrap();
        let tgt: i64 = sqlx::query_scalar(
            "INSERT INTO accounts (name, kind, currency, iban) VALUES ('Cash', 'savings', 'EUR', 'AT000000000000000011') RETURNING id"
        ).fetch_one(&pool).await.unwrap();

        // Real Sparkasse-out and TR-in (1 day apart)
        let real_out: i64 = sqlx::query_scalar(
            "INSERT INTO transactions (account_id, booking_date, amount_cents, currency, counterparty_iban, kind, source)
             VALUES (?1, '2026-05-04', -23000, 'EUR', 'AT000000000000000011', 'transfer', 'sparkasse_csv') RETURNING id"
        ).bind(src).fetch_one(&pool).await.unwrap();
        let real_in: i64 = sqlx::query_scalar(
            "INSERT INTO transactions (account_id, booking_date, amount_cents, currency, counterparty_iban, kind, source)
             VALUES (?1, '2026-05-05', 23000, 'EUR', 'AT000000000000000010', 'transfer', 'tr_csv') RETURNING id"
        ).bind(tgt).fetch_one(&pool).await.unwrap();

        // Create phantom mirrors on both sides
        let mirror_on_tgt: i64 = sqlx::query_scalar(
            "INSERT INTO transactions (account_id, booking_date, amount_cents, currency, kind, source, paired_tx_id)
             VALUES (?1, '2026-05-04', 23000, 'EUR', 'transfer', 'auto_pair', ?2) RETURNING id"
        ).bind(tgt).bind(real_out).fetch_one(&pool).await.unwrap();
        let mirror_on_src: i64 = sqlx::query_scalar(
            "INSERT INTO transactions (account_id, booking_date, amount_cents, currency, kind, source, paired_tx_id)
             VALUES (?1, '2026-05-05', -23000, 'EUR', 'transfer', 'auto_pair', ?2) RETURNING id"
        ).bind(src).bind(real_in).fetch_one(&pool).await.unwrap();

        // Both real Tx point to their respective mirror partners
        sqlx::query("UPDATE transactions SET paired_tx_id = ?1 WHERE id = ?2")
            .bind(mirror_on_tgt).bind(real_out).execute(&pool).await.unwrap();
        sqlx::query("UPDATE transactions SET paired_tx_id = ?1 WHERE id = ?2")
            .bind(mirror_on_src).bind(real_in).execute(&pool).await.unwrap();

        // Before cleanup: 4 Tx
        let (cnt_before,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM transactions")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(cnt_before, 4);

        // Run cleanup
        let fixed = cleanup_phantom_mirrors_inner(&pool).await.unwrap();
        assert_eq!(fixed, 1, "exactly 1 mirror pair should be fixed as a double-sided case");

        // After cleanup: only 2 real Tx remain
        let (cnt_after,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM transactions")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(cnt_after, 2, "both mirrors should be deleted");

        // real_out and real_in should be linked directly to each other
        let (out_paired,): (Option<i64>,) = sqlx::query_as(
            "SELECT paired_tx_id FROM transactions WHERE id = ?1"
        ).bind(real_out).fetch_one(&pool).await.unwrap();
        assert_eq!(out_paired, Some(real_in), "real_out.paired_tx_id → real_in");

        let (in_paired,): (Option<i64>,) = sqlx::query_as(
            "SELECT paired_tx_id FROM transactions WHERE id = ?1"
        ).bind(real_in).fetch_one(&pool).await.unwrap();
        assert_eq!(in_paired, Some(real_out), "real_in.paired_tx_id → real_out");

        // Verify that the mirrors are truly gone
        let (m1_exists,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM transactions WHERE id = ?1")
            .bind(mirror_on_tgt).fetch_one(&pool).await.unwrap();
        let (m2_exists,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM transactions WHERE id = ?1")
            .bind(mirror_on_src).fetch_one(&pool).await.unwrap();
        assert_eq!(m1_exists, 0, "mirror_on_tgt should be deleted");
        assert_eq!(m2_exists, 0, "mirror_on_src should be deleted");
    }

    #[tokio::test]
    async fn detect_then_cleanup_handles_double_sided_already_paired_case() {
        let pool = connect_memory().await.unwrap();
        let src: i64 = sqlx::query_scalar(
            "INSERT INTO accounts (name, kind, currency, iban) VALUES ('Main', 'bank', 'EUR', 'AT000000000000000010') RETURNING id"
        ).fetch_one(&pool).await.unwrap();
        let tgt: i64 = sqlx::query_scalar(
            "INSERT INTO accounts (name, kind, currency, iban) VALUES ('Cash', 'savings', 'EUR', 'AT000000000000000011') RETURNING id"
        ).fetch_one(&pool).await.unwrap();

        // Real Sparkasse-out + TR-in (3 days apart — within the ±3-day window)
        sqlx::query(
            "INSERT INTO transactions (account_id, booking_date, amount_cents, currency, counterparty_iban, kind, source)
             VALUES (?1, '2026-05-04', -23000, 'EUR', 'AT000000000000000011', 'expense', 'sparkasse_csv'),
                    (?2, '2026-05-07', 23000, 'EUR', 'AT000000000000000010', 'income', 'tr_csv')"
        ).bind(src).bind(tgt).execute(&pool).await.unwrap();

        // Detection sets kind='transfer' + paired_tx_id (no mirror because both IBANs are known)
        let paired = crate::import_flow::detect_inter_account_transfers(&pool).await.unwrap();
        cleanup_phantom_mirrors_inner(&pool).await.unwrap();

        assert_eq!(paired, 1);
        let (cnt,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM transactions WHERE source='auto_pair'")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(cnt, 0, "no auto_pair mirror remains");
        let (paired_real,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM transactions WHERE paired_tx_id IS NOT NULL")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(paired_real, 2, "both real Tx paired with each other");
    }
}
