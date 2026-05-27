use serde::Deserialize;
use sqlx::SqlitePool;

use super::{DbError, DbResult};
use crate::model::{Institution, InstitutionSummary};

const INSTITUTION_COLUMNS: &str =
    "id, name, icon, color, bic, country, note, archived, created_at";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewInstitutionPayload {
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub bic: Option<String>,
    pub country: Option<String>,
    pub note: Option<String>,
}

/// Deserializer helper: converts a present JSON field (even if it is `null`)
/// into `Some(T::deserialize(...))`.  Missing fields remain `None` (via `#[serde(default)]`).
fn deserialize_some<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T: serde::Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    T::deserialize(deserializer).map(Some)
}

/// PATCH payload for `update_institution`.
///
/// Semantics of the optional `Option<Option<String>>` fields:
/// - `None` (field absent in JSON)  → keep the old value
/// - `Some(None)` (field is JSON `null`) → explicitly clear the field
/// - `Some(Some(value))` (field has a value) → set field to `value`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInstitutionPayload {
    pub name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_some")]
    pub icon: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_some")]
    pub color: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_some")]
    pub bic: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_some")]
    pub country: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_some")]
    pub note: Option<Option<String>>,
    pub archived: Option<bool>,
}

fn normalize_opt(v: Option<String>) -> Option<String> {
    v.and_then(|s| if s.trim().is_empty() { None } else { Some(s) })
}

pub async fn create_institution(
    pool: &SqlitePool,
    p: NewInstitutionPayload,
) -> DbResult<Institution> {
    if p.name.trim().is_empty() {
        return Err(DbError::Decode("name must not be empty".into()));
    }
    let sql = format!(
        "INSERT INTO institutions (name, icon, color, bic, country, note)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         RETURNING {INSTITUTION_COLUMNS}"
    );
    Ok(sqlx::query_as::<_, Institution>(&sql)
        .bind(p.name.trim())
        .bind(normalize_opt(p.icon))
        .bind(normalize_opt(p.color))
        .bind(normalize_opt(p.bic))
        .bind(normalize_opt(p.country))
        .bind(normalize_opt(p.note))
        .fetch_one(pool)
        .await?)
}

pub async fn get_institution(pool: &SqlitePool, id: i64) -> DbResult<Institution> {
    let sql = format!("SELECT {INSTITUTION_COLUMNS} FROM institutions WHERE id = ?1");
    Ok(sqlx::query_as::<_, Institution>(&sql)
        .bind(id)
        .fetch_one(pool)
        .await?)
}

pub async fn list_institutions(
    pool: &SqlitePool,
    include_archived: bool,
) -> DbResult<Vec<Institution>> {
    let sql = if include_archived {
        format!(
            "SELECT {INSTITUTION_COLUMNS} FROM institutions \
             ORDER BY archived ASC, LOWER(name) ASC"
        )
    } else {
        format!(
            "SELECT {INSTITUTION_COLUMNS} FROM institutions \
             WHERE archived = 0 \
             ORDER BY LOWER(name) ASC"
        )
    };
    Ok(sqlx::query_as::<_, Institution>(&sql).fetch_all(pool).await?)
}

pub async fn update_institution(
    pool: &SqlitePool,
    id: i64,
    p: UpdateInstitutionPayload,
) -> DbResult<Institution> {
    if let Some(n) = &p.name {
        if n.trim().is_empty() {
            return Err(DbError::Decode("name must not be empty".into()));
        }
    }
    let current = get_institution(pool, id).await?;
    let sql = format!(
        "UPDATE institutions SET
            name     = ?1,
            icon     = ?2,
            color    = ?3,
            bic      = ?4,
            country  = ?5,
            note     = ?6,
            archived = ?7
         WHERE id = ?8
         RETURNING {INSTITUTION_COLUMNS}"
    );
    Ok(sqlx::query_as::<_, Institution>(&sql)
        .bind(p.name.map(|s| s.trim().to_string()).unwrap_or(current.name))
        .bind(match p.icon { Some(inner) => normalize_opt(inner), None => current.icon })
        .bind(match p.color { Some(inner) => normalize_opt(inner), None => current.color })
        .bind(match p.bic { Some(inner) => normalize_opt(inner), None => current.bic })
        .bind(match p.country { Some(inner) => normalize_opt(inner), None => current.country })
        .bind(match p.note { Some(inner) => normalize_opt(inner), None => current.note })
        .bind(p.archived.unwrap_or(current.archived))
        .bind(id)
        .fetch_one(pool)
        .await?)
}

