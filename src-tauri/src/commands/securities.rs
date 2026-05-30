use tauri::State;

use crate::commands::accounts::{CommandError, DbState};
use crate::db::securities::{self as db_securities, NewSecurityPayload, UpdateSecurityPayload};
use crate::model::Security;

const ALLOWED_ASSET_TYPES: &[&str] = &[
    "stock",
    "etf_equity",
    "etf_bond",
    "etf_reit",
    "bond",
    "crypto",
    "other",
];

fn normalize_isin(input: &str) -> Result<String, CommandError> {
    let cleaned: String = input
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_uppercase();
    if cleaned.len() != 12 {
        return Err(CommandError {
            message: format!("isin must be 12 chars, got {}", cleaned.len()),
        });
    }
    let bytes = cleaned.as_bytes();
    let country_ok = bytes[0].is_ascii_uppercase() && bytes[1].is_ascii_uppercase();
    let body_ok = bytes[2..11]
        .iter()
        .all(|c| c.is_ascii_alphanumeric() && !c.is_ascii_lowercase());
    let check_ok = bytes[11].is_ascii_digit();
    if !(country_ok && body_ok && check_ok) {
        return Err(CommandError {
            message: format!("isin format invalid: {cleaned}"),
        });
    }
    Ok(cleaned)
}

fn validate_asset_type(t: &str) -> Result<(), CommandError> {
    if ALLOWED_ASSET_TYPES.contains(&t) {
        Ok(())
    } else {
        Err(CommandError {
            message: format!("asset_type must be one of {ALLOWED_ASSET_TYPES:?}, got {t:?}"),
        })
    }
}

#[tauri::command]
pub async fn list_securities(
    state: State<'_, DbState>,
    include_archived: Option<bool>,
) -> Result<Vec<Security>, CommandError> {
    Ok(db_securities::list_securities(&state.pool(), include_archived.unwrap_or(false)).await?)
}

#[tauri::command]
pub async fn get_security(state: State<'_, DbState>, id: i64) -> Result<Security, CommandError> {
    Ok(db_securities::get_security(&state.pool(), id).await?)
}

#[tauri::command]
pub async fn create_security(
    state: State<'_, DbState>,
    payload: NewSecurityPayload,
) -> Result<Security, CommandError> {
    if payload.name.trim().is_empty() {
        return Err(CommandError {
            message: "name must not be empty".into(),
        });
    }
    let isin = normalize_isin(&payload.isin)?;
    validate_asset_type(&payload.asset_type)?;
    let p = NewSecurityPayload { isin, ..payload };
    let sec = db_securities::create_security(&state.pool(), p).await?;

    // Spawn background fetch of historical prices for newly created security
    let pool_for_bg = state.pool();
    tauri::async_runtime::spawn(async move {
        let provider = crate::pricing_provider::yahoo::YahooProvider::new();
        if let Err(e) = crate::db::portfolio::refresh_all_prices(&pool_for_bg, &provider).await {
            eprintln!("[notchkeep] history fetch after create_security failed: {e}");
        }
    });

    Ok(sec)
}

#[tauri::command]
pub async fn update_security(
    state: State<'_, DbState>,
    id: i64,
    payload: UpdateSecurityPayload,
) -> Result<Security, CommandError> {
    if let Some(n) = &payload.name {
        if n.trim().is_empty() {
            return Err(CommandError {
                message: "name must not be empty".into(),
            });
        }
    }
    let isin = match &payload.isin {
        Some(raw) => Some(normalize_isin(raw)?),
        None => None,
    };
    if let Some(t) = &payload.asset_type {
        validate_asset_type(t)?;
    }
    let p = UpdateSecurityPayload { isin, ..payload };
    Ok(db_securities::update_security(&state.pool(), id, p).await?)
}

#[tauri::command]
pub async fn delete_security(state: State<'_, DbState>, id: i64) -> Result<bool, CommandError> {
    Ok(db_securities::delete_security(&state.pool(), id).await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_isin_uppercases_and_strips_whitespace() {
        assert_eq!(normalize_isin(" ie00 bk5b qt80 ").unwrap(), "IE00BK5BQT80");
    }

    #[test]
    fn normalize_isin_rejects_wrong_length() {
        assert!(normalize_isin("DE0007164").is_err());
        assert!(normalize_isin("DE00071646001234").is_err());
    }

    #[test]
    fn normalize_isin_rejects_bad_country_or_check() {
        assert!(normalize_isin("123456789012").is_err()); // no country letters
        assert!(normalize_isin("DE000716460X").is_err()); // check digit not numeric
    }

    #[test]
    fn validate_asset_type_accepts_whitelist() {
        for t in ALLOWED_ASSET_TYPES {
            assert!(validate_asset_type(t).is_ok());
        }
        assert!(validate_asset_type("derivative").is_err());
    }
}
