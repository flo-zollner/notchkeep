use tauri::State;

use crate::commands::accounts::{CommandError, DbState};
use crate::db::recurring::{
    self as db_recurring,
    DetectedRecurring, NewRecurringPayload, RecurringOverview,
    RecurringPayment, UpdateRecurringPayload,
};

fn validate_date(s: &str) -> Result<(), CommandError> {
    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map(|_| ())
        .map_err(|e| CommandError {
            message: format!("invalid date {s:?}: {e}"),
        })
}

#[tauri::command]
pub async fn list_recurring(
    state: State<'_, DbState>,
    include_archived: bool,
) -> Result<Vec<RecurringPayment>, CommandError> {
    Ok(db_recurring::list_recurring(&state.pool(), include_archived).await?)
}

#[tauri::command]
pub async fn get_recurring(
    state: State<'_, DbState>,
    id: i64,
) -> Result<RecurringPayment, CommandError> {
    Ok(db_recurring::get_recurring(&state.pool(), id).await?)
}

#[tauri::command]
pub async fn create_recurring(
    state: State<'_, DbState>,
    payload: NewRecurringPayload,
) -> Result<RecurringPayment, CommandError> {
    validate_date(&payload.anchor_date)?;
    Ok(db_recurring::create_recurring(&state.pool(), payload).await?)
}

#[tauri::command]
pub async fn update_recurring(
    state: State<'_, DbState>,
    id: i64,
    payload: UpdateRecurringPayload,
) -> Result<RecurringPayment, CommandError> {
    if let Some(d) = payload.anchor_date.as_deref() {
        validate_date(d)?;
    }
    Ok(db_recurring::update_recurring(&state.pool(), id, payload).await?)
}

#[tauri::command]
pub async fn delete_recurring(
    state: State<'_, DbState>,
    id: i64,
) -> Result<bool, CommandError> {
    Ok(db_recurring::delete_recurring(&state.pool(), id).await?)
}

#[tauri::command]
pub async fn recurring_overview(
    state: State<'_, DbState>,
    months_ahead: u32,
) -> Result<Vec<RecurringOverview>, CommandError> {
    Ok(db_recurring::recurring_overview(&state.pool(), months_ahead).await?)
}

#[tauri::command]
pub async fn detect_recurring(
    state: State<'_, DbState>,
) -> Result<Vec<DetectedRecurring>, CommandError> {
    Ok(db_recurring::detect_recurring(&state.pool()).await?)
}