pub async fn institution_account_count(pool: &SqlitePool, id: i64) -> DbResult<i64> {
    let (cnt,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM accounts WHERE institution_id = ?1",
    )
    .bind(id)
    .fetch_one(pool)
    .await?;
    Ok(cnt)
}

pub async fn delete_institution(pool: &SqlitePool, id: i64) -> DbResult<bool> {
    let res = sqlx::query("DELETE FROM institutions WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}

pub async fn institution_balance(pool: &SqlitePool, id: i64) -> DbResult<i64> {
    let (sum,): (i64,) = sqlx::query_as(
        "SELECT COALESCE(SUM(t.amount_cents), 0)
           FROM transactions t
           JOIN accounts a ON a.id = t.account_id
          WHERE a.institution_id = ?1",
    )
    .bind(id)
    .fetch_one(pool)
    .await?;
    Ok(sum)
}

pub async fn get_institution_by_name(
    pool: &SqlitePool,
    name: &str,
) -> DbResult<Option<Institution>> {
    let sql = format!(
        "SELECT {INSTITUTION_COLUMNS} FROM institutions \
         WHERE LOWER(name) = LOWER(?1) LIMIT 1"
    );
    Ok(sqlx::query_as::<_, Institution>(&sql)
        .bind(name)
        .fetch_optional(pool)
        .await?)
}

pub async fn upsert_institution_by_name(
    pool: &SqlitePool,
    name: &str,
    icon: Option<&str>,
    color: Option<&str>,
    bic: Option<&str>,
    country: Option<&str>,
) -> DbResult<Institution> {
    if let Some(existing) = get_institution_by_name(pool, name).await? {
        return Ok(existing);
    }
    create_institution(pool, NewInstitutionPayload {
        name: name.to_string(),
        icon: icon.map(String::from),
        color: color.map(String::from),
        bic: bic.map(String::from),
        country: country.map(String::from),
        note: None,
    }).await
}

pub async fn list_institutions_with_summary(
    pool: &SqlitePool,
) -> DbResult<Vec<InstitutionSummary>> {
    Ok(sqlx::query_as::<_, InstitutionSummary>(
        "SELECT i.id, i.name, i.icon, i.color, i.bic, i.country, i.note,
                i.archived, i.created_at,
                COUNT(DISTINCT a.id)                  AS account_count,
                COALESCE(SUM(t.amount_cents), 0)      AS balance_cents
           FROM institutions i
           LEFT JOIN accounts a     ON a.institution_id = i.id
           LEFT JOIN transactions t ON t.account_id     = a.id
          WHERE i.archived = 0
          GROUP BY i.id
          ORDER BY LOWER(i.name) ASC",
    )
    .fetch_all(pool)
    .await?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_memory;

    #[tokio::test]
    async fn create_institution_round_trip() {
        let pool = connect_memory().await.unwrap();
        let inst = create_institution(&pool, NewInstitutionPayload {
            name: "Trade Republic".into(),
            icon: Some("bank".into()),
            color: Some("oklch(0.55 0.13 230)".into()),
            bic: Some("TRADDEFFXXX".into()),
            country: Some("DE".into()),
            note: None,
        }).await.unwrap();
        assert_eq!(inst.name, "Trade Republic");
        assert_eq!(inst.bic.as_deref(), Some("TRADDEFFXXX"));
        assert!(!inst.archived);
    }

    #[tokio::test]
    async fn create_institution_empty_name_errors() {
        let pool = connect_memory().await.unwrap();
        let err = create_institution(&pool, NewInstitutionPayload {
            name: "  ".into(), icon: None, color: None, bic: None, country: None, note: None,
        }).await.unwrap_err();
        assert!(err.to_string().contains("name"));
    }

    #[tokio::test]
    async fn create_institution_normalizes_empty_strings_to_none() {
        let pool = connect_memory().await.unwrap();
        let inst = create_institution(&pool, NewInstitutionPayload {
            name: "X".into(),
            icon: Some("".into()),
            color: Some("  ".into()),
            bic: None,
            country: None,
            note: Some("".into()),
        }).await.unwrap();
        assert!(inst.icon.is_none());
        assert!(inst.color.is_none());
        assert!(inst.note.is_none());
    }

    #[tokio::test]
    async fn create_institution_bic_check_rejects_lowercase() {
        let pool = connect_memory().await.unwrap();
        let err = create_institution(&pool, NewInstitutionPayload {
            name: "X".into(), icon: None, color: None,
            bic: Some("traddeff".into()),
            country: None, note: None,
        }).await.unwrap_err();
        assert!(err.to_string().to_lowercase().contains("check"), "got: {err}");
    }

    #[tokio::test]
    async fn create_institution_bic_check_accepts_8_and_11() {
        let pool = connect_memory().await.unwrap();
        create_institution(&pool, NewInstitutionPayload {
            name: "A".into(), icon: None, color: None,
            bic: Some("TRADDEFF".into()),
            country: None, note: None,
        }).await.unwrap();
        create_institution(&pool, NewInstitutionPayload {
            name: "B".into(), icon: None, color: None,
            bic: Some("TRADDEFFXXX".into()),
            country: None, note: None,
        }).await.unwrap();
    }

    #[tokio::test]
    async fn create_institution_country_check_rejects_lowercase() {
        let pool = connect_memory().await.unwrap();
        let err = create_institution(&pool, NewInstitutionPayload {
            name: "X".into(), icon: None, color: None, bic: None,
            country: Some("de".into()), note: None,
        }).await.unwrap_err();
        assert!(err.to_string().to_lowercase().contains("check"), "got: {err}");
    }

    #[tokio::test]
    async fn create_institution_country_check_rejects_3_chars() {
        let pool = connect_memory().await.unwrap();
        let err = create_institution(&pool, NewInstitutionPayload {
            name: "X".into(), icon: None, color: None, bic: None,
            country: Some("DEU".into()), note: None,
        }).await.unwrap_err();
        assert!(err.to_string().to_lowercase().contains("check"), "got: {err}");
    }

    #[tokio::test]
    async fn name_unique_constraint_is_case_insensitive() {
        let pool = connect_memory().await.unwrap();
        create_institution(&pool, NewInstitutionPayload {
            name: "Trade Republic".into(), icon: None, color: None, bic: None, country: None, note: None,
        }).await.unwrap();
        let err = create_institution(&pool, NewInstitutionPayload {
            name: "trade republic".into(), icon: None, color: None, bic: None, country: None, note: None,
        }).await.unwrap_err();
        assert!(err.to_string().to_lowercase().contains("unique"), "got: {err}");
    }

    #[tokio::test]
    async fn bic_unique_constraint_allows_multiple_null() {
        let pool = connect_memory().await.unwrap();
        create_institution(&pool, NewInstitutionPayload {
            name: "A".into(), icon: None, color: None, bic: None, country: None, note: None,
        }).await.unwrap();
        create_institution(&pool, NewInstitutionPayload {
            name: "B".into(), icon: None, color: None, bic: None, country: None, note: None,
        }).await.unwrap();
    }

    #[tokio::test]
    async fn get_institution_returns_inserted() {
        let pool = connect_memory().await.unwrap();
        let inst = create_institution(&pool, NewInstitutionPayload {
            name: "X".into(), icon: None, color: None, bic: None, country: None, note: None,
        }).await.unwrap();
        let fetched = get_institution(&pool, inst.id).await.unwrap();
        assert_eq!(fetched.id, inst.id);
        assert_eq!(fetched.name, "X");
    }

    #[tokio::test]
    async fn get_institution_unknown_id_errors() {
        let pool = connect_memory().await.unwrap();
        let err = get_institution(&pool, 9999).await.unwrap_err();
        assert!(err.to_string().to_lowercase().contains("no rows"));
    }

    #[tokio::test]
    async fn list_institutions_excludes_archived_by_default() {
        let pool = connect_memory().await.unwrap();
        let a = create_institution(&pool, NewInstitutionPayload {
            name: "active".into(), icon: None, color: None, bic: None, country: None, note: None,
        }).await.unwrap();
        let _b = create_institution(&pool, NewInstitutionPayload {
            name: "archived".into(), icon: None, color: None, bic: None, country: None, note: None,
        }).await.unwrap();
        sqlx::query("UPDATE institutions SET archived = 1 WHERE name = 'archived'")
            .execute(&pool).await.unwrap();

        let visible = list_institutions(&pool, false).await.unwrap();
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].id, a.id);

        let all = list_institutions(&pool, true).await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn update_institution_coalesces_partial_payload() {
        let pool = connect_memory().await.unwrap();
        let inst = create_institution(&pool, NewInstitutionPayload {
            name: "alt".into(),
            icon: Some("bank".into()),
            color: None,
            bic: Some("TRADDEFFXXX".into()),
            country: Some("DE".into()),
            note: None,
        }).await.unwrap();

        // None = field not sent → old value is preserved.
        let updated = update_institution(&pool, inst.id, UpdateInstitutionPayload {
            name: Some("neu".into()),
            icon: None, color: None, bic: None, country: None, note: None,
            archived: None,
        }).await.unwrap();
        assert_eq!(updated.name, "neu");
        assert_eq!(updated.icon.as_deref(), Some("bank"));
        assert_eq!(updated.bic.as_deref(), Some("TRADDEFFXXX"));
        assert_eq!(updated.country.as_deref(), Some("DE"));
        assert!(!updated.archived);

        let archived = update_institution(&pool, inst.id, UpdateInstitutionPayload {
            name: None, icon: None, color: None, bic: None, country: None, note: None,
            archived: Some(true),
        }).await.unwrap();
        assert!(archived.archived);
    }

    #[tokio::test]
    async fn update_institution_can_clear_optional_fields() {
        let pool = connect_memory().await.unwrap();
        let inst = create_institution(&pool, NewInstitutionPayload {
            name: "X".into(),
            icon: Some("bank".into()),
            color: Some("blue".into()),
            bic: Some("TRADDEFFXXX".into()),
            country: Some("DE".into()),
            note: Some("hi".into()),
        }).await.unwrap();
        assert_eq!(inst.icon.as_deref(), Some("bank"));

        // Some(None) = field explicitly set to null → field is cleared.
        // None = field not sent → old value is preserved.
        let cleared = update_institution(&pool, inst.id, UpdateInstitutionPayload {
            name: None,
            icon: Some(None),    // explicitly clear
            color: None,         // keep
            bic: Some(None),     // explicitly clear
            country: None,       // keep
            note: Some(None),    // explicitly clear
            archived: None,
        }).await.unwrap();
        assert!(cleared.icon.is_none(), "icon should be cleared");
        assert_eq!(cleared.color.as_deref(), Some("blue"), "color preserved");
        assert!(cleared.bic.is_none(), "bic should be cleared");
        assert_eq!(cleared.country.as_deref(), Some("DE"), "country preserved");
        assert!(cleared.note.is_none(), "note should be cleared");
    }

    #[tokio::test]
    async fn institution_account_count_returns_zero_when_unused() {
        let pool = connect_memory().await.unwrap();
        let inst = create_institution(&pool, NewInstitutionPayload {
            name: "X".into(), icon: None, color: None, bic: None, country: None, note: None,
        }).await.unwrap();
        assert_eq!(institution_account_count(&pool, inst.id).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn institution_account_count_counts_assigned_accounts() {
        let pool = connect_memory().await.unwrap();
        let inst = create_institution(&pool, NewInstitutionPayload {
            name: "X".into(), icon: None, color: None, bic: None, country: None, note: None,
        }).await.unwrap();
        crate::db::accounts::create_account(&pool, "A1", "bank", "EUR", None, None, Some(inst.id)).await.unwrap();
        crate::db::accounts::create_account(&pool, "A2", "broker", "EUR", None, None, Some(inst.id)).await.unwrap();
        crate::db::accounts::create_account(&pool, "A3", "cash", "EUR", None, None, None).await.unwrap();
        assert_eq!(institution_account_count(&pool, inst.id).await.unwrap(), 2);
    }

    #[tokio::test]
    async fn delete_institution_sets_account_institution_id_to_null() {
        let pool = connect_memory().await.unwrap();
        let inst = create_institution(&pool, NewInstitutionPayload {
            name: "X".into(), icon: None, color: None, bic: None, country: None, note: None,
        }).await.unwrap();
        let acc = crate::db::accounts::create_account(&pool, "A", "bank", "EUR", None, None, Some(inst.id))
            .await.unwrap();
        assert!(delete_institution(&pool, inst.id).await.unwrap());
        let reloaded = crate::db::accounts::get_account(&pool, acc.id).await.unwrap();
        assert!(reloaded.institution_id.is_none());
    }

    #[tokio::test]
    async fn delete_institution_returns_false_for_unknown_id() {
        let pool = connect_memory().await.unwrap();
        assert!(!delete_institution(&pool, 9999).await.unwrap());
    }

    #[tokio::test]
    async fn institution_balance_sums_assigned_account_tx() {
        let pool = connect_memory().await.unwrap();
        let inst = create_institution(&pool, NewInstitutionPayload {
            name: "X".into(), icon: None, color: None, bic: None, country: None, note: None,
        }).await.unwrap();
        let a1 = crate::db::accounts::create_account(&pool, "A1", "bank", "EUR", None, None, Some(inst.id)).await.unwrap();
        let a2 = crate::db::accounts::create_account(&pool, "A2", "broker", "EUR", None, None, Some(inst.id)).await.unwrap();
        let other = crate::db::accounts::create_account(&pool, "O", "cash", "EUR", None, None, None).await.unwrap();
        sqlx::query(
            "INSERT INTO transactions (account_id, booking_date, amount_cents, currency, counterparty, source)
             VALUES
                (?1, '2026-05-01', 100000, 'EUR', 'a', 'manual'),
                (?2, '2026-05-02',  50000, 'EUR', 'b', 'manual'),
                (?3, '2026-05-03', 999999, 'EUR', 'c', 'manual')",
        )
        .bind(a1.id).bind(a2.id).bind(other.id)
        .execute(&pool).await.unwrap();

        assert_eq!(institution_balance(&pool, inst.id).await.unwrap(), 150000);
    }

    #[tokio::test]
    async fn institution_balance_zero_when_no_accounts() {
        let pool = connect_memory().await.unwrap();
        let inst = create_institution(&pool, NewInstitutionPayload {
            name: "X".into(), icon: None, color: None, bic: None, country: None, note: None,
        }).await.unwrap();
        assert_eq!(institution_balance(&pool, inst.id).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn upsert_institution_by_name_creates_if_missing() {
        let pool = connect_memory().await.unwrap();
        let inst = upsert_institution_by_name(
            &pool, "Trade Republic", Some("bank"), Some("blue"), None, Some("DE"),
        ).await.unwrap();
        assert_eq!(inst.name, "Trade Republic");
        assert_eq!(inst.country.as_deref(), Some("DE"));
    }

    #[tokio::test]
    async fn upsert_institution_by_name_returns_existing_if_present() {
        let pool = connect_memory().await.unwrap();
        let first = upsert_institution_by_name(
            &pool, "Trade Republic", Some("bank"), None, None, None,
        ).await.unwrap();
        let second = upsert_institution_by_name(
            &pool, "Trade Republic", Some("other-icon"), Some("red"), None, Some("AT"),
        ).await.unwrap();
        assert_eq!(first.id, second.id, "no duplicate");
        // Fields are NOT overwritten (idempotent, no update):
        assert_eq!(second.icon.as_deref(), Some("bank"));
    }

    #[tokio::test]
    async fn upsert_institution_by_name_matches_case_insensitive() {
        let pool = connect_memory().await.unwrap();
        let first = upsert_institution_by_name(&pool, "Trade Republic", None, None, None, None).await.unwrap();
        let second = upsert_institution_by_name(&pool, "trade republic", None, None, None, None).await.unwrap();
        assert_eq!(first.id, second.id);
    }

    #[tokio::test]
    async fn list_institutions_with_summary_aggregates() {
        let pool = connect_memory().await.unwrap();
        let i1 = create_institution(&pool, NewInstitutionPayload {
            name: "A".into(), icon: None, color: None, bic: None, country: None, note: None,
        }).await.unwrap();
        let i2 = create_institution(&pool, NewInstitutionPayload {
            name: "B".into(), icon: None, color: None, bic: None, country: None, note: None,
        }).await.unwrap();

        let a1 = crate::db::accounts::create_account(&pool, "A1", "bank", "EUR", None, None, Some(i1.id)).await.unwrap();
        let a2 = crate::db::accounts::create_account(&pool, "A2", "broker", "EUR", None, None, Some(i1.id)).await.unwrap();
        let _b1 = crate::db::accounts::create_account(&pool, "B1", "bank", "EUR", None, None, Some(i2.id)).await.unwrap();

        sqlx::query(
            "INSERT INTO transactions (account_id, booking_date, amount_cents, currency, counterparty, source)
             VALUES
                (?1, '2026-05-01', 200000, 'EUR', 'x', 'manual'),
                (?2, '2026-05-01', -50000, 'EUR', 'y', 'manual')",
        )
        .bind(a1.id).bind(a2.id).execute(&pool).await.unwrap();

        let summaries = list_institutions_with_summary(&pool).await.unwrap();
        let s_a = summaries.iter().find(|s| s.id == i1.id).expect("A in summary");
        let s_b = summaries.iter().find(|s| s.id == i2.id).expect("B in summary");
        assert_eq!(s_a.account_count, 2);
        assert_eq!(s_a.balance_cents, 150000);
        assert_eq!(s_b.account_count, 1);
        assert_eq!(s_b.balance_cents, 0);
    }
}
