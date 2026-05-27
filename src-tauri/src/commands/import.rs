use tauri::{AppHandle, Emitter, State};

use crate::commands::accounts::{CommandError, DbState};
use crate::import_flow::{apply_rules_to_uncategorized, import_raw_transactions, ImportReport};
use crate::importers::{source_file_hash, Importer};
use crate::importers::trade_republic_csv::TradeRepublicCsv;
use crate::importers::flatex_pdf::FlatexPdf;

const FUZZY_THRESHOLD: f64 = 0.85;

/// Triggers a background refresh of all security prices and FX rates after an
/// import. Emits the same `price_refresh_status` event lifecycle as on app
/// startup (`lib.rs`) — the sidebar shows the spinner, and pages listening for
/// the `completed` event (e.g. /settings/currencies, dashboard) can reload
/// automatically.
fn spawn_post_import_refresh(app: AppHandle, pool: sqlx::SqlitePool) {
    let _ = app.emit(
        "price_refresh_status",
        serde_json::json!({ "stage": "started" }),
    );
    tauri::async_runtime::spawn(async move {
        let provider = crate::pricing_provider::yahoo::YahooProvider::new();
        match crate::db::portfolio::refresh_all_prices(&pool, &provider).await {
            Ok(report) => {
                let _ = app.emit(
                    "price_refresh_status",
                    serde_json::json!({ "stage": "completed", "report": report }),
                );
            }
            Err(e) => {
                eprintln!("[notchkeep] post-import price refresh failed: {e}");
                let _ = app.emit(
                    "price_refresh_status",
                    serde_json::json!({ "stage": "failed", "error": e.to_string() }),
                );
            }
        }
    });
}

/// Ensures that `account_id` is assigned to a Trade Republic institution.
/// If the account already has an institution, nothing is changed (respecting the user's choice).
/// If `institution_id IS NULL`, TR is upserted and assigned.
pub(crate) async fn ensure_tr_institution_for_account(
    pool: &sqlx::SqlitePool,
    account_id: i64,
) -> crate::db::DbResult<()> {
    let acc = crate::db::accounts::get_account(pool, account_id).await?;
    if acc.institution_id.is_some() {
        return Ok(());
    }
    let tr = crate::db::institutions::upsert_institution_by_name(
        pool,
        "Trade Republic",
        Some("bank"),
        Some("oklch(0.55 0.13 230)"),
        None,
        Some("DE"),
    ).await?;
    let mut updated = acc;
    updated.institution_id = Some(tr.id);
    crate::db::accounts::update_account(pool, &updated).await?;
    Ok(())
}

/// Ensures that `account_id` is assigned to a flatexDEGIRO institution.
/// If the account already has an institution, nothing is changed.
pub(crate) async fn ensure_flatex_institution_for_account(
    pool: &sqlx::SqlitePool,
    account_id: i64,
) -> crate::db::DbResult<()> {
    let acc = crate::db::accounts::get_account(pool, account_id).await?;
    if acc.institution_id.is_some() {
        return Ok(());
    }
    let inst = crate::db::institutions::upsert_institution_by_name(
        pool,
        "flatexDEGIRO",
        Some("bank"),
        Some("oklch(0.55 0.13 25)"),
        None,
        Some("AT"),
    ).await?;
    let mut updated = acc;
    updated.institution_id = Some(inst.id);
    crate::db::accounts::update_account(pool, &updated).await?;
    Ok(())
}

/// Ensures that `account_id` is assigned to an Erste Bank / Sparkasse institution.
/// If the account already has an institution, nothing is changed.
pub(crate) async fn ensure_sparkasse_institution_for_account(
    pool: &sqlx::SqlitePool,
    account_id: i64,
) -> crate::db::DbResult<()> {
    let acc = crate::db::accounts::get_account(pool, account_id).await?;
    if acc.institution_id.is_some() {
        return Ok(());
    }
    let inst = crate::db::institutions::upsert_institution_by_name(
        pool,
        "Erste Bank / Sparkasse",
        Some("bank"),
        Some("oklch(0.55 0.13 250)"),
        None,
        Some("AT"),
    ).await?;
    let mut updated = acc;
    updated.institution_id = Some(inst.id);
    crate::db::accounts::update_account(pool, &updated).await?;
    Ok(())
}

