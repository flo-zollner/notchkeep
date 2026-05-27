use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use super::{DbError, DbResult};
use crate::model::Goal;

use chrono::Datelike;

const GOAL_COLUMNS: &str =
    "id, name, category_id, target_cents, start_date, target_date, \
     icon, color, note, archived, created_at";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewGoalPayload {
    pub name: String,
    pub category_id: i64,
    pub target_cents: i64,
    /// If `None`, defaults to today (YYYY-MM-DD) when inserted.
    pub start_date: Option<String>,
    pub target_date: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateGoalPayload {
    pub name: Option<String>,
    pub category_id: Option<i64>,
    pub target_cents: Option<i64>,
    pub start_date: Option<String>,
    pub target_date: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub note: Option<String>,
    pub archived: Option<bool>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GoalProgress {
    pub goal_id: i64,
    pub current_cents: i64,
    pub monthly_avg_cents: i64,
    pub forecast_date: Option<String>,   // YYYY-MM-01
    pub on_track: Option<bool>,
}

pub async fn create_goal(pool: &SqlitePool, payload: NewGoalPayload) -> DbResult<Goal> {
    if payload.target_cents <= 0 {
        return Err(DbError::Decode("target_cents must be > 0".into()));
    }
    let start_date = payload
        .start_date
        .unwrap_or_else(|| chrono::Local::now().format("%Y-%m-%d").to_string());

    let sql = format!(
        "INSERT INTO goals
            (name, category_id, target_cents, start_date, target_date, icon, color, note)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         RETURNING {GOAL_COLUMNS}"
    );
    Ok(sqlx::query_as::<_, Goal>(&sql)
        .bind(payload.name)
        .bind(payload.category_id)
        .bind(payload.target_cents)
        .bind(start_date)
        .bind(normalize_opt(payload.target_date))
        .bind(normalize_opt(payload.icon))
        .bind(normalize_opt(payload.color))
        .bind(normalize_opt(payload.note))
        .fetch_one(pool)
        .await?)
}

pub async fn get_goal(pool: &SqlitePool, id: i64) -> DbResult<Goal> {
    let sql = format!("SELECT {GOAL_COLUMNS} FROM goals WHERE id = ?1");
    Ok(sqlx::query_as::<_, Goal>(&sql)
        .bind(id)
        .fetch_one(pool)
        .await?)
}

pub async fn update_goal(
    pool: &SqlitePool,
    id: i64,
    p: UpdateGoalPayload,
) -> DbResult<Goal> {
    if let Some(tc) = p.target_cents {
        if tc <= 0 {
            return Err(DbError::Decode("target_cents must be > 0".into()));
        }
    }
    // Existierendes Goal laden — wir machen partielle Updates via COALESCE.
    let current = get_goal(pool, id).await?;

    let sql = format!(
        "UPDATE goals SET
            name         = ?1,
            category_id  = ?2,
            target_cents = ?3,
            start_date   = ?4,
            target_date  = ?5,
            icon         = ?6,
            color        = ?7,
            note         = ?8,
            archived     = ?9
         WHERE id = ?10
         RETURNING {GOAL_COLUMNS}"
    );

    Ok(sqlx::query_as::<_, Goal>(&sql)
        .bind(p.name.unwrap_or(current.name))
        .bind(p.category_id.unwrap_or(current.category_id))
        .bind(p.target_cents.unwrap_or(current.target_cents))
        .bind(p.start_date.unwrap_or(current.start_date))
        .bind(normalize_opt(p.target_date).or(current.target_date))
        .bind(normalize_opt(p.icon).or(current.icon))
        .bind(normalize_opt(p.color).or(current.color))
        .bind(normalize_opt(p.note).or(current.note))
        .bind(p.archived.unwrap_or(current.archived))
        .bind(id)
        .fetch_one(pool)
        .await?)
}

pub async fn list_goals(pool: &SqlitePool, include_archived: bool) -> DbResult<Vec<Goal>> {
    let sql = if include_archived {
        format!(
            "SELECT {GOAL_COLUMNS} FROM goals \
             ORDER BY archived ASC, created_at DESC"
        )
    } else {
        format!(
            "SELECT {GOAL_COLUMNS} FROM goals \
             WHERE archived = 0 \
             ORDER BY created_at DESC"
        )
    };
    Ok(sqlx::query_as::<_, Goal>(&sql).fetch_all(pool).await?)
}

pub async fn delete_goal(pool: &SqlitePool, id: i64) -> DbResult<bool> {
    let res = sqlx::query("DELETE FROM goals WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}

/// Wandelt `Some("")` in `None` um — UI sendet leere Strings für „Feld leeren".
fn normalize_opt(v: Option<String>) -> Option<String> {
    v.and_then(|s| if s.trim().is_empty() { None } else { Some(s) })
}

pub async fn goal_progress(pool: &SqlitePool, id: i64) -> DbResult<GoalProgress> {
    let goal = get_goal(pool, id).await?;
    compute_progress(pool, &goal).await
}

pub async fn list_goal_progress(
    pool: &SqlitePool,
    include_archived: bool,
) -> DbResult<Vec<GoalProgress>> {
    let goals = list_goals(pool, include_archived).await?;
    let mut out = Vec::with_capacity(goals.len());
    for g in &goals {
        out.push(compute_progress(pool, g).await?);
    }
    Ok(out)
}

async fn compute_progress(pool: &SqlitePool, goal: &Goal) -> DbResult<GoalProgress> {
    // 1) currentCents: Summe positiver Tx der Kategorie ab start_date.
    let (current,): (Option<i64>,) = sqlx::query_as(
        "SELECT SUM(amount_cents) FROM transactions
         WHERE category_id = ?1
           AND amount_cents > 0
           AND kind != 'transfer'
           AND booking_date >= ?2",
    )
    .bind(goal.category_id)
    .bind(&goal.start_date)
    .fetch_one(pool)
    .await?;
    let current_cents = current.unwrap_or(0);

    // 2) monthlyAvg: Σ positiver Tx der Kategorie in den 6 vollen Kalendermonaten
    //    VOR dem aktuellen Monat, geteilt durch 6.
    let today = chrono::Local::now().date_naive();
    let (cy, cm) = (today.year(), today.month());
    let (start_y, start_m) = sub_months(cy, cm, 6);
    let (end_y, end_m) = (cy, cm);
    let window_start = format!("{start_y:04}-{start_m:02}-01");
    let window_end_exclusive = format!("{end_y:04}-{end_m:02}-01");

    let (window_sum,): (Option<i64>,) = sqlx::query_as(
        "SELECT SUM(amount_cents) FROM transactions
         WHERE category_id = ?1
           AND amount_cents > 0
           AND kind != 'transfer'
           AND booking_date >= ?2
           AND booking_date <  ?3",
    )
    .bind(goal.category_id)
    .bind(&window_start)
    .bind(&window_end_exclusive)
    .fetch_one(pool)
    .await?;
    let monthly_avg_cents = window_sum.unwrap_or(0) / 6;

    // 3) Forecast.
    let forecast_date = if current_cents >= goal.target_cents {
        Some(format!("{cy:04}-{cm:02}-01"))
    } else if monthly_avg_cents > 0 {
        let remaining = goal.target_cents - current_cents;
        let raw = (remaining + monthly_avg_cents - 1) / monthly_avg_cents;
        let months_needed = raw.clamp(0, i32::MAX as i64) as u32;
        let (fy, fm) = add_months(cy, cm, months_needed);
        Some(format!("{fy:04}-{fm:02}-01"))
    } else {
        None
    };

    // 4) onTrack
    let on_track = match (&goal.target_date, &forecast_date) {
        (Some(td), Some(fd)) => Some(fd.as_str() <= td.as_str()),
        _ => None,
    };

    Ok(GoalProgress {
        goal_id: goal.id,
        current_cents,
        monthly_avg_cents,
        forecast_date,
        on_track,
    })
}

fn sub_months(year: i32, month: u32, delta: u32) -> (i32, u32) {
    let total = year * 12 + (month as i32) - 1 - (delta as i32);
    (total.div_euclid(12), (total.rem_euclid(12) + 1) as u32)
}

fn add_months(year: i32, month: u32, delta: u32) -> (i32, u32) {
    let total = year * 12 + (month as i32) - 1 + (delta as i32);
    (total.div_euclid(12), (total.rem_euclid(12) + 1) as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::accounts as db_accounts;
    use crate::db::connect_memory;
    use crate::db::transactions as db_transactions;
    use crate::importers::RawTransaction;

    async fn seed_category(pool: &SqlitePool, name: &str) -> i64 {
        let (id,): (i64,) = sqlx::query_as(
            "INSERT INTO categories (name) VALUES (?1) RETURNING id",
        )
        .bind(name)
        .fetch_one(pool)
        .await
        .unwrap();
        id
    }

    async fn seed_account(pool: &SqlitePool) -> i64 {
        let a = db_accounts::create_account(pool, "TR", "bank", "EUR", None, None, None).await.unwrap();
        a.id
    }

    async fn insert_tx(
        pool: &SqlitePool,
        account_id: i64,
        date: &str,
        amount_cents: i64,
        category_id: Option<i64>,
    ) {
        let raw = RawTransaction {
            booking_date: chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap(),
            amount_cents,
            currency: "EUR".into(),
            counterparty: Some("seed".into()),
            purpose: None,
            raw_ref: None,
            kind: None,
            trade: None,
            counterparty_iban: None,
        };
        let outcome = db_transactions::insert_raw_transaction(pool, account_id, "test", None, &raw, category_id)
            .await
            .unwrap();
        match outcome {
            db_transactions::InsertOutcome::Inserted(_) => {}
            db_transactions::InsertOutcome::Skipped => panic!("duplicate seed"),
        }
    }

    #[tokio::test]
    async fn migration_creates_goals_table() {
        let pool = connect_memory().await.unwrap();
        let (count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM sqlite_master \
             WHERE type='table' AND name='goals'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count, 1, "goals table missing");
    }

    #[tokio::test]
    async fn create_then_get_goal_roundtrip_minimal() {
        let pool = connect_memory().await.unwrap();
        let cat = seed_category(&pool, "Sparen").await;
        let payload = NewGoalPayload {
            name: "Notgroschen".into(),
            category_id: cat,
            target_cents: 1_500_000,
            start_date: Some("2026-05-01".into()),
            target_date: None,
            icon: None,
            color: None,
            note: None,
        };
        let created = create_goal(&pool, payload).await.unwrap();
        assert!(created.id > 0);
        assert_eq!(created.name, "Notgroschen");
        assert_eq!(created.target_cents, 1_500_000);
        assert_eq!(created.start_date, "2026-05-01");
        assert!(created.target_date.is_none());
        assert!(!created.archived);

        let fetched = get_goal(&pool, created.id).await.unwrap();
        assert_eq!(fetched.id, created.id);
        assert_eq!(fetched.name, "Notgroschen");
    }

    #[tokio::test]
    async fn create_goal_default_start_date_is_today() {
        let pool = connect_memory().await.unwrap();
        let cat = seed_category(&pool, "Sparen").await;
        let created = create_goal(
            &pool,
            NewGoalPayload {
                name: "X".into(),
                category_id: cat,
                target_cents: 100,
                start_date: None,
                target_date: None,
                icon: None,
                color: None,
                note: None,
            },
        )
        .await
        .unwrap();
        // YYYY-MM-DD
        assert_eq!(created.start_date.len(), 10);
        assert_eq!(&created.start_date[4..5], "-");
        assert_eq!(&created.start_date[7..8], "-");
    }

    #[tokio::test]
    async fn update_goal_partial_keeps_unset_fields() {
        let pool = connect_memory().await.unwrap();
        let cat = seed_category(&pool, "Sparen").await;
        let cat2 = seed_category(&pool, "Urlaub").await;
        let g = create_goal(
            &pool,
            NewGoalPayload {
                name: "Notgroschen".into(),
                category_id: cat,
                target_cents: 1_000_000,
                start_date: Some("2026-01-01".into()),
                target_date: Some("2026-12-31".into()),
                icon: Some("piggy".into()),
                color: Some("var(--c1)".into()),
                note: Some("zur Sicherheit".into()),
            },
        )
        .await
        .unwrap();

        let updated = update_goal(
            &pool,
            g.id,
            UpdateGoalPayload {
                name: Some("Notgroschen XL".into()),
                category_id: None,
                target_cents: Some(2_000_000),
                start_date: None,
                target_date: None,
                icon: None,
                color: None,
                note: None,
                archived: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(updated.name, "Notgroschen XL");
        assert_eq!(updated.target_cents, 2_000_000);
        assert_eq!(updated.category_id, cat); // unverändert
        assert_eq!(updated.start_date, "2026-01-01"); // unverändert
        assert_eq!(updated.target_date.as_deref(), Some("2026-12-31"));
        assert_eq!(updated.icon.as_deref(), Some("piggy"));
        assert!(!updated.archived);
        // sanity: cat2 wurde nicht verwendet
        let _ = cat2;
    }

    #[tokio::test]
    async fn update_goal_can_archive_and_change_category() {
        let pool = connect_memory().await.unwrap();
        let cat = seed_category(&pool, "Sparen").await;
        let cat2 = seed_category(&pool, "Urlaub").await;
        let g = create_goal(
            &pool,
            NewGoalPayload {
                name: "X".into(),
                category_id: cat,
                target_cents: 100,
                start_date: None,
                target_date: None,
                icon: None,
                color: None,
                note: None,
            },
        )
        .await
        .unwrap();

        let updated = update_goal(
            &pool,
            g.id,
            UpdateGoalPayload {
                name: None,
                category_id: Some(cat2),
                target_cents: None,
                start_date: None,
                target_date: None,
                icon: None,
                color: None,
                note: None,
                archived: Some(true),
            },
        )
        .await
        .unwrap();

        assert_eq!(updated.category_id, cat2);
        assert!(updated.archived);
    }

    #[tokio::test]
    async fn list_goals_orders_active_first_then_archived() {
        let pool = connect_memory().await.unwrap();
        let cat = seed_category(&pool, "Sparen").await;
        let a = create_goal(
            &pool,
            NewGoalPayload {
                name: "Aktiv".into(),
                category_id: cat,
                target_cents: 100,
                start_date: None,
                target_date: None,
                icon: None, color: None, note: None,
            },
        ).await.unwrap();
        let b = create_goal(
            &pool,
            NewGoalPayload {
                name: "Archiviert".into(),
                category_id: cat,
                target_cents: 100,
                start_date: None,
                target_date: None,
                icon: None, color: None, note: None,
            },
        ).await.unwrap();
        update_goal(&pool, b.id, UpdateGoalPayload {
            name: None, category_id: None, target_cents: None,
            start_date: None, target_date: None,
            icon: None, color: None, note: None,
            archived: Some(true),
        }).await.unwrap();

        let without = list_goals(&pool, false).await.unwrap();
        assert_eq!(without.len(), 1);
        assert_eq!(without[0].id, a.id);

        let with_archived = list_goals(&pool, true).await.unwrap();
        assert_eq!(with_archived.len(), 2);
        // archived = 0 zuerst, dann archived = 1
        assert_eq!(with_archived[0].id, a.id);
        assert_eq!(with_archived[1].id, b.id);
    }

    #[tokio::test]
    async fn delete_goal_removes_row() {
        let pool = connect_memory().await.unwrap();
        let cat = seed_category(&pool, "Sparen").await;
        let g = create_goal(
            &pool,
            NewGoalPayload {
                name: "X".into(),
                category_id: cat,
                target_cents: 100,
                start_date: None,
                target_date: None,
                icon: None, color: None, note: None,
            },
        ).await.unwrap();
        assert!(delete_goal(&pool, g.id).await.unwrap());
        assert!(get_goal(&pool, g.id).await.is_err());
        // erneutes Löschen → false
        assert!(!delete_goal(&pool, g.id).await.unwrap());
    }

    #[tokio::test]
    async fn delete_category_with_goal_is_restricted() {
        // Mit ON DELETE RESTRICT muss das Löschen einer Kategorie fehlschlagen,
        // solange ein Goal sie referenziert.
        let pool = connect_memory().await.unwrap();
        let cat = seed_category(&pool, "Sparen").await;
        let _g = create_goal(
            &pool,
            NewGoalPayload {
                name: "X".into(),
                category_id: cat,
                target_cents: 100,
                start_date: None,
                target_date: None,
                icon: None, color: None, note: None,
            },
        ).await.unwrap();
        let res = sqlx::query("DELETE FROM categories WHERE id = ?1")
            .bind(cat)
            .execute(&pool)
            .await;
        assert!(res.is_err(), "expected RESTRICT to block category delete");
    }

    #[tokio::test]
    async fn create_goal_rejects_non_positive_target_cents() {
        let pool = connect_memory().await.unwrap();
        let cat = seed_category(&pool, "X").await;
        let err = create_goal(&pool, NewGoalPayload {
            name: "Bad".into(),
            category_id: cat,
            target_cents: 0,
            start_date: None,
            target_date: None,
            icon: None,
            color: None,
            note: None,
        }).await.unwrap_err();
        assert!(err.to_string().contains("target_cents"), "got: {err}");
    }

    #[tokio::test]
    async fn goal_progress_no_tx_is_zero() {
        let pool = connect_memory().await.unwrap();
        let cat = seed_category(&pool, "Sparen").await;
        let g = create_goal(
            &pool,
            NewGoalPayload {
                name: "X".into(),
                category_id: cat,
                target_cents: 100_00,
                start_date: Some("2026-01-01".into()),
                target_date: None,
                icon: None, color: None, note: None,
            },
        ).await.unwrap();

        let p = goal_progress(&pool, g.id).await.unwrap();
        assert_eq!(p.goal_id, g.id);
        assert_eq!(p.current_cents, 0);
        assert_eq!(p.monthly_avg_cents, 0);
        assert!(p.forecast_date.is_none());
        assert!(p.on_track.is_none());
    }

    #[tokio::test]
    async fn goal_progress_sums_only_positive_tx_after_start_date() {
        let pool = connect_memory().await.unwrap();
        let cat = seed_category(&pool, "Sparen").await;
        let other = seed_category(&pool, "Sonstiges").await;
        let acc = seed_account(&pool).await;

        insert_tx(&pool, acc, "2025-12-15", 50_00, Some(cat)).await;
        insert_tx(&pool, acc, "2026-01-15", 100_00, Some(cat)).await;
        insert_tx(&pool, acc, "2026-02-10", 200_00, Some(cat)).await;
        insert_tx(&pool, acc, "2026-02-20", -50_00, Some(cat)).await;
        insert_tx(&pool, acc, "2026-02-25", 999_00, Some(other)).await;

        let g = create_goal(
            &pool,
            NewGoalPayload {
                name: "X".into(),
                category_id: cat,
                target_cents: 1_000_00,
                start_date: Some("2026-01-01".into()),
                target_date: None,
                icon: None, color: None, note: None,
            },
        ).await.unwrap();

        let p = goal_progress(&pool, g.id).await.unwrap();
        assert_eq!(p.current_cents, 300_00, "100€ + 200€");
    }

    #[tokio::test]
    async fn goal_progress_monthly_avg_uses_last_6_full_months() {
        let pool = connect_memory().await.unwrap();
        let cat = seed_category(&pool, "Sparen").await;
        let acc = seed_account(&pool).await;

        // Today is May 2026. Last 6 full months = Nov 2025 .. Apr 2026.
        insert_tx(&pool, acc, "2026-02-15", 600_00, Some(cat)).await;
        insert_tx(&pool, acc, "2025-09-15", 900_00, Some(cat)).await;
        let today = chrono::Local::now().date_naive();
        let in_this_month = today.format("%Y-%m-05").to_string();
        insert_tx(&pool, acc, &in_this_month, 1200_00, Some(cat)).await;
        insert_tx(&pool, acc, "2026-03-15", -300_00, Some(cat)).await;

        let g = create_goal(
            &pool,
            NewGoalPayload {
                name: "X".into(),
                category_id: cat,
                target_cents: 10_000_00,
                start_date: Some("2024-01-01".into()),
                target_date: None,
                icon: None, color: None, note: None,
            },
        ).await.unwrap();

        let p = goal_progress(&pool, g.id).await.unwrap();
        assert_eq!(p.monthly_avg_cents, 100_00, "600€ / 6 = 100€");
    }

    #[tokio::test]
    async fn goal_progress_forecast_and_on_track() {
        let pool = connect_memory().await.unwrap();
        let cat = seed_category(&pool, "Sparen").await;
        let acc = seed_account(&pool).await;
        insert_tx(&pool, acc, "2026-02-15", 600_00, Some(cat)).await;

        let g = create_goal(
            &pool,
            NewGoalPayload {
                name: "X".into(),
                category_id: cat,
                target_cents: 500_00,
                start_date: Some("2099-01-01".into()),
                target_date: Some("2099-12-01".into()),
                icon: None, color: None, note: None,
            },
        ).await.unwrap();

        let p = goal_progress(&pool, g.id).await.unwrap();
        assert_eq!(p.current_cents, 0);
        assert_eq!(p.monthly_avg_cents, 100_00);
        let fc = p.forecast_date.expect("forecast set");
        assert_eq!(fc.len(), 10);
        assert_eq!(&fc[8..10], "01");
        assert_eq!(p.on_track, Some(true));
    }

    #[tokio::test]
    async fn goal_progress_on_track_false_when_target_date_too_early() {
        let pool = connect_memory().await.unwrap();
        let cat = seed_category(&pool, "Sparen").await;
        let acc = seed_account(&pool).await;
        insert_tx(&pool, acc, "2026-02-15", 600_00, Some(cat)).await;

        let g = create_goal(
            &pool,
            NewGoalPayload {
                name: "X".into(),
                category_id: cat,
                target_cents: 500_00,
                start_date: Some("2099-01-01".into()),
                target_date: Some("2026-06-01".into()),
                icon: None, color: None, note: None,
            },
        ).await.unwrap();

        let p = goal_progress(&pool, g.id).await.unwrap();
        assert_eq!(p.on_track, Some(false));
    }

    #[tokio::test]
    async fn goal_progress_reached_forecast_is_this_month() {
        let pool = connect_memory().await.unwrap();
        let cat = seed_category(&pool, "Sparen").await;
        let acc = seed_account(&pool).await;
        insert_tx(&pool, acc, "2026-04-15", 100_00, Some(cat)).await;

        let g = create_goal(
            &pool,
            NewGoalPayload {
                name: "X".into(),
                category_id: cat,
                target_cents: 50_00,
                start_date: Some("2026-01-01".into()),
                target_date: Some("2026-12-01".into()),
                icon: None, color: None, note: None,
            },
        ).await.unwrap();

        let p = goal_progress(&pool, g.id).await.unwrap();
        let today = chrono::Local::now().date_naive();
        assert_eq!(
            p.forecast_date.as_deref(),
            Some(today.format("%Y-%m-01").to_string().as_str())
        );
        assert_eq!(p.on_track, Some(true));
    }

    #[tokio::test]
    async fn goal_progress_zero_rate_no_forecast() {
        let pool = connect_memory().await.unwrap();
        let cat = seed_category(&pool, "Sparen").await;
        let g = create_goal(
            &pool,
            NewGoalPayload {
                name: "X".into(),
                category_id: cat,
                target_cents: 100_00,
                start_date: Some("2099-01-01".into()),
                target_date: Some("2099-12-01".into()),
                icon: None, color: None, note: None,
            },
        ).await.unwrap();

        let p = goal_progress(&pool, g.id).await.unwrap();
        assert_eq!(p.monthly_avg_cents, 0);
        assert!(p.forecast_date.is_none());
        assert!(p.on_track.is_none());
    }

    #[test]
    fn sub_months_within_year() {
        assert_eq!(sub_months(2026, 5, 6), (2025, 11));
    }

    #[test]
    fn sub_months_crosses_year_boundary() {
        assert_eq!(sub_months(2026, 1, 1), (2025, 12));
        assert_eq!(sub_months(2026, 3, 14), (2025, 1));
    }

    #[test]
    fn add_months_within_year() {
        assert_eq!(add_months(2026, 5, 7), (2026, 12));
    }

    #[test]
    fn add_months_crosses_year_boundary() {
        assert_eq!(add_months(2026, 12, 1), (2027, 1));
        assert_eq!(add_months(2026, 5, 19), (2027, 12));
    }

    #[tokio::test]
    async fn list_goal_progress_respects_include_archived() {
        let pool = connect_memory().await.unwrap();
        let cat = seed_category(&pool, "Sparen").await;
        let acc = seed_account(&pool).await;
        insert_tx(&pool, acc, "2026-02-15", 600_00, Some(cat)).await;

        let active = create_goal(
            &pool,
            NewGoalPayload {
                name: "Aktiv".into(),
                category_id: cat,
                target_cents: 1_000_00,
                start_date: Some("2026-01-01".into()),
                target_date: None,
                icon: None, color: None, note: None,
            },
        ).await.unwrap();
        let archived = create_goal(
            &pool,
            NewGoalPayload {
                name: "Archiv".into(),
                category_id: cat,
                target_cents: 1_000_00,
                start_date: Some("2026-01-01".into()),
                target_date: None,
                icon: None, color: None, note: None,
            },
        ).await.unwrap();
        update_goal(&pool, archived.id, UpdateGoalPayload {
            name: None, category_id: None, target_cents: None,
            start_date: None, target_date: None,
            icon: None, color: None, note: None,
            archived: Some(true),
        }).await.unwrap();

        let without = list_goal_progress(&pool, false).await.unwrap();
        assert_eq!(without.len(), 1);
        assert_eq!(without[0].goal_id, active.id);
        assert_eq!(without[0].current_cents, 600_00);

        let with_archived = list_goal_progress(&pool, true).await.unwrap();
        assert_eq!(with_archived.len(), 2);
    }

    /// Transfers (kind='transfer') must not be counted in goal progress.
    #[tokio::test]
    async fn goal_progress_ignores_transfer_kind() {
        let pool = connect_memory().await.unwrap();
        let cat = seed_category(&pool, "TransferCat").await;
        let acc = seed_account(&pool).await;

        // Normal positive Tx — counts toward goal.
        insert_tx(&pool, acc, "2026-05-10", 200_00, Some(cat)).await;

        // Transfer — must NOT count toward goal.
        sqlx::query(
            "INSERT INTO transactions
             (account_id, booking_date, amount_cents, currency, category_id, kind, source, imported_at)
             VALUES (?1, '2026-05-15', 500_00, 'EUR', ?2, 'transfer', 'manual', '2026-05-19T00:00:00Z')",
        )
        .bind(acc)
        .bind(cat)
        .execute(&pool)
        .await
        .unwrap();

        let g = create_goal(
            &pool,
            NewGoalPayload {
                name: "Sparziel".into(),
                category_id: cat,
                target_cents: 1_000_00,
                start_date: Some("2026-01-01".into()),
                target_date: None,
                icon: None,
                color: None,
                note: None,
            },
        )
        .await
        .unwrap();

        let p = goal_progress(&pool, g.id).await.unwrap();
        // Only the 200_00 regular Tx should count; transfer (500_00) must be excluded.
        assert_eq!(p.current_cents, 200_00);
    }
}
