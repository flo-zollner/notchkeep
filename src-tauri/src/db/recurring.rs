use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use super::{DbError, DbResult};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecurringPayment {
    pub id: i64,
    pub name: String,
    pub account_id: i64,
    pub category_id: Option<i64>,
    pub amount_cents: i64,
    pub frequency: String,
    pub anchor_date: String,
    pub counterparty: Option<String>,
    pub note: Option<String>,
    pub archived: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewRecurringPayload {
    pub name: String,
    pub account_id: i64,
    pub category_id: Option<i64>,
    pub amount_cents: i64,
    pub frequency: String,
    pub anchor_date: String,
    pub counterparty: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRecurringPayload {
    pub name: Option<String>,
    pub category_id: Option<Option<i64>>,
    pub amount_cents: Option<i64>,
    pub frequency: Option<String>,
    pub anchor_date: Option<String>,
    pub counterparty: Option<Option<String>>,
    pub note: Option<Option<String>>,
    pub archived: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Occurrence {
    pub due_date: String,
    pub status: String,
    pub matched_tx_id: Option<i64>,
    pub matched_amount_cents: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecurringOverview {
    pub recurring: RecurringPayment,
    pub occurrences: Vec<Occurrence>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedRecurring {
    pub counterparty: String,
    pub account_id: i64,
    pub amount_cents: i64,
    pub frequency: String,
    pub anchor_date: String,
    pub sample_count: usize,
}

const ALLOWED_FREQUENCIES: &[&str] = &["weekly", "monthly", "quarterly", "yearly"];

const COLS: &str = "id, name, account_id, category_id, amount_cents, frequency, \
                    anchor_date, counterparty, note, archived, created_at";

pub async fn create_recurring(
    pool: &SqlitePool,
    p: NewRecurringPayload,
) -> DbResult<RecurringPayment> {
    if p.name.trim().is_empty() {
        return Err(DbError::Decode("name must not be empty".into()));
    }
    if p.amount_cents == 0 {
        return Err(DbError::Decode("amount_cents must not be 0".into()));
    }
    if !ALLOWED_FREQUENCIES.contains(&p.frequency.as_str()) {
        return Err(DbError::Decode(format!(
            "frequency must be one of {ALLOWED_FREQUENCIES:?}, got {:?}",
            p.frequency,
        )));
    }

    let sql = format!(
        "INSERT INTO recurring_payments
            (name, account_id, category_id, amount_cents, frequency,
             anchor_date, counterparty, note)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         RETURNING {COLS}"
    );
    let row: RecurringPayment = sqlx::query_as(&sql)
        .bind(p.name.trim())
        .bind(p.account_id)
        .bind(p.category_id)
        .bind(p.amount_cents)
        .bind(&p.frequency)
        .bind(&p.anchor_date)
        .bind(p.counterparty.as_deref())
        .bind(p.note.as_deref())
        .fetch_one(pool)
        .await?;
    Ok(row)
}

pub async fn get_recurring(pool: &SqlitePool, id: i64) -> DbResult<RecurringPayment> {
    let sql = format!("SELECT {COLS} FROM recurring_payments WHERE id = ?1");
    Ok(sqlx::query_as(&sql).bind(id).fetch_one(pool).await?)
}

pub async fn list_recurring(
    pool: &SqlitePool,
    include_archived: bool,
) -> DbResult<Vec<RecurringPayment>> {
    let sql = if include_archived {
        format!("SELECT {COLS} FROM recurring_payments ORDER BY archived ASC, name COLLATE NOCASE ASC")
    } else {
        format!("SELECT {COLS} FROM recurring_payments WHERE archived = 0 ORDER BY name COLLATE NOCASE ASC")
    };
    Ok(sqlx::query_as(&sql).fetch_all(pool).await?)
}

pub async fn update_recurring(
    pool: &SqlitePool,
    id: i64,
    p: UpdateRecurringPayload,
) -> DbResult<RecurringPayment> {
    let current = get_recurring(pool, id).await?;
    let name = match p.name {
        Some(s) if s.trim().is_empty() => {
            return Err(DbError::Decode("name must not be empty".into()));
        }
        Some(s) => s.trim().to_string(),
        None => current.name,
    };
    let category_id = p.category_id.unwrap_or(current.category_id);
    let amount_cents = match p.amount_cents {
        Some(a) if a == 0 => {
            return Err(DbError::Decode("amount_cents must not be 0".into()));
        }
        Some(a) => a,
        None => current.amount_cents,
    };
    let frequency = match p.frequency {
        Some(f) => {
            if !ALLOWED_FREQUENCIES.contains(&f.as_str()) {
                return Err(DbError::Decode(format!(
                    "frequency must be one of {ALLOWED_FREQUENCIES:?}, got {f:?}"
                )));
            }
            f
        }
        None => current.frequency,
    };
    let anchor_date = p.anchor_date.unwrap_or(current.anchor_date);
    let counterparty = p.counterparty.unwrap_or(current.counterparty);
    let note = p.note.unwrap_or(current.note);
    let archived = p.archived.unwrap_or(current.archived);

    let sql = format!(
        "UPDATE recurring_payments SET
            name = ?1, category_id = ?2, amount_cents = ?3, frequency = ?4,
            anchor_date = ?5, counterparty = ?6, note = ?7, archived = ?8
         WHERE id = ?9
         RETURNING {COLS}"
    );
    let row: RecurringPayment = sqlx::query_as(&sql)
        .bind(&name)
        .bind(category_id)
        .bind(amount_cents)
        .bind(&frequency)
        .bind(&anchor_date)
        .bind(counterparty.as_deref())
        .bind(note.as_deref())
        .bind(archived)
        .bind(id)
        .fetch_one(pool)
        .await?;
    Ok(row)
}

pub async fn delete_recurring(pool: &SqlitePool, id: i64) -> DbResult<bool> {
    let res = sqlx::query("DELETE FROM recurring_payments WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}

/// Normalisiert Counterparty-Strings für Fuzzy-Match.
/// - lowercase (ASCII)
/// - alphanumerische + Whitespace bewahren, alles andere entfernen
/// - Whitespace kollabieren + trimmen
pub fn normalize_counterparty(s: &str) -> String {
    s.chars()
        .filter_map(|c| {
            if c.is_alphanumeric() {
                Some(c.to_ascii_lowercase())
            } else if c.is_whitespace() {
                Some(' ')
            } else {
                None
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Liefert die Approximation in Tagen für eine Frequency. Wird für
/// Auto-Detect (intervall-clustering) und für `project_due_dates` genutzt.
/// `None` bei unbekannter frequency.
pub fn period_days(freq: &str) -> Option<i64> {
    match freq {
        "weekly" => Some(7),
        "monthly" => Some(30),
        "quarterly" => Some(91),
        "yearly" => Some(365),
        _ => None,
    }
}

/// Generiert alle Due-Dates für eine Recurring im Fenster `[today − 5d, cutoff]`.
/// Die untere Grenze ist `today − 5` damit Fälligkeiten, die heute noch im
/// Match-Fenster liegen, mit angezeigt werden („heute fällig").
///
/// Iteration: `anchor + N × period_days` für N = 0, 1, 2, ... bis cutoff erreicht.
pub fn project_due_dates(
    anchor: chrono::NaiveDate,
    frequency: &str,
    today: chrono::NaiveDate,
    cutoff: chrono::NaiveDate,
) -> Vec<chrono::NaiveDate> {
    let Some(days) = period_days(frequency) else {
        return Vec::new();
    };
    let lower_bound = today - chrono::Duration::days(5);
    let mut out: Vec<chrono::NaiveDate> = Vec::new();
    for n in 0_i64.. {
        let due = anchor + chrono::Duration::days(days * n);
        if due > cutoff {
            break;
        }
        if due >= lower_bound {
            out.push(due);
        }
        if n > 10_000 {
            break;
        }
    }
    out
}

/// Tx-Match-Heuristik (pure):
/// - normalize(tx_counterparty) contains normalize(rec_counterparty)
/// - |tx_amount - rec_amount| ≤ |rec_amount| × 10%
/// - |tx_date - due_date| ≤ 5 Tage
pub fn tx_matches_recurring(
    tx_counterparty: &str,
    tx_amount_cents: i64,
    tx_date: chrono::NaiveDate,
    rec_counterparty: &str,
    rec_amount_cents: i64,
    due_date: chrono::NaiveDate,
) -> bool {
    if rec_counterparty.trim().is_empty() {
        return false;
    }
    let tx_norm = normalize_counterparty(tx_counterparty);
    let rec_norm = normalize_counterparty(rec_counterparty);
    if rec_norm.is_empty() || (!tx_norm.contains(&rec_norm) && !rec_norm.contains(&tx_norm)) {
        return false;
    }

    let tolerance = (rec_amount_cents.abs() as i128 / 10) as i64;
    if (tx_amount_cents - rec_amount_cents).abs() > tolerance {
        return false;
    }

    let diff_days = (tx_date - due_date).num_days().abs();
    if diff_days > 5 {
        return false;
    }
    true
}

/// Lädt alle non-archivierten Recurring-Definitionen und projiziert
/// Fälligkeiten in den nächsten `months_ahead` Monaten. Pro Fälligkeit wird
/// die Tx-Match-Heuristik gegen die Transactions-Tabelle gefahren.
///
/// Tx werden einmalig pro Range geladen (today-10..cutoff+10), dann linearer
/// Match-Loop in Rust.
pub async fn recurring_overview(
    pool: &SqlitePool,
    months_ahead: u32,
) -> DbResult<Vec<RecurringOverview>> {
    use chrono::NaiveDate;

    if months_ahead < 1 || months_ahead > 24 {
        return Err(DbError::Decode(format!(
            "months_ahead must be in 1..=24, got {months_ahead}"
        )));
    }

    let recs = list_recurring(pool, false).await?;
    if recs.is_empty() {
        return Ok(Vec::new());
    }

    let today = chrono::Utc::now().date_naive();
    let cutoff = today + chrono::Duration::days((months_ahead as i64) * 30);

    let tx_lower = (today - chrono::Duration::days(10)).format("%Y-%m-%d").to_string();
    let tx_upper = (cutoff + chrono::Duration::days(10)).format("%Y-%m-%d").to_string();

    #[derive(sqlx::FromRow)]
    struct TxRow {
        id: i64,
        account_id: i64,
        booking_date: String,
        amount_cents: i64,
        counterparty: Option<String>,
    }

    let txs: Vec<TxRow> = sqlx::query_as(
        "SELECT id, account_id, booking_date, amount_cents, counterparty
           FROM transactions
          WHERE booking_date >= ?1 AND booking_date <= ?2"
    )
    .bind(&tx_lower)
    .bind(&tx_upper)
    .fetch_all(pool)
    .await?;

    let mut out: Vec<RecurringOverview> = Vec::new();
    for rec in recs {
        let anchor = match NaiveDate::parse_from_str(&rec.anchor_date, "%Y-%m-%d") {
            Ok(d) => d,
            Err(_) => continue,
        };
        let dues = project_due_dates(anchor, &rec.frequency, today, cutoff);

        let mut occurrences: Vec<Occurrence> = Vec::with_capacity(dues.len());
        for due in dues {
            // Suche besten Tx-Match: gleiches account, contains-Counterparty,
            // ±10% Amount, ±5 Tage. Sortiere matches nach |date-diff| ASC und
            // nimm den nächsten.
            let mut best: Option<(i64, i64, i64)> = None;
            for tx in &txs {
                if tx.account_id != rec.account_id {
                    continue;
                }
                let tx_date = match NaiveDate::parse_from_str(&tx.booking_date, "%Y-%m-%d") {
                    Ok(d) => d,
                    Err(_) => continue,
                };
                let tx_cp = tx.counterparty.as_deref().unwrap_or("");
                let rec_cp = rec.counterparty.as_deref().unwrap_or("");
                if tx_matches_recurring(tx_cp, tx.amount_cents, tx_date,
                                         rec_cp, rec.amount_cents, due) {
                    let diff = (tx_date - due).num_days().abs();
                    if best.map(|(_, _, d)| diff < d).unwrap_or(true) {
                        best = Some((tx.id, tx.amount_cents, diff));
                    }
                }
            }
            let (status, matched_tx_id, matched_amount_cents) = match best {
                Some((id, amt, _)) => ("paid".to_string(), Some(id), Some(amt)),
                None => ("pending".to_string(), None, None),
            };
            occurrences.push(Occurrence {
                due_date: due.format("%Y-%m-%d").to_string(),
                status,
                matched_tx_id,
                matched_amount_cents,
            });
        }

        out.push(RecurringOverview { recurring: rec, occurrences });
    }

    // Sortiere: erste pending Due-Date ASC, dann Name.
    out.sort_by(|a, b| {
        let a_first_pending = a.occurrences.iter()
            .find(|o| o.status == "pending")
            .map(|o| o.due_date.clone());
        let b_first_pending = b.occurrences.iter()
            .find(|o| o.status == "pending")
            .map(|o| o.due_date.clone());
        match (a_first_pending, b_first_pending) {
            (Some(a), Some(b)) => a.cmp(&b),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        }.then_with(|| a.recurring.name.cmp(&b.recurring.name))
    });

    Ok(out)
}

/// Auto-Detect aus den letzten 12 Monaten Tx-Historie.
///
/// 1. SQL: lade alle Tx mit non-empty counterparty der letzten 12 Monate.
/// 2. Group by (account_id, normalize(counterparty)).
/// 3. Für Cluster mit ≥ 3 Tx:
///    - Sortiere nach booking_date ASC, berechne consecutive day-Δ.
///    - Median + StdDev. Variationskoeffizient σ/μ. Wenn > 0.25 → skip.
///    - Frequency = nearest match in {7,30,91,365}-Set.
///    - Median amount.
///    - Anchor = letzte Tx-Datum.
///    - Counterparty = original (unnormalized) der letzten Tx.
/// 4. Pre-Filter: matches eine existierende recurring? → drop.
pub async fn detect_recurring(pool: &SqlitePool) -> DbResult<Vec<DetectedRecurring>> {
    use chrono::NaiveDate;
    use std::collections::HashMap;

    let cutoff = (chrono::Utc::now().date_naive() - chrono::Duration::days(365))
        .format("%Y-%m-%d").to_string();

    #[derive(sqlx::FromRow)]
    struct Row {
        account_id: i64,
        booking_date: String,
        amount_cents: i64,
        counterparty: String,
    }

    let rows: Vec<Row> = sqlx::query_as(
        "SELECT account_id, booking_date, amount_cents, counterparty
           FROM transactions
          WHERE booking_date >= ?1
            AND counterparty IS NOT NULL
            AND counterparty != ''
            AND amount_cents != 0
          ORDER BY booking_date ASC"
    )
    .bind(&cutoff)
    .fetch_all(pool)
    .await?;

    // Group by (account_id, normalized_counterparty).
    let mut clusters: HashMap<(i64, String), Vec<Row>> = HashMap::new();
    for row in rows {
        let norm = normalize_counterparty(&row.counterparty);
        if norm.is_empty() {
            continue;
        }
        clusters.entry((row.account_id, norm)).or_default().push(row);
    }

    let existing = list_recurring(pool, true).await?;

    let mut out: Vec<DetectedRecurring> = Vec::new();
    for ((account_id, _norm), mut txs) in clusters {
        if txs.len() < 3 {
            continue;
        }
        txs.sort_by(|a, b| a.booking_date.cmp(&b.booking_date));

        let mut parsed_dates: Vec<NaiveDate> = Vec::with_capacity(txs.len());
        for tx in &txs {
            match NaiveDate::parse_from_str(&tx.booking_date, "%Y-%m-%d") {
                Ok(d) => parsed_dates.push(d),
                Err(_) => continue,
            }
        }
        if parsed_dates.len() < 3 {
            continue;
        }
        let mut deltas: Vec<i64> = Vec::with_capacity(parsed_dates.len() - 1);
        for win in parsed_dates.windows(2) {
            deltas.push((win[1] - win[0]).num_days());
        }

        let median = {
            let mut sorted = deltas.clone();
            sorted.sort();
            sorted[sorted.len() / 2]
        };
        let mean = (deltas.iter().sum::<i64>() as f64) / (deltas.len() as f64);
        let variance: f64 = deltas.iter()
            .map(|d| (*d as f64 - mean).powi(2))
            .sum::<f64>() / (deltas.len() as f64);
        let std_dev = variance.sqrt();
        let cv = if mean > 0.0 { std_dev / mean } else { 1.0 };
        if cv > 0.25 {
            continue;
        }

        let candidates: [(i64, &str); 4] = [
            (7, "weekly"), (30, "monthly"), (91, "quarterly"), (365, "yearly")
        ];
        let (_, frequency) = candidates.iter()
            .min_by_key(|(d, _)| (median - d).abs())
            .copied()
            .unwrap();

        let mut amounts: Vec<i64> = txs.iter().map(|t| t.amount_cents).collect();
        amounts.sort();
        let median_amount = amounts[amounts.len() / 2];

        let last_counterparty = txs.last().unwrap().counterparty.clone();
        let anchor_date = txs.last().unwrap().booking_date.clone();

        // Pre-Filter: existing recurring covers this cluster?
        let covered = existing.iter().any(|r| {
            r.account_id == account_id
                && r.counterparty.as_deref().map(|c| {
                    let r_norm = normalize_counterparty(c);
                    let t_norm = normalize_counterparty(&last_counterparty);
                    !r_norm.is_empty() && (r_norm.contains(&t_norm) || t_norm.contains(&r_norm))
                }).unwrap_or(false)
                && (r.amount_cents - median_amount).abs() <= (median_amount.abs() / 10)
        });
        if covered {
            continue;
        }

        out.push(DetectedRecurring {
            counterparty: last_counterparty,
            account_id,
            amount_cents: median_amount,
            frequency: frequency.to_string(),
            anchor_date,
            sample_count: txs.len(),
        });
    }

    out.sort_by(|a, b| b.sample_count.cmp(&a.sample_count));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_memory;

    async fn seed_account(pool: &SqlitePool) -> i64 {
        let (id,): (i64,) = sqlx::query_as(
            "INSERT INTO accounts (name, kind, currency)
             VALUES ('Giro', 'bank', 'EUR') RETURNING id",
        )
        .fetch_one(pool)
        .await
        .unwrap();
        id
    }

    fn sample_payload(account_id: i64) -> NewRecurringPayload {
        NewRecurringPayload {
            name: "Miete".into(),
            account_id,
            category_id: None,
            amount_cents: -100_000,
            frequency: "monthly".into(),
            anchor_date: "2026-06-01".into(),
            counterparty: Some("Vermieter".into()),
            note: None,
        }
    }

    #[tokio::test]
    async fn create_and_get_recurring() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool).await;

        let created = create_recurring(&pool, sample_payload(acc)).await.unwrap();
        assert_eq!(created.name, "Miete");
        assert_eq!(created.amount_cents, -100_000);
        assert_eq!(created.frequency, "monthly");
        assert!(!created.archived);

        let fetched = get_recurring(&pool, created.id).await.unwrap();
        assert_eq!(fetched.id, created.id);
        assert_eq!(fetched.counterparty.as_deref(), Some("Vermieter"));
    }

    #[tokio::test]
    async fn list_filters_archived_by_default() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool).await;

        let r1 = create_recurring(&pool, sample_payload(acc)).await.unwrap();
        let r2 = create_recurring(
            &pool,
            NewRecurringPayload {
                name: "Netflix".into(),
                ..sample_payload(acc)
            },
        )
        .await
        .unwrap();

        update_recurring(
            &pool,
            r1.id,
            UpdateRecurringPayload {
                archived: Some(true),
                name: None,
                category_id: None,
                amount_cents: None,
                frequency: None,
                anchor_date: None,
                counterparty: None,
                note: None,
            },
        )
        .await
        .unwrap();

        let visible = list_recurring(&pool, false).await.unwrap();
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].id, r2.id);

        let all = list_recurring(&pool, true).await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn update_partial_keeps_other_fields() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool).await;
        let r = create_recurring(&pool, sample_payload(acc)).await.unwrap();

        let updated = update_recurring(
            &pool,
            r.id,
            UpdateRecurringPayload {
                amount_cents: Some(-120_000),
                name: None,
                category_id: None,
                frequency: None,
                anchor_date: None,
                counterparty: None,
                note: None,
                archived: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(updated.amount_cents, -120_000);
        assert_eq!(updated.name, "Miete");
        assert_eq!(updated.anchor_date, "2026-06-01");
        assert_eq!(updated.frequency, "monthly");
    }

    #[tokio::test]
    async fn delete_removes_and_returns_bool() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool).await;
        let r = create_recurring(&pool, sample_payload(acc)).await.unwrap();

        assert!(delete_recurring(&pool, r.id).await.unwrap());
        assert!(get_recurring(&pool, r.id).await.is_err());
        assert!(!delete_recurring(&pool, r.id).await.unwrap());
    }

    #[tokio::test]
    async fn create_rejects_invalid_input() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool).await;

        let mut p = sample_payload(acc);
        p.name = "   ".into();
        assert!(create_recurring(&pool, p).await.is_err());

        let mut p = sample_payload(acc);
        p.amount_cents = 0;
        assert!(create_recurring(&pool, p).await.is_err());

        let mut p = sample_payload(acc);
        p.frequency = "fortnightly".into();
        assert!(create_recurring(&pool, p).await.is_err());
    }

    #[test]
    fn normalize_counterparty_strips_punct_and_lowercases() {
        assert_eq!(normalize_counterparty("Netflix Inc."), "netflix inc");
        assert_eq!(normalize_counterparty("AMAZON.DE"), "amazonde");
        assert_eq!(normalize_counterparty("  REWE SAGT DANKE!  "), "rewe sagt danke");
        assert_eq!(normalize_counterparty(""), "");
    }

    #[test]
    fn period_days_for_each_frequency() {
        assert_eq!(period_days("weekly"), Some(7));
        assert_eq!(period_days("monthly"), Some(30));
        assert_eq!(period_days("quarterly"), Some(91));
        assert_eq!(period_days("yearly"), Some(365));
        assert_eq!(period_days("unknown"), None);
    }

    #[test]
    fn project_due_dates_within_window() {
        use chrono::NaiveDate;
        let anchor = NaiveDate::parse_from_str("2026-01-15", "%Y-%m-%d").unwrap();
        let today = NaiveDate::parse_from_str("2026-05-01", "%Y-%m-%d").unwrap();
        let cutoff = NaiveDate::parse_from_str("2026-07-31", "%Y-%m-%d").unwrap();

        let dates = project_due_dates(anchor, "monthly", today, cutoff);
        // Anchor 2026-01-15 + 4*30 = 2026-05-15 (in window today-5..cutoff)
        // Anchor 2026-01-15 + 5*30 = 2026-06-14
        // Anchor 2026-01-15 + 6*30 = 2026-07-14
        // Anchor 2026-01-15 + 7*30 = 2026-08-13 > cutoff, raus.
        assert_eq!(dates.len(), 3);
        assert_eq!(dates[0].format("%Y-%m-%d").to_string(), "2026-05-15");
        assert_eq!(dates[1].format("%Y-%m-%d").to_string(), "2026-06-14");
        assert_eq!(dates[2].format("%Y-%m-%d").to_string(), "2026-07-14");
    }

    #[test]
    fn tx_matches_recurring_basic() {
        use chrono::NaiveDate;
        let due = NaiveDate::parse_from_str("2026-06-01", "%Y-%m-%d").unwrap();

        // Tx: gleicher Counterparty, gleicher Betrag, due-Tag → match
        let tx_date = NaiveDate::parse_from_str("2026-06-01", "%Y-%m-%d").unwrap();
        assert!(tx_matches_recurring("Vermieter GmbH", -100_000, tx_date, "Vermieter", -100_000, due));

        // Counterparty case-insensitive contains
        assert!(tx_matches_recurring("vermieter", -100_000, tx_date, "Vermieter GmbH", -100_000, due));

        // Betrag ±10% — 100_000 ± 10_000
        assert!(tx_matches_recurring("Vermieter", -109_999, tx_date, "Vermieter", -100_000, due));
        assert!(!tx_matches_recurring("Vermieter", -111_000, tx_date, "Vermieter", -100_000, due));

        // Datum ±5 Tage
        let tx_5 = NaiveDate::parse_from_str("2026-06-06", "%Y-%m-%d").unwrap();
        assert!(tx_matches_recurring("Vermieter", -100_000, tx_5, "Vermieter", -100_000, due));
        let tx_6 = NaiveDate::parse_from_str("2026-06-07", "%Y-%m-%d").unwrap();
        assert!(!tx_matches_recurring("Vermieter", -100_000, tx_6, "Vermieter", -100_000, due));

        // Counterparty mismatch
        assert!(!tx_matches_recurring("Andere", -100_000, tx_date, "Vermieter", -100_000, due));
    }

    async fn seed_tx(
        pool: &SqlitePool, account_id: i64, date: &str,
        amount_cents: i64, counterparty: &str,
    ) -> i64 {
        let (id,): (i64,) = sqlx::query_as(
            "INSERT INTO transactions
                (account_id, booking_date, amount_cents, currency,
                 counterparty, source, kind, imported_at)
             VALUES (?1, ?2, ?3, 'EUR', ?4, 'manual', 'expense', '2026-05-20T00:00:00Z')
             RETURNING id",
        )
        .bind(account_id).bind(date).bind(amount_cents).bind(counterparty)
        .fetch_one(pool).await.unwrap();
        id
    }

    #[tokio::test]
    async fn recurring_overview_empty_returns_empty() {
        let pool = connect_memory().await.unwrap();
        let result = recurring_overview(&pool, 3).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn recurring_overview_marks_paid_when_tx_matches() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool).await;
        let today = chrono::Utc::now().date_naive();
        let anchor = (today - chrono::Duration::days(28)).format("%Y-%m-%d").to_string();

        create_recurring(&pool, NewRecurringPayload {
            name: "Miete".into(), account_id: acc, category_id: None,
            amount_cents: -100_000, frequency: "monthly".into(),
            anchor_date: anchor.clone(), counterparty: Some("Vermieter".into()),
            note: None,
        }).await.unwrap();

        let due = (today + chrono::Duration::days(2)).format("%Y-%m-%d").to_string();
        seed_tx(&pool, acc, &due, -100_000, "Vermieter GmbH").await;

        let overview = recurring_overview(&pool, 3).await.unwrap();
        assert_eq!(overview.len(), 1);
        let occ = &overview[0].occurrences[0];
        assert_eq!(occ.due_date, due);
        assert_eq!(occ.status, "paid");
        assert!(occ.matched_tx_id.is_some());
    }

    #[tokio::test]
    async fn recurring_overview_marks_pending_when_no_match() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool).await;
        let today = chrono::Utc::now().date_naive();
        let anchor = (today - chrono::Duration::days(28)).format("%Y-%m-%d").to_string();

        create_recurring(&pool, NewRecurringPayload {
            name: "Miete".into(), account_id: acc, category_id: None,
            amount_cents: -100_000, frequency: "monthly".into(),
            anchor_date: anchor, counterparty: Some("Vermieter".into()),
            note: None,
        }).await.unwrap();

        let due = (today + chrono::Duration::days(2)).format("%Y-%m-%d").to_string();
        seed_tx(&pool, acc, &due, -100_000, "Supermarkt").await;

        let overview = recurring_overview(&pool, 3).await.unwrap();
        let occ = &overview[0].occurrences[0];
        assert_eq!(occ.status, "pending");
        assert!(occ.matched_tx_id.is_none());
    }

    #[tokio::test]
    async fn recurring_overview_rejects_invalid_months_ahead() {
        let pool = connect_memory().await.unwrap();
        assert!(recurring_overview(&pool, 0).await.is_err());
        assert!(recurring_overview(&pool, 25).await.is_err());
        assert!(recurring_overview(&pool, 1).await.is_ok());
        assert!(recurring_overview(&pool, 24).await.is_ok());
    }

    #[tokio::test]
    async fn detect_empty_db_returns_empty() {
        let pool = connect_memory().await.unwrap();
        let result = detect_recurring(&pool).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn detect_finds_stable_monthly_cluster() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool).await;
        // Use recent dates so cutoff (365 days ago) includes them
        let today = chrono::Utc::now().date_naive();
        let dates: Vec<String> = (0..5)
            .map(|i| (today - chrono::Duration::days(30 * (4 - i) as i64))
                .format("%Y-%m-%d").to_string())
            .collect();
        for d in &dates {
            seed_tx(&pool, acc, d, -1_299, "Netflix Inc.").await;
        }

        let detected = detect_recurring(&pool).await.unwrap();
        assert_eq!(detected.len(), 1);
        let d = &detected[0];
        assert_eq!(d.account_id, acc);
        assert_eq!(d.frequency, "monthly");
        assert_eq!(d.amount_cents, -1_299);
        assert_eq!(d.counterparty, "Netflix Inc.");
        assert_eq!(d.sample_count, 5);
    }

    #[tokio::test]
    async fn detect_skips_irregular_clusters() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool).await;
        // Use very irregular intervals so CV > 0.25
        let today = chrono::Utc::now().date_naive();
        let day_offsets: Vec<i64> = vec![-300, -250, -50, -10];
        for off in &day_offsets {
            let d = (today + chrono::Duration::days(*off)).format("%Y-%m-%d").to_string();
            seed_tx(&pool, acc, &d, -1_000, "Sporadisch").await;
        }

        let detected = detect_recurring(&pool).await.unwrap();
        assert!(detected.is_empty(), "irregular cluster should be skipped");
    }

    #[tokio::test]
    async fn detect_drops_clusters_covered_by_existing_recurring() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool).await;
        let today = chrono::Utc::now().date_naive();

        create_recurring(&pool, NewRecurringPayload {
            name: "Netflix".into(), account_id: acc, category_id: None,
            amount_cents: -1_299, frequency: "monthly".into(),
            anchor_date: (today - chrono::Duration::days(30)).format("%Y-%m-%d").to_string(),
            counterparty: Some("Netflix".into()), note: None,
        }).await.unwrap();

        for i in 0..3 {
            let d = (today - chrono::Duration::days(30 * (2 - i) as i64))
                .format("%Y-%m-%d").to_string();
            seed_tx(&pool, acc, &d, -1_299, "Netflix Inc.").await;
        }

        let detected = detect_recurring(&pool).await.unwrap();
        assert!(detected.is_empty(), "cluster covered by existing recurring should be dropped");
    }
}
