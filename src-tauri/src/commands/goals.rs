use tauri::State;

use crate::commands::accounts::{CommandError, DbState};
use crate::db::goals::{
    self as db_goals, GoalProgress, NewGoalPayload, UpdateGoalPayload,
};
use crate::model::Goal;

#[tauri::command]
pub async fn list_goals(
    state: State<'_, DbState>,
    include_archived: Option<bool>,
) -> Result<Vec<Goal>, CommandError> {
    Ok(db_goals::list_goals(&state.pool(), include_archived.unwrap_or(false)).await?)
}

#[tauri::command]
pub async fn get_goal(
    state: State<'_, DbState>,
    id: i64,
) -> Result<Goal, CommandError> {
    Ok(db_goals::get_goal(&state.pool(), id).await?)
}

#[tauri::command]
pub async fn create_goal(
    state: State<'_, DbState>,
    payload: NewGoalPayload,
) -> Result<Goal, CommandError> {
    if payload.name.trim().is_empty() {
        return Err(CommandError { message: "name must not be empty".into() });
    }
    if payload.target_cents <= 0 {
        return Err(CommandError { message: "target_cents must be > 0".into() });
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
    Ok(db_goals::create_goal(&state.pool(), payload).await?)
}

#[tauri::command]
pub async fn update_goal(
    state: State<'_, DbState>,
    id: i64,
    payload: UpdateGoalPayload,
) -> Result<Goal, CommandError> {
    if let Some(n) = &payload.name {
        if n.trim().is_empty() {
            return Err(CommandError { message: "name must not be empty".into() });
        }
    }
    if let Some(tc) = payload.target_cents {
        if tc <= 0 {
            return Err(CommandError { message: "target_cents must be > 0".into() });
        }
    }
    validate_date_opt("start_date", payload.start_date.as_deref())?;
    validate_date_opt("target_date", payload.target_date.as_deref())?;
    Ok(db_goals::update_goal(&state.pool(), id, payload).await?)
}

#[tauri::command]
pub async fn delete_goal(
    state: State<'_, DbState>,
    id: i64,
) -> Result<bool, CommandError> {
    Ok(db_goals::delete_goal(&state.pool(), id).await?)
}

#[tauri::command]
pub async fn goal_progress(
    state: State<'_, DbState>,
    id: i64,
) -> Result<GoalProgress, CommandError> {
    Ok(db_goals::goal_progress(&state.pool(), id).await?)
}

#[tauri::command]
pub async fn list_goal_progress(
    state: State<'_, DbState>,
    include_archived: Option<bool>,
) -> Result<Vec<GoalProgress>, CommandError> {
    Ok(db_goals::list_goal_progress(&state.pool(), include_archived.unwrap_or(false)).await?)
}

fn validate_date_opt(field: &str, value: Option<&str>) -> Result<(), CommandError> {
    let Some(s) = value else { return Ok(()) };
    if s.len() != 10 || &s[4..5] != "-" || &s[7..8] != "-" {
        return Err(CommandError {
            message: format!("{field} must be YYYY-MM-DD, got {s:?}"),
        });
    }
    Ok(())
}
