use tauri::State;

use crate::commands::accounts::{CommandError, DbState};
use crate::db::bucket_allocations::{self as db_alloc, BucketAllocation, NewBucketAllocationPayload};

#[tauri::command]
pub async fn ready_to_assign(state: State<'_, DbState>) -> Result<i64, CommandError> {
    Ok(db_alloc::ready_to_assign(&state.pool()).await?)
}

#[tauri::command]
pub async fn list_bucket_allocations(
    state: State<'_, DbState>,
    bucket_id: Option<i64>,
) -> Result<Vec<BucketAllocation>, CommandError> {
    Ok(db_alloc::list_allocations(&state.pool(), bucket_id).await?)
}

#[tauri::command]
pub async fn create_bucket_allocation(
    state: State<'_, DbState>,
    payload: NewBucketAllocationPayload,
) -> Result<BucketAllocation, CommandError> {
    Ok(db_alloc::create_allocation(&state.pool(), payload).await?)
}

#[tauri::command]
pub async fn move_between_buckets(
    state: State<'_, DbState>,
    from_bucket: i64,
    to_bucket: i64,
    amount_cents: i64,
    occurred_on: Option<String>,
) -> Result<(), CommandError> {
    db_alloc::move_between_buckets(&state.pool(), from_bucket, to_bucket, amount_cents, occurred_on)
        .await?;
    Ok(())
}
