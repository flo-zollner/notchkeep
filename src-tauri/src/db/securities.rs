use serde::Deserialize;
use sqlx::SqlitePool;

use super::{DbError, DbResult};
use crate::model::Security;

const SECURITY_COLUMNS: &str = "id, isin, symbol, name, currency, asset_type, \
     country, sector, note, archived, created_at";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewSecurityPayload {
    pub isin: String,
    pub symbol: Option<String>,
    pub name: String,
    pub currency: Option<String>,
    pub asset_type: String,
    pub country: Option<String>,
    pub sector: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSecurityPayload {
    pub isin: Option<String>,
    pub symbol: Option<String>,
    pub name: Option<String>,
    pub currency: Option<String>,
    pub asset_type: Option<String>,
    pub country: Option<String>,
    pub sector: Option<String>,
    pub note: Option<String>,
    pub archived: Option<bool>,
}

fn normalize_opt(v: Option<String>) -> Option<String> {
    v.and_then(|s| if s.trim().is_empty() { None } else { Some(s) })
}

pub async fn create_security(pool: &SqlitePool, p: NewSecurityPayload) -> DbResult<Security> {
    if p.name.trim().is_empty() {
        return Err(DbError::Decode("name must not be empty".into()));
    }
    if p.isin.trim().is_empty() {
        return Err(DbError::Decode("isin must not be empty".into()));
    }
    let currency = p
        .currency
        .as_deref()
        .map(|c| c.trim().to_uppercase())
        .filter(|c| !c.is_empty())
        .unwrap_or_else(|| "EUR".to_string());
    let sql = format!(
        "INSERT INTO securities
            (isin, symbol, name, currency, asset_type, country, sector, note)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         RETURNING {SECURITY_COLUMNS}"
    );
    Ok(sqlx::query_as::<_, Security>(&sql)
        .bind(p.isin)
        .bind(normalize_opt(p.symbol))
        .bind(p.name)
        .bind(currency)
        .bind(p.asset_type)
        .bind(normalize_opt(p.country))
        .bind(normalize_opt(p.sector))
        .bind(normalize_opt(p.note))
        .fetch_one(pool)
        .await?)
}

pub async fn get_security(pool: &SqlitePool, id: i64) -> DbResult<Security> {
    let sql = format!("SELECT {SECURITY_COLUMNS} FROM securities WHERE id = ?1");
    Ok(sqlx::query_as::<_, Security>(&sql)
        .bind(id)
        .fetch_one(pool)
        .await?)
}

pub async fn list_securities(pool: &SqlitePool, include_archived: bool) -> DbResult<Vec<Security>> {
    let sql = if include_archived {
        format!(
            "SELECT {SECURITY_COLUMNS} FROM securities \
             ORDER BY archived ASC, name COLLATE NOCASE ASC"
        )
    } else {
        format!(
            "SELECT {SECURITY_COLUMNS} FROM securities \
             WHERE archived = 0 \
             ORDER BY name COLLATE NOCASE ASC"
        )
    };
    Ok(sqlx::query_as::<_, Security>(&sql).fetch_all(pool).await?)
}

pub async fn delete_security(pool: &SqlitePool, id: i64) -> DbResult<bool> {
    let res = sqlx::query("DELETE FROM securities WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}

pub async fn update_security(
    pool: &SqlitePool,
    id: i64,
    p: UpdateSecurityPayload,
) -> DbResult<Security> {
    if let Some(n) = &p.name {
        if n.trim().is_empty() {
            return Err(DbError::Decode("name must not be empty".into()));
        }
    }
    if let Some(i) = &p.isin {
        if i.trim().is_empty() {
            return Err(DbError::Decode("isin must not be empty".into()));
        }
    }
    let current = get_security(pool, id).await?;
    let sql = format!(
        "UPDATE securities SET
            isin       = ?1,
            symbol     = ?2,
            name       = ?3,
            currency   = ?4,
            asset_type = ?5,
            country    = ?6,
            sector     = ?7,
            note       = ?8,
            archived   = ?9
         WHERE id = ?10
         RETURNING {SECURITY_COLUMNS}"
    );
    Ok(sqlx::query_as::<_, Security>(&sql)
        .bind(p.isin.unwrap_or(current.isin))
        .bind(normalize_opt(p.symbol).or(current.symbol))
        .bind(p.name.unwrap_or(current.name))
        .bind(
            p.currency
                .map(|c| c.to_uppercase())
                .unwrap_or(current.currency),
        )
        .bind(p.asset_type.unwrap_or(current.asset_type))
        .bind(normalize_opt(p.country).or(current.country))
        .bind(normalize_opt(p.sector).or(current.sector))
        .bind(normalize_opt(p.note).or(current.note))
        .bind(p.archived.unwrap_or(current.archived))
        .bind(id)
        .fetch_one(pool)
        .await?)
}

/// Helper for CSV import: look up a security by ISIN, or create it.
/// Returns the security_id. Pure DB layer, no UI feedback.
pub async fn resolve_or_create_security(
    pool: &SqlitePool,
    isin: &str,
    name: &str,
    asset_class_raw: &str,
) -> DbResult<i64> {
    if let Some((id,)) = sqlx::query_as::<_, (i64,)>("SELECT id FROM securities WHERE isin = ?1")
        .bind(isin)
        .fetch_optional(pool)
        .await?
    {
        return Ok(id);
    }

    let asset_type = map_asset_class(asset_class_raw);
    let (new_id,): (i64,) = sqlx::query_as(
        "INSERT INTO securities (isin, name, currency, asset_type)
         VALUES (?1, ?2, 'EUR', ?3) RETURNING id",
    )
    .bind(isin)
    .bind(name)
    .bind(asset_type)
    .fetch_one(pool)
    .await?;
    Ok(new_id)
}

fn map_asset_class(raw: &str) -> &'static str {
    match raw.to_uppercase().as_str() {
        "STOCK" => "stock",
        "FUND" => "etf_equity",
        "BOND" => "bond",
        "CRYPTO" => "crypto",
        _ => "other",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_memory;

    #[tokio::test]
    async fn create_security_round_trips_with_defaults() {
        let pool = connect_memory().await.unwrap();
        let s = create_security(
            &pool,
            NewSecurityPayload {
                isin: "IE00BK5BQT80".into(),
                symbol: Some("VWCE.DE".into()),
                name: "Vanguard FTSE All-World".into(),
                currency: None,
                asset_type: "etf_equity".into(),
                country: None,
                sector: None,
                note: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(s.isin, "IE00BK5BQT80");
        assert_eq!(s.name, "Vanguard FTSE All-World");
        assert_eq!(s.currency, "EUR");
        assert_eq!(s.asset_type, "etf_equity");
        assert!(s.symbol.is_some());
        assert!(!s.archived);
    }

    #[tokio::test]
    async fn create_security_rejects_duplicate_isin() {
        let pool = connect_memory().await.unwrap();
        let p = NewSecurityPayload {
            isin: "DE0007164600".into(),
            symbol: None,
            name: "SAP".into(),
            currency: None,
            asset_type: "stock".into(),
            country: Some("DE".into()),
            sector: Some("Technology".into()),
            note: None,
        };
        create_security(&pool, p.clone()).await.unwrap();
        assert!(create_security(&pool, p).await.is_err());
    }

    #[tokio::test]
    async fn create_security_rejects_empty_name_or_isin() {
        let pool = connect_memory().await.unwrap();
        let bad_name = NewSecurityPayload {
            isin: "IE00BK5BQT80".into(),
            symbol: None,
            name: "  ".into(),
            currency: None,
            asset_type: "stock".into(),
            country: None,
            sector: None,
            note: None,
        };
        assert!(create_security(&pool, bad_name).await.is_err());

        let bad_isin = NewSecurityPayload {
            isin: "  ".into(),
            symbol: None,
            name: "Foo".into(),
            currency: None,
            asset_type: "stock".into(),
            country: None,
            sector: None,
            note: None,
        };
        assert!(create_security(&pool, bad_isin).await.is_err());
    }

    #[tokio::test]
    async fn get_security_returns_inserted_row() {
        let pool = connect_memory().await.unwrap();
        let s = create_security(
            &pool,
            NewSecurityPayload {
                isin: "IE00BK5BQT80".into(),
                symbol: None,
                name: "x".into(),
                currency: None,
                asset_type: "etf_equity".into(),
                country: None,
                sector: None,
                note: None,
            },
        )
        .await
        .unwrap();

        let fetched = get_security(&pool, s.id).await.unwrap();
        assert_eq!(fetched.id, s.id);
        assert_eq!(fetched.isin, "IE00BK5BQT80");
    }

    #[tokio::test]
    async fn list_securities_filters_archived() {
        let pool = connect_memory().await.unwrap();
        let active = create_security(
            &pool,
            NewSecurityPayload {
                isin: "IE00BK5BQT80".into(),
                symbol: None,
                name: "active".into(),
                currency: None,
                asset_type: "etf_equity".into(),
                country: None,
                sector: None,
                note: None,
            },
        )
        .await
        .unwrap();
        let archived = create_security(
            &pool,
            NewSecurityPayload {
                isin: "DE0007164600".into(),
                symbol: None,
                name: "archived".into(),
                currency: None,
                asset_type: "stock".into(),
                country: None,
                sector: None,
                note: None,
            },
        )
        .await
        .unwrap();
        sqlx::query("UPDATE securities SET archived = 1 WHERE id = ?1")
            .bind(archived.id)
            .execute(&pool)
            .await
            .unwrap();

        let active_only = list_securities(&pool, false).await.unwrap();
        assert_eq!(active_only.len(), 1);
        assert_eq!(active_only[0].id, active.id);

        let all = list_securities(&pool, true).await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn delete_security_removes_row_and_returns_true() {
        let pool = connect_memory().await.unwrap();
        let s = create_security(
            &pool,
            NewSecurityPayload {
                isin: "IE00BK5BQT80".into(),
                symbol: None,
                name: "x".into(),
                currency: None,
                asset_type: "etf_equity".into(),
                country: None,
                sector: None,
                note: None,
            },
        )
        .await
        .unwrap();

        assert!(delete_security(&pool, s.id).await.unwrap());
        assert!(get_security(&pool, s.id).await.is_err());
        assert!(!delete_security(&pool, 99_999).await.unwrap());
    }

    #[tokio::test]
    async fn update_security_coalesces_partial_payload() {
        let pool = connect_memory().await.unwrap();
        let s = create_security(
            &pool,
            NewSecurityPayload {
                isin: "IE00BK5BQT80".into(),
                symbol: Some("VWCE.DE".into()),
                name: "alt".into(),
                currency: Some("EUR".into()),
                asset_type: "etf_equity".into(),
                country: Some("IE".into()),
                sector: None,
                note: Some("note".into()),
            },
        )
        .await
        .unwrap();

        // Change name only, other fields stay.
        let updated = update_security(
            &pool,
            s.id,
            UpdateSecurityPayload {
                name: Some("neu".into()),
                isin: None,
                symbol: None,
                currency: None,
                asset_type: None,
                country: None,
                sector: None,
                note: None,
                archived: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(updated.name, "neu");
        assert_eq!(updated.symbol.as_deref(), Some("VWCE.DE"));
        assert_eq!(updated.country.as_deref(), Some("IE"));
        assert_eq!(updated.note.as_deref(), Some("note"));
        assert!(!updated.archived);

        // Set archived = true.
        let archived = update_security(
            &pool,
            s.id,
            UpdateSecurityPayload {
                archived: Some(true),
                isin: None,
                symbol: None,
                name: None,
                currency: None,
                asset_type: None,
                country: None,
                sector: None,
                note: None,
            },
        )
        .await
        .unwrap();
        assert!(archived.archived);
    }

    #[tokio::test]
    async fn resolve_returns_existing_security_id() {
        let pool = connect_memory().await.unwrap();
        sqlx::query(
            "INSERT INTO securities (isin, name, currency, asset_type)
             VALUES ('US0378331005', 'Apple', 'EUR', 'stock')",
        )
        .execute(&pool)
        .await
        .unwrap();
        let (existing_id,): (i64,) =
            sqlx::query_as("SELECT id FROM securities WHERE isin = 'US0378331005'")
                .fetch_one(&pool)
                .await
                .unwrap();

        let resolved =
            resolve_or_create_security(&pool, "US0378331005", "ignored — already exists", "STOCK")
                .await
                .unwrap();
        assert_eq!(resolved, existing_id);
    }

    #[tokio::test]
    async fn resolve_creates_new_security_with_mapped_asset_type() {
        let pool = connect_memory().await.unwrap();
        let id =
            resolve_or_create_security(&pool, "LU0290358497", "Xtrackers II EUR Overnight", "FUND")
                .await
                .unwrap();
        assert!(id > 0);

        let (name, asset_type, isin): (String, String, String) =
            sqlx::query_as("SELECT name, asset_type, isin FROM securities WHERE id = ?")
                .bind(id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(isin, "LU0290358497");
        assert_eq!(name, "Xtrackers II EUR Overnight");
        assert_eq!(asset_type, "etf_equity");
    }

    #[tokio::test]
    async fn resolve_maps_unknown_asset_class_to_other() {
        let pool = connect_memory().await.unwrap();
        let id = resolve_or_create_security(&pool, "XX0000000001", "Mystery", "WARRANT")
            .await
            .unwrap();
        let (asset_type,): (String,) =
            sqlx::query_as("SELECT asset_type FROM securities WHERE id = ?")
                .bind(id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(asset_type, "other");
    }

    #[test]
    fn map_asset_class_handles_known_and_unknown() {
        assert_eq!(map_asset_class("STOCK"), "stock");
        assert_eq!(map_asset_class("stock"), "stock");
        assert_eq!(map_asset_class("FUND"), "etf_equity");
        assert_eq!(map_asset_class("BOND"), "bond");
        assert_eq!(map_asset_class("CRYPTO"), "crypto");
        assert_eq!(map_asset_class(""), "other");
        assert_eq!(map_asset_class("WARRANT"), "other");
    }
}
