use sqlx::SqlitePool;

use crate::importers::RawTransaction;

use super::DbResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertOutcome {
    Inserted(i64),
    Skipped,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InsertCounts {
    pub inserted: usize,
    pub skipped: usize,
}

/// Fügt eine `RawTransaction` ein. Konflikt auf `idx_transactions_dedup`
/// (account_id, booking_date, amount_cents, counterparty, source_file_hash)
/// → `Skipped` statt Fehler.
pub async fn insert_raw_transaction(
    pool: &SqlitePool,
    account_id: i64,
    source: &str,
    source_file_hash: Option<&str>,
    raw: &RawTransaction,
    category_id: Option<i64>,
) -> DbResult<InsertOutcome> {
    let kind = raw.kind.as_deref().map(String::from).unwrap_or_else(|| {
        if raw.amount_cents > 0 { "income".to_string() } else { "expense".to_string() }
    });

    let row: Option<(i64,)> = sqlx::query_as(
        "INSERT INTO transactions
            (account_id, booking_date, amount_cents, currency,
             counterparty, purpose, raw_ref, category_id, source, source_file_hash, kind,
             counterparty_iban)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
         ON CONFLICT DO NOTHING
         RETURNING id",
    )
    .bind(account_id)
    .bind(raw.booking_date.format("%Y-%m-%d").to_string())
    .bind(raw.amount_cents)
    .bind(&raw.currency)
    .bind(raw.counterparty.as_deref())
    .bind(raw.purpose.as_deref())
    .bind(raw.raw_ref.as_deref())
    .bind(category_id)
    .bind(source)
    .bind(source_file_hash)
    .bind(kind)
    .bind(raw.counterparty_iban.as_deref())
    .fetch_optional(pool)
    .await?;

    Ok(match row {
        Some((id,)) => InsertOutcome::Inserted(id),
        None => InsertOutcome::Skipped,
    })
}

/// Bulk-Insert mit Dedup-Counter.
pub async fn insert_raw_transactions(
    pool: &SqlitePool,
    account_id: i64,
    source: &str,
    source_file_hash: Option<&str>,
    raws: &[(RawTransaction, Option<i64>)],
) -> DbResult<InsertCounts> {
    let mut counts = InsertCounts::default();
    for (raw, category_id) in raws {
        match insert_raw_transaction(pool, account_id, source, source_file_hash, raw, *category_id)
            .await?
        {
            InsertOutcome::Inserted(_) => counts.inserted += 1,
            InsertOutcome::Skipped => counts.skipped += 1,
        }
    }
    Ok(counts)
}

#[cfg(test)]
mod tests {
    use crate::db::connect_memory;
    use crate::db::transactions::{
        insert_raw_transaction, insert_raw_transactions, InsertOutcome,
    };
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

    fn sample_tx() -> RawTransaction {
        RawTransaction {
            booking_date: NaiveDate::from_ymd_opt(2026, 5, 10).unwrap(),
            amount_cents: -1299,
            currency: "EUR".to_string(),
            counterparty: Some("REWE Markt".to_string()),
            purpose: Some("Einkauf".to_string()),
            raw_ref: Some("TXN-001".to_string()),
            kind: None,
            trade: None,
            counterparty_iban: None,
        }
    }

    #[tokio::test]
    async fn inserts_single_raw_transaction() {
        let pool = connect_memory().await.unwrap();
        let account_id = seed_account(&pool, "TR").await;

        let outcome = insert_raw_transaction(
            &pool,
            account_id,
            "tr_csv",
            Some("hash-abc"),
            &sample_tx(),
            None,
        )
        .await
        .unwrap();

        match outcome {
            InsertOutcome::Inserted(id) => assert!(id > 0),
            _ => panic!("expected Inserted"),
        }

        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM transactions")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn second_identical_insert_is_skipped() {
        let pool = connect_memory().await.unwrap();
        let account_id = seed_account(&pool, "TR").await;
        let tx = sample_tx();

        let first =
            insert_raw_transaction(&pool, account_id, "tr_csv", Some("h"), &tx, None)
                .await
                .unwrap();
        let second =
            insert_raw_transaction(&pool, account_id, "tr_csv", Some("h"), &tx, None)
                .await
                .unwrap();

        assert!(matches!(first, InsertOutcome::Inserted(_)));
        assert_eq!(second, InsertOutcome::Skipped);

        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM transactions")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn different_source_file_hash_is_not_dedup() {
        let pool = connect_memory().await.unwrap();
        let account_id = seed_account(&pool, "TR").await;
        let tx = sample_tx();

        insert_raw_transaction(&pool, account_id, "tr_csv", Some("hash-1"), &tx, None)
            .await
            .unwrap();
        let second =
            insert_raw_transaction(&pool, account_id, "tr_csv", Some("hash-2"), &tx, None)
                .await
                .unwrap();

        assert!(matches!(second, InsertOutcome::Inserted(_)));
        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM transactions")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn different_account_is_not_dedup() {
        let pool = connect_memory().await.unwrap();
        let a = seed_account(&pool, "TR").await;
        let b = seed_account(&pool, "Giro").await;
        let tx = sample_tx();

        insert_raw_transaction(&pool, a, "tr_csv", Some("h"), &tx, None)
            .await
            .unwrap();
        let second = insert_raw_transaction(&pool, b, "tr_csv", Some("h"), &tx, None)
            .await
            .unwrap();

        assert!(matches!(second, InsertOutcome::Inserted(_)));
    }

    #[tokio::test]
    async fn bulk_counts_inserted_and_skipped() {
        let pool = connect_memory().await.unwrap();
        let account_id = seed_account(&pool, "TR").await;

        let tx1 = sample_tx();
        let mut tx2 = sample_tx();
        tx2.amount_cents = -500;
        tx2.raw_ref = Some("TXN-002".into());
        let tx3 = sample_tx();

        let batch = vec![(tx1, None), (tx2, None), (tx3, None)];
        let counts =
            insert_raw_transactions(&pool, account_id, "tr_csv", Some("h"), &batch)
                .await
                .unwrap();

        assert_eq!(counts.inserted, 2);
        assert_eq!(counts.skipped, 1);
    }

    #[tokio::test]
    async fn insert_raw_transaction_uses_kind_override() {
        use chrono::NaiveDate;
        use crate::importers::RawTransaction;
        let pool = connect_memory().await.unwrap();
        let (acc_id,): (i64,) = sqlx::query_as(
            "INSERT INTO accounts (name, kind, currency) VALUES ('Broker','broker','EUR') RETURNING id"
        ).fetch_one(&pool).await.unwrap();

        let raw = RawTransaction {
            booking_date: NaiveDate::from_ymd_opt(2026, 5, 13).unwrap(),
            amount_cents: -120_000,
            currency: "EUR".into(),
            counterparty: None,
            purpose: None,
            raw_ref: Some("kindtest-1".into()),
            kind: Some("buy".into()),
            trade: None,
            counterparty_iban: None,
        };
        let out = insert_raw_transaction(&pool, acc_id, "tr_csv", None, &raw, None)
            .await.unwrap();
        let tx_id = match out {
            InsertOutcome::Inserted(id) => id,
            _ => panic!("expected Inserted"),
        };

        let (kind,): (String,) = sqlx::query_as("SELECT kind FROM transactions WHERE id = ?")
            .bind(tx_id).fetch_one(&pool).await.unwrap();
        assert_eq!(kind, "buy");
    }

    #[tokio::test]
    async fn insert_raw_transaction_defaults_kind_from_amount_sign() {
        use chrono::NaiveDate;
        use crate::importers::RawTransaction;
        let pool = connect_memory().await.unwrap();
        let (acc_id,): (i64,) = sqlx::query_as(
            "INSERT INTO accounts (name, kind, currency) VALUES ('Cash','bank','EUR') RETURNING id"
        ).fetch_one(&pool).await.unwrap();

        let make_raw = |amount: i64, raw_ref: &str| RawTransaction {
            booking_date: NaiveDate::from_ymd_opt(2026, 5, 13).unwrap(),
            amount_cents: amount,
            currency: "EUR".into(),
            counterparty: None,
            purpose: None,
            raw_ref: Some(raw_ref.into()),
            kind: None,
            trade: None,
            counterparty_iban: None,
        };

        // positive amount → income
        let r1 = make_raw(5_000, "default-1");
        let out1 = insert_raw_transaction(&pool, acc_id, "tr_csv", None, &r1, None).await.unwrap();
        let tx1 = match out1 { InsertOutcome::Inserted(id) => id, _ => panic!() };
        let (kind1,): (String,) = sqlx::query_as("SELECT kind FROM transactions WHERE id = ?")
            .bind(tx1).fetch_one(&pool).await.unwrap();
        assert_eq!(kind1, "income");

        // negative amount → expense
        let r2 = make_raw(-5_000, "default-2");
        let out2 = insert_raw_transaction(&pool, acc_id, "tr_csv", None, &r2, None).await.unwrap();
        let tx2 = match out2 { InsertOutcome::Inserted(id) => id, _ => panic!() };
        let (kind2,): (String,) = sqlx::query_as("SELECT kind FROM transactions WHERE id = ?")
            .bind(tx2).fetch_one(&pool).await.unwrap();
        assert_eq!(kind2, "expense");
    }

    #[tokio::test]
    async fn category_id_is_persisted() {
        let pool = connect_memory().await.unwrap();
        let account_id = seed_account(&pool, "TR").await;
        let (cat_id,): (i64,) = sqlx::query_as(
            "INSERT INTO categories (name) VALUES ('Lebensmittel') RETURNING id",
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        insert_raw_transaction(
            &pool,
            account_id,
            "tr_csv",
            Some("h"),
            &sample_tx(),
            Some(cat_id),
        )
        .await
        .unwrap();

        let (stored,): (Option<i64>,) =
            sqlx::query_as("SELECT category_id FROM transactions LIMIT 1")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(stored, Some(cat_id));
    }
}