#[tauri::command]
pub async fn import_trade_republic_csv(
    app: AppHandle,
    state: State<'_, DbState>,
    account_id: i64,
    bytes: Vec<u8>,
) -> Result<ImportReport, CommandError> {
    let hash = source_file_hash(&bytes);
    let parsed = TradeRepublicCsv
        .parse(&bytes)
        .map_err(|e| CommandError { message: e.to_string() })?;
    let pool = state.pool();
    let mut report = import_raw_transactions(
        &pool,
        account_id,
        "tr_csv",
        Some(&hash),
        parsed.raws,
        FUZZY_THRESHOLD,
    )
    .await?;
    report.warnings.extend(parsed.warnings);

    // After import: assign account to the TR institution if none is set yet.
    let _ = ensure_tr_institution_for_account(&pool, account_id).await;

    // Apply rules to all uncategorized transactions — catches re-imports + newly created rules.
    let retro = apply_rules_to_uncategorized(&pool).await.unwrap_or(0);
    report.categorized_by_rule += retro;

    // IBAN-based transfer detection: sets kind = 'transfer' where counterparty_iban
    // matches one of the user's own accounts.
    let _ = crate::import_flow::detect_inter_account_transfers(&pool).await;
    let _ = crate::commands::transactions::cleanup_phantom_mirrors_inner(&pool).await;

    // Background refresh for prices + FX rates (see spawn_post_import_refresh).
    spawn_post_import_refresh(app, state.pool());

    Ok(report)
}

#[tauri::command]
pub async fn import_sparkasse_csv(
    app: AppHandle,
    state: State<'_, DbState>,
    account_id: i64,
    bytes: Vec<u8>,
) -> Result<ImportReport, CommandError> {
    let hash = source_file_hash(&bytes);
    let parsed = crate::importers::sparkasse_csv::SparkasseCsv
        .parse(&bytes)
        .map_err(|e| CommandError { message: e.to_string() })?;
    let pool = state.pool();
    let mut report = import_raw_transactions(
        &pool, account_id, "sparkasse_csv", Some(&hash), parsed.raws, FUZZY_THRESHOLD,
    ).await?;
    report.warnings.extend(parsed.warnings);

    let _ = ensure_sparkasse_institution_for_account(&pool, account_id).await;

    let retro = apply_rules_to_uncategorized(&pool).await.unwrap_or(0);
    report.categorized_by_rule += retro;

    let _ = crate::import_flow::detect_inter_account_transfers(&pool).await;
    let _ = crate::commands::transactions::cleanup_phantom_mirrors_inner(&pool).await;

    spawn_post_import_refresh(app, state.pool());

    Ok(report)
}

