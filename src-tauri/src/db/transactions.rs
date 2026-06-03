use sqlx::SqlitePool;

use crate::importers::RawTransaction;

use super::{DbError, DbResult};

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

/// Inserts a `RawTransaction`. Conflict on `idx_transactions_dedup`
/// (account_id, booking_date, amount_cents, counterparty, source_file_hash)
/// → `Skipped` instead of an error.
pub async fn insert_raw_transaction(
    pool: &SqlitePool,
    account_id: i64,
    source: &str,
    source_file_hash: Option<&str>,
    raw: &RawTransaction,
    category_id: Option<i64>,
) -> DbResult<InsertOutcome> {
    let kind = raw.kind.as_deref().map(String::from).unwrap_or_else(|| {
        if raw.amount_cents > 0 {
            "income".to_string()
        } else {
            "expense".to_string()
        }
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

/// Bulk insert with dedup counter.
pub async fn insert_raw_transactions(
    pool: &SqlitePool,
    account_id: i64,
    source: &str,
    source_file_hash: Option<&str>,
    raws: &[(RawTransaction, Option<i64>)],
) -> DbResult<InsertCounts> {
    let mut counts = InsertCounts::default();
    for (raw, category_id) in raws {
        match insert_raw_transaction(
            pool,
            account_id,
            source,
            source_file_hash,
            raw,
            *category_id,
        )
        .await?
        {
            InsertOutcome::Inserted(_) => counts.inserted += 1,
            InsertOutcome::Skipped => counts.skipped += 1,
        }
    }
    Ok(counts)
}

/// Assigns a transaction to a bucket (`Some`) or detaches it (`None`).
/// Only outflows (amount_cents < 0) may be assigned to a bucket — income lands
/// in the "unassigned" pool (Ready to Assign), not in a bucket.
pub async fn set_transaction_bucket(
    pool: &SqlitePool,
    transaction_id: i64,
    bucket_id: Option<i64>,
) -> DbResult<()> {
    if bucket_id.is_some() {
        let (amount,): (i64,) =
            sqlx::query_as("SELECT amount_cents FROM transactions WHERE id = ?1")
                .bind(transaction_id)
                .fetch_one(pool)
                .await?;
        if amount >= 0 {
            return Err(DbError::Decode(
                "only outflows can be assigned to a bucket".into(),
            ));
        }
    }
    sqlx::query("UPDATE transactions SET bucket_id = ?1 WHERE id = ?2")
        .bind(bucket_id)
        .bind(transaction_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Soft-delete a transaction by physically moving it — and its securities_trades
/// child, if any — into the trash tables (migration 0005), preserving the
/// original id. Restorable via [`restore_transaction`]. Returns false if no live
/// transaction with that id exists. This keeps soft-delete out of the ~60 read
/// queries on `transactions`: the row is simply absent until restored.
pub async fn move_transaction_to_trash(pool: &SqlitePool, id: i64) -> DbResult<bool> {
    let mut tx = pool.begin().await?;
    let moved = sqlx::query(
        "INSERT INTO deleted_transactions
            (id, account_id, booking_date, value_date, amount_cents, currency, counterparty,
             purpose, raw_ref, category_id, source, source_file_hash, imported_at, manual_note,
             bucket_id, kind, counterparty_iban, paired_tx_id)
         SELECT id, account_id, booking_date, value_date, amount_cents, currency, counterparty,
             purpose, raw_ref, category_id, source, source_file_hash, imported_at, manual_note,
             bucket_id, kind, counterparty_iban, paired_tx_id
           FROM transactions WHERE id = ?1",
    )
    .bind(id)
    .execute(&mut *tx)
    .await?;
    if moved.rows_affected() == 0 {
        tx.rollback().await?;
        return Ok(false);
    }
    sqlx::query(
        "INSERT INTO deleted_securities_trades
            (tx_id, security_id, side, shares_micro, unit_price_micro, fee_cents, tax_cents,
             fx_rate_micro, account_id, kest_cents, withholding_tax_cents, fusion_group)
         SELECT tx_id, security_id, side, shares_micro, unit_price_micro, fee_cents, tax_cents,
             fx_rate_micro, account_id, kest_cents, withholding_tax_cents, fusion_group
           FROM securities_trades WHERE tx_id = ?1",
    )
    .bind(id)
    .execute(&mut *tx)
    .await?;
    // CASCADE removes the live securities_trades child together with the parent.
    sqlx::query("DELETE FROM transactions WHERE id = ?1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    // A partner that referenced this tx loses the link (re-established on restore).
    sqlx::query("UPDATE transactions SET paired_tx_id = NULL WHERE paired_tx_id = ?1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(true)
}

/// Move a previously trashed transaction (and its trade child) back into the
/// live tables with its original id. References that became dangling while in
/// the trash (partner / category / bucket hard-removed) are dropped to NULL so
/// FK constraints hold. Returns false if nothing with that id is in the trash.
pub async fn restore_transaction(pool: &SqlitePool, id: i64) -> DbResult<bool> {
    let mut tx = pool.begin().await?;
    let moved = sqlx::query(
        "INSERT INTO transactions
            (id, account_id, booking_date, value_date, amount_cents, currency, counterparty,
             purpose, raw_ref, category_id, source, source_file_hash, imported_at, manual_note,
             bucket_id, kind, counterparty_iban, paired_tx_id)
         SELECT id, account_id, booking_date, value_date, amount_cents, currency, counterparty,
             purpose, raw_ref,
             CASE WHEN category_id IS NOT NULL
                       AND EXISTS (SELECT 1 FROM categories c WHERE c.id = dt.category_id)
                  THEN category_id ELSE NULL END,
             source, source_file_hash, imported_at, manual_note,
             CASE WHEN bucket_id IS NOT NULL
                       AND EXISTS (SELECT 1 FROM buckets b WHERE b.id = dt.bucket_id)
                  THEN bucket_id ELSE NULL END,
             kind, counterparty_iban,
             CASE WHEN paired_tx_id IS NOT NULL
                       AND EXISTS (SELECT 1 FROM transactions t WHERE t.id = dt.paired_tx_id)
                  THEN paired_tx_id ELSE NULL END
           FROM deleted_transactions dt WHERE dt.id = ?1",
    )
    .bind(id)
    .execute(&mut *tx)
    .await?;
    if moved.rows_affected() == 0 {
        tx.rollback().await?;
        return Ok(false);
    }
    sqlx::query(
        "INSERT INTO securities_trades
            (tx_id, security_id, side, shares_micro, unit_price_micro, fee_cents, tax_cents,
             fx_rate_micro, account_id, kest_cents, withholding_tax_cents, fusion_group)
         SELECT tx_id, security_id, side, shares_micro, unit_price_micro, fee_cents, tax_cents,
             fx_rate_micro, account_id, kest_cents, withholding_tax_cents, fusion_group
           FROM deleted_securities_trades WHERE tx_id = ?1",
    )
    .bind(id)
    .execute(&mut *tx)
    .await?;
    sqlx::query("DELETE FROM deleted_transactions WHERE id = ?1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM deleted_securities_trades WHERE tx_id = ?1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    // Re-establish the pairing from the partner's side if it still exists.
    sqlx::query(
        "UPDATE transactions SET paired_tx_id = ?1
          WHERE id = (SELECT paired_tx_id FROM transactions WHERE id = ?1)
            AND paired_tx_id IS NULL",
    )
    .bind(id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use crate::db::connect_memory;
    use crate::db::transactions::{
        insert_raw_transaction, insert_raw_transactions, set_transaction_bucket, InsertOutcome,
    };
    use crate::importers::RawTransaction;
    use chrono::NaiveDate;
    use sqlx::SqlitePool;

    #[tokio::test]
    async fn set_bucket_rejects_inflow() {
        let pool = connect_memory().await.unwrap();
        let acc = crate::db::accounts::create_account(&pool, "A", "bank", "EUR", None, None, None)
            .await
            .unwrap();
        let b = crate::db::buckets::create_bucket(
            &pool,
            crate::db::buckets::NewBucketPayload {
                name: "x".into(),
                icon: None,
                color: None,
                note: None,
                target_cents: None,
                start_date: None,
                target_date: None,
            },
        )
        .await
        .unwrap();
        let (income_id,): (i64,) = sqlx::query_as(
            "INSERT INTO transactions
                (account_id, booking_date, amount_cents, currency, counterparty, source, kind)
             VALUES (?1, '2026-05-01', 10000, 'EUR', 'Lohn', 'manual', 'income') RETURNING id",
        )
        .bind(acc.id)
        .fetch_one(&pool)
        .await
        .unwrap();
        let (exp_id,): (i64,) = sqlx::query_as(
            "INSERT INTO transactions
                (account_id, booking_date, amount_cents, currency, counterparty, source, kind)
             VALUES (?1, '2026-05-02', -2000, 'EUR', 'Edeka', 'manual', 'expense') RETURNING id",
        )
        .bind(acc.id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert!(
            set_transaction_bucket(&pool, income_id, Some(b.id))
                .await
                .is_err(),
            "income must not be assignable to a bucket"
        );
        assert!(
            set_transaction_bucket(&pool, exp_id, Some(b.id))
                .await
                .is_ok(),
            "outflow may be assigned"
        );
        assert!(
            set_transaction_bucket(&pool, exp_id, None).await.is_ok(),
            "detaching is always allowed"
        );
    }

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

        let first = insert_raw_transaction(&pool, account_id, "tr_csv", Some("h"), &tx, None)
            .await
            .unwrap();
        let second = insert_raw_transaction(&pool, account_id, "tr_csv", Some("h"), &tx, None)
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
        let second = insert_raw_transaction(&pool, account_id, "tr_csv", Some("hash-2"), &tx, None)
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
        let counts = insert_raw_transactions(&pool, account_id, "tr_csv", Some("h"), &batch)
            .await
            .unwrap();

        assert_eq!(counts.inserted, 2);
        assert_eq!(counts.skipped, 1);
    }

    #[tokio::test]
    async fn insert_raw_transaction_uses_kind_override() {
        use crate::importers::RawTransaction;
        use chrono::NaiveDate;
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
            .await
            .unwrap();
        let tx_id = match out {
            InsertOutcome::Inserted(id) => id,
            _ => panic!("expected Inserted"),
        };

        let (kind,): (String,) = sqlx::query_as("SELECT kind FROM transactions WHERE id = ?")
            .bind(tx_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(kind, "buy");
    }

    #[tokio::test]
    async fn insert_raw_transaction_defaults_kind_from_amount_sign() {
        use crate::importers::RawTransaction;
        use chrono::NaiveDate;
        let pool = connect_memory().await.unwrap();
        let (acc_id,): (i64,) = sqlx::query_as(
            "INSERT INTO accounts (name, kind, currency) VALUES ('Cash','bank','EUR') RETURNING id",
        )
        .fetch_one(&pool)
        .await
        .unwrap();

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
        let out1 = insert_raw_transaction(&pool, acc_id, "tr_csv", None, &r1, None)
            .await
            .unwrap();
        let tx1 = match out1 {
            InsertOutcome::Inserted(id) => id,
            _ => panic!(),
        };
        let (kind1,): (String,) = sqlx::query_as("SELECT kind FROM transactions WHERE id = ?")
            .bind(tx1)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(kind1, "income");

        // negative amount → expense
        let r2 = make_raw(-5_000, "default-2");
        let out2 = insert_raw_transaction(&pool, acc_id, "tr_csv", None, &r2, None)
            .await
            .unwrap();
        let tx2 = match out2 {
            InsertOutcome::Inserted(id) => id,
            _ => panic!(),
        };
        let (kind2,): (String,) = sqlx::query_as("SELECT kind FROM transactions WHERE id = ?")
            .bind(tx2)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(kind2, "expense");
    }

    #[tokio::test]
    async fn trash_and_restore_plain_transaction_round_trips() {
        let pool = connect_memory().await.unwrap();
        let account_id = seed_account(&pool, "Giro").await;
        let (tx_id,): (i64,) = sqlx::query_as(
            "INSERT INTO transactions
                (account_id, booking_date, amount_cents, currency, counterparty, source, kind)
             VALUES (?1, '2026-05-01', -1000, 'EUR', 'REWE', 'manual', 'expense') RETURNING id",
        )
        .bind(account_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        // move to trash
        let moved = crate::db::transactions::move_transaction_to_trash(&pool, tx_id)
            .await
            .unwrap();
        assert!(moved, "should report moved=true");

        let (live,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM transactions")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(live, 0, "live table must be empty after trash");

        let (deleted,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM deleted_transactions")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(deleted, 1, "trash table must hold 1 row");

        // restore
        let restored = crate::db::transactions::restore_transaction(&pool, tx_id)
            .await
            .unwrap();
        assert!(restored, "should report restored=true");

        let (live2,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM transactions")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(live2, 1, "live table must hold 1 row after restore");

        let (deleted2,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM deleted_transactions")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(deleted2, 0, "trash table must be empty after restore");

        // id is preserved
        let (id_back,): (i64,) = sqlx::query_as("SELECT id FROM transactions WHERE id = ?1")
            .bind(tx_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(id_back, tx_id, "restored row must have original id");
    }

    #[tokio::test]
    async fn move_unknown_returns_false() {
        let pool = connect_memory().await.unwrap();
        let result = crate::db::transactions::move_transaction_to_trash(&pool, 99_999)
            .await
            .unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn restore_unknown_returns_false() {
        let pool = connect_memory().await.unwrap();
        let result = crate::db::transactions::restore_transaction(&pool, 99_999)
            .await
            .unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn paired_transactions_unlink_on_trash_and_relink_on_restore() {
        let pool = connect_memory().await.unwrap();
        let account_id = seed_account(&pool, "Giro").await;

        let (tx_a,): (i64,) = sqlx::query_as(
            "INSERT INTO transactions
                (account_id, booking_date, amount_cents, currency, counterparty, source, kind)
             VALUES (?1, '2026-05-02', -2000, 'EUR', 'Transfer', 'manual', 'expense') RETURNING id",
        )
        .bind(account_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        let (tx_b,): (i64,) = sqlx::query_as(
            "INSERT INTO transactions
                (account_id, booking_date, amount_cents, currency, counterparty, source, kind)
             VALUES (?1, '2026-05-02', 2000, 'EUR', 'Transfer', 'manual', 'income') RETURNING id",
        )
        .bind(account_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        // link both sides
        sqlx::query("UPDATE transactions SET paired_tx_id = ?1 WHERE id = ?2")
            .bind(tx_b)
            .bind(tx_a)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("UPDATE transactions SET paired_tx_id = ?1 WHERE id = ?2")
            .bind(tx_a)
            .bind(tx_b)
            .execute(&pool)
            .await
            .unwrap();

        // trash tx_a — partner's paired_tx_id should become NULL
        crate::db::transactions::move_transaction_to_trash(&pool, tx_a)
            .await
            .unwrap();

        let (partner_pair,): (Option<i64>,) =
            sqlx::query_as("SELECT paired_tx_id FROM transactions WHERE id = ?1")
                .bind(tx_b)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(
            partner_pair.is_none(),
            "partner's paired_tx_id must be NULL while tx_a is in trash"
        );

        // restore tx_a — pairing should be re-established on partner's side
        crate::db::transactions::restore_transaction(&pool, tx_a)
            .await
            .unwrap();

        let (partner_pair2,): (Option<i64>,) =
            sqlx::query_as("SELECT paired_tx_id FROM transactions WHERE id = ?1")
                .bind(tx_b)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            partner_pair2,
            Some(tx_a),
            "partner's paired_tx_id must be restored to tx_a"
        );

        let (self_pair,): (Option<i64>,) =
            sqlx::query_as("SELECT paired_tx_id FROM transactions WHERE id = ?1")
                .bind(tx_a)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            self_pair,
            Some(tx_b),
            "tx_a's own paired_tx_id must still point to tx_b after restore"
        );
    }

    #[tokio::test]
    async fn category_id_is_persisted() {
        let pool = connect_memory().await.unwrap();
        let account_id = seed_account(&pool, "TR").await;
        let (cat_id,): (i64,) =
            sqlx::query_as("INSERT INTO categories (name) VALUES ('Lebensmittel') RETURNING id")
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
