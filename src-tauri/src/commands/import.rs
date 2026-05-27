use tauri::{AppHandle, Emitter, State};

use crate::commands::accounts::{CommandError, DbState};
use crate::import_flow::{apply_rules_to_uncategorized, import_raw_transactions, ImportReport};
use crate::importers::{source_file_hash, Importer};
use crate::importers::trade_republic_csv::TradeRepublicCsv;
use crate::importers::flatex_pdf::FlatexPdf;

const FUZZY_THRESHOLD: f64 = 0.85;

/// Stößt nach einem Import den Background-Refresh aller Wertpapier-Kurse und
/// FX-Raten an. Dabei wird derselbe `price_refresh_status`-Event-Lebenszyklus
/// emittiert wie beim App-Start (`lib.rs`) — die Sidebar zeigt den Spinner,
/// und Seiten, die auf das `completed`-Event hören (z.B. /settings/currencies,
/// Dashboard), können sich automatisch neu laden.
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
                eprintln!("[budget-app] post-import refresh failed: {e}");
                let _ = app.emit(
                    "price_refresh_status",
                    serde_json::json!({ "stage": "failed", "error": e.to_string() }),
                );
            }
        }
    });
}

/// Stellt sicher, dass `account_id` einem Trade-Republic-Institut zugewiesen ist.
/// Wenn das Konto bereits ein Institut hat, wird nichts geändert (User-Wahl respektieren).
/// Wenn `institution_id IS NULL`, wird TR upserted und zugewiesen.
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

/// Stellt sicher, dass `account_id` einem flatexDEGIRO-Institut zugewiesen ist.
/// Wenn das Konto bereits ein Institut hat, wird nichts geändert.
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

/// Stellt sicher, dass `account_id` einem Erste-Bank-/Sparkasse-Institut
/// zugewiesen ist. Wenn das Konto bereits ein Institut hat, wird nichts geändert.
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

    // Nach dem Import: Konto dem TR-Institut zuweisen, falls noch keins gesetzt.
    let _ = ensure_tr_institution_for_account(&pool, account_id).await;

    // Wendet Regeln auf alle uncategorized Tx an — fängt Re-Imports + frische Regeln ab.
    let retro = apply_rules_to_uncategorized(&pool).await.unwrap_or(0);
    report.categorized_by_rule += retro;

    // IBAN-basierte Transfer-Detection: setzt kind = 'transfer' wo counterparty_iban
    // zu einem eigenen Konto matched.
    let _ = crate::import_flow::detect_inter_account_transfers(&pool).await;
    let _ = crate::commands::transactions::cleanup_phantom_mirrors_inner(&pool).await;

    // Hintergrund-Refresh für Kurse + FX-Raten (siehe spawn_post_import_refresh).
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

    // Auto-Routing: Flatex-PDFs enthalten Cash-Bewegungen + Trade-Details. Cash
    // muss am Cashkonto landen, Trade-Detail am Depot. Wenn der User das Depot
    // selber gewählt hat und im Institut genau ein Cashkonto existiert, routen
    // wir die Tx-account_id dort hin — `resolve_trade_account` im Import-Flow
    // setzt dann securities_trades.account_id auf das Depot zurück.
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
        // Chronologisch sortieren (relevant für Multi-Position-Belege wie
        // Sparplan-Sammelabrechnung oder Krypto-Sammel). Macht FIFO-Lots
        // stabil und Tx-IDs entsprechen der Beleg-Chronologie.
        raws.sort_by_key(|r| r.booking_date);
        combined.parsed += raws.len();

        // Symmetrischer Sparplan-Dedup:
        // - Wenn raws Sparplan-Positionen enthält und die Einzelbelege schon
        //   in der DB sind → SP-Positionen skippen.
        // - Wenn raws Einzelbelege enthält und SP-Tx schon in der DB sind →
        //   die alten SP-Tx löschen (destruktiv) zugunsten der detaillierteren
        //   Einzelbelege.
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
    // Institut wird am ursprünglich gewählten Konto verankert (User-Intent),
    // auch wenn Tx über die Verrechnung landen.
    let _ = ensure_flatex_institution_for_account(&pool, account_id).await;

    // IBAN-basierte Transfer-Detection + Phantom-Mirror-Cleanup.
    let _ = crate::import_flow::detect_inter_account_transfers(&pool).await;
    let _ = crate::commands::transactions::cleanup_phantom_mirrors_inner(&pool).await;

    // Hintergrund-Refresh für Kurse + FX-Raten (siehe spawn_post_import_refresh).
    spawn_post_import_refresh(app, state.pool());

    Ok(combined)
}