#[tauri::command]
pub async fn import_flatex_pdfs(
    app: AppHandle,
    state: State<'_, DbState>,
    account_id: i64,
    files: Vec<Vec<u8>>,
) -> Result<ImportReport, CommandError> {
    let pool = state.pool();

    // Auto-routing: Flatex PDFs contain both cash movements and trade details. Cash
    // must land on the cash account, the trade detail on the depot. If the user
    // selected the depot themselves and exactly one cash account exists in the
    // institution, we route the Tx account_id there — `resolve_trade_account` in
    // the import flow then sets securities_trades.account_id back to the depot.
    let effective_account_id = crate::db::accounts::resolve_cash_settlement_account(
        &pool, account_id,
    )
    .await
    .ok()
    .flatten()
    .unwrap_or(account_id);

    let mut combined = ImportReport::default();
    for bytes in &files {
        let hash = source_file_hash(bytes);
        let parsed = FlatexPdf
            .parse(bytes)
            .map_err(|e| CommandError { message: e.to_string() })?;
        combined.warnings.extend(parsed.warnings);
        let mut raws = parsed.raws;
        // Sort chronologically (relevant for multi-position documents like
        // savings-plan batch statements or crypto batches). Makes FIFO lots
        // stable and Tx IDs match document chronology.
        raws.sort_by_key(|r| r.booking_date);
        combined.parsed += raws.len();

        // Symmetric savings-plan dedup:
        // - If raws contains savings-plan positions and the individual documents
        //   are already in the DB → skip SP positions.
        // - If raws contains individual documents and SP transactions are already
        //   in the DB → delete the old SP transactions (destructive) in favour
        //   of the more detailed individual documents.
        let displaced = displace_sparplan_duplicates(&pool, effective_account_id, &raws).await?;
        combined.skipped += displaced;
        let (raws_clean, skipped_dup) =
            filter_sparplan_duplicates(&pool, effective_account_id, raws).await?;
        combined.skipped += skipped_dup;

        let report = import_raw_transactions(
            &pool, effective_account_id, "flatex_pdf", Some(&hash), raws_clean, FUZZY_THRESHOLD,
        ).await?;
        combined.inserted += report.inserted;
        combined.skipped += report.skipped;
        combined.categorized_by_rule += report.categorized_by_rule;
        combined.categorized_by_fuzzy += report.categorized_by_fuzzy;
        combined.warnings.extend(report.warnings);
    }
    // Institution is anchored to the originally selected account (user intent),
    // even when transactions land on the settlement account.
    let _ = ensure_flatex_institution_for_account(&pool, account_id).await;

    // IBAN-based transfer detection + phantom mirror cleanup.
    let _ = crate::import_flow::detect_inter_account_transfers(&pool).await;
    let _ = crate::commands::transactions::cleanup_phantom_mirrors_inner(&pool).await;

    // Background refresh for prices + FX rates (see spawn_post_import_refresh).
    spawn_post_import_refresh(app, state.pool());

    Ok(combined)
}

/// Replaces existing savings-plan batch positions in the DB with newly
/// imported individual documents. Symmetric counterpart to
/// `filter_sparplan_duplicates` — handles the case where the user imported
/// the savings-plan batch statement FIRST and now pushes the individual
/// documents afterwards.
///
/// For each non-SP Tx in `raws` with trade data: if a SP Tx with the same
/// (account_id, booking_date, amount_cents, ISIN) exists in the DB, the
/// SP Tx is deleted (securities_trades via ON DELETE CASCADE). Returns the
/// number of deleted SP transactions.
///
/// **Destructive operation** — should only be called when the caller will
/// insert the matching individual document afterwards.
async fn displace_sparplan_duplicates(
    pool: &sqlx::SqlitePool,
    account_id: i64,
    raws: &[crate::importers::RawTransaction],
) -> Result<usize, CommandError> {
    let mut deleted = 0usize;
    for raw in raws {
        let is_sparplan = raw.raw_ref.as_deref().is_some_and(|r| r.starts_with("SP"));
        if is_sparplan { continue; }
        let Some(trade) = &raw.trade else { continue };

        let date_str = raw.booking_date.format("%Y-%m-%d").to_string();
        let sp_tx_ids: Vec<(i64,)> = sqlx::query_as(
            "SELECT t.id FROM transactions t
               JOIN securities_trades st ON st.tx_id = t.id
               JOIN securities s ON s.id = st.security_id
              WHERE t.account_id = ?1
                AND t.booking_date = ?2
                AND t.amount_cents = ?3
                AND s.isin = ?4
                AND t.raw_ref LIKE 'SP%'"
        )
        .bind(account_id)
        .bind(&date_str)
        .bind(raw.amount_cents)
        .bind(&trade.isin)
        .fetch_all(pool)
        .await?;

        for (tx_id,) in sp_tx_ids {
            // securities_trades.tx_id has ON DELETE CASCADE — trade row is deleted along with it.
            sqlx::query("DELETE FROM transactions WHERE id = ?1")
                .bind(tx_id)
                .execute(pool)
                .await?;
            deleted += 1;
        }
    }
    Ok(deleted)
}

