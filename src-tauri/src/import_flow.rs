use chrono::NaiveDate;
use sqlx::SqlitePool;

use crate::categorization::flow::{categorize, CategorizationOutcome};
use crate::categorization::fuzzy::HistoryEntry;
use crate::categorization::rules::{match_rule, MatchContext, Rule};
use crate::db::rules::list_rules;
use crate::db::transactions::{insert_raw_transaction, InsertOutcome};
use crate::db::{DbError, DbResult};
use crate::importers::RawTransaction;

/// Row type for uncategorized transaction queries (id, account_id, date, amount, counterparty,
/// purpose, manual_note).
type TransactionRuleRow = (
    i64,
    i64,
    String,
    i64,
    Option<String>,
    Option<String>,
    Option<String>,
);

/// Row type for transfer-candidate queries (id, account_id, date, amount, currency,
/// counterparty_iban, purpose).
type TransferCandidateRow = (
    i64,
    i64,
    String,
    i64,
    String,
    Option<String>,
    Option<String>,
);

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
pub struct ImportReport {
    pub parsed: usize,
    pub inserted: usize,
    pub skipped: usize,
    pub categorized_by_rule: usize,
    pub categorized_by_fuzzy: usize,
    pub warnings: Vec<String>,
}

/// Orchestrates the import flow: load rules and history, categorize, insert
/// with deduplication.
pub async fn import_raw_transactions(
    pool: &SqlitePool,
    account_id: i64,
    source: &str,
    source_file_hash: Option<&str>,
    raws: Vec<RawTransaction>,
    fuzzy_threshold: f64,
) -> DbResult<ImportReport> {
    let rules = list_rules(pool).await?;
    let history = load_history(pool).await?;

    // Securities Tx (buy/sell or with trade detail) automatically receive the
    // default "Investitionen" category as a fallback when neither a rule nor
    // fuzzy matching fires.
    let invest_cat_id: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM categories WHERE parent_id IS NULL AND name = 'Investitionen' LIMIT 1",
    )
    .fetch_optional(pool)
    .await?;

    let mut report = ImportReport {
        parsed: raws.len(),
        ..Default::default()
    };

    for raw in &raws {
        let ctx = MatchContext::new(raw, Some(account_id));
        let outcome = categorize(&ctx, &rules, &history, fuzzy_threshold);
        let category_id = match &outcome {
            CategorizationOutcome::Rule { category_id, .. } => Some(*category_id),
            CategorizationOutcome::Fuzzy { category_id, .. } => Some(*category_id),
            CategorizationOutcome::None => {
                let is_invest = raw.trade.is_some()
                    || matches!(raw.kind.as_deref(), Some("buy") | Some("sell"));
                if is_invest {
                    invest_cat_id
                } else {
                    None
                }
            }
        };
        match insert_raw_transaction(pool, account_id, source, source_file_hash, raw, category_id)
            .await?
        {
            InsertOutcome::Inserted(tx_id) => {
                report.inserted += 1;
                match outcome {
                    CategorizationOutcome::Rule { .. } => report.categorized_by_rule += 1,
                    CategorizationOutcome::Fuzzy { .. } => report.categorized_by_fuzzy += 1,
                    CategorizationOutcome::None => {}
                }

                // 6d: append trade row when the row is a securities transaction.
                if let Some(trade) = &raw.trade {
                    let sec_id = crate::db::securities::resolve_or_create_security(
                        pool,
                        &trade.isin,
                        &trade.name,
                        &trade.asset_class_raw,
                    )
                    .await?;

                    // Variant B: assign the trade detail to the institution's depot account
                    // when the Tx currently belongs to the settlement/overnight account AND
                    // exactly one non-archived broker account exists in the same institution.
                    let target_account_id =
                        crate::db::accounts::resolve_trade_account(pool, account_id)
                            .await
                            .ok()
                            .flatten();

                    crate::db::trades::insert_trade_row(
                        pool,
                        tx_id,
                        sec_id,
                        &trade.side,
                        trade.shares_micro,
                        trade.unit_price_micro,
                        trade.fee_cents,
                        trade.kest_cents,
                        trade.withholding_tax_cents,
                        trade.fx_rate_micro,
                        target_account_id,
                        trade.fusion_group.as_deref(),
                    )
                    .await?;
                }
            }
            InsertOutcome::Skipped => report.skipped += 1,
        }
    }

    Ok(report)
}

/// Applies all active rules to all previously uncategorized transactions.
/// Returns the number of newly categorized rows. Useful as a post-import step
/// (after a CSV re-import, skipped Tx retain their old category_ids; newly
/// created rules are retroactively applied to existing uncategorized Tx).
pub async fn apply_rules_to_uncategorized(pool: &SqlitePool) -> DbResult<usize> {
    let rules = list_rules(pool).await?;
    let rules: Vec<_> = rules.into_iter().filter(|r| r.enabled).collect();
    if rules.is_empty() {
        return Ok(0);
    }

    use crate::categorization::rules::{first_matching_rule, MatchContext};
    let rows: Vec<TransactionRuleRow> = sqlx::query_as(
        "SELECT id, account_id, booking_date, amount_cents, counterparty, purpose, manual_note
             FROM transactions
             WHERE category_id IS NULL",
    )
    .fetch_all(pool)
    .await?;

    let mut updated = 0usize;
    for (id, account_id, date_str, amount_cents, counterparty, purpose, manual_note) in rows {
        let booking_date = match NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
            Ok(d) => d,
            Err(_) => continue,
        };
        let description = manual_note.or(purpose);
        let raw = RawTransaction {
            booking_date,
            amount_cents,
            currency: String::new(),
            counterparty,
            purpose: description,
            raw_ref: None,
            kind: None,
            trade: None,
            counterparty_iban: None,
        };
        let ctx = MatchContext::new(&raw, Some(account_id));
        if let Some(rule) = first_matching_rule(&rules, &ctx) {
            sqlx::query("UPDATE transactions SET category_id = ? WHERE id = ?")
                .bind(rule.target_category_id)
                .bind(id)
                .execute(pool)
                .await?;
            updated += 1;
        }
    }
    Ok(updated)
}

/// Applies an existing rule to all stored transactions and overwrites their
/// `category_id` with the rule target. Returns the number of actually changed
/// rows.
///
/// Reuses the in-memory match logic (`match_rule`) so that regex and range ops
/// behave identically to the import path — no SQL-translation mismatch.
///
/// Description matching uses `manual_note ?? purpose` so that manually entered
/// entries with a note remain reachable.
pub async fn bulk_assign_category_by_rule(pool: &SqlitePool, rule_id: i64) -> DbResult<usize> {
    let rule = list_rules(pool)
        .await?
        .into_iter()
        .find(|r| r.id == rule_id)
        .ok_or_else(|| DbError::Decode(format!("rule {rule_id} not found")))?;

    let matching_ids = matching_transaction_ids(pool, &rule).await?;
    if matching_ids.is_empty() {
        return Ok(0);
    }

    let placeholders = matching_ids
        .iter()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!("UPDATE transactions SET category_id = ? WHERE id IN ({placeholders})");
    let mut q = sqlx::query(&sql).bind(rule.target_category_id);
    for id in &matching_ids {
        q = q.bind(*id);
    }
    let affected = q.execute(pool).await?.rows_affected();
    Ok(affected as usize)
}

/// Counts how many existing transactions a (possibly not yet saved) rule would
/// match. Used by the UI editor as a live-preview counter.
pub async fn count_matching_transactions(pool: &SqlitePool, rule: &Rule) -> DbResult<usize> {
    Ok(matching_transaction_ids(pool, rule).await?.len())
}

async fn matching_transaction_ids(pool: &SqlitePool, rule: &Rule) -> DbResult<Vec<i64>> {
    let rows: Vec<TransactionRuleRow> = sqlx::query_as(
        "SELECT id, account_id, booking_date, amount_cents, counterparty, purpose, manual_note
             FROM transactions",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .filter_map(
            |(id, account_id, date_str, amount_cents, counterparty, purpose, manual_note)| {
                let booking_date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").ok()?;
                let description = manual_note.or(purpose);
                let raw = RawTransaction {
                    booking_date,
                    amount_cents,
                    currency: String::new(),
                    counterparty,
                    purpose: description,
                    raw_ref: None,
                    kind: None,
                    trade: None,
                    counterparty_iban: None,
                };
                let ctx = MatchContext::new(&raw, Some(account_id));
                match_rule(rule, &ctx).then_some(id)
            },
        )
        .collect())
}

