use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use super::DbResult;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportFilter {
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    pub account_id: Option<i64>,
    pub category_id: Option<i64>,
    pub search: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow, Serialize)]
pub struct ExportRow {
    pub id: i64,
    pub booking_date: String,
    pub amount_cents: i64,
    pub currency: String,
    pub account_name: String,
    pub account_id: i64,
    pub category_name: Option<String>,
    pub category_id: Option<i64>,
    pub counterparty: Option<String>,
    pub purpose: Option<String>,
    pub manual_note: Option<String>,
    pub source: String,
    pub source_file_hash: Option<String>,
    pub raw_ref: Option<String>,
}

pub async fn fetch_export_rows(
    pool: &SqlitePool,
    filter: &ExportFilter,
) -> DbResult<Vec<ExportRow>> {
    let from = filter.from.map(|d| d.format("%Y-%m-%d").to_string());
    let to = filter.to.map(|d| d.format("%Y-%m-%d").to_string());
    let search_like = filter.search.as_ref().and_then(|s| {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(format!("%{trimmed}%"))
        }
    });

    let rows: Vec<ExportRow> = sqlx::query_as(
        "SELECT t.id                AS id,
                t.booking_date      AS booking_date,
                t.amount_cents      AS amount_cents,
                t.currency          AS currency,
                a.name              AS account_name,
                t.account_id        AS account_id,
                c.name              AS category_name,
                t.category_id       AS category_id,
                t.counterparty      AS counterparty,
                t.purpose           AS purpose,
                t.manual_note       AS manual_note,
                t.source            AS source,
                t.source_file_hash  AS source_file_hash,
                t.raw_ref           AS raw_ref
         FROM transactions t
         JOIN accounts a ON a.id = t.account_id
         LEFT JOIN categories c ON c.id = t.category_id
         WHERE (?1 IS NULL OR t.booking_date >= ?1)
           AND (?2 IS NULL OR t.booking_date <= ?2)
           AND (?3 IS NULL OR t.account_id = ?3)
           AND (?4 IS NULL OR t.category_id = ?4)
           AND (?5 IS NULL OR t.counterparty LIKE ?5
                           OR t.purpose      LIKE ?5
                           OR t.manual_note  LIKE ?5)
         ORDER BY t.booking_date ASC, t.id ASC",
    )
    .bind(&from)
    .bind(&to)
    .bind(filter.account_id)
    .bind(filter.category_id)
    .bind(&search_like)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub fn write_export_csv<W: std::io::Write>(
    rows: &[ExportRow],
    writer: W,
) -> std::io::Result<()> {
    let mut wtr = csv::WriterBuilder::new()
        .delimiter(b',')
        .quote_style(csv::QuoteStyle::Necessary)
        .terminator(csv::Terminator::CRLF)
        .from_writer(writer);

    wtr.write_record([
        "id",
        "date",
        "amount",
        "currency",
        "account",
        "account_id",
        "category",
        "category_id",
        "counterparty",
        "purpose",
        "note",
        "source",
        "source_file_hash",
        "raw_ref",
    ])?;

    for r in rows {
        wtr.write_record([
            r.id.to_string(),
            r.booking_date.clone(),
            format_amount(r.amount_cents),
            r.currency.clone(),
            r.account_name.clone(),
            r.account_id.to_string(),
            r.category_name.clone().unwrap_or_default(),
            r.category_id.map(|v| v.to_string()).unwrap_or_default(),
            r.counterparty.clone().unwrap_or_default(),
            r.purpose.clone().unwrap_or_default(),
            r.manual_note.clone().unwrap_or_default(),
            r.source.clone(),
            r.source_file_hash.clone().unwrap_or_default(),
            r.raw_ref.clone().unwrap_or_default(),
        ])?;
    }

    wtr.flush()?;
    Ok(())
}

