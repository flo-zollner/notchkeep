use tauri::State;

use crate::commands::accounts::{CommandError, DbState};
use crate::db::bucket_rules as db_br;
use crate::db::bucket_rules::{BucketRule, NewBucketRulePayload};

#[tauri::command]
pub async fn list_bucket_rules(state: State<'_, DbState>) -> Result<Vec<BucketRule>, CommandError> {
    Ok(db_br::list_bucket_rules(&state.pool()).await?)
}

#[tauri::command]
pub async fn create_bucket_rule(
    state: State<'_, DbState>,
    payload: NewBucketRulePayload,
) -> Result<i64, CommandError> {
    Ok(db_br::create_bucket_rule(&state.pool(), &payload).await?)
}

#[tauri::command]
pub async fn update_bucket_rule(
    state: State<'_, DbState>,
    rule: BucketRule,
) -> Result<(), CommandError> {
    Ok(db_br::update_bucket_rule(&state.pool(), &rule).await?)
}

#[tauri::command]
pub async fn delete_bucket_rule(state: State<'_, DbState>, id: i64) -> Result<(), CommandError> {
    Ok(db_br::delete_bucket_rule(&state.pool(), id).await?)
}

#[tauri::command]
pub async fn apply_bucket_rules_now(
    state: State<'_, DbState>,
    days: u32,
) -> Result<usize, CommandError> {
    // Clamp the lookback window: an unbounded `days` (e.g. u32::MAX) would
    // overflow the chrono date subtraction in the DB layer and trigger a
    // full-table scan. 10 years is well beyond any real use.
    if days == 0 || days > 3650 {
        return Err(CommandError {
            message: "days must be 1..=3650".into(),
        });
    }
    Ok(db_br::apply_bucket_rules_to_recent_income(&state.pool(), days).await?)
}
