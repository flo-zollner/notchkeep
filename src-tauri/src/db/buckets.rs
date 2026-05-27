use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use super::{DbError, DbResult};
use crate::model::Bucket;

const BUCKET_COLUMNS: &str = "id, name, icon, color, note, target_cents, \
     start_date, target_date, archived, created_at";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewBucketPayload {
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub note: Option<String>,
    pub target_cents: Option<i64>,
    pub start_date: Option<String>,
    pub target_date: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateBucketPayload {
    pub name: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub note: Option<String>,
    pub target_cents: Option<i64>,
    pub start_date: Option<String>,
    pub target_date: Option<String>,
    pub archived: Option<bool>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct BucketProgress {
    pub bucket_id: i64,
    pub current_cents: i64,
    pub tx_count: i64,
}

fn normalize_opt(v: Option<String>) -> Option<String> {
    v.and_then(|s| if s.trim().is_empty() { None } else { Some(s) })
}

pub async fn create_bucket(pool: &SqlitePool, p: NewBucketPayload) -> DbResult<Bucket> {
    if p.name.trim().is_empty() {
        return Err(DbError::Decode("name must not be empty".into()));
    }
    if let Some(tc) = p.target_cents {
        if tc < 0 {
            return Err(DbError::Decode("target_cents must be >= 0".into()));
        }
    }
    let sql = format!(
        "INSERT INTO buckets
            (name, icon, color, note, target_cents, start_date, target_date)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         RETURNING {BUCKET_COLUMNS}"
    );
    Ok(sqlx::query_as::<_, Bucket>(&sql)
        .bind(p.name)
        .bind(normalize_opt(p.icon))
        .bind(normalize_opt(p.color))
        .bind(normalize_opt(p.note))
        .bind(p.target_cents)
        .bind(normalize_opt(p.start_date))
        .bind(normalize_opt(p.target_date))
        .fetch_one(pool)
        .await?)
}

pub async fn get_bucket(pool: &SqlitePool, id: i64) -> DbResult<Bucket> {
    let sql = format!("SELECT {BUCKET_COLUMNS} FROM buckets WHERE id = ?1");
    Ok(sqlx::query_as::<_, Bucket>(&sql)
        .bind(id)
        .fetch_one(pool)
        .await?)
}

pub async fn list_buckets(pool: &SqlitePool, include_archived: bool) -> DbResult<Vec<Bucket>> {
    let sql = if include_archived {
        format!(
            "SELECT {BUCKET_COLUMNS} FROM buckets \
             ORDER BY archived ASC, created_at DESC"
        )
    } else {
        format!(
            "SELECT {BUCKET_COLUMNS} FROM buckets \
             WHERE archived = 0 \
             ORDER BY created_at DESC"
        )
    };
    Ok(sqlx::query_as::<_, Bucket>(&sql).fetch_all(pool).await?)
}

pub async fn delete_bucket(pool: &SqlitePool, id: i64) -> DbResult<bool> {
    let res = sqlx::query("DELETE FROM buckets WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}

pub async fn update_bucket(
    pool: &SqlitePool,
    id: i64,
    p: UpdateBucketPayload,
) -> DbResult<Bucket> {
    if let Some(n) = &p.name {
        if n.trim().is_empty() {
            return Err(DbError::Decode("name must not be empty".into()));
        }
    }
    if let Some(tc) = p.target_cents {
        if tc < 0 {
            return Err(DbError::Decode("target_cents must be >= 0".into()));
        }
    }
    let current = get_bucket(pool, id).await?;
    let sql = format!(
        "UPDATE buckets SET
            name         = ?1,
            icon         = ?2,
            color        = ?3,
            note         = ?4,
            target_cents = ?5,
            start_date   = ?6,
            target_date  = ?7,
            archived     = ?8
         WHERE id = ?9
         RETURNING {BUCKET_COLUMNS}"
    );
    Ok(sqlx::query_as::<_, Bucket>(&sql)
        .bind(p.name.unwrap_or(current.name))
        .bind(normalize_opt(p.icon).or(current.icon))
        .bind(normalize_opt(p.color).or(current.color))
        .bind(normalize_opt(p.note).or(current.note))
        .bind(p.target_cents.or(current.target_cents))
        .bind(normalize_opt(p.start_date).or(current.start_date))
        .bind(normalize_opt(p.target_date).or(current.target_date))
        .bind(p.archived.unwrap_or(current.archived))
        .bind(id)
        .fetch_one(pool)
        .await?)
}

pub async fn bucket_balance(pool: &SqlitePool, id: i64) -> DbResult<i64> {
    let (sum,): (i64,) = sqlx::query_as(
        "SELECT COALESCE(SUM(amount_cents), 0) FROM transactions WHERE bucket_id = ?1",
    )
    .bind(id)
    .fetch_one(pool)
    .await?;
    Ok(sum)
}

pub async fn list_bucket_progress(pool: &SqlitePool) -> DbResult<Vec<BucketProgress>> {
    let rows = sqlx::query_as::<_, BucketProgress>(
        "SELECT bucket_id AS bucket_id,
                COALESCE(SUM(amount_cents), 0) AS current_cents,
                COUNT(*) AS tx_count
           FROM transactions
          WHERE bucket_id IS NOT NULL
          GROUP BY bucket_id",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_memory;

    #[tokio::test]
    async fn create_and_get_bucket_round_trips() {
        let pool = connect_memory().await.unwrap();
        let b = create_bucket(&pool, NewBucketPayload {
            name: "Urlaub 2026".into(),
            icon: Some("plane".into()),
            color: Some("var(--c1)".into()),
            note: None,
            target_cents: Some(200_000),
            start_date: Some("2026-01-01".into()),
            target_date: Some("2026-08-01".into()),
        }).await.unwrap();
        assert_eq!(b.name, "Urlaub 2026");
        assert_eq!(b.target_cents, Some(200_000));
        assert!(!b.archived);

        let fetched = get_bucket(&pool, b.id).await.unwrap();
        assert_eq!(fetched.id, b.id);
        assert_eq!(fetched.name, b.name);
    }

    #[tokio::test]
    async fn list_buckets_excludes_archived_by_default() {
        let pool = connect_memory().await.unwrap();
        let a = create_bucket(&pool, NewBucketPayload {
            name: "active".into(), icon: None, color: None, note: None,
            target_cents: None, start_date: None, target_date: None,
        }).await.unwrap();
        let _b = create_bucket(&pool, NewBucketPayload {
            name: "archived".into(), icon: None, color: None, note: None,
            target_cents: None, start_date: None, target_date: None,
        }).await.unwrap();
        sqlx::query("UPDATE buckets SET archived = 1 WHERE name = 'archived'")
            .execute(&pool).await.unwrap();

        let listed = list_buckets(&pool, false).await.unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, a.id);

        let all = list_buckets(&pool, true).await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn update_bucket_coalesces_partial_payload() {
        let pool = connect_memory().await.unwrap();
        let b = create_bucket(&pool, NewBucketPayload {
            name: "alt".into(),
            icon: Some("plane".into()), color: None, note: None,
            target_cents: Some(100_000),
            start_date: Some("2026-01-01".into()), target_date: None,
        }).await.unwrap();

        // Partielles Update: nur name, andere Felder bleiben.
        let updated = update_bucket(&pool, b.id, UpdateBucketPayload {
            name: Some("neu".into()),
            icon: None, color: None, note: None,
            target_cents: None, start_date: None, target_date: None,
            archived: None,
        }).await.unwrap();
        assert_eq!(updated.name, "neu");
        assert_eq!(updated.icon.as_deref(), Some("plane"));
        assert_eq!(updated.target_cents, Some(100_000));
        assert!(!updated.archived);

        // archived = true setzen.
        let archived = update_bucket(&pool, b.id, UpdateBucketPayload {
            name: None, icon: None, color: None, note: None,
            target_cents: None, start_date: None, target_date: None,
            archived: Some(true),
        }).await.unwrap();
        assert!(archived.archived);
    }

    #[tokio::test]
    async fn delete_bucket_sets_tx_bucket_id_to_null() {
        let pool = connect_memory().await.unwrap();
        let acc = crate::db::accounts::create_account(
            &pool, "A", "bank", "EUR", None, None, None,
        ).await.unwrap();
        let b = create_bucket(&pool, NewBucketPayload {
            name: "test".into(), icon: None, color: None, note: None,
            target_cents: None, start_date: None, target_date: None,
        }).await.unwrap();
        sqlx::query(
            "INSERT INTO transactions
                (account_id, booking_date, amount_cents, currency, counterparty, source, bucket_id)
             VALUES (?1, '2026-05-01', 1000, 'EUR', 'x', 'manual', ?2)",
        )
        .bind(acc.id).bind(b.id)
        .execute(&pool).await.unwrap();

        assert!(delete_bucket(&pool, b.id).await.unwrap());

        let (cnt,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM transactions WHERE bucket_id IS NOT NULL")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(cnt, 0);
        let (still_there,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM transactions")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(still_there, 1, "Tx selbst bleibt erhalten");
    }

    #[tokio::test]
    async fn delete_bucket_returns_false_for_unknown_id() {
        let pool = connect_memory().await.unwrap();
        assert!(!delete_bucket(&pool, 9_999).await.unwrap());
    }

    #[tokio::test]
    async fn bucket_balance_sums_all_assigned_tx_net() {
        let pool = connect_memory().await.unwrap();
        let acc = crate::db::accounts::create_account(
            &pool, "A", "bank", "EUR", None, None, None,
        ).await.unwrap();
        let b = create_bucket(&pool, NewBucketPayload {
            name: "x".into(), icon: None, color: None, note: None,
            target_cents: None, start_date: None, target_date: None,
        }).await.unwrap();
        sqlx::query(
            "INSERT INTO transactions
                (account_id, booking_date, amount_cents, currency, counterparty, source, bucket_id)
             VALUES
                (?1, '2026-05-01',  20000, 'EUR', 'in',  'manual', ?2),
                (?1, '2026-05-02',  -5000, 'EUR', 'out', 'manual', ?2),
                (?1, '2026-05-03',  10000, 'EUR', 'x',   'manual', NULL)",
        )
        .bind(acc.id).bind(b.id)
        .execute(&pool).await.unwrap();

        assert_eq!(bucket_balance(&pool, b.id).await.unwrap(), 15000);
    }

    #[tokio::test]
    async fn bucket_balance_empty_is_zero() {
        let pool = connect_memory().await.unwrap();
        let b = create_bucket(&pool, NewBucketPayload {
            name: "x".into(), icon: None, color: None, note: None,
            target_cents: None, start_date: None, target_date: None,
        }).await.unwrap();
        assert_eq!(bucket_balance(&pool, b.id).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn list_bucket_progress_groups_correctly() {
        let pool = connect_memory().await.unwrap();
        let acc = crate::db::accounts::create_account(
            &pool, "A", "bank", "EUR", None, None, None,
        ).await.unwrap();
        let b1 = create_bucket(&pool, NewBucketPayload {
            name: "b1".into(), icon: None, color: None, note: None,
            target_cents: None, start_date: None, target_date: None,
        }).await.unwrap();
        let b2 = create_bucket(&pool, NewBucketPayload {
            name: "b2".into(), icon: None, color: None, note: None,
            target_cents: None, start_date: None, target_date: None,
        }).await.unwrap();
        sqlx::query(
            "INSERT INTO transactions
                (account_id, booking_date, amount_cents, currency, counterparty, source, bucket_id)
             VALUES
                (?1, '2026-05-01', 100, 'EUR', 'a', 'manual', ?2),
                (?1, '2026-05-02',  50, 'EUR', 'b', 'manual', ?2),
                (?1, '2026-05-03', 300, 'EUR', 'c', 'manual', ?3)",
        )
        .bind(acc.id).bind(b1.id).bind(b2.id)
        .execute(&pool).await.unwrap();

        let mut progress = list_bucket_progress(&pool).await.unwrap();
        progress.sort_by_key(|p| p.bucket_id);
        assert_eq!(progress.len(), 2);
        assert_eq!(progress[0].bucket_id, b1.id);
        assert_eq!(progress[0].current_cents, 150);
        assert_eq!(progress[0].tx_count, 2);
        assert_eq!(progress[1].bucket_id, b2.id);
        assert_eq!(progress[1].current_cents, 300);
        assert_eq!(progress[1].tx_count, 1);
    }

    #[tokio::test]
    async fn create_bucket_normalizes_empty_strings_to_none() {
        let pool = connect_memory().await.unwrap();
        let b = create_bucket(&pool, NewBucketPayload {
            name: "x".into(),
            icon: Some("".into()),
            color: Some("  ".into()),
            note: Some("".into()),
            target_cents: None,
            start_date: None, target_date: None,
        }).await.unwrap();
        assert!(b.icon.is_none());
        assert!(b.color.is_none());
        assert!(b.note.is_none());
    }
}