/// Pre-import filter for savings-plan batch statement positions.
///
/// Savings-plan documents aggregate multiple months of a running plan —
/// the same purchases also exist as individual documents
/// (`KaufFondsZertifikate`). If the user imports both, each savings-plan
/// month would be double-booked.
///
/// Heuristic: a Tx with `raw_ref.starts_with("SP")` is skipped when
/// a Tx with the same (account_id, booking_date, amount_cents) AND the
/// same ISIN on a securities_trades row already exists in the DB and
/// does NOT come from a savings-plan batch statement.
async fn filter_sparplan_duplicates(
    pool: &sqlx::SqlitePool,
    account_id: i64,
    raws: Vec<crate::importers::RawTransaction>,
) -> Result<(Vec<crate::importers::RawTransaction>, usize), CommandError> {
    let mut out = Vec::with_capacity(raws.len());
    let mut skipped = 0usize;
    for raw in raws {
        let is_sparplan = raw.raw_ref.as_deref().is_some_and(|r| r.starts_with("SP"));
        if is_sparplan {
            if let Some(trade) = &raw.trade {
                let date_str = raw.booking_date.format("%Y-%m-%d").to_string();
                let exists: (i64,) = sqlx::query_as(
                    "SELECT COUNT(*) FROM transactions t
                       JOIN securities_trades st ON st.tx_id = t.id
                       JOIN securities s ON s.id = st.security_id
                      WHERE t.account_id = ?1
                        AND t.booking_date = ?2
                        AND t.amount_cents = ?3
                        AND s.isin = ?4
                        AND (t.raw_ref IS NULL OR t.raw_ref NOT LIKE 'SP%')"
                )
                .bind(account_id)
                .bind(&date_str)
                .bind(raw.amount_cents)
                .bind(&trade.isin)
                .fetch_one(pool)
                .await?;
                if exists.0 > 0 {
                    skipped += 1;
                    continue;
                }
            }
        }
        out.push(raw);
    }
    Ok((out, skipped))
}

#[cfg(test)]
mod tests {
    use crate::db::connect_memory;
    use crate::db::institutions::{get_institution_by_name, list_institutions};

    #[tokio::test]
    async fn ensure_tr_institution_creates_if_missing() {
        let pool = connect_memory().await.unwrap();
        let acc = crate::db::accounts::create_account(&pool, "TR", "bank", "EUR", None, None, None)
            .await.unwrap();
        super::ensure_tr_institution_for_account(&pool, acc.id).await.unwrap();
        let reloaded = crate::db::accounts::get_account(&pool, acc.id).await.unwrap();
        assert!(reloaded.institution_id.is_some());
        let tr = get_institution_by_name(&pool, "Trade Republic").await.unwrap();
        assert!(tr.is_some());
    }

    #[tokio::test]
    async fn ensure_tr_institution_is_idempotent() {
        let pool = connect_memory().await.unwrap();
        let acc = crate::db::accounts::create_account(&pool, "TR", "bank", "EUR", None, None, None)
            .await.unwrap();
        super::ensure_tr_institution_for_account(&pool, acc.id).await.unwrap();
        super::ensure_tr_institution_for_account(&pool, acc.id).await.unwrap();
        let listed = list_institutions(&pool, false).await.unwrap();
        assert_eq!(listed.len(), 1, "no duplicate institutions");
    }

