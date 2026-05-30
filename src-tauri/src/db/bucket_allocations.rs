use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use super::{DbError, DbResult};

#[derive(Debug, Clone, Serialize, PartialEq, Eq, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct BucketAllocation {
    pub id: i64,
    pub bucket_id: i64,
    pub amount_cents: i64,
    pub occurred_on: String,
    pub note: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewBucketAllocationPayload {
    pub bucket_id: i64,
    pub amount_cents: i64,
    /// YYYY-MM-DD; if None, today's date is used.
    pub occurred_on: Option<String>,
    pub note: Option<String>,
}

async fn today(pool: &SqlitePool) -> DbResult<String> {
    let (d,): (String,) = sqlx::query_as("SELECT strftime('%Y-%m-%d','now')")
        .fetch_one(pool)
        .await?;
    Ok(d)
}

pub async fn create_allocation(
    pool: &SqlitePool,
    p: NewBucketAllocationPayload,
) -> DbResult<BucketAllocation> {
    if p.amount_cents == 0 {
        return Err(DbError::Decode("amount_cents must not be 0".into()));
    }
    let occurred = match p.occurred_on {
        Some(s) if !s.trim().is_empty() => s,
        _ => today(pool).await?,
    };
    let note = p
        .note
        .and_then(|s| if s.trim().is_empty() { None } else { Some(s) });
    Ok(sqlx::query_as::<_, BucketAllocation>(
        "INSERT INTO bucket_allocations (bucket_id, amount_cents, occurred_on, note)
         VALUES (?1, ?2, ?3, ?4)
         RETURNING id, bucket_id, amount_cents, occurred_on, note, created_at",
    )
    .bind(p.bucket_id)
    .bind(p.amount_cents)
    .bind(occurred)
    .bind(note)
    .fetch_one(pool)
    .await?)
}

pub async fn list_allocations(
    pool: &SqlitePool,
    bucket_id: Option<i64>,
) -> DbResult<Vec<BucketAllocation>> {
    let rows = match bucket_id {
        Some(id) => {
            sqlx::query_as::<_, BucketAllocation>(
                "SELECT id, bucket_id, amount_cents, occurred_on, note, created_at
                   FROM bucket_allocations WHERE bucket_id = ?1
                  ORDER BY occurred_on DESC, id DESC",
            )
            .bind(id)
            .fetch_all(pool)
            .await?
        }
        None => {
            sqlx::query_as::<_, BucketAllocation>(
                "SELECT id, bucket_id, amount_cents, occurred_on, note, created_at
                   FROM bucket_allocations
                  ORDER BY occurred_on DESC, id DESC",
            )
            .fetch_all(pool)
            .await?
        }
    };
    Ok(rows)
}

/// Cash balance of a bucket = allocations + transactions assigned to it (outflows).
pub async fn bucket_cash_balance(pool: &SqlitePool, bucket_id: i64) -> DbResult<i64> {
    let (alloc,): (i64,) = sqlx::query_as(
        "SELECT COALESCE(SUM(amount_cents),0) FROM bucket_allocations WHERE bucket_id = ?1",
    )
    .bind(bucket_id)
    .fetch_one(pool)
    .await?;
    let (assigned,): (i64,) = sqlx::query_as(
        "SELECT COALESCE(SUM(amount_cents),0) FROM transactions WHERE bucket_id = ?1",
    )
    .bind(bucket_id)
    .fetch_one(pool)
    .await?;
    Ok(alloc + assigned)
}

/// "Ready to Assign" = sum(cash account balances) - sum(all bucket cash balances).
/// Cash accounts = accounts.kind != 'broker'. May go negative (under-funded).
pub async fn ready_to_assign(pool: &SqlitePool) -> DbResult<i64> {
    let (cash,): (i64,) = sqlx::query_as(
        "SELECT COALESCE(SUM(amount_cents),0) FROM transactions
          WHERE account_id IN (SELECT id FROM accounts WHERE kind != 'broker')",
    )
    .fetch_one(pool)
    .await?;
    let (alloc,): (i64,) =
        sqlx::query_as("SELECT COALESCE(SUM(amount_cents),0) FROM bucket_allocations")
            .fetch_one(pool)
            .await?;
    let (assigned,): (i64,) = sqlx::query_as(
        "SELECT COALESCE(SUM(amount_cents),0) FROM transactions WHERE bucket_id IS NOT NULL",
    )
    .fetch_one(pool)
    .await?;
    Ok(cash - (alloc + assigned))
}