fn format_amount(cents: i64) -> String {
    let abs = cents.unsigned_abs();
    let euro = abs / 100;
    let rest = abs % 100;
    let sign = if cents < 0 { "-" } else { "" };
    format!("{sign}{euro}.{rest:02}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_memory;
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

    #[allow(clippy::too_many_arguments)]
    async fn seed_tx(
        pool: &SqlitePool,
        account_id: i64,
        date: &str,
        amount_cents: i64,
        category_id: Option<i64>,
        counterparty: Option<&str>,
        purpose: Option<&str>,
        manual_note: Option<&str>,
    ) -> i64 {
        let (id,): (i64,) = sqlx::query_as(
            "INSERT INTO transactions
                (account_id, booking_date, amount_cents, currency,
                 counterparty, purpose, raw_ref, category_id, source,
                 source_file_hash, manual_note)
             VALUES (?1, ?2, ?3, 'EUR', ?4, ?5, NULL, ?6, 'manual', NULL, ?7)
             RETURNING id",
        )
        .bind(account_id)
        .bind(date)
        .bind(amount_cents)
        .bind(counterparty)
        .bind(purpose)
        .bind(category_id)
        .bind(manual_note)
        .fetch_one(pool)
        .await
        .unwrap();
        id
    }

    #[tokio::test]
    async fn fetch_export_rows_empty_db_returns_empty() {
        let pool = connect_memory().await.unwrap();
        let rows = fetch_export_rows(&pool, &ExportFilter::default())
            .await
            .unwrap();
        assert!(rows.is_empty());
    }

    #[tokio::test]
    async fn fetch_returns_rows_ordered_by_date_then_id() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool, "TR").await;
        let id_b = seed_tx(&pool, acc, "2026-05-10", -100, None, Some("B"), None, None).await;
        let id_a = seed_tx(&pool, acc, "2026-05-10", -200, None, Some("A"), None, None).await;
        let id_c = seed_tx(&pool, acc, "2026-04-10", -300, None, Some("C"), None, None).await;

        let rows = fetch_export_rows(&pool, &ExportFilter::default())
            .await
            .unwrap();
        let ids: Vec<i64> = rows.iter().map(|r| r.id).collect();
        assert_eq!(ids, vec![id_c, id_b, id_a]);
    }

    #[tokio::test]
    async fn filter_from_to_inclusive() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool, "TR").await;
        seed_tx(&pool, acc, "2026-04-30", -100, None, Some("vor"), None, None).await;
        let id_lo = seed_tx(&pool, acc, "2026-05-01", -100, None, Some("low"), None, None).await;
        let id_hi = seed_tx(&pool, acc, "2026-05-31", -100, None, Some("high"), None, None).await;
        seed_tx(&pool, acc, "2026-06-01", -100, None, Some("nach"), None, None).await;

        let f = ExportFilter {
            from: Some(NaiveDate::from_ymd_opt(2026, 5, 1).unwrap()),
            to: Some(NaiveDate::from_ymd_opt(2026, 5, 31).unwrap()),
            ..Default::default()
        };
        let rows = fetch_export_rows(&pool, &f).await.unwrap();
        let ids: Vec<i64> = rows.iter().map(|r| r.id).collect();
        assert_eq!(ids, vec![id_lo, id_hi]);
    }

    #[tokio::test]
    async fn filter_account_id_restricts() {
        let pool = connect_memory().await.unwrap();
        let acc1 = seed_account(&pool, "A1").await;
        let acc2 = seed_account(&pool, "A2").await;
        seed_tx(&pool, acc1, "2026-05-01", -100, None, Some("a"), None, None).await;
        seed_tx(&pool, acc2, "2026-05-01", -200, None, Some("b"), None, None).await;

        let f = ExportFilter {
            account_id: Some(acc1),
            ..Default::default()
        };
        let rows = fetch_export_rows(&pool, &f).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].account_id, acc1);
    }

    #[tokio::test]
    async fn filter_category_id_restricts_and_keeps_null_out() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool, "TR").await;
        let cat = seed_category(&pool, "Lebensmittel").await;
        let id_with = seed_tx(&pool, acc, "2026-05-01", -100, Some(cat), Some("a"), None, None).await;
        seed_tx(&pool, acc, "2026-05-01", -200, None, Some("b"), None, None).await;

        let f = ExportFilter {
            category_id: Some(cat),
            ..Default::default()
        };
        let rows = fetch_export_rows(&pool, &f).await.unwrap();
        let ids: Vec<i64> = rows.iter().map(|r| r.id).collect();
        assert_eq!(ids, vec![id_with]);
        assert_eq!(rows[0].category_name.as_deref(), Some("Lebensmittel"));
    }

    #[tokio::test]
    async fn filter_search_matches_counterparty_purpose_or_note() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool, "TR").await;
        let id_cp = seed_tx(&pool, acc, "2026-05-01", -1, None, Some("REWE Markt"), None, None).await;
        let id_pp = seed_tx(&pool, acc, "2026-05-02", -1, None, Some("X"), Some("Einkauf REWE"), None).await;
        let id_nt = seed_tx(&pool, acc, "2026-05-03", -1, None, Some("X"), None, Some("Rewe-Notiz")).await;
        seed_tx(&pool, acc, "2026-05-04", -1, None, Some("Edeka"), Some("Y"), None).await;

        let f = ExportFilter {
            search: Some("rewe".to_string()),
            ..Default::default()
        };
        let rows = fetch_export_rows(&pool, &f).await.unwrap();
        let ids: std::collections::HashSet<i64> = rows.iter().map(|r| r.id).collect();
        assert_eq!(ids, [id_cp, id_pp, id_nt].into_iter().collect());
    }

    #[tokio::test]
    async fn filter_combinator_is_and() {
        let pool = connect_memory().await.unwrap();
        let acc1 = seed_account(&pool, "A1").await;
        let acc2 = seed_account(&pool, "A2").await;
        let id_match = seed_tx(&pool, acc1, "2026-05-10", -100, None, Some("REWE"), None, None).await;
        seed_tx(&pool, acc1, "2026-04-10", -100, None, Some("REWE"), None, None).await;
        seed_tx(&pool, acc2, "2026-05-10", -100, None, Some("REWE"), None, None).await;

        let f = ExportFilter {
            from: Some(NaiveDate::from_ymd_opt(2026, 5, 1).unwrap()),
            account_id: Some(acc1),
            search: Some("REWE".to_string()),
            ..Default::default()
        };
        let rows = fetch_export_rows(&pool, &f).await.unwrap();
        let ids: Vec<i64> = rows.iter().map(|r| r.id).collect();
        assert_eq!(ids, vec![id_match]);
    }

    #[tokio::test]
    async fn empty_search_string_is_ignored() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool, "TR").await;
        seed_tx(&pool, acc, "2026-05-01", -1, None, Some("X"), None, None).await;
        let f = ExportFilter {
            search: Some("   ".to_string()),
            ..Default::default()
        };
        let rows = fetch_export_rows(&pool, &f).await.unwrap();
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn write_export_csv_empty_writes_header_only() {
        let mut buf = Vec::new();
        write_export_csv(&[], &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert_eq!(
            s,
            "id,date,amount,currency,account,account_id,category,category_id,counterparty,purpose,note,source,source_file_hash,raw_ref\r\n"
        );
    }

    fn sample_row() -> ExportRow {
        ExportRow {
            id: 42,
            booking_date: "2026-05-17".to_string(),
            amount_cents: -1299,
            currency: "EUR".to_string(),
            account_name: "Trade Republic".to_string(),
            account_id: 1,
            category_name: Some("Lebensmittel".to_string()),
            category_id: Some(5),
            counterparty: Some("REWE Markt".to_string()),
            purpose: Some("Einkauf".to_string()),
            manual_note: None,
            source: "tr_csv".to_string(),
            source_file_hash: Some("abc".to_string()),
            raw_ref: Some("TXN-001".to_string()),
        }
    }

    #[test]
    fn write_export_csv_writes_full_row() {
        let mut buf = Vec::new();
        write_export_csv(&[sample_row()], &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        let lines: Vec<&str> = s.split("\r\n").collect();
        assert_eq!(lines.len(), 3); // header, row, trailing empty
        assert_eq!(
            lines[1],
            "42,2026-05-17,-12.99,EUR,Trade Republic,1,Lebensmittel,5,REWE Markt,Einkauf,,tr_csv,abc,TXN-001"
        );
        assert_eq!(lines[2], "");
    }

    #[test]
    fn write_export_csv_formats_amount_with_two_decimals() {
        assert_eq!(format_amount(-1299), "-12.99");
        assert_eq!(format_amount(150_000), "1500.00");
        assert_eq!(format_amount(0), "0.00");
        assert_eq!(format_amount(-5), "-0.05");
        assert_eq!(format_amount(7), "0.07");
    }

    #[test]
    fn write_export_csv_null_category_serializes_empty() {
        let mut row = sample_row();
        row.category_id = None;
        row.category_name = None;
        let mut buf = Vec::new();
        write_export_csv(&[row], &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        let line = s.split("\r\n").nth(1).unwrap();
        let cols: Vec<&str> = line.split(',').collect();
        assert_eq!(cols[6], ""); // category
        assert_eq!(cols[7], ""); // category_id
    }

    #[test]
    fn write_export_csv_quotes_text_containing_special_chars() {
        let mut row = sample_row();
        row.purpose = Some(r#"Hallo, "Welt""#.to_string());
        let mut buf = Vec::new();
        write_export_csv(&[row], &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains(r#""Hallo, ""Welt"""""#) || s.contains(r#""Hallo, ""Welt"""#));
    }
}
