use serde::Serialize;
use tauri::State;

use crate::db::accounts as db_accounts;
use crate::model::Account;

#[derive(Debug, Serialize)]
pub struct CommandError {
    pub message: String,
}

impl<E: std::fmt::Display> From<E> for CommandError {
    fn from(e: E) -> Self {
        Self { message: e.to_string() }
    }
}

pub struct DbState(pub std::sync::RwLock<sqlx::SqlitePool>);

impl DbState {
    /// Liefert einen geclonten Pool-Handle (cheap, sqlx::SqlitePool ist intern Arc).
    /// Read-Lock wird für die Dauer des Clones gehalten (mikrosekunden).
    pub fn pool(&self) -> sqlx::SqlitePool {
        self.0.read().expect("DbState RwLock poisoned").clone()
    }

    /// Tauscht den internen Pool atomar aus und returnt den alten Pool zum Schließen.
    /// Aufrufer muss `old_pool.close().await` von außen aufrufen.
    pub fn swap(&self, new_pool: sqlx::SqlitePool) -> sqlx::SqlitePool {
        let mut guard = self.0.write().expect("DbState RwLock poisoned");
        std::mem::replace(&mut *guard, new_pool)
    }
}

#[tauri::command]
pub async fn get_accounts(
    state: State<'_, DbState>,
) -> Result<Vec<Account>, CommandError> {
    Ok(db_accounts::list_accounts(&state.pool()).await?)
}

#[tauri::command]
pub async fn create_account(
    state: State<'_, DbState>,
    name: String,
    kind: String,
    currency: Option<String>,
    parent_id: Option<i64>,
    iban: Option<String>,
    institution_id: Option<i64>,
) -> Result<Account, CommandError> {
    if name.trim().is_empty() {
        return Err(CommandError { message: "name must not be empty".into() });
    }
    let currency = currency.unwrap_or_else(|| "EUR".to_string());
    let normalized_iban = normalize_iban(iban.as_deref())?;
    Ok(db_accounts::create_account(
        &state.pool(),
        &name,
        &kind,
        &currency,
        parent_id,
        normalized_iban.as_deref(),
        institution_id,
    ).await?)
}

#[tauri::command]
pub async fn get_account(
    state: State<'_, DbState>,
    id: i64,
) -> Result<Account, CommandError> {
    Ok(db_accounts::get_account(&state.pool(), id).await?)
}

#[tauri::command]
pub async fn update_account(
    state: State<'_, DbState>,
    account: Account,
) -> Result<(), CommandError> {
    if account.name.trim().is_empty() {
        return Err(CommandError { message: "name must not be empty".into() });
    }
    if let Some(s) = account.last4.as_deref() {
        if !(s.len() == 4 && s.chars().all(|c| c.is_ascii_digit())) {
            return Err(CommandError {
                message: format!("last4 must be exactly 4 digits, got: {s:?}"),
            });
        }
    }
    let mut acc = account;
    acc.iban = normalize_iban(acc.iban.as_deref())?;
    let pool = state.pool();
    if let Some(pid) = acc.parent_id {
        db_accounts::validate_no_cycle(&pool, acc.id, pid).await?;
    }
    db_accounts::update_account(&pool, &acc).await?;
    Ok(())
}

/// Trimt + uppercaset IBAN. Leer → None. Format-Check (`^[A-Z]{2}\d{2}[A-Z0-9]{11,30}$`).
fn normalize_iban(raw: Option<&str>) -> Result<Option<String>, CommandError> {
    let Some(s) = raw else { return Ok(None) };
    let cleaned: String = s.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_uppercase();
    if cleaned.is_empty() {
        return Ok(None);
    }
    if cleaned.len() < 15 || cleaned.len() > 34 {
        return Err(CommandError {
            message: format!("iban length must be 15-34, got {} for {cleaned:?}", cleaned.len()),
        });
    }
    let mut chars = cleaned.chars();
    let c1 = chars.next().unwrap();
    let c2 = chars.next().unwrap();
    let d1 = chars.next().unwrap();
    let d2 = chars.next().unwrap();
    if !c1.is_ascii_uppercase() || !c2.is_ascii_uppercase()
        || !d1.is_ascii_digit() || !d2.is_ascii_digit()
        || !chars.all(|c| c.is_ascii_alphanumeric() && (c.is_ascii_uppercase() || c.is_ascii_digit()))
    {
        return Err(CommandError {
            message: format!("iban format invalid: {cleaned:?}"),
        });
    }
    Ok(Some(cleaned))
}

#[tauri::command]
pub async fn account_balance(
    state: State<'_, DbState>,
    id: i64,
) -> Result<i64, CommandError> {
    Ok(db_accounts::account_balance(&state.pool(), id).await?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_memory;

    #[tokio::test]
    async fn create_then_list_accounts() {
        let pool = connect_memory().await.unwrap();
        let inserted = db_accounts::create_account(&pool, "TR Verrechnung", "bank", "EUR", None, None, None)
            .await
            .unwrap();
        assert_eq!(inserted.name, "TR Verrechnung");

        let listed = db_accounts::list_accounts(&pool).await.unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, inserted.id);
    }

    #[tokio::test]
    async fn normalize_iban_accepts_valid() {
        let result = normalize_iban(Some(" de89 3704 0044 0532 0130 00 ")).unwrap();
        assert_eq!(result, Some("DE89370400440532013000".into()));
    }

    #[tokio::test]
    async fn normalize_iban_rejects_too_short() {
        let err = normalize_iban(Some("DE89")).unwrap_err();
        assert!(err.message.contains("length"));
    }

    #[tokio::test]
    async fn normalize_iban_rejects_bad_country_code() {
        let err = normalize_iban(Some("12893704004405320130001")).unwrap_err();
        assert!(err.message.contains("format"));
    }

    #[tokio::test]
    async fn normalize_iban_empty_returns_none() {
        assert!(normalize_iban(Some("   ")).unwrap().is_none());
        assert!(normalize_iban(Some("")).unwrap().is_none());
        assert!(normalize_iban(None).unwrap().is_none());
    }

    #[tokio::test]
    async fn create_account_persists_institution_id() {
        let pool = connect_memory().await.unwrap();
        sqlx::query("INSERT INTO institutions (name) VALUES ('TestBank')")
            .execute(&pool).await.unwrap();
        let (inst_id,): (i64,) = sqlx::query_as("SELECT id FROM institutions WHERE name='TestBank'")
            .fetch_one(&pool).await.unwrap();
        let acc = db_accounts::create_account(&pool, "x", "bank", "EUR", None, None, Some(inst_id))
            .await.unwrap();
        assert_eq!(acc.institution_id, Some(inst_id));
    }
}
