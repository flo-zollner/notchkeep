use tauri::State;

use crate::commands::accounts::{CommandError, DbState};
use crate::db::institutions::{
    self as db_inst, NewInstitutionPayload, UpdateInstitutionPayload,
};
use crate::model::{Institution, InstitutionSummary};

#[tauri::command]
pub async fn list_institutions(
    state: State<'_, DbState>,
    include_archived: Option<bool>,
) -> Result<Vec<Institution>, CommandError> {
    Ok(db_inst::list_institutions(&state.pool(), include_archived.unwrap_or(false)).await?)
}

#[tauri::command]
pub async fn list_institutions_with_summary(
    state: State<'_, DbState>,
) -> Result<Vec<InstitutionSummary>, CommandError> {
    Ok(db_inst::list_institutions_with_summary(&state.pool()).await?)
}

#[tauri::command]
pub async fn get_institution(
    state: State<'_, DbState>,
    id: i64,
) -> Result<Institution, CommandError> {
    Ok(db_inst::get_institution(&state.pool(), id).await?)
}

fn normalize_str_uppercase(v: &mut Option<String>) {
    if let Some(s) = v {
        let cleaned: String = s.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_uppercase();
        *v = if cleaned.is_empty() { None } else { Some(cleaned) };
    }
}

fn normalize_payload(p: &mut NewInstitutionPayload) {
    p.name = p.name.trim().to_string();
    normalize_str_uppercase(&mut p.bic);
    normalize_str_uppercase(&mut p.country);
}

fn normalize_str_uppercase_inner(v: &mut Option<Option<String>>) {
    if let Some(Some(s)) = v {
        let cleaned: String = s.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_uppercase();
        // Leer nach Trim → explizit löschen (Some(None)); not-present bleibt unverändert.
        *v = Some(if cleaned.is_empty() { None } else { Some(cleaned) });
    }
    // None outer = preserve; Some(None) = clear → keine Änderung nötig.
}

fn normalize_update_payload(p: &mut UpdateInstitutionPayload) {
    if let Some(n) = &mut p.name { *n = n.trim().to_string(); }
    normalize_str_uppercase_inner(&mut p.bic);
    normalize_str_uppercase_inner(&mut p.country);
}

fn validate_bic(bic: Option<&str>) -> Result<(), CommandError> {
    let Some(s) = bic else { return Ok(()); };
    if !(s.len() == 8 || s.len() == 11) {
        return Err(CommandError { message: format!("bic must be 8 or 11 chars, got {}", s.len()) });
    }
    if !s.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()) {
        return Err(CommandError { message: "bic must be A-Z, 0-9 only".into() });
    }
    Ok(())
}

fn validate_country(country: Option<&str>) -> Result<(), CommandError> {
    let Some(s) = country else { return Ok(()); };
    if s.len() != 2 || !s.chars().all(|c| c.is_ascii_uppercase()) {
        return Err(CommandError { message: "country must be 2 uppercase letters (ISO 3166-1)".into() });
    }
    Ok(())
}

fn validate_new_payload(p: &NewInstitutionPayload) -> Result<(), CommandError> {
    if p.name.trim().is_empty() {
        return Err(CommandError { message: "name must not be empty".into() });
    }
    validate_bic(p.bic.as_deref())?;
    validate_country(p.country.as_deref())?;
    Ok(())
}

#[tauri::command]
pub async fn create_institution(
    state: State<'_, DbState>,
    payload: NewInstitutionPayload,
) -> Result<Institution, CommandError> {
    let mut p = payload;
    normalize_payload(&mut p);
    validate_new_payload(&p)?;
    Ok(db_inst::create_institution(&state.pool(), p).await?)
}

fn validate_update_payload(p: &UpdateInstitutionPayload) -> Result<(), CommandError> {
    if let Some(n) = &p.name {
        if n.trim().is_empty() {
            return Err(CommandError { message: "name must not be empty".into() });
        }
    }
    validate_bic(p.bic.as_ref().and_then(|o| o.as_deref()))?;
    validate_country(p.country.as_ref().and_then(|o| o.as_deref()))?;
    Ok(())
}

#[tauri::command]
pub async fn update_institution(
    state: State<'_, DbState>,
    id: i64,
    payload: UpdateInstitutionPayload,
) -> Result<Institution, CommandError> {
    let mut p = payload;
    normalize_update_payload(&mut p);
    validate_update_payload(&p)?;
    Ok(db_inst::update_institution(&state.pool(), id, p).await?)
}

/// Inner-Helper: testbar ohne Tauri-State.
pub(crate) async fn delete_institution_inner(
    pool: &sqlx::SqlitePool,
    id: i64,
) -> Result<(), CommandError> {
    let count = db_inst::institution_account_count(pool, id).await?;
    if count > 0 {
        return Err(CommandError {
            message: format!("institution-has-accounts:{count}"),
        });
    }
    db_inst::delete_institution(pool, id).await?;
    Ok(())
}