    #[tokio::test]
    async fn ensure_tr_institution_does_not_overwrite_existing_assignment() {
        let pool = connect_memory().await.unwrap();
        sqlx::query("INSERT INTO institutions (name) VALUES ('Other')")
            .execute(&pool).await.unwrap();
        let (other_id,): (i64,) = sqlx::query_as("SELECT id FROM institutions WHERE name='Other'")
            .fetch_one(&pool).await.unwrap();
        let acc = crate::db::accounts::create_account(&pool, "TR", "bank", "EUR", None, None, Some(other_id))
            .await.unwrap();
        super::ensure_tr_institution_for_account(&pool, acc.id).await.unwrap();
        let reloaded = crate::db::accounts::get_account(&pool, acc.id).await.unwrap();
        assert_eq!(reloaded.institution_id, Some(other_id), "existing assignment preserved");
    }

    #[tokio::test]
    async fn ensure_flatex_institution_creates_if_missing() {
        let pool = connect_memory().await.unwrap();
        let acc = crate::db::accounts::create_account(&pool, "Flatex Verrechnung", "bank", "EUR", None, None, None)
            .await.unwrap();
        super::ensure_flatex_institution_for_account(&pool, acc.id).await.unwrap();
        let reloaded = crate::db::accounts::get_account(&pool, acc.id).await.unwrap();
        assert!(reloaded.institution_id.is_some());
        let inst = crate::db::institutions::get_institution_by_name(&pool, "flatexDEGIRO").await.unwrap();
        assert!(inst.is_some());
    }