/// Loads the most recently assigned category_id for each counterparty as a
/// history record for fuzzy matching.
async fn load_history(pool: &SqlitePool) -> DbResult<Vec<HistoryEntry>> {
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT counterparty, category_id FROM transactions t1
         WHERE counterparty IS NOT NULL
           AND category_id IS NOT NULL
           AND booking_date = (
               SELECT MAX(booking_date) FROM transactions t2
               WHERE t2.counterparty = t1.counterparty
                 AND t2.category_id IS NOT NULL
           )
         GROUP BY counterparty",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(counterparty, category_id)| HistoryEntry {
            counterparty,
            category_id,
        })
        .collect())
}

/// Detects inter-account transfers (counterparty_iban matches a known account)
/// and pairs them with the counter-entry on the target account.
///
/// Per candidate Tx:
/// 1. Set kind='transfer' (if not already).
/// 2. Search for an existing Tx on the target with (date, -amount, paired_tx_id IS NULL)
///    → if found: link both via paired_tx_id, both kind='transfer'.
/// 3. Otherwise: create a mirror Tx (source='auto_pair') and link it.
///
/// Idempotent: Tx with paired_tx_id IS NOT NULL are skipped. kind='transfer'
/// is NO LONGER an exclude criterion (existing transfer Tx that were marked
/// before auto-pair will be re-processed). Trade sides (buy/sell/dividend/...)
/// remain excluded.
///
/// Returns the number of Tx that received a new or linked mirror (= number of pairs).
pub async fn detect_inter_account_transfers(pool: &SqlitePool) -> DbResult<usize> {
    // Candidates: paired_tx_id IS NULL + counterparty_iban set + IBAN match.
    // kind NOT IN trade-sides (transfer is NO LONGER excluded).
    let candidates: Vec<TransferCandidateRow> = sqlx::query_as(
        "SELECT t.id, t.account_id, t.booking_date, t.amount_cents, t.currency,
                t.counterparty_iban, t.purpose
           FROM transactions t
          WHERE t.counterparty_iban IS NOT NULL
            AND t.paired_tx_id IS NULL
            AND t.kind NOT IN ('buy', 'sell', 'dividend', 'corporate_action', 'tax', 'fee')
            AND UPPER(REPLACE(t.counterparty_iban, ' ', '')) IN (
                SELECT UPPER(REPLACE(iban, ' ', '')) FROM accounts WHERE iban IS NOT NULL
            )",
    )
    .fetch_all(pool)
    .await?;

    let mut paired = 0usize;
    for (
        orig_id,
        orig_account_id,
        booking_date,
        amount_cents,
        currency,
        counterparty_iban,
        purpose,
    ) in candidates
    {
        // Re-check: if this Tx was already paired by an earlier iteration
        // (e.g. because both sides appeared as candidates in the Vec), skip it.
        let (already_paired,): (bool,) =
            sqlx::query_as("SELECT paired_tx_id IS NOT NULL FROM transactions WHERE id = ?1")
                .bind(orig_id)
                .fetch_one(pool)
                .await?;
        if already_paired {
            continue;
        }

        // Target account via normalized IBAN
        let target: Option<(i64, String, Option<String>)> = sqlx::query_as(
            "SELECT id, name, iban FROM accounts
              WHERE iban IS NOT NULL
                AND UPPER(REPLACE(iban, ' ', '')) = UPPER(REPLACE(?1, ' ', ''))
              LIMIT 1",
        )
        .bind(counterparty_iban.as_deref().unwrap_or(""))
        .fetch_optional(pool)
        .await?;

        let Some((target_account_id, _target_name, _target_iban)) = target else {
            continue;
        };

        if target_account_id == orig_account_id {
            // Self-transfer: same account on both sides (e.g. Sparkasse
            // smart-saving bucket reallocation). Mark the original as transfer
            // and search for the counter-entry on the same account.
            sqlx::query("UPDATE transactions SET kind = 'transfer' WHERE id = ?1")
                .bind(orig_id)
                .execute(pool)
                .await?;

            let partner: Option<(i64,)> = sqlx::query_as(
                "SELECT id FROM transactions
                  WHERE account_id = ?1
                    AND booking_date BETWEEN date(?2, '-3 days') AND date(?2, '+3 days')
                    AND amount_cents = ?3
                    AND currency = ?4
                    AND counterparty_iban IS NULL
                    AND paired_tx_id IS NULL
                    AND id != ?5
                  ORDER BY ABS(julianday(booking_date) - julianday(?2)), id
                  LIMIT 1",
            )
            .bind(orig_account_id)
            .bind(&booking_date)
            .bind(-amount_cents)
            .bind(&currency)
            .bind(orig_id)
            .fetch_optional(pool)
            .await?;

            if let Some((partner_id,)) = partner {
                sqlx::query("UPDATE transactions SET kind='transfer', paired_tx_id=?1 WHERE id=?2")
                    .bind(orig_id)
                    .bind(partner_id)
                    .execute(pool)
                    .await?;
                sqlx::query("UPDATE transactions SET paired_tx_id=?1 WHERE id=?2")
                    .bind(partner_id)
                    .bind(orig_id)
                    .execute(pool)
                    .await?;
            }

            paired += 1;
            continue;
        }

        // Source account info
        let source: Option<(String, Option<String>)> =
            sqlx::query_as("SELECT name, iban FROM accounts WHERE id = ?1")
                .bind(orig_account_id)
                .fetch_optional(pool)
                .await?;
        let Some((source_name, source_iban)) = source else {
            continue;
        };

        // Set original to kind='transfer' (no-op if already set)
        sqlx::query("UPDATE transactions SET kind = 'transfer' WHERE id = ?1")
            .bind(orig_id)
            .execute(pool)
            .await?;

        // SMART-DEDUP: search for an existing match on the target account.
        // Criteria: same target, ±3 days booking_date (bank value-date lag!),
        // inverted amount, currency, paired_tx_id IS NULL, NOT the source Tx itself.
        // Tie-break: nearest date, then lowest id.
        let existing: Option<(i64,)> = sqlx::query_as(
            "SELECT id FROM transactions
              WHERE account_id = ?1
                AND booking_date BETWEEN date(?2, '-3 days') AND date(?2, '+3 days')
                AND amount_cents = ?3
                AND currency = ?4
                AND paired_tx_id IS NULL
                AND id != ?5
              ORDER BY ABS(julianday(booking_date) - julianday(?2)), id
              LIMIT 1",
        )
        .bind(target_account_id)
        .bind(&booking_date)
        .bind(-amount_cents)
        .bind(&currency)
        .bind(orig_id)
        .fetch_optional(pool)
        .await?;

        if let Some((existing_id,)) = existing {
            // LINK instead of creating a new mirror
            sqlx::query("UPDATE transactions SET kind='transfer', paired_tx_id=?1 WHERE id=?2")
                .bind(orig_id)
                .bind(existing_id)
                .execute(pool)
                .await?;
            sqlx::query("UPDATE transactions SET paired_tx_id=?1 WHERE id=?2")
                .bind(existing_id)
                .bind(orig_id)
                .execute(pool)
                .await?;
        } else {
            // CREATE a new mirror
            let mirror: Option<(i64,)> = sqlx::query_as(
                "INSERT INTO transactions
                    (account_id, booking_date, amount_cents, currency,
                     counterparty, purpose, raw_ref, category_id, source,
                     source_file_hash, kind, counterparty_iban, paired_tx_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, NULL, 'auto_pair',
                         NULL, 'transfer', ?7, ?8)
                 RETURNING id",
            )
            .bind(target_account_id)
            .bind(&booking_date)
            .bind(-amount_cents)
            .bind(&currency)
            .bind(&source_name)
            .bind(purpose.as_deref())
            .bind(source_iban.as_deref())
            .bind(orig_id)
            .fetch_optional(pool)
            .await?;

            let Some((mirror_id,)) = mirror else {
                continue; // dedup conflict with another entry — rare
            };
            sqlx::query("UPDATE transactions SET paired_tx_id=?1 WHERE id=?2")
                .bind(mirror_id)
                .bind(orig_id)
                .execute(pool)
                .await?;
        }

        paired += 1;
    }

    Ok(paired)
}

