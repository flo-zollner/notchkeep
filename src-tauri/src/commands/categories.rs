use serde::Deserialize;
use tauri::State;

use crate::commands::accounts::{CommandError, DbState};
use crate::model::Category;

const CATEGORY_COLUMNS: &str =
    "id, parent_id, name, color, icon, rollover_enabled";

#[tauri::command]
pub async fn list_categories(
    state: State<'_, DbState>,
) -> Result<Vec<Category>, CommandError> {
    let sql = format!(
        "SELECT {CATEGORY_COLUMNS} FROM categories ORDER BY parent_id NULLS FIRST, name"
    );
    let rows = sqlx::query_as::<_, Category>(&sql)
        .fetch_all(&state.pool())
        .await?;
    Ok(rows)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewCategory {
    pub name: String,
    pub parent_id: Option<i64>,
    pub color: Option<String>,
    pub icon: Option<String>,
    pub rollover_enabled: bool,
}

#[tauri::command]
pub async fn create_category(
    state: State<'_, DbState>,
    cat: NewCategory,
) -> Result<Category, CommandError> {
    let sql = format!(
        "INSERT INTO categories (parent_id, name, color, icon, rollover_enabled)
         VALUES (?1, ?2, ?3, ?4, ?5)
         RETURNING {CATEGORY_COLUMNS}"
    );
    let row: Category = sqlx::query_as(&sql)
        .bind(cat.parent_id)
        .bind(&cat.name)
        .bind(cat.color.as_deref())
        .bind(cat.icon.as_deref())
        .bind(cat.rollover_enabled)
        .fetch_one(&state.pool())
        .await?;
    Ok(row)
}

#[tauri::command]
pub async fn update_category(
    state: State<'_, DbState>,
    cat: Category,
) -> Result<(), CommandError> {
    sqlx::query(
        "UPDATE categories SET
            parent_id        = ?1,
            name             = ?2,
            color            = ?3,
            icon             = ?4,
            rollover_enabled = ?5
         WHERE id = ?6",
    )
    .bind(cat.parent_id)
    .bind(&cat.name)
    .bind(cat.color.as_deref())
    .bind(cat.icon.as_deref())
    .bind(cat.rollover_enabled)
    .bind(cat.id)
    .execute(&state.pool())
    .await?;
    Ok(())
}

#[tauri::command]
pub async fn delete_category(
    state: State<'_, DbState>,
    id: i64,
) -> Result<(), CommandError> {
    sqlx::query("DELETE FROM categories WHERE id = ?")
        .bind(id)
        .execute(&state.pool())
        .await?;
    Ok(())
}

/// Merge: moves all transactions + rules.target_category_id from `from_id` to `to_id`,
/// then deletes from_id. Sub-categories of from_id become children of to_id.
/// Atomic within a single transaction.
#[tauri::command]
pub async fn merge_categories(
    state: State<'_, DbState>,
    from_id: i64,
    to_id: i64,
) -> Result<u64, CommandError> {
    if from_id == to_id {
        return Err(CommandError { message: "from_id == to_id".into() });
    }
    let pool = state.pool();
    Ok(merge_categories_db(&pool, from_id, to_id).await?)
}

/// Pure-DB helper for `merge_categories` — extracted for testability.
pub(crate) async fn merge_categories_db(
    pool: &sqlx::SqlitePool,
    from_id: i64,
    to_id: i64,
) -> Result<u64, crate::db::DbError> {
    if from_id == to_id {
        return Err(crate::db::DbError::Decode("from_id == to_id".into()));
    }
    let mut tx = pool.begin().await?;

    // Update transactions
    let tx_rows = sqlx::query("UPDATE transactions SET category_id = ?1 WHERE category_id = ?2")
        .bind(to_id).bind(from_id)
        .execute(&mut *tx).await?
        .rows_affected();

    // Update rule targets
    sqlx::query("UPDATE rules SET target_category_id = ?1 WHERE target_category_id = ?2")
        .bind(to_id).bind(from_id)
        .execute(&mut *tx).await?;

    // Re-parent sub-categories
    sqlx::query("UPDATE categories SET parent_id = ?1 WHERE parent_id = ?2")
        .bind(to_id).bind(from_id)
        .execute(&mut *tx).await?;

    // Budget overrides: simply delete the from-rows (user budgets remain on to_id if present)
    sqlx::query("DELETE FROM category_budgets WHERE category_id = ?1")
        .bind(from_id)
        .execute(&mut *tx).await?;

    // Delete source
    sqlx::query("DELETE FROM categories WHERE id = ?1")
        .bind(from_id)
        .execute(&mut *tx).await?;

    tx.commit().await?;
    Ok(tx_rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_memory;

    #[tokio::test]
    async fn category_persists_rollover_enabled() {
        let pool = connect_memory().await.unwrap();
        let inserted: Category = sqlx::query_as(
            "INSERT INTO categories (parent_id, name, color, icon, rollover_enabled)
             VALUES (NULL, 'Lebensmittel', NULL, NULL, 1)
             RETURNING id, parent_id, name, color, icon, rollover_enabled",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(inserted.rollover_enabled);
        assert_eq!(inserted.name, "Lebensmittel");
    }

    #[tokio::test]
    async fn seed_creates_default_categories() {
        let pool = connect_memory().await.unwrap();
        let cats: Vec<Category> = sqlx::query_as(
            "SELECT id, parent_id, name, color, icon, rollover_enabled
             FROM categories ORDER BY name",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        let names: Vec<&str> = cats.iter().map(|c| c.name.as_str()).collect();
        for expected in [
            "Lebensmittel",
            "Wohnen",
            "Transport",
            "Freizeit",
            "Gesundheit",
            "Einkommen",
        ] {
            assert!(
                names.contains(&expected),
                "expected seed category '{expected}' to exist, got: {names:?}"
            );
        }
        for c in &cats {
            assert!(
                c.icon.is_some(),
                "seed category '{}' should have an icon",
                c.name
            );
        }
    }

    #[tokio::test]
    async fn category_rollover_enabled_defaults_to_false() {
        let pool = connect_memory().await.unwrap();
        let inserted: Category = sqlx::query_as(
            "INSERT INTO categories (parent_id, name, color, icon)
             VALUES (NULL, 'Sonstiges', NULL, NULL)
             RETURNING id, parent_id, name, color, icon, rollover_enabled",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(!inserted.rollover_enabled);
    }

    // --- merge_categories tests ---

    async fn seed_cat(pool: &sqlx::SqlitePool, name: &str, parent: Option<i64>) -> i64 {
        let (id,): (i64,) = sqlx::query_as(
            "INSERT INTO categories (parent_id, name) VALUES (?1, ?2) RETURNING id",
        )
        .bind(parent)
        .bind(name)
        .fetch_one(pool)
        .await
        .unwrap();
        id
    }

    async fn seed_account(pool: &sqlx::SqlitePool) -> i64 {
        let (id,): (i64,) = sqlx::query_as(
            "INSERT INTO accounts (name, kind, currency) VALUES ('Test', 'bank', 'EUR') RETURNING id",
        )
        .fetch_one(pool)
        .await
        .unwrap();
        id
    }

    #[tokio::test]
    async fn merge_moves_transactions() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool).await;
        let cat_a = seed_cat(&pool, "MergeA", None).await;
        let cat_b = seed_cat(&pool, "MergeB", None).await;

        // Insert 3 transactions with category A
        for (date, cp) in [("2026-05-01", "cp1"), ("2026-05-02", "cp2"), ("2026-05-03", "cp3")] {
            sqlx::query(
                "INSERT INTO transactions
                    (account_id, booking_date, amount_cents, currency,
                     counterparty, source, category_id)
                 VALUES (?1, ?2, -1000, 'EUR', ?3, 'manual', ?4)",
            )
            .bind(acc).bind(date).bind(cp).bind(cat_a)
            .execute(&pool).await.unwrap();
        }

        let moved = super::merge_categories_db(&pool, cat_a, cat_b).await.unwrap();
        assert_eq!(moved, 3, "all 3 tx should be moved");

        // All tx now have cat_b
        let (count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM transactions WHERE category_id = ?",
        ).bind(cat_b).fetch_one(&pool).await.unwrap();
        assert_eq!(count, 3);

        // Cat A is deleted
        let (exists,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM categories WHERE id = ?",
        ).bind(cat_a).fetch_one(&pool).await.unwrap();
        assert_eq!(exists, 0, "source category A must be deleted after merge");
    }

    #[tokio::test]
    async fn merge_reassigns_subcategories() {
        let pool = connect_memory().await.unwrap();
        let cat_a = seed_cat(&pool, "ParentA", None).await;
        let cat_a1 = seed_cat(&pool, "ChildA1", Some(cat_a)).await;
        let cat_b = seed_cat(&pool, "ParentB", None).await;

        super::merge_categories_db(&pool, cat_a, cat_b).await.unwrap();

        // cat_a1 should now have parent_id = cat_b
        let (parent_id,): (Option<i64>,) = sqlx::query_as(
            "SELECT parent_id FROM categories WHERE id = ?",
        ).bind(cat_a1).fetch_one(&pool).await.unwrap();
        assert_eq!(parent_id, Some(cat_b), "sub-category must be re-parented to target");
    }

    #[tokio::test]
    async fn merge_rejects_self_merge() {
        let pool = connect_memory().await.unwrap();
        let cat = seed_cat(&pool, "Solo", None).await;

        let result = super::merge_categories_db(&pool, cat, cat).await;
        assert!(result.is_err(), "self-merge must return Err");
    }
}