/// Moves `amount_cents` (> 0) from `from_bucket` to `to_bucket` as two allocation
/// rows in one transaction.
pub async fn move_between_buckets(
    pool: &SqlitePool,
    from_bucket: i64,
    to_bucket: i64,
    amount_cents: i64,
    occurred_on: Option<String>,
) -> DbResult<()> {
    if amount_cents <= 0 {
        return Err(DbError::Decode("amount_cents must be > 0".into()));
    }
    if from_bucket == to_bucket {
        return Err(DbError::Decode("from and to bucket must differ".into()));
    }
    let occurred = match occurred_on {
        Some(s) if !s.trim().is_empty() => s,
        _ => today(pool).await?,
    };
    let mut tx = pool.begin().await?;
    sqlx::query(
        "INSERT INTO bucket_allocations (bucket_id, amount_cents, occurred_on, note)
         VALUES (?1, ?2, ?3, ?4)",
    )
    .bind(from_bucket)
    .bind(-amount_cents)
    .bind(&occurred)
    .bind(Some(format!("→ Topf {to_bucket}")))
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO bucket_allocations (bucket_id, amount_cents, occurred_on, note)
         VALUES (?1, ?2, ?3, ?4)",
    )
    .bind(to_bucket)
    .bind(amount_cents)
    .bind(&occurred)
    .bind(Some(format!("← Topf {from_bucket}")))
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::buckets::{create_bucket, NewBucketPayload};
    use crate::db::connect_memory;

    async fn seed_bucket(pool: &SqlitePool, name: &str) -> i64 {
        create_bucket(
            pool,
            NewBucketPayload {
                name: name.into(),
                icon: None,
                color: None,
                note: None,
                target_cents: None,
                start_date: None,
                target_date: None,
            },
        )
        .await
        .unwrap()
        .id
    }

    async fn seed_cash_account(pool: &SqlitePool) -> i64 {
        crate::db::accounts::create_account(pool, "Giro", "bank", "EUR", None, None, None)
            .await
            .unwrap()
            .id
    }

    #[tokio::test]
    async fn create_and_list_allocations() {
        let pool = connect_memory().await.unwrap();
        let b = seed_bucket(&pool, "Urlaub").await;

        create_allocation(
            &pool,
            NewBucketAllocationPayload {
                bucket_id: b,
                amount_cents: 30_000,
                occurred_on: Some("2026-05-01".into()),
                note: Some("Mai".into()),
            },
        )
        .await
        .unwrap();
        create_allocation(
            &pool,
            NewBucketAllocationPayload {
                bucket_id: b,
                amount_cents: -5_000,
                occurred_on: None,
                note: None,
            },
        )
        .await
        .unwrap();

        let rows = list_allocations(&pool, Some(b)).await.unwrap();
        assert_eq!(rows.len(), 2);
        let total: i64 = rows.iter().map(|r| r.amount_cents).sum();
        assert_eq!(total, 25_000);
        assert!(
            rows.iter().all(|r| r.occurred_on.len() == 10),
            "occurred_on is YYYY-MM-DD"
        );
    }

    #[tokio::test]
    async fn create_allocation_rejects_zero() {
        let pool = connect_memory().await.unwrap();
        let b = seed_bucket(&pool, "x").await;
        assert!(create_allocation(
            &pool,
            NewBucketAllocationPayload {
                bucket_id: b,
                amount_cents: 0,
                occurred_on: None,
                note: None,
            },
        )
        .await
        .is_err());
    }

    #[tokio::test]
    async fn ready_to_assign_is_cash_minus_reserved() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_cash_account(&pool).await;
        let b = seed_bucket(&pool, "Notgroschen").await;

        // 1000 EUR income on the cash account
        sqlx::query(
            "INSERT INTO transactions
                (account_id, booking_date, amount_cents, currency, counterparty, source, kind)
             VALUES (?1, '2026-05-01', 100000, 'EUR', 'Lohn', 'manual', 'income')",
        )
        .bind(acc)
        .execute(&pool)
        .await
        .unwrap();

        // No allocation yet: everything unassigned
        assert_eq!(ready_to_assign(&pool).await.unwrap(), 100000);

        // Reserve 300 EUR into the bucket
        create_allocation(
            &pool,
            NewBucketAllocationPayload {
                bucket_id: b,
                amount_cents: 30000,
                occurred_on: None,
                note: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(ready_to_assign(&pool).await.unwrap(), 70000);

        // 50 EUR expense out of the bucket -> RTA unchanged
        sqlx::query(
            "INSERT INTO transactions
                (account_id, booking_date, amount_cents, currency, counterparty, source, kind, bucket_id)
             VALUES (?1, '2026-05-02', -5000, 'EUR', 'Edeka', 'manual', 'expense', ?2)",
        )
        .bind(acc)
        .bind(b)
        .execute(&pool)
        .await
        .unwrap();
        assert_eq!(
            ready_to_assign(&pool).await.unwrap(),
            70000,
            "spending from a bucket does not change RTA"
        );

        // Broker account does NOT count towards RTA
        let depot =
            crate::db::accounts::create_account(&pool, "Depot", "broker", "EUR", None, None, None)
                .await
                .unwrap()
                .id;
        sqlx::query(
            "INSERT INTO transactions
                (account_id, booking_date, amount_cents, currency, counterparty, source, kind)
             VALUES (?1, '2026-05-03', 500000, 'EUR', 'Einlage', 'manual', 'income')",
        )
        .bind(depot)
        .execute(&pool)
        .await
        .unwrap();
        assert_eq!(
            ready_to_assign(&pool).await.unwrap(),
            70000,
            "broker account is not cash"
        );
    }

    #[tokio::test]
    async fn move_between_buckets_writes_two_rows() {
        let pool = connect_memory().await.unwrap();
        let a = seed_bucket(&pool, "A").await;
        let b = seed_bucket(&pool, "B").await;
        create_allocation(
            &pool,
            NewBucketAllocationPayload {
                bucket_id: a,
                amount_cents: 50000,
                occurred_on: None,
                note: None,
            },
        )
        .await
        .unwrap();

        move_between_buckets(&pool, a, b, 20000, None)
            .await
            .unwrap();

        assert_eq!(bucket_cash_balance(&pool, a).await.unwrap(), 30000);
        assert_eq!(bucket_cash_balance(&pool, b).await.unwrap(), 20000);
        assert_eq!(list_allocations(&pool, Some(a)).await.unwrap().len(), 2);
        assert_eq!(list_allocations(&pool, Some(b)).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn move_between_buckets_rejects_nonpositive_or_same() {
        let pool = connect_memory().await.unwrap();
        let a = seed_bucket(&pool, "A").await;
        let b = seed_bucket(&pool, "B").await;
        assert!(move_between_buckets(&pool, a, b, 0, None).await.is_err());
        assert!(move_between_buckets(&pool, a, a, 100, None).await.is_err());
    }
}