#[cfg(test)]
mod tests {
    use crate::categorization::rules::{Combinator, MatchField, MatchOp, RuleCondition};
    use crate::db::connect_memory;
    use crate::db::rules::{insert_rule, NewRule};
    use crate::import_flow::{import_raw_transactions, ImportReport};
    use crate::importers::RawTransaction;
    use chrono::NaiveDate;
    use sqlx::SqlitePool;

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
        let (id,): (i64,) =
            sqlx::query_as("INSERT INTO categories (name) VALUES (?1) RETURNING id")
                .bind(name)
                .fetch_one(pool)
                .await
                .unwrap();
        id
    }

    fn tx(cp: Option<&str>, amount: i64, day: u32) -> RawTransaction {
        RawTransaction {
            booking_date: NaiveDate::from_ymd_opt(2026, 5, day).unwrap(),
            amount_cents: amount,
            currency: "EUR".into(),
            counterparty: cp.map(str::to_string),
            purpose: None,
            raw_ref: None,
            kind: None,
            trade: None,
            counterparty_iban: None,
        }
    }

    fn single_condition(field: MatchField, op: MatchOp) -> Vec<RuleCondition> {
        vec![RuleCondition { field, op }]
    }

    #[tokio::test]
    async fn fresh_import_inserts_all() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool, "TR").await;
        let raws = vec![tx(Some("REWE"), -1299, 1), tx(Some("EDEKA"), -500, 2)];

        let report = import_raw_transactions(&pool, acc, "tr_csv", Some("h"), raws, 0.85)
            .await
            .unwrap();

        assert_eq!(
            report,
            ImportReport {
                parsed: 2,
                inserted: 2,
                skipped: 0,
                categorized_by_rule: 0,
                categorized_by_fuzzy: 0,
                warnings: vec![],
            }
        );
    }

    #[tokio::test]
    async fn rerunning_same_import_skips_all() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool, "TR").await;
        let raws = vec![tx(Some("REWE"), -1299, 1), tx(Some("EDEKA"), -500, 2)];

        import_raw_transactions(&pool, acc, "tr_csv", Some("h"), raws.clone(), 0.85)
            .await
            .unwrap();
        let report2 = import_raw_transactions(&pool, acc, "tr_csv", Some("h"), raws, 0.85)
            .await
            .unwrap();

        assert_eq!(report2.parsed, 2);
        assert_eq!(report2.inserted, 0);
        assert_eq!(report2.skipped, 2);
    }

    #[tokio::test]
    async fn rule_categorizes_new_transaction() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool, "TR").await;
        let cat = seed_category(&pool, "Lebensmittel").await;
        insert_rule(
            &pool,
            &NewRule {
                priority: 100,
                name: "REWE".into(),
                combinator: Combinator::And,
                conditions: single_condition(
                    MatchField::Counterparty,
                    MatchOp::Contains("REWE".into()),
                ),
                target_category_id: cat,
                enabled: true,
            },
        )
        .await
        .unwrap();

        let raws = vec![tx(Some("REWE Markt"), -1299, 1)];
        let report = import_raw_transactions(&pool, acc, "tr_csv", Some("h"), raws, 0.85)
            .await
            .unwrap();

        assert_eq!(report.inserted, 1);
        assert_eq!(report.categorized_by_rule, 1);
        assert_eq!(report.categorized_by_fuzzy, 0);

        let (stored,): (Option<i64>,) =
            sqlx::query_as("SELECT category_id FROM transactions LIMIT 1")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(stored, Some(cat));
    }

    #[tokio::test]
    async fn or_rule_categorizes_either_counterparty() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool, "TR").await;
        let cat = seed_category(&pool, "Lebensmittel").await;
        insert_rule(
            &pool,
            &NewRule {
                priority: 100,
                name: "Supermarkt".into(),
                combinator: Combinator::Or,
                conditions: vec![
                    RuleCondition {
                        field: MatchField::Counterparty,
                        op: MatchOp::Contains("REWE".into()),
                    },
                    RuleCondition {
                        field: MatchField::Counterparty,
                        op: MatchOp::Contains("EDEKA".into()),
                    },
                ],
                target_category_id: cat,
                enabled: true,
            },
        )
        .await
        .unwrap();

        let raws = vec![tx(Some("REWE"), -1299, 1), tx(Some("EDEKA"), -500, 2)];
        let report = import_raw_transactions(&pool, acc, "tr_csv", Some("h"), raws, 0.85)
            .await
            .unwrap();

        assert_eq!(report.inserted, 2);
        assert_eq!(report.categorized_by_rule, 2);
    }

    #[tokio::test]
    async fn fuzzy_uses_history_from_db() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool, "TR").await;
        let cat = seed_category(&pool, "Lebensmittel").await;

        sqlx::query(
            "INSERT INTO transactions
                (account_id, booking_date, amount_cents, currency, counterparty,
                 category_id, source)
             VALUES (?1, '2026-04-01', -1000, 'EUR', 'REWE Markt Berlin', ?2, 'manual')",
        )
        .bind(acc)
        .bind(cat)
        .execute(&pool)
        .await
        .unwrap();

        let raws = vec![tx(Some("REWE Markt Hamburg"), -1200, 2)];
        let report = import_raw_transactions(&pool, acc, "tr_csv", Some("h"), raws, 0.85)
            .await
            .unwrap();

        assert_eq!(report.inserted, 1);
        assert_eq!(report.categorized_by_rule, 0);
        assert_eq!(report.categorized_by_fuzzy, 1);

        let (stored,): (Option<i64>,) = sqlx::query_as(
            "SELECT category_id FROM transactions WHERE counterparty = 'REWE Markt Hamburg'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(stored, Some(cat));
    }

    #[tokio::test]
    async fn bulk_assign_categorizes_matching_existing_transactions() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool, "TR").await;
        let cat = seed_category(&pool, "Lebensmittel").await;

        for (date, cp) in [
            ("2026-05-01", "REWE Markt"),
            ("2026-05-02", "REWE Express"),
            ("2026-05-03", "Edeka"),
        ] {
            sqlx::query(
                "INSERT INTO transactions
                    (account_id, booking_date, amount_cents, currency,
                     counterparty, source)
                 VALUES (?, ?, -1000, 'EUR', ?, 'manual')",
            )
            .bind(acc)
            .bind(date)
            .bind(cp)
            .execute(&pool)
            .await
            .unwrap();
        }

        let rule_id = insert_rule(
            &pool,
            &NewRule {
                priority: 100,
                name: "REWE".into(),
                combinator: Combinator::And,
                conditions: single_condition(
                    MatchField::Counterparty,
                    MatchOp::Contains("REWE".into()),
                ),
                target_category_id: cat,
                enabled: true,
            },
        )
        .await
        .unwrap();

        let count = crate::import_flow::bulk_assign_category_by_rule(&pool, rule_id)
            .await
            .unwrap();

        assert_eq!(count, 2);
        let (categorized,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM transactions WHERE category_id = ?")
                .bind(cat)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(categorized, 2);
    }

    #[tokio::test]
    async fn bulk_assign_matches_description_via_manual_note() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool, "TR").await;
        let cat = seed_category(&pool, "Freizeit").await;

        sqlx::query(
            "INSERT INTO transactions
                (account_id, booking_date, amount_cents, currency,
                 counterparty, manual_note, source)
             VALUES (?, '2026-05-04', -2000, 'EUR', 'Bar X', 'Geburtstag Anna', 'manual')",
        )
        .bind(acc)
        .execute(&pool)
        .await
        .unwrap();

        let rule_id = insert_rule(
            &pool,
            &NewRule {
                priority: 100,
                name: "Geschenke".into(),
                combinator: Combinator::And,
                conditions: single_condition(
                    MatchField::Description,
                    MatchOp::Contains("Geburtstag".into()),
                ),
                target_category_id: cat,
                enabled: true,
            },
        )
        .await
        .unwrap();

        let count = crate::import_flow::bulk_assign_category_by_rule(&pool, rule_id)
            .await
            .unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn bulk_assign_with_account_condition_filters_by_account() {
        let pool = connect_memory().await.unwrap();
        let acc_a = seed_account(&pool, "A").await;
        let acc_b = seed_account(&pool, "B").await;
        let cat = seed_category(&pool, "Lebensmittel").await;

        for (acc, cp) in [(acc_a, "REWE"), (acc_b, "REWE")] {
            sqlx::query(
                "INSERT INTO transactions
                    (account_id, booking_date, amount_cents, currency,
                     counterparty, source)
                 VALUES (?, '2026-05-01', -1000, 'EUR', ?, 'manual')",
            )
            .bind(acc)
            .bind(cp)
            .execute(&pool)
            .await
            .unwrap();
        }

        let rule_id = insert_rule(
            &pool,
            &NewRule {
                priority: 100,
                name: "REWE auf A".into(),
                combinator: Combinator::And,
                conditions: vec![
                    RuleCondition {
                        field: MatchField::Counterparty,
                        op: MatchOp::Contains("REWE".into()),
                    },
                    RuleCondition {
                        field: MatchField::Account,
                        op: MatchOp::Equals(acc_a.to_string()),
                    },
                ],
                target_category_id: cat,
                enabled: true,
            },
        )
        .await
        .unwrap();

        let count = crate::import_flow::bulk_assign_category_by_rule(&pool, rule_id)
            .await
            .unwrap();
        assert_eq!(count, 1);

        let cat_on_a: (Option<i64>,) =
            sqlx::query_as("SELECT category_id FROM transactions WHERE account_id = ?")
                .bind(acc_a)
                .fetch_one(&pool)
                .await
                .unwrap();
        let cat_on_b: (Option<i64>,) =
            sqlx::query_as("SELECT category_id FROM transactions WHERE account_id = ?")
                .bind(acc_b)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(cat_on_a.0, Some(cat));
        assert_eq!(cat_on_b.0, None);
    }

    #[tokio::test]
    async fn bulk_assign_returns_zero_when_no_matches() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool, "TR").await;
        let cat = seed_category(&pool, "Lebensmittel").await;
        sqlx::query(
            "INSERT INTO transactions
                (account_id, booking_date, amount_cents, currency,
                 counterparty, source)
             VALUES (?, '2026-05-01', -500, 'EUR', 'Edeka', 'manual')",
        )
        .bind(acc)
        .execute(&pool)
        .await
        .unwrap();

        let rule_id = insert_rule(
            &pool,
            &NewRule {
                priority: 100,
                name: "REWE".into(),
                combinator: Combinator::And,
                conditions: single_condition(
                    MatchField::Counterparty,
                    MatchOp::Contains("REWE".into()),
                ),
                target_category_id: cat,
                enabled: true,
            },
        )
        .await
        .unwrap();

        let count = crate::import_flow::bulk_assign_category_by_rule(&pool, rule_id)
            .await
            .unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn bulk_assign_errors_for_missing_rule() {
        let pool = connect_memory().await.unwrap();
        let result = crate::import_flow::bulk_assign_category_by_rule(&pool, 9999).await;
        assert!(result.is_err(), "expected error for missing rule id");
    }

    #[tokio::test]
    async fn import_buy_row_creates_trade_and_security() {
        use crate::importers::{RawTradeFields, RawTransaction};
        let pool = connect_memory().await.unwrap();
        let (acc_id,): (i64,) = sqlx::query_as(
            "INSERT INTO accounts (name, kind, currency) VALUES ('Broker','broker','EUR') RETURNING id"
        ).fetch_one(&pool).await.unwrap();

        let raw = RawTransaction {
            booking_date: NaiveDate::from_ymd_opt(2026, 5, 13).unwrap(),
            amount_cents: -1_200_000,
            currency: "EUR".into(),
            counterparty: Some("TR Trade".into()),
            purpose: None,
            raw_ref: Some("buy-e2e-1".into()),
            kind: Some("buy".into()),
            trade: Some(RawTradeFields {
                isin: "LU0290358497".into(),
                asset_class_raw: "FUND".into(),
                name: "Xtrackers II EUR Overnight".into(),
                side: "buy".into(),
                shares_micro: 80_473_721,
                unit_price_micro: Some(149_117_000),
                fee_cents: 100,
                kest_cents: 0,
                withholding_tax_cents: 0,
                fx_rate_micro: None,
                fusion_group: None,
            }),
            counterparty_iban: None,
        };

        let report =
            import_raw_transactions(&pool, acc_id, "tr_csv", Some("hashX"), vec![raw], 0.85)
                .await
                .unwrap();
        assert_eq!(report.inserted, 1);

        let (sec_count,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM securities WHERE isin = 'LU0290358497'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(sec_count, 1);

        let (trade_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM securities_trades")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(trade_count, 1);

        let (kind,): (String,) =
            sqlx::query_as("SELECT kind FROM transactions WHERE raw_ref = 'buy-e2e-1'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(kind, "buy");
    }

    #[tokio::test]
    async fn reimport_buy_row_is_idempotent() {
        use crate::importers::{RawTradeFields, RawTransaction};
        let pool = connect_memory().await.unwrap();
        let (acc_id,): (i64,) = sqlx::query_as(
            "INSERT INTO accounts (name, kind, currency) VALUES ('Broker','broker','EUR') RETURNING id"
        ).fetch_one(&pool).await.unwrap();

        let raw = RawTransaction {
            booking_date: NaiveDate::from_ymd_opt(2026, 5, 13).unwrap(),
            amount_cents: -1_200_000,
            currency: "EUR".into(),
            counterparty: None,
            purpose: None,
            raw_ref: Some("dup-1".into()),
            kind: Some("buy".into()),
            trade: Some(RawTradeFields {
                isin: "LU0290358497".into(),
                asset_class_raw: "FUND".into(),
                name: "Xtrackers".into(),
                side: "buy".into(),
                shares_micro: 80_000_000,
                unit_price_micro: Some(149_000_000),
                fee_cents: 0,
                kest_cents: 0,
                withholding_tax_cents: 0,
                fx_rate_micro: None,
                fusion_group: None,
            }),
            counterparty_iban: None,
        };

        let r1 = import_raw_transactions(
            &pool,
            acc_id,
            "tr_csv",
            Some("hashX"),
            vec![raw.clone()],
            0.85,
        )
        .await
        .unwrap();
        let r2 = import_raw_transactions(&pool, acc_id, "tr_csv", Some("hashX"), vec![raw], 0.85)
            .await
            .unwrap();

        assert_eq!(r1.inserted, 1);
        assert_eq!(r2.inserted, 0);
        assert_eq!(r2.skipped, 1);

        // exactly 1 trade row despite double import
        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM securities_trades")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 1);
    }

    // --- detect_inter_account_transfers tests ---

    async fn seed_account_with_iban(pool: &SqlitePool, name: &str, iban: &str) -> i64 {
        let (id,): (i64,) = sqlx::query_as(
            "INSERT INTO accounts (name, kind, currency, iban) VALUES (?1, 'bank', 'EUR', ?2) RETURNING id",
        )
        .bind(name)
        .bind(iban)
        .fetch_one(pool)
        .await
        .unwrap();
        id
    }

    async fn insert_tx_with_iban(
        pool: &SqlitePool,
        account_id: i64,
        counterparty_iban: Option<&str>,
        kind: &str,
    ) -> i64 {
        // Use kind as counterparty to avoid dedup-index collisions across kinds.
        let (id,): (i64,) = sqlx::query_as(
            "INSERT INTO transactions
                (account_id, booking_date, amount_cents, currency, counterparty,
                 source, kind, counterparty_iban)
             VALUES (?1, '2026-05-01', -1000, 'EUR', ?2, 'tr_csv', ?2, ?3) RETURNING id",
        )
        .bind(account_id)
        .bind(kind)
        .bind(counterparty_iban)
        .fetch_one(pool)
        .await
        .unwrap();
        id
    }

    #[tokio::test]
    async fn detect_marks_matching_tx_as_transfer() {
        let pool = connect_memory().await.unwrap();
        let acc_a = seed_account(&pool, "Giro").await;
        seed_account_with_iban(&pool, "Savings", "DE89370400440532013000").await;
        let tx_id =
            insert_tx_with_iban(&pool, acc_a, Some("DE89370400440532013000"), "expense").await;

        let updated = crate::import_flow::detect_inter_account_transfers(&pool)
            .await
            .unwrap();
        assert_eq!(updated, 1);

        let (kind,): (String,) = sqlx::query_as("SELECT kind FROM transactions WHERE id = ?")
            .bind(tx_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(kind, "transfer");
    }

    #[tokio::test]
    async fn detect_skips_when_iban_does_not_match() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool, "Giro").await;
        seed_account_with_iban(&pool, "Savings", "DE89370400440532013000").await;
        let tx_id =
            insert_tx_with_iban(&pool, acc, Some("DE00000000000000000000"), "expense").await;

        let updated = crate::import_flow::detect_inter_account_transfers(&pool)
            .await
            .unwrap();
        assert_eq!(updated, 0);

        let (kind,): (String,) = sqlx::query_as("SELECT kind FROM transactions WHERE id = ?")
            .bind(tx_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(kind, "expense"); // unchanged
    }

    #[tokio::test]
    async fn detect_skips_trade_kinds() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool, "Broker").await;
        seed_account_with_iban(&pool, "OtherAccount", "DE89370400440532013000").await;

        // Trade-side kinds are excluded. kind='transfer' with
        // paired_tx_id IS NULL is now RE-PROCESSED (Bug 1 fix).
        for kind in ["buy", "sell", "dividend", "corporate_action", "tax"] {
            insert_tx_with_iban(&pool, acc, Some("DE89370400440532013000"), kind).await;
        }

        let updated = crate::import_flow::detect_inter_account_transfers(&pool)
            .await
            .unwrap();
        assert_eq!(updated, 0, "trade kinds should not be re-tagged");
    }

    #[tokio::test]
    async fn detect_normalizes_whitespace_in_iban() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool, "Giro").await;
        // Account-IBAN with spaces
        seed_account_with_iban(&pool, "Savings", "DE89 3704 0044 0532 0130 00").await;

        // tx-IBAN without spaces → should match
        let (id1,): (i64,) = sqlx::query_as(
            "INSERT INTO transactions
                (account_id, booking_date, amount_cents, currency, counterparty,
                 source, kind, counterparty_iban)
             VALUES (?1, '2026-05-01', -1000, 'EUR', 'cp-nospace', 'tr_csv', 'expense', 'DE89370400440532013000') RETURNING id",
        )
        .bind(acc)
        .fetch_one(&pool)
        .await
        .unwrap();

        // tx-IBAN with spaces → also should match (reversed variant)
        let (id2,): (i64,) = sqlx::query_as(
            "INSERT INTO transactions
                (account_id, booking_date, amount_cents, currency, counterparty,
                 source, kind, counterparty_iban)
             VALUES (?1, '2026-05-02', -2000, 'EUR', 'cp-space', 'tr_csv', 'expense', 'DE89 3704 0044 0532 0130 00') RETURNING id",
        )
        .bind(acc)
        .fetch_one(&pool)
        .await
        .unwrap();

        let updated = crate::import_flow::detect_inter_account_transfers(&pool)
            .await
            .unwrap();
        assert_eq!(updated, 2);

        let (k1,): (String,) = sqlx::query_as("SELECT kind FROM transactions WHERE id = ?")
            .bind(id1)
            .fetch_one(&pool)
            .await
            .unwrap();
        let (k2,): (String,) = sqlx::query_as("SELECT kind FROM transactions WHERE id = ?")
            .bind(id2)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(k1, "transfer");
        assert_eq!(k2, "transfer");
    }

    #[tokio::test]
    async fn detect_is_case_insensitive() {
        // accounts.iban has a CHECK constraint requiring uppercase, so the
        // account IBAN is always uppercase. The tx counterparty_iban has no such
        // constraint and can be lowercase — UPPER() in the SQL normalizes both.
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool, "Giro").await;
        // Account-IBAN uppercase (as required by DB constraint)
        seed_account_with_iban(&pool, "Savings", "DE89370400440532013000").await;

        // tx counterparty_iban lowercase → should still match after UPPER() normalization
        let (tx_id,): (i64,) = sqlx::query_as(
            "INSERT INTO transactions
                (account_id, booking_date, amount_cents, currency, counterparty,
                 source, kind, counterparty_iban)
             VALUES (?1, '2026-05-01', -1000, 'EUR', 'cp-lower', 'tr_csv', 'expense', 'de89370400440532013000') RETURNING id",
        )
        .bind(acc)
        .fetch_one(&pool)
        .await
        .unwrap();

        let updated = crate::import_flow::detect_inter_account_transfers(&pool)
            .await
            .unwrap();
        assert_eq!(updated, 1);

        let (kind,): (String,) = sqlx::query_as("SELECT kind FROM transactions WHERE id = ?")
            .bind(tx_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(kind, "transfer");
    }

    #[tokio::test]
    async fn detect_ignores_tx_with_null_iban() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool, "Giro").await;
        seed_account_with_iban(&pool, "Savings", "DE89370400440532013000").await;

        // Tx without counterparty_iban (NULL) — kind should stay 'expense'
        let (tx_id,): (i64,) = sqlx::query_as(
            "INSERT INTO transactions
                (account_id, booking_date, amount_cents, currency, counterparty,
                 source, kind)
             VALUES (?1, '2026-05-01', -1000, 'EUR', 'some-counterparty', 'tr_csv', 'expense') RETURNING id",
        )
        .bind(acc)
        .fetch_one(&pool)
        .await
        .unwrap();

        let updated = crate::import_flow::detect_inter_account_transfers(&pool)
            .await
            .unwrap();
        assert_eq!(updated, 0);

        let (kind,): (String,) = sqlx::query_as("SELECT kind FROM transactions WHERE id = ?")
            .bind(tx_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(kind, "expense");
    }

    #[tokio::test]
    async fn detect_ignores_accounts_with_null_iban() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool, "Giro").await; // no IBAN on this account
                                                     // A second account also without IBAN
        seed_account(&pool, "Savings").await;

        // tx with a counterparty_iban that doesn't match any account iban (all NULL)
        let (tx_id,): (i64,) = sqlx::query_as(
            "INSERT INTO transactions
                (account_id, booking_date, amount_cents, currency, counterparty,
                 source, kind, counterparty_iban)
             VALUES (?1, '2026-05-01', -1000, 'EUR', 'someone', 'tr_csv', 'expense', 'DE89370400440532013000') RETURNING id",
        )
        .bind(acc)
        .fetch_one(&pool)
        .await
        .unwrap();

        let updated = crate::import_flow::detect_inter_account_transfers(&pool)
            .await
            .unwrap();
        assert_eq!(updated, 0, "no account has an IBAN, nothing should match");

        let (kind,): (String,) = sqlx::query_as("SELECT kind FROM transactions WHERE id = ?")
            .bind(tx_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(kind, "expense");
    }

    #[tokio::test]
    async fn detect_returns_count_of_updated_rows() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool, "Giro").await;
        seed_account_with_iban(&pool, "Savings", "DE89370400440532013000").await;

        // 2 matching tx
        for (day, cp) in [("2026-05-01", "cp1"), ("2026-05-02", "cp2")] {
            sqlx::query(
                "INSERT INTO transactions
                    (account_id, booking_date, amount_cents, currency, counterparty,
                     source, kind, counterparty_iban)
                 VALUES (?1, ?2, -1000, 'EUR', ?3, 'tr_csv', 'expense', 'DE89370400440532013000')",
            )
            .bind(acc)
            .bind(day)
            .bind(cp)
            .execute(&pool)
            .await
            .unwrap();
        }
        // 1 non-matching tx (different IBAN)
        sqlx::query(
            "INSERT INTO transactions
                (account_id, booking_date, amount_cents, currency, counterparty,
                 source, kind, counterparty_iban)
             VALUES (?1, '2026-05-03', -500, 'EUR', 'cp3', 'tr_csv', 'expense', 'DE00000000000000000000')",
        )
        .bind(acc).execute(&pool).await.unwrap();

        let updated = crate::import_flow::detect_inter_account_transfers(&pool)
            .await
            .unwrap();
        assert_eq!(updated, 2, "only the 2 matching rows should be counted");
    }

    #[tokio::test]
    async fn import_routes_trade_to_depot_when_tx_at_verrechnung() {
        // Setup: TR institution with settlement account + depot.
        // Importing a trade row into the settlement account → securities_trades.account_id = depot.
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

        use crate::importers::{RawTradeFields, RawTransaction};
        use chrono::NaiveDate;
        let raw = RawTransaction {
            booking_date: NaiveDate::from_ymd_opt(2026, 5, 15).unwrap(),
            amount_cents: -50_000,
            currency: "EUR".into(),
            counterparty: Some("VWCE".into()),
            purpose: None,
            raw_ref: Some("tr-buy-1".into()),
            kind: Some("buy".into()),
            trade: Some(RawTradeFields {
                isin: "IE00BK5BQT80".into(),
                name: "Vanguard FTSE All-World".into(),
                asset_class_raw: "ETF".into(),
                side: "buy".into(),
                shares_micro: 5_000_000,
                unit_price_micro: Some(10_000_000),
                fee_cents: 0,
                kest_cents: 0,
                withholding_tax_cents: 0,
                fx_rate_micro: None,
                fusion_group: None,
            }),
            counterparty_iban: None,
        };

        let report =
            import_raw_transactions(&pool, verrechnung.id, "tr_csv", Some("h1"), vec![raw], 0.85)
                .await
                .unwrap();
        assert_eq!(report.inserted, 1);

        // securities_trades.account_id should point to the depot
        let (st_acc,): (Option<i64>,) =
            sqlx::query_as("SELECT account_id FROM securities_trades LIMIT 1")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            st_acc,
            Some(depot.id),
            "trade should be routed to the depot"
        );
    }

    #[tokio::test]
    async fn import_leaves_trade_account_null_when_no_broker_in_institution() {
        // Institution has only a bank account, no broker → no routing, account_id NULL.
        let pool = connect_memory().await.unwrap();
        sqlx::query("INSERT INTO institutions (name) VALUES ('BankOnly')")
            .execute(&pool)
            .await
            .unwrap();
        let (inst_id,): (i64,) =
            sqlx::query_as("SELECT id FROM institutions WHERE name='BankOnly'")
                .fetch_one(&pool)
                .await
                .unwrap();
        let bank = crate::db::accounts::create_account(
            &pool,
            "Bank",
            "bank",
            "EUR",
            None,
            None,
            Some(inst_id),
        )
        .await
        .unwrap();

        use crate::importers::{RawTradeFields, RawTransaction};
        use chrono::NaiveDate;
        let raw = RawTransaction {
            booking_date: NaiveDate::from_ymd_opt(2026, 5, 15).unwrap(),
            amount_cents: -50_000,
            currency: "EUR".into(),
            counterparty: Some("X".into()),
            purpose: None,
            raw_ref: Some("tr-buy-2".into()),
            kind: Some("buy".into()),
            trade: Some(RawTradeFields {
                isin: "IE00B4L5Y983".into(),
                name: "iShares".into(),
                asset_class_raw: "ETF".into(),
                side: "buy".into(),
                shares_micro: 1_000_000,
                unit_price_micro: Some(10_000_000),
                fee_cents: 0,
                kest_cents: 0,
                withholding_tax_cents: 0,
                fx_rate_micro: None,
                fusion_group: None,
            }),
            counterparty_iban: None,
        };

        import_raw_transactions(&pool, bank.id, "tr_csv", Some("h2"), vec![raw], 0.85)
            .await
            .unwrap();

        let (st_acc,): (Option<i64>,) =
            sqlx::query_as("SELECT account_id FROM securities_trades LIMIT 1")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            st_acc, None,
            "no broker in institution: account_id stays NULL"
        );
    }

    #[tokio::test]
    async fn import_leaves_trade_account_null_when_already_at_broker() {
        // Tx already at broker → securities_trades.account_id NULL (falls back to tx.account_id).
        let pool = connect_memory().await.unwrap();
        sqlx::query("INSERT INTO institutions (name) VALUES ('TR')")
            .execute(&pool)
            .await
            .unwrap();
        let (inst_id,): (i64,) = sqlx::query_as("SELECT id FROM institutions WHERE name='TR'")
            .fetch_one(&pool)
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

        use crate::importers::{RawTradeFields, RawTransaction};
        use chrono::NaiveDate;
        let raw = RawTransaction {
            booking_date: NaiveDate::from_ymd_opt(2026, 5, 15).unwrap(),
            amount_cents: -50_000,
            currency: "EUR".into(),
            counterparty: Some("X".into()),
            purpose: None,
            raw_ref: Some("tr-buy-3".into()),
            kind: Some("buy".into()),
            trade: Some(RawTradeFields {
                isin: "IE00BK5BQT81".into(),
                name: "X".into(),
                asset_class_raw: "ETF".into(),
                side: "buy".into(),
                shares_micro: 5_000_000,
                unit_price_micro: Some(10_000_000),
                fee_cents: 0,
                kest_cents: 0,
                withholding_tax_cents: 0,
                fx_rate_micro: None,
                fusion_group: None,
            }),
            counterparty_iban: None,
        };

        import_raw_transactions(&pool, depot.id, "tr_csv", Some("h3"), vec![raw], 0.85)
            .await
            .unwrap();

        let (st_acc,): (Option<i64>,) =
            sqlx::query_as("SELECT account_id FROM securities_trades LIMIT 1")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(st_acc, None, "Tx already at broker: no redundant routing");
    }

    #[tokio::test]
    async fn auto_pair_creates_mirror_on_target_account() {
        use crate::import_flow::detect_inter_account_transfers;
        let pool = connect_memory().await.unwrap();

        // Two accounts with IBANs (valid format: 2 letters + 2 digits + min 11 chars = 15 total)
        let src: i64 = sqlx::query_scalar(
            "INSERT INTO accounts (name, kind, currency, iban) VALUES ('Main', 'bank', 'EUR', 'AT000000000000000010') RETURNING id"
        ).fetch_one(&pool).await.unwrap();
        let tgt: i64 = sqlx::query_scalar(
            "INSERT INTO accounts (name, kind, currency, iban) VALUES ('Cash', 'bank', 'EUR', 'AT000000000000000011') RETURNING id"
        ).fetch_one(&pool).await.unwrap();

        // Source Tx with counterparty_iban pointing to target
        let orig_id: i64 = sqlx::query_scalar(
            "INSERT INTO transactions (account_id, booking_date, amount_cents, currency, counterparty, counterparty_iban, kind, source)
             VALUES (?1, '2026-05-15', -10000, 'EUR', 'Max Mustermann', 'AT000000000000000011', 'expense', 'sparkasse_csv') RETURNING id"
        ).bind(src).fetch_one(&pool).await.unwrap();

        let paired = detect_inter_account_transfers(&pool).await.unwrap();
        assert_eq!(paired, 1);

        // Original is now kind='transfer' and has paired_tx_id
        let (orig_kind, orig_paired): (String, Option<i64>) =
            sqlx::query_as("SELECT kind, paired_tx_id FROM transactions WHERE id = ?1")
                .bind(orig_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(orig_kind, "transfer");
        assert!(orig_paired.is_some());

        // Mirror exists on target, inverted amount, source='auto_pair'
        let mirror_id = orig_paired.unwrap();
        let (m_acc, m_amount, m_kind, m_source, m_paired, m_cp): (i64, i64, String, String, Option<i64>, Option<String>) = sqlx::query_as(
            "SELECT account_id, amount_cents, kind, source, paired_tx_id, counterparty FROM transactions WHERE id = ?1"
        ).bind(mirror_id).fetch_one(&pool).await.unwrap();
        assert_eq!(m_acc, tgt);
        assert_eq!(m_amount, 10000); // inverted
        assert_eq!(m_kind, "transfer");
        assert_eq!(m_source, "auto_pair");
        assert_eq!(m_paired, Some(orig_id));
        assert_eq!(m_cp.as_deref(), Some("Main"));
    }

    #[tokio::test]
    async fn auto_pair_is_idempotent_on_re_run() {
        use crate::import_flow::detect_inter_account_transfers;
        let pool = connect_memory().await.unwrap();
        let src: i64 = sqlx::query_scalar(
            "INSERT INTO accounts (name, kind, currency, iban) VALUES ('Main', 'bank', 'EUR', 'AT000000000000000010') RETURNING id"
        ).fetch_one(&pool).await.unwrap();
        let _tgt: i64 = sqlx::query_scalar(
            "INSERT INTO accounts (name, kind, currency, iban) VALUES ('Cash', 'bank', 'EUR', 'AT000000000000000011') RETURNING id"
        ).fetch_one(&pool).await.unwrap();
        sqlx::query(
            "INSERT INTO transactions (account_id, booking_date, amount_cents, currency, counterparty_iban, kind, source)
             VALUES (?1, '2026-05-15', -100, 'EUR', 'AT000000000000000011', 'expense', 'sparkasse_csv')"
        ).bind(src).execute(&pool).await.unwrap();

        let first = detect_inter_account_transfers(&pool).await.unwrap();
        let second = detect_inter_account_transfers(&pool).await.unwrap();
        let third = detect_inter_account_transfers(&pool).await.unwrap();
        assert_eq!(first, 1);
        assert_eq!(second, 0);
        assert_eq!(third, 0);

        // Total tx count = 2 (original + mirror), no duplicates
        let (cnt,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM transactions")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(cnt, 2);
    }

    #[tokio::test]
    async fn auto_pair_self_transfer_marks_as_transfer() {
        // Self-transfer (counterparty IBAN = own account IBAN) is now handled:
        // original gets kind='transfer', no mirror created.
        use crate::import_flow::detect_inter_account_transfers;
        let pool = connect_memory().await.unwrap();
        let src: i64 = sqlx::query_scalar(
            "INSERT INTO accounts (name, kind, currency, iban) VALUES ('Main', 'bank', 'EUR', 'AT000000000000000010') RETURNING id"
        ).fetch_one(&pool).await.unwrap();
        // Tx whose counterparty IBAN points to the SAME account (no matching +partner)
        let tx_id: i64 = sqlx::query_scalar(
            "INSERT INTO transactions (account_id, booking_date, amount_cents, currency, counterparty_iban, kind, source)
             VALUES (?1, '2026-05-15', -100, 'EUR', 'AT000000000000000010', 'expense', 'test') RETURNING id"
        ).bind(src).fetch_one(&pool).await.unwrap();

        let paired = detect_inter_account_transfers(&pool).await.unwrap();
        assert_eq!(
            paired, 1,
            "self-transfer without partner counts as 1 processed"
        );

        let (kind,): (String,) = sqlx::query_as("SELECT kind FROM transactions WHERE id=?1")
            .bind(tx_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(
            kind, "transfer",
            "self-transfer without partner marked as transfer"
        );

        // No mirror should be created
        let (cnt,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM transactions WHERE source='auto_pair'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(cnt, 0, "no mirror for self-transfer");
    }

    #[tokio::test]
    async fn auto_pair_inverts_amount_for_incoming() {
        use crate::import_flow::detect_inter_account_transfers;
        let pool = connect_memory().await.unwrap();
        let src: i64 = sqlx::query_scalar(
            "INSERT INTO accounts (name, kind, currency, iban) VALUES ('Main', 'bank', 'EUR', 'AT000000000000000010') RETURNING id"
        ).fetch_one(&pool).await.unwrap();
        let _tgt: i64 = sqlx::query_scalar(
            "INSERT INTO accounts (name, kind, currency, iban) VALUES ('Other', 'bank', 'EUR', 'AT000000000000000011') RETURNING id"
        ).fetch_one(&pool).await.unwrap();
        // INCOMING: source receives +500 from AT000000000000000011
        sqlx::query(
            "INSERT INTO transactions (account_id, booking_date, amount_cents, currency, counterparty_iban, kind, source)
             VALUES (?1, '2026-05-15', 500, 'EUR', 'AT000000000000000011', 'income', 'test')"
        ).bind(src).execute(&pool).await.unwrap();

        detect_inter_account_transfers(&pool).await.unwrap();

        let (mirror_amount,): (i64,) =
            sqlx::query_as("SELECT amount_cents FROM transactions WHERE source = 'auto_pair'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(mirror_amount, -500); // outgoing on the target side
    }

    #[tokio::test]
    async fn auto_pair_links_existing_match_instead_of_creating_duplicate() {
        use crate::import_flow::detect_inter_account_transfers;
        let pool = connect_memory().await.unwrap();

        let src: i64 = sqlx::query_scalar(
            "INSERT INTO accounts (name, kind, currency, iban) VALUES ('Main', 'bank', 'EUR', 'AT000000000000000010') RETURNING id"
        ).fetch_one(&pool).await.unwrap();
        let tgt: i64 = sqlx::query_scalar(
            "INSERT INTO accounts (name, kind, currency, iban) VALUES ('Other', 'bank', 'EUR', 'AT000000000000000011') RETURNING id"
        ).fetch_one(&pool).await.unwrap();

        // Two ALREADY-imported sides of the same transfer
        let src_id: i64 = sqlx::query_scalar(
            "INSERT INTO transactions (account_id, booking_date, amount_cents, currency, counterparty_iban, kind, source)
             VALUES (?1, '2026-05-15', -10000, 'EUR', 'AT000000000000000011', 'expense', 'sparkasse_csv') RETURNING id"
        ).bind(src).fetch_one(&pool).await.unwrap();
        let tgt_id: i64 = sqlx::query_scalar(
            "INSERT INTO transactions (account_id, booking_date, amount_cents, currency, counterparty_iban, kind, source)
             VALUES (?1, '2026-05-15', 10000, 'EUR', 'AT000000000000000010', 'income', 'sparkasse_csv') RETURNING id"
        ).bind(tgt).fetch_one(&pool).await.unwrap();

        let paired = detect_inter_account_transfers(&pool).await.unwrap();
        // 1 pair created — both existing Tx linked
        assert_eq!(paired, 1);

        // No new Tx (3rd insert) should exist
        let (cnt,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM transactions")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(
            cnt, 2,
            "no mirror should be auto-created when both sides existed"
        );

        // Both linked
        let (src_paired, src_kind): (Option<i64>, String) =
            sqlx::query_as("SELECT paired_tx_id, kind FROM transactions WHERE id=?1")
                .bind(src_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        let (tgt_paired, tgt_kind): (Option<i64>, String) =
            sqlx::query_as("SELECT paired_tx_id, kind FROM transactions WHERE id=?1")
                .bind(tgt_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(src_paired, Some(tgt_id));
        assert_eq!(tgt_paired, Some(src_id));
        assert_eq!(src_kind, "transfer");
        assert_eq!(tgt_kind, "transfer");
    }

    #[tokio::test]
    async fn auto_pair_processes_existing_unpaired_transfer_tx() {
        // Tx that already had kind='transfer' (from old detection) but
        // paired_tx_id NULL — should still be re-processed.
        use crate::import_flow::detect_inter_account_transfers;
        let pool = connect_memory().await.unwrap();

        let src: i64 = sqlx::query_scalar(
            "INSERT INTO accounts (name, kind, currency, iban) VALUES ('Main', 'bank', 'EUR', 'AT000000000000000010') RETURNING id"
        ).fetch_one(&pool).await.unwrap();
        let _tgt: i64 = sqlx::query_scalar(
            "INSERT INTO accounts (name, kind, currency, iban) VALUES ('Other', 'bank', 'EUR', 'AT000000000000000011') RETURNING id"
        ).fetch_one(&pool).await.unwrap();

        // kind is already 'transfer', paired_tx_id NULL
        sqlx::query(
            "INSERT INTO transactions (account_id, booking_date, amount_cents, currency, counterparty_iban, kind, source)
             VALUES (?1, '2026-05-15', -10000, 'EUR', 'AT000000000000000011', 'transfer', 'sparkasse_csv')"
        ).bind(src).execute(&pool).await.unwrap();

        let paired = detect_inter_account_transfers(&pool).await.unwrap();
        assert_eq!(
            paired, 1,
            "kind=transfer + paired_tx_id NULL must be processed"
        );

        let (cnt,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM transactions WHERE source='auto_pair'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(cnt, 1);
    }

    #[tokio::test]
    async fn auto_pair_smart_dedup_doesnt_match_already_paired_tx() {
        // If an existing Tx on the target already has a paired_tx_id (belongs to
        // another pair), it must not be used for the new link — a mirror is
        // created instead.
        use crate::import_flow::detect_inter_account_transfers;
        let pool = connect_memory().await.unwrap();

        let src: i64 = sqlx::query_scalar(
            "INSERT INTO accounts (name, kind, currency, iban) VALUES ('Main', 'bank', 'EUR', 'AT000000000000000010') RETURNING id"
        ).fetch_one(&pool).await.unwrap();
        let tgt: i64 = sqlx::query_scalar(
            "INSERT INTO accounts (name, kind, currency, iban) VALUES ('Other', 'bank', 'EUR', 'AT000000000000000011') RETURNING id"
        ).fetch_one(&pool).await.unwrap();

        // Dummy Tx as FK target for paired_tx_id
        let dummy_id: i64 = sqlx::query_scalar(
            "INSERT INTO transactions (account_id, booking_date, amount_cents, currency, kind, source)
             VALUES (?1, '2026-01-01', 1, 'EUR', 'transfer', 'manual') RETURNING id"
        ).bind(tgt).fetch_one(&pool).await.unwrap();

        // Existing Tx on target is already assigned to ANOTHER pair
        sqlx::query(
            "INSERT INTO transactions (account_id, booking_date, amount_cents, currency, kind, source, paired_tx_id)
             VALUES (?1, '2026-05-15', 10000, 'EUR', 'transfer', 'tr_csv', ?2)"
        ).bind(tgt).bind(dummy_id).execute(&pool).await.unwrap();

        // New source Tx that "would match"
        sqlx::query(
            "INSERT INTO transactions (account_id, booking_date, amount_cents, currency, counterparty_iban, kind, source)
             VALUES (?1, '2026-05-15', -10000, 'EUR', 'AT000000000000000011', 'expense', 'sparkasse_csv')"
        ).bind(src).execute(&pool).await.unwrap();

        let paired = detect_inter_account_transfers(&pool).await.unwrap();
        assert_eq!(paired, 1);
        // Now 3 Tx on tgt: the already-paired one + new mirror
        let (auto_count,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM transactions WHERE source='auto_pair'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            auto_count, 1,
            "new mirror must be created because the existing Tx is already paired"
        );
    }

    #[tokio::test]
    async fn apply_rules_to_uncategorized_returns_correct_count() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool, "TR").await;
        let cat = seed_category(&pool, "Lebensmittel").await;
        insert_rule(
            &pool,
            &NewRule {
                priority: 100,
                name: "REWE".into(),
                combinator: Combinator::And,
                conditions: single_condition(
                    MatchField::Counterparty,
                    MatchOp::Contains("REWE".into()),
                ),
                target_category_id: cat,
                enabled: true,
            },
        )
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO transactions
                (account_id, booking_date, amount_cents, currency, counterparty, source, kind)
             VALUES
                (?1, '2026-05-01', -1000, 'EUR', 'REWE Berlin', 'manual', 'expense'),
                (?1, '2026-05-02', -1000, 'EUR', 'REWE Hamburg', 'manual', 'expense'),
                (?1, '2026-05-03', -1000, 'EUR', 'EDEKA', 'manual', 'expense')",
        )
        .bind(acc)
        .execute(&pool)
        .await
        .unwrap();

        let count = crate::import_flow::apply_rules_to_uncategorized(&pool)
            .await
            .unwrap();
        assert_eq!(
            count, 2,
            "two REWE tx should be categorized, EDEKA stays uncategorized"
        );

        let rows: Vec<(String, Option<i64>)> =
            sqlx::query_as("SELECT counterparty, category_id FROM transactions ORDER BY id")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert_eq!(rows[0], ("REWE Berlin".into(), Some(cat)));
        assert_eq!(rows[1], ("REWE Hamburg".into(), Some(cat)));
        assert_eq!(rows[2], ("EDEKA".into(), None));
    }

    #[tokio::test]
    async fn self_transfer_pairs_internal_bucket_moves() {
        use crate::import_flow::detect_inter_account_transfers;
        let pool = connect_memory().await.unwrap();

        let acc: i64 = sqlx::query_scalar(
            "INSERT INTO accounts (name, kind, currency, iban) VALUES ('Sparkonto', 'savings', 'EUR', 'AT000000000000000010') RETURNING id"
        ).fetch_one(&pool).await.unwrap();

        // Outgoing with self-IBAN
        let out_id: i64 = sqlx::query_scalar(
            "INSERT INTO transactions (account_id, booking_date, amount_cents, currency, counterparty_iban, kind, purpose, source)
             VALUES (?1, '2026-05-13', -500000, 'EUR', 'AT000000000000000010', 'expense', 'Umbuchung Studyfund', 'sparkasse_csv') RETURNING id"
        ).bind(acc).fetch_one(&pool).await.unwrap();

        // Incoming without IBAN, same day, same |amount|
        let in_id: i64 = sqlx::query_scalar(
            "INSERT INTO transactions (account_id, booking_date, amount_cents, currency, kind, purpose, source)
             VALUES (?1, '2026-05-13', 500000, 'EUR', 'income', 'von \"Studyfund\"', 'sparkasse_csv') RETURNING id"
        ).bind(acc).fetch_one(&pool).await.unwrap();

        let paired = detect_inter_account_transfers(&pool).await.unwrap();
        assert_eq!(paired, 1, "self-transfer counted as 1 pair");

        // Both are now kind=transfer, linked
        let (out_kind, out_paired): (String, Option<i64>) =
            sqlx::query_as("SELECT kind, paired_tx_id FROM transactions WHERE id=?1")
                .bind(out_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        let (in_kind, in_paired): (String, Option<i64>) =
            sqlx::query_as("SELECT kind, paired_tx_id FROM transactions WHERE id=?1")
                .bind(in_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(out_kind, "transfer");
        assert_eq!(in_kind, "transfer");
        assert_eq!(out_paired, Some(in_id));
        assert_eq!(in_paired, Some(out_id));

        // No mirror Tx created
        let (auto_count,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM transactions WHERE source='auto_pair'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(auto_count, 0, "no mirror needed for self-transfer");
    }

    #[tokio::test]
    async fn smart_dedup_matches_existing_within_3_day_window() {
        use crate::import_flow::detect_inter_account_transfers;
        let pool = connect_memory().await.unwrap();
        let src: i64 = sqlx::query_scalar(
            "INSERT INTO accounts (name, kind, currency, iban) VALUES ('Main', 'bank', 'EUR', 'AT000000000000000010') RETURNING id"
        ).fetch_one(&pool).await.unwrap();
        let tgt: i64 = sqlx::query_scalar(
            "INSERT INTO accounts (name, kind, currency, iban) VALUES ('Cash', 'savings', 'EUR', 'AT000000000000000011') RETURNING id"
        ).fetch_one(&pool).await.unwrap();

        // Source outgoing on day X
        let src_id: i64 = sqlx::query_scalar(
            "INSERT INTO transactions (account_id, booking_date, amount_cents, currency, counterparty_iban, kind, source)
             VALUES (?1, '2026-05-15', -10000, 'EUR', 'AT000000000000000011', 'expense', 'sparkasse_csv') RETURNING id"
        ).bind(src).fetch_one(&pool).await.unwrap();
        // Target incoming on day X+2 (bank settlement lag)
        let tgt_id: i64 = sqlx::query_scalar(
            "INSERT INTO transactions (account_id, booking_date, amount_cents, currency, kind, source)
             VALUES (?1, '2026-05-17', 10000, 'EUR', 'income', 'tr_csv') RETURNING id"
        ).bind(tgt).fetch_one(&pool).await.unwrap();

        let paired = detect_inter_account_transfers(&pool).await.unwrap();
        assert_eq!(
            paired, 1,
            "should link via fuzzy 3-day window, not create duplicate mirror"
        );
        let (cnt,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM transactions")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(
            cnt, 2,
            "no mirror should be created since fuzzy match found"
        );

        // Both linked
        let (s_paired,): (Option<i64>,) =
            sqlx::query_as("SELECT paired_tx_id FROM transactions WHERE id=?1")
                .bind(src_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(s_paired, Some(tgt_id));
    }

    #[tokio::test]
    async fn self_transfer_marks_single_sided_when_no_partner() {
        use crate::import_flow::detect_inter_account_transfers;
        let pool = connect_memory().await.unwrap();
        let acc: i64 = sqlx::query_scalar(
            "INSERT INTO accounts (name, kind, currency, iban) VALUES ('S', 'savings', 'EUR', 'AT000000000000000010') RETURNING id"
        ).fetch_one(&pool).await.unwrap();

        // Outgoing only, no incoming
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO transactions (account_id, booking_date, amount_cents, currency, counterparty_iban, kind, source)
             VALUES (?1, '2026-05-13', -100, 'EUR', 'AT000000000000000010', 'expense', 'sparkasse_csv') RETURNING id"
        ).bind(acc).fetch_one(&pool).await.unwrap();

        detect_inter_account_transfers(&pool).await.unwrap();
        let (kind,): (String,) = sqlx::query_as("SELECT kind FROM transactions WHERE id=?1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(
            kind, "transfer",
            "self-transfer without partner still marked as transfer"
        );
    }
}
