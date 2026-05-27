use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use sqlx::SqlitePool;

use super::DbResult;

/// Stale-Lock-Schwelle: ein Lock eines anderen Geräts gilt als veraltet, wenn
/// er älter als das ist (Crash auf anderem Gerät o.Ä.).
pub const STALE_AFTER: Duration = Duration::hours(24);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LockHolder {
    pub device_id: String,
    pub hostname: String,
    pub acquired_at: DateTime<Utc>,
}

/// Resultat von `acquire`.
#[derive(Debug)]
pub enum AcquireOutcome {
    /// Lock war frei oder bereits von uns gehalten → jetzt unser.
    Acquired,
    /// Ein anderes Gerät hält den Lock und er ist noch frisch (< STALE_AFTER).
    /// UI soll Warnung zeigen; `force_acquire` kann ihn übernehmen.
    HeldByOther(LockHolder),
}

/// Versucht den Sync-Lock zu erwerben. Schreibt nur, wenn:
/// - keine Zeile existiert, ODER
/// - die existierende Zeile gehört uns (gleiches `device_id`), ODER
/// - der bestehende Lock ist älter als `STALE_AFTER`.
pub async fn acquire(
    pool: &SqlitePool,
    device_id: &str,
    hostname: &str,
) -> DbResult<AcquireOutcome> {
    let now = Utc::now();
    let current: Option<(String, String, String)> = sqlx::query_as(
        "SELECT device_id, hostname, acquired_at FROM sync_lock WHERE id = 1",
    )
    .fetch_optional(pool)
    .await?;

    if let Some((other_device, other_host, acquired_at_str)) = current {
        if other_device != device_id {
            let acquired_at = DateTime::parse_from_rfc3339(&acquired_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| now - STALE_AFTER - Duration::seconds(1));
            if now.signed_duration_since(acquired_at) < STALE_AFTER {
                return Ok(AcquireOutcome::HeldByOther(LockHolder {
                    device_id: other_device,
                    hostname: other_host,
                    acquired_at,
                }));
            }
        }
    }

    upsert_lock(pool, device_id, hostname, now).await?;
    Ok(AcquireOutcome::Acquired)
}

/// Übernimmt den Lock unbedingt — z.B. nachdem die UI den User gewarnt hat
/// und er „trotzdem öffnen" gewählt hat.
pub async fn force_acquire(
    pool: &SqlitePool,
    device_id: &str,
    hostname: &str,
) -> DbResult<()> {
    upsert_lock(pool, device_id, hostname, Utc::now()).await
}

/// Gibt den Lock frei, aber nur wenn er uns gehört.
pub async fn release(pool: &SqlitePool, device_id: &str) -> DbResult<()> {
    sqlx::query("DELETE FROM sync_lock WHERE id = 1 AND device_id = ?1")
        .bind(device_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Liest den aktuellen Lock-Halter aus, ohne ihn zu verändern.
pub async fn current_holder(pool: &SqlitePool) -> DbResult<Option<LockHolder>> {
    let row: Option<(String, String, String)> = sqlx::query_as(
        "SELECT device_id, hostname, acquired_at FROM sync_lock WHERE id = 1",
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.and_then(|(device_id, hostname, acquired_at_str)| {
        DateTime::parse_from_rfc3339(&acquired_at_str)
            .map(|dt| LockHolder {
                device_id,
                hostname,
                acquired_at: dt.with_timezone(&Utc),
            })
            .ok()
    }))
}

async fn upsert_lock(
    pool: &SqlitePool,
    device_id: &str,
    hostname: &str,
    at: DateTime<Utc>,
) -> DbResult<()> {
    let at_str = at.to_rfc3339();
    sqlx::query(
        "INSERT INTO sync_lock (id, device_id, hostname, acquired_at)
         VALUES (1, ?1, ?2, ?3)
         ON CONFLICT(id) DO UPDATE SET
            device_id = excluded.device_id,
            hostname = excluded.hostname,
            acquired_at = excluded.acquired_at",
    )
    .bind(device_id)
    .bind(hostname)
    .bind(at_str)
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_memory;

    #[tokio::test]
    async fn first_acquire_succeeds() {
        let pool = connect_memory().await.unwrap();
        let r = acquire(&pool, "dev-A", "host-A").await.unwrap();
        assert!(matches!(r, AcquireOutcome::Acquired));
    }

    #[tokio::test]
    async fn reacquire_by_same_device_succeeds() {
        let pool = connect_memory().await.unwrap();
        acquire(&pool, "dev-A", "host-A").await.unwrap();
        let r = acquire(&pool, "dev-A", "host-A").await.unwrap();
        assert!(matches!(r, AcquireOutcome::Acquired));
    }

    #[tokio::test]
    async fn fresh_other_lock_blocks() {
        let pool = connect_memory().await.unwrap();
        acquire(&pool, "dev-A", "host-A").await.unwrap();
        let r = acquire(&pool, "dev-B", "host-B").await.unwrap();
        match r {
            AcquireOutcome::HeldByOther(h) => {
                assert_eq!(h.device_id, "dev-A");
                assert_eq!(h.hostname, "host-A");
            }
            _ => panic!("expected HeldByOther"),
        }
    }

    #[tokio::test]
    async fn stale_other_lock_is_taken_over() {
        let pool = connect_memory().await.unwrap();
        let stale = Utc::now() - STALE_AFTER - Duration::hours(1);
        upsert_lock(&pool, "dev-A", "host-A", stale).await.unwrap();
        let r = acquire(&pool, "dev-B", "host-B").await.unwrap();
        assert!(matches!(r, AcquireOutcome::Acquired));
        let (owner,): (String,) =
            sqlx::query_as("SELECT device_id FROM sync_lock WHERE id = 1")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(owner, "dev-B");
    }

    #[tokio::test]
    async fn release_only_clears_own_lock() {
        let pool = connect_memory().await.unwrap();
        acquire(&pool, "dev-A", "host-A").await.unwrap();
        release(&pool, "dev-B").await.unwrap();
        let row: Option<(String,)> =
            sqlx::query_as("SELECT device_id FROM sync_lock WHERE id = 1")
                .fetch_optional(&pool)
                .await
                .unwrap();
        assert_eq!(row.unwrap().0, "dev-A");

        release(&pool, "dev-A").await.unwrap();
        let row: Option<(String,)> =
            sqlx::query_as("SELECT device_id FROM sync_lock WHERE id = 1")
                .fetch_optional(&pool)
                .await
                .unwrap();
        assert!(row.is_none());
    }

    #[tokio::test]
    async fn current_holder_returns_none_when_unlocked() {
        let pool = connect_memory().await.unwrap();
        let holder = current_holder(&pool).await.unwrap();
        assert!(holder.is_none());
    }

    #[tokio::test]
    async fn current_holder_returns_self_after_acquire() {
        let pool = connect_memory().await.unwrap();
        acquire(&pool, "dev-1", "host-x").await.unwrap();
        let holder = current_holder(&pool).await.unwrap().unwrap();
        assert_eq!(holder.device_id, "dev-1");
        assert_eq!(holder.hostname, "host-x");
    }

    #[tokio::test]
    async fn force_acquire_takes_over_fresh_lock() {
        let pool = connect_memory().await.unwrap();
        acquire(&pool, "dev-A", "host-A").await.unwrap();
        force_acquire(&pool, "dev-B", "host-B").await.unwrap();
        let (owner,): (String,) =
            sqlx::query_as("SELECT device_id FROM sync_lock WHERE id = 1")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(owner, "dev-B");
    }
}