    #[tokio::test]
    async fn filter_sparplan_skips_duplicate_when_einzelbeleg_already_imported() {
        let pool = connect_memory().await.unwrap();
        let acc = crate::db::accounts::create_account(&pool, "F", "bank", "EUR", None, None, None)
            .await.unwrap();

        // Setup: security + individual-purchase Tx (raw_ref="195830560/1") for LU on 2022-06-02, -100 EUR, 7.46 shares
        sqlx::query("INSERT INTO securities (isin, name, asset_type, currency)
                     VALUES ('LU1781541179', 'LYXOR', 'etf_equity', 'EUR')")
            .execute(&pool).await.unwrap();
        let (sec_id,): (i64,) = sqlx::query_as("SELECT id FROM securities WHERE isin='LU1781541179'")
            .fetch_one(&pool).await.unwrap();
        sqlx::query("INSERT INTO transactions (account_id, booking_date, amount_cents, currency, counterparty, source, kind, raw_ref)
                     VALUES (?1, '2022-06-02', -10000, 'EUR', 'flatexDEGIRO', 'flatex_pdf', 'buy', '195830560/1')")
            .bind(acc.id).execute(&pool).await.unwrap();
        let (tx_id,): (i64,) = sqlx::query_as("SELECT id FROM transactions WHERE raw_ref='195830560/1'")
            .fetch_one(&pool).await.unwrap();
        sqlx::query("INSERT INTO securities_trades
                     (tx_id, security_id, side, shares_micro, fee_cents, tax_cents, kest_cents, withholding_tax_cents)
                     VALUES (?1, ?2, 'buy', 7463422, 0, 0, 0, 0)")
            .bind(tx_id).bind(sec_id).execute(&pool).await.unwrap();

        // Savings-plan batch Tx: raw_ref prefix "SP", same date + amount + ISIN
        use chrono::NaiveDate;
        use crate::importers::{RawTradeFields, RawTransaction};
        let sp_dup = RawTransaction {
            booking_date: NaiveDate::from_ymd_opt(2022, 6, 2).unwrap(),
            amount_cents: -10000,
            currency: "EUR".into(),
            counterparty: Some("flatexDEGIRO".into()),
            purpose: Some("LYXOR".into()),
            raw_ref: Some("SP0001818664-0".into()),
            kind: Some("buy".into()),
            trade: Some(RawTradeFields {
                isin: "LU1781541179".into(),
                asset_class_raw: "FUND".into(),
                name: "LYXOR".into(),
                side: "buy".into(),
                shares_micro: 7_463_422,
                unit_price_micro: Some(13_400_000),
                fee_cents: 0,
                kest_cents: 0,
                withholding_tax_cents: 0,
                fx_rate_micro: None,
                fusion_group: None,
            }),
            counterparty_iban: None,
        };
        // Savings-plan position for a different day (NOT a duplicate) — should pass through
        let sp_new = RawTransaction {
            booking_date: NaiveDate::from_ymd_opt(2022, 7, 4).unwrap(),
            amount_cents: -10000,
            raw_ref: Some("SP0001818664-1".into()),
            ..sp_dup.clone()
        };

        let (filtered, skipped) = super::filter_sparplan_duplicates(
            &pool, acc.id, vec![sp_dup, sp_new]
        ).await.unwrap();
        assert_eq!(skipped, 1, "SP Tx with same date+amount+ISIN should be skipped");
        assert_eq!(filtered.len(), 1, "new SP Tx (different day) should pass through");
        assert_eq!(filtered[0].raw_ref.as_deref(), Some("SP0001818664-1"));
    }

    #[tokio::test]
    async fn displace_sparplan_removes_existing_sp_tx_when_einzelbeleg_arrives() {
        let pool = connect_memory().await.unwrap();
        let acc = crate::db::accounts::create_account(&pool, "F", "bank", "EUR", None, None, None)
            .await.unwrap();

        sqlx::query("INSERT INTO securities (isin, name, asset_type, currency)
                     VALUES ('LU1781541179', 'LYXOR', 'etf_equity', 'EUR')")
            .execute(&pool).await.unwrap();
        let (sec_id,): (i64,) = sqlx::query_as("SELECT id FROM securities WHERE isin='LU1781541179'")
            .fetch_one(&pool).await.unwrap();

        // Setup: SP Tx already in the DB (user had imported savings-plan batch first)
        sqlx::query("INSERT INTO transactions (account_id, booking_date, amount_cents, currency, counterparty, source, kind, raw_ref)
                     VALUES (?1, '2022-06-02', -10000, 'EUR', 'flatexDEGIRO', 'flatex_pdf', 'buy', 'SP0001818664-0')")
            .bind(acc.id).execute(&pool).await.unwrap();
        let (sp_tx_id,): (i64,) = sqlx::query_as("SELECT id FROM transactions WHERE raw_ref='SP0001818664-0'")
            .fetch_one(&pool).await.unwrap();
        sqlx::query("INSERT INTO securities_trades
                     (tx_id, security_id, side, shares_micro, fee_cents, tax_cents, kest_cents, withholding_tax_cents)
                     VALUES (?1, ?2, 'buy', 7463422, 0, 0, 0, 0)")
            .bind(sp_tx_id).bind(sec_id).execute(&pool).await.unwrap();

        // Now the individual document arrives (raw_ref WITHOUT "SP" prefix)
        use chrono::NaiveDate;
        use crate::importers::{RawTradeFields, RawTransaction};
        let einzel = RawTransaction {
            booking_date: NaiveDate::from_ymd_opt(2022, 6, 2).unwrap(),
            amount_cents: -10000,
            currency: "EUR".into(),
            counterparty: Some("flatexDEGIRO".into()),
            purpose: None,
            raw_ref: Some("195830560/1".into()),
            kind: Some("buy".into()),
            trade: Some(RawTradeFields {
                isin: "LU1781541179".into(), asset_class_raw: "FUND".into(),
                name: "LYXOR".into(), side: "buy".into(),
                shares_micro: 7_463_422, unit_price_micro: Some(13_400_000),
                fee_cents: 0, kest_cents: 0, withholding_tax_cents: 0, fx_rate_micro: None, fusion_group: None,
            }),
            counterparty_iban: None,
        };

        let deleted = super::displace_sparplan_duplicates(&pool, acc.id, &[einzel])
            .await.unwrap();
        assert_eq!(deleted, 1);

        // SP Tx + its securities_trades row are gone (CASCADE)
        let (sp_remaining,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM transactions WHERE raw_ref = 'SP0001818664-0'"
        ).fetch_one(&pool).await.unwrap();
        assert_eq!(sp_remaining, 0, "SP Tx should be deleted");
        let (st_remaining,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM securities_trades"
        ).fetch_one(&pool).await.unwrap();
        assert_eq!(st_remaining, 0, "associated securities_trades should be gone via CASCADE");
    }

    #[tokio::test]
    async fn displace_sparplan_ignores_non_sp_existing_tx() {
        // If the existing Tx does NOT have an SP prefix, displace must not
        // touch it (otherwise normal individual invoices would be deleted on re-import).
        let pool = connect_memory().await.unwrap();
        let acc = crate::db::accounts::create_account(&pool, "F", "bank", "EUR", None, None, None)
            .await.unwrap();
        sqlx::query("INSERT INTO securities (isin, name, asset_type, currency)
                     VALUES ('LU1781541179', 'X', 'etf_equity', 'EUR')")
            .execute(&pool).await.unwrap();
        let (sec_id,): (i64,) = sqlx::query_as("SELECT id FROM securities WHERE isin='LU1781541179'")
            .fetch_one(&pool).await.unwrap();
        sqlx::query("INSERT INTO transactions (account_id, booking_date, amount_cents, currency, counterparty, source, kind, raw_ref)
                     VALUES (?1, '2022-06-02', -10000, 'EUR', 'cp', 'flatex_pdf', 'buy', '195830560/1')")
            .bind(acc.id).execute(&pool).await.unwrap();
        let (tx_id,): (i64,) = sqlx::query_as("SELECT id FROM transactions LIMIT 1")
            .fetch_one(&pool).await.unwrap();
        sqlx::query("INSERT INTO securities_trades (tx_id, security_id, side, shares_micro, fee_cents, tax_cents, kest_cents, withholding_tax_cents)
                     VALUES (?1, ?2, 'buy', 7463422, 0, 0, 0, 0)")
            .bind(tx_id).bind(sec_id).execute(&pool).await.unwrap();

        use chrono::NaiveDate;
        use crate::importers::{RawTradeFields, RawTransaction};
        // Re-import of the same individual invoice — displace must NOT delete the existing Tx
        let same_einzel = RawTransaction {
            booking_date: NaiveDate::from_ymd_opt(2022, 6, 2).unwrap(),
            amount_cents: -10000,
            currency: "EUR".into(),
            counterparty: Some("cp".into()),
            purpose: None,
            raw_ref: Some("195830560/1".into()),
            kind: Some("buy".into()),
            trade: Some(RawTradeFields {
                isin: "LU1781541179".into(), asset_class_raw: "FUND".into(),
                name: "X".into(), side: "buy".into(),
                shares_micro: 7_463_422, unit_price_micro: None,
                fee_cents: 0, kest_cents: 0, withholding_tax_cents: 0, fx_rate_micro: None, fusion_group: None,
            }),
            counterparty_iban: None,
        };
        let deleted = super::displace_sparplan_duplicates(&pool, acc.id, &[same_einzel])
            .await.unwrap();
        assert_eq!(deleted, 0, "displace must only delete SP Tx, not regular ones");
        let (remaining,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM transactions")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(remaining, 1);
    }

    #[tokio::test]
    async fn filter_sparplan_keeps_non_sp_tx_untouched() {
        // Non-SP Tx (e.g. regular purchases) should pass through unchanged.
        let pool = connect_memory().await.unwrap();
        let acc = crate::db::accounts::create_account(&pool, "F", "bank", "EUR", None, None, None)
            .await.unwrap();
        use chrono::NaiveDate;
        use crate::importers::{RawTradeFields, RawTransaction};
        let plain = RawTransaction {
            booking_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            amount_cents: -50000,
            currency: "EUR".into(),
            counterparty: Some("flatexDEGIRO".into()),
            purpose: None,
            raw_ref: Some("999/1".into()),   // no "SP" prefix
            kind: Some("buy".into()),
            trade: Some(RawTradeFields {
                isin: "LU1781541179".into(), asset_class_raw: "FUND".into(),
                name: "X".into(), side: "buy".into(),
                shares_micro: 1_000_000, unit_price_micro: None,
                fee_cents: 0, kest_cents: 0, withholding_tax_cents: 0, fx_rate_micro: None, fusion_group: None,
            }),
            counterparty_iban: None,
        };
        let (filtered, skipped) = super::filter_sparplan_duplicates(
            &pool, acc.id, vec![plain]
        ).await.unwrap();
        assert_eq!(skipped, 0);
        assert_eq!(filtered.len(), 1);
    }

    #[tokio::test]
    async fn ensure_flatex_institution_does_not_overwrite_existing() {
        let pool = connect_memory().await.unwrap();
        sqlx::query("INSERT INTO institutions (name) VALUES ('Other')")
            .execute(&pool).await.unwrap();
        let (other_id,): (i64,) = sqlx::query_as("SELECT id FROM institutions WHERE name='Other'")
            .fetch_one(&pool).await.unwrap();
        let acc = crate::db::accounts::create_account(&pool, "X", "bank", "EUR", None, None, Some(other_id))
            .await.unwrap();
        super::ensure_flatex_institution_for_account(&pool, acc.id).await.unwrap();
        let reloaded = crate::db::accounts::get_account(&pool, acc.id).await.unwrap();
        assert_eq!(reloaded.institution_id, Some(other_id));
    }

    #[tokio::test]
    async fn ensure_sparkasse_institution_creates_if_missing() {
        let pool = connect_memory().await.unwrap();
        let acc = crate::db::accounts::create_account(&pool, "Sparkasse Verrechnung", "bank", "EUR", None, None, None)
            .await.unwrap();
        super::ensure_sparkasse_institution_for_account(&pool, acc.id).await.unwrap();
        let reloaded = crate::db::accounts::get_account(&pool, acc.id).await.unwrap();
        assert!(reloaded.institution_id.is_some());
        let inst = crate::db::institutions::get_institution_by_name(&pool, "Erste Bank / Sparkasse").await.unwrap();
        assert!(inst.is_some());
    }

    #[tokio::test]
    async fn ensure_sparkasse_institution_is_idempotent() {
        let pool = connect_memory().await.unwrap();
        let acc = crate::db::accounts::create_account(&pool, "Sparkasse", "bank", "EUR", None, None, None)
            .await.unwrap();
        super::ensure_sparkasse_institution_for_account(&pool, acc.id).await.unwrap();
        super::ensure_sparkasse_institution_for_account(&pool, acc.id).await.unwrap();
        let listed = list_institutions(&pool, false).await.unwrap();
        assert_eq!(listed.len(), 1, "no duplicate institutions");
    }

    #[tokio::test]
    async fn ensure_sparkasse_institution_does_not_overwrite_existing() {
        let pool = connect_memory().await.unwrap();
        sqlx::query("INSERT INTO institutions (name) VALUES ('Other')")
            .execute(&pool).await.unwrap();
        let (other_id,): (i64,) = sqlx::query_as("SELECT id FROM institutions WHERE name='Other'")
            .fetch_one(&pool).await.unwrap();
        let acc = crate::db::accounts::create_account(&pool, "Sparkasse", "bank", "EUR", None, None, Some(other_id))
            .await.unwrap();
        super::ensure_sparkasse_institution_for_account(&pool, acc.id).await.unwrap();
        let reloaded = crate::db::accounts::get_account(&pool, acc.id).await.unwrap();
        assert_eq!(reloaded.institution_id, Some(other_id), "existing assignment preserved");
    }
}
