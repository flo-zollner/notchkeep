use tauri::State;

use crate::commands::accounts::{CommandError, DbState};
use crate::db::breakdowns::{self as db_breakdowns, BreakdownRowInput};
use crate::model::SecurityBreakdown;

const ALLOWED_DIMENSIONS: &[&str] = &["country", "sector"];

fn validate_dimension(d: &str) -> Result<(), CommandError> {
    if ALLOWED_DIMENSIONS.contains(&d) {
        Ok(())
    } else {
        Err(CommandError {
            message: format!("dimension must be one of {ALLOWED_DIMENSIONS:?}, got {d:?}"),
        })
    }
}

#[tauri::command]
pub async fn get_breakdown(
    state: State<'_, DbState>,
    security_id: i64,
    dimension: String,
) -> Result<Vec<SecurityBreakdown>, CommandError> {
    validate_dimension(&dimension)?;
    Ok(db_breakdowns::get_breakdown(&state.pool(), security_id, &dimension).await?)
}

#[tauri::command]
pub async fn set_breakdown(
    state: State<'_, DbState>,
    security_id: i64,
    dimension: String,
    rows: Vec<BreakdownRowInput>,
) -> Result<(), CommandError> {
    validate_dimension(&dimension)?;
    Ok(db_breakdowns::set_breakdown(&state.pool(), security_id, &dimension, &rows).await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_dimension_accepts_whitelist() {
        assert!(validate_dimension("country").is_ok());
        assert!(validate_dimension("sector").is_ok());
        assert!(validate_dimension("industry").is_err());
        assert!(validate_dimension("").is_err());
    }
}