#[tauri::command]
pub async fn delete_institution(
    state: State<'_, DbState>,
    id: i64,
) -> Result<(), CommandError> {
    delete_institution_inner(&state.pool(), id).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_memory;
    use crate::db::institutions::create_institution;

    #[tokio::test]
    async fn list_get_round_trip_via_db_layer() {
        // State<'_> nicht direkt instanziierbar; gegen db_inst testen.
        let pool = connect_memory().await.unwrap();
        let inst = create_institution(&pool, NewInstitutionPayload {
            name: "X".into(), icon: None, color: None, bic: None, country: None, note: None,
        }).await.unwrap();
        let listed = db_inst::list_institutions(&pool, false).await.unwrap();
        assert_eq!(listed.len(), 1);
        let got = db_inst::get_institution(&pool, inst.id).await.unwrap();
        assert_eq!(got.id, inst.id);
    }

    fn payload(name: &str) -> NewInstitutionPayload {
        NewInstitutionPayload {
            name: name.into(), icon: None, color: None, bic: None, country: None, note: None,
        }
    }

    #[tokio::test]
    async fn create_command_validates_empty_name() {
        let err = validate_new_payload(&payload("  ")).unwrap_err();
        assert!(err.message.contains("name"));
    }

    #[tokio::test]
    async fn create_command_validates_bic_lowercase() {
        let mut p = payload("X");
        p.bic = Some("traddeff".into());
        let err = validate_new_payload(&p).unwrap_err();
        assert!(err.message.contains("bic"));
    }

    #[tokio::test]
    async fn create_command_validates_bic_invalid_length() {
        let mut p = payload("X");
        p.bic = Some("ABC".into());
        let err = validate_new_payload(&p).unwrap_err();
        assert!(err.message.contains("bic"));
    }

    #[tokio::test]
    async fn create_command_accepts_8_and_11_char_bic() {
        let mut p = payload("X");
        p.bic = Some("TRADDEFF".into());
        validate_new_payload(&p).unwrap();
        let mut p11 = payload("Y");
        p11.bic = Some("TRADDEFFXXX".into());
        validate_new_payload(&p11).unwrap();
    }

    #[tokio::test]
    async fn create_command_validates_country_format() {
        let mut p = payload("X");
        p.country = Some("DEU".into());
        let err = validate_new_payload(&p).unwrap_err();
        assert!(err.message.contains("country"));
    }

    #[tokio::test]
    async fn create_command_normalizes_bic_uppercase_and_trims() {
        let mut p = payload("X");
        p.bic = Some("  traddeff  ".into());
        normalize_payload(&mut p);
        assert_eq!(p.bic.as_deref(), Some("TRADDEFF"));
    }

    #[tokio::test]
    async fn create_command_normalizes_country_uppercase() {
        let mut p = payload("X");
        p.country = Some("de".into());
        normalize_payload(&mut p);
        assert_eq!(p.country.as_deref(), Some("DE"));
    }

    #[tokio::test]
    async fn update_validates_bic_format() {
        let mut p = UpdateInstitutionPayload {
            name: None, icon: None, color: None,
            bic: Some(Some("invalid!".into())),
            country: None, note: None, archived: None,
        };
        normalize_update_payload(&mut p);
        let err = validate_update_payload(&p).unwrap_err();
        assert!(err.message.contains("bic"));
    }

    #[tokio::test]
    async fn delete_with_accounts_returns_structured_error() {
        let pool = connect_memory().await.unwrap();
        let inst = create_institution(&pool, NewInstitutionPayload {
            name: "X".into(), icon: None, color: None, bic: None, country: None, note: None,
        }).await.unwrap();
        crate::db::accounts::create_account(&pool, "A", "bank", "EUR", None, None, Some(inst.id)).await.unwrap();

        let err = delete_institution_inner(&pool, inst.id).await.unwrap_err();
        assert!(err.message.starts_with("institution-has-accounts:"),
            "expected prefix institution-has-accounts:, got {}", err.message);
        assert!(err.message.ends_with(":1"));
    }

    #[tokio::test]
    async fn delete_succeeds_when_no_accounts() {
        let pool = connect_memory().await.unwrap();
        let inst = create_institution(&pool, NewInstitutionPayload {
            name: "X".into(), icon: None, color: None, bic: None, country: None, note: None,
        }).await.unwrap();
        delete_institution_inner(&pool, inst.id).await.unwrap();
        assert!(db_inst::get_institution(&pool, inst.id).await.is_err());
    }
}