/// Ersetzt bestehende Sparplan-Sammel-Positionen in der DB durch neu
/// importierte Einzelbelege. Symmetrisches Gegenstück zu
/// `filter_sparplan_duplicates` — handhabt den Fall, dass der User die
/// Sparplan-Sammelabrechnung ZUERST importiert hat und jetzt die Einzelbelege
/// nachschiebt.
///
/// Pro Non-SP-Tx in `raws` mit Trade-Daten: wenn in der DB eine SP-Tx mit
/// gleichem (account_id, booking_date, amount_cents, ISIN) existiert, wird
/// die SP-Tx gelöscht (securities_trades via ON DELETE CASCADE mit). Returnt
/// die Anzahl gelöschter SP-Tx.
///
/// **Destruktive Operation** — sollte nur aufgerufen werden, wenn der Caller
/// danach den passenden Einzelbeleg inserted.
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
            // securities_trades.tx_id ist ON DELETE CASCADE — Trade-Zeile wird mit gelöscht.
            sqlx::query("DELETE FROM transactions WHERE id = ?1")
                .bind(tx_id)
                .execute(pool)
                .await?;
            deleted += 1;
        }
    }
    Ok(deleted)
}

/// Pre-Import-Filter für Sparplan-Sammelabrechnungs-Positionen.
///
/// Sparplan-Belege fassen mehrere Monate eines laufenden Sparplans zusammen —
/// dieselben Käufe gibt's typischerweise auch als Einzelbeleg
/// (`KaufFondsZertifikate`). Wenn der User beides importiert hat, würde jeder
/// Sparplan-Monat doppelt verbucht.
///
/// Heuristik: eine Tx mit `raw_ref.starts_with("SP")` wird übersprungen, wenn
/// in der DB bereits eine Tx mit gleichem (account_id, booking_date,
/// amount_cents) UND gleicher ISIN auf einer securities_trades-Zeile existiert,
/// die NICHT aus einer Sparplan-Sammelabrechnung stammt.
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
    use crate::importers::Importer;

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

        // Setup: Security + Einzelkauf-Tx (raw_ref="195830560/1") für LU am 2022-06-02, -100 EUR, 7.46 Stücke
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

        // Sparplan-Sammelabrechnung-Tx: raw_ref-Prefix "SP", gleicher Tag + Betrag + ISIN
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
        // Sparplan-Position für anderen Tag (KEIN Duplikat) — sollte durchkommen
        let sp_new = RawTransaction {
            booking_date: NaiveDate::from_ymd_opt(2022, 7, 4).unwrap(),
            amount_cents: -10000,
            raw_ref: Some("SP0001818664-1".into()),
            ..sp_dup.clone()
        };

        let (filtered, skipped) = super::filter_sparplan_duplicates(
            &pool, acc.id, vec![sp_dup, sp_new]
        ).await.unwrap();
        assert_eq!(skipped, 1, "SP-Tx mit gleichem Datum+Betrag+ISIN sollte geskippt werden");
        assert_eq!(filtered.len(), 1, "neue SP-Tx (anderer Tag) sollte durchkommen");
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

        // Setup: SP-Tx ist schon in der DB (User hatte Sparplan-Sammelabrechnung zuerst)
        sqlx::query("INSERT INTO transactions (account_id, booking_date, amount_cents, currency, counterparty, source, kind, raw_ref)
                     VALUES (?1, '2022-06-02', -10000, 'EUR', 'flatexDEGIRO', 'flatex_pdf', 'buy', 'SP0001818664-0')")
            .bind(acc.id).execute(&pool).await.unwrap();
        let (sp_tx_id,): (i64,) = sqlx::query_as("SELECT id FROM transactions WHERE raw_ref='SP0001818664-0'")
            .fetch_one(&pool).await.unwrap();
        sqlx::query("INSERT INTO securities_trades
                     (tx_id, security_id, side, shares_micro, fee_cents, tax_cents, kest_cents, withholding_tax_cents)
                     VALUES (?1, ?2, 'buy', 7463422, 0, 0, 0, 0)")
            .bind(sp_tx_id).bind(sec_id).execute(&pool).await.unwrap();

        // Jetzt kommt der Einzelbeleg (raw_ref OHNE "SP"-Prefix)
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

        // SP-Tx + ihre securities_trades-Zeile sind weg (CASCADE)
        let (sp_remaining,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM transactions WHERE raw_ref = 'SP0001818664-0'"
        ).fetch_one(&pool).await.unwrap();
        assert_eq!(sp_remaining, 0, "SP-Tx sollte gelöscht sein");
        let (st_remaining,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM securities_trades"
        ).fetch_one(&pool).await.unwrap();
        assert_eq!(st_remaining, 0, "zugehörige securities_trades sollte via CASCADE weg sein");
    }

    #[tokio::test]
    async fn displace_sparplan_ignores_non_sp_existing_tx() {
        // Wenn die existierende Tx KEIN SP-Prefix hat, darf displace sie nicht
        // antasten (Sonst würden normale Einzelbelege bei Re-Import gelöscht).
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
        // Re-Import des gleichen Einzelbelegs — displace darf NICHT die existing Tx löschen
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
        assert_eq!(deleted, 0, "displace darf nur SP-Tx löschen, nicht reguläre");
        let (remaining,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM transactions")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(remaining, 1);
    }

    #[tokio::test]
    async fn filter_sparplan_keeps_non_sp_tx_untouched() {
        // Non-SP-Tx (z.B. normale Käufe) sollen unverändert durchgehen.
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
            raw_ref: Some("999/1".into()),   // kein "SP"-Prefix
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
