use tauri::State;

use crate::commands::accounts::{CommandError, DbState};
use crate::db::buckets::{
    self as db_buckets, BucketProgress, NewBucketPayload, UpdateBucketPayload,
};
use crate::model::Bucket;

#[tauri::command]
pub async fn list_buckets(
    state: State<'_, DbState>,
    include_archived: Option<bool>,
) -> Result<Vec<Bucket>, CommandError> {
    Ok(db_buckets::list_buckets(&state.pool(), include_archived.unwrap_or(false)).await?)
}

#[tauri::command]
pub async fn get_bucket(state: State<'_, DbState>, id: i64) -> Result<Bucket, CommandError> {
    Ok(db_buckets::get_bucket(&state.pool(), id).await?)
}

#[tauri::command]
pub async fn create_bucket(
    state: State<'_, DbState>,
    payload: NewBucketPayload,
) -> Result<Bucket, CommandError> {
    if payload.name.trim().is_empty() {
        return Err(CommandError {
            message: "name must not be empty".into(),
        });
    }
    if let Some(tc) = payload.target_cents {
        if tc < 0 {
            return Err(CommandError {
                message: "target_cents must be >= 0".into(),
            });
        }
    }
    validate_date_opt("start_date", payload.start_date.as_deref())?;
    validate_date_opt("target_date", payload.target_date.as_deref())?;
    if let (Some(s), Some(t)) = (&payload.start_date, &payload.target_date) {
        if t.as_str() < s.as_str() {
            return Err(CommandError {
                message: "target_date must be >= start_date".into(),
            });
        }
    }
    Ok(db_buckets::create_bucket(&state.pool(), payload).await?)
}

#[tauri::command]
pub async fn update_bucket(
    state: State<'_, DbState>,
    id: i64,
    payload: UpdateBucketPayload,
) -> Result<Bucket, CommandError> {
    if let Some(n) = &payload.name {
        if n.trim().is_empty() {
            return Err(CommandError {
                message: "name must not be empty".into(),
            });
        }
    }
    if let Some(tc) = payload.target_cents {
        if tc < 0 {
            return Err(CommandError {
                message: "target_cents must be >= 0".into(),
            });
        }
    }
    validate_date_opt("start_date", payload.start_date.as_deref())?;
    validate_date_opt("target_date", payload.target_date.as_deref())?;
    Ok(db_buckets::update_bucket(&state.pool(), id, payload).await?)
}

#[tauri::command]
pub async fn delete_bucket(state: State<'_, DbState>, id: i64) -> Result<bool, CommandError> {
    Ok(db_buckets::delete_bucket(&state.pool(), id).await?)
}

#[tauri::command]
pub async fn bucket_balance(state: State<'_, DbState>, id: i64) -> Result<i64, CommandError> {
    Ok(db_buckets::bucket_balance(&state.pool(), id).await?)
}

#[tauri::command]
pub async fn list_bucket_progress(
    state: State<'_, DbState>,
) -> Result<Vec<BucketProgress>, CommandError> {
    Ok(db_buckets::list_bucket_progress(&state.pool()).await?)
}

fn validate_date_opt(field: &str, value: Option<&str>) -> Result<(), CommandError> {
    let Some(s) = value else { return Ok(()) };
    if s.is_empty() {
        return Ok(());
    }
    if s.len() != 10 || &s[4..5] != "-" || &s[7..8] != "-" {
        return Err(CommandError {
            message: format!("{field} must be YYYY-MM-DD, got {s:?}"),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_memory;

    #[tokio::test]
    async fn create_bucket_command_round_trip() {
        let pool = connect_memory().await.unwrap();
        // State<'_> cannot be easily instantiated in cargo-test; use db_buckets directly instead.
        let p = NewBucketPayload {
            name: "x".into(),
            icon: None,
            color: None,
            note: None,
            target_cents: None,
            start_date: None,
            target_date: None,
        };
        let b = db_buckets::create_bucket(&pool, p).await.unwrap();
        assert_eq!(b.name, "x");
    }

    #[test]
    fn validate_date_opt_accepts_iso() {
        validate_date_opt("d", Some("2026-05-17")).unwrap();
        validate_date_opt("d", None).unwrap();
        validate_date_opt("d", Some("")).unwrap();
    }

    #[test]
    fn validate_date_opt_rejects_bad_format() {
        assert!(validate_date_opt("d", Some("2026/05/17")).is_err());
        assert!(validate_date_opt("d", Some("17.05.2026")).is_err());
    }
}
