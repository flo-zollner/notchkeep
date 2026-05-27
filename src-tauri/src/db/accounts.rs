use sqlx::SqlitePool;

use crate::db::{DbError, DbResult};
use crate::model::Account;

const ACCOUNT_COLUMNS: &str =
    "id, name, kind, currency, icon, color, note, last4, archived, parent_id, iban, institution_id, created_at";

pub async fn list_accounts(pool: &SqlitePool) -> DbResult<Vec<Account>> {
    let sql = format!("SELECT {ACCOUNT_COLUMNS} FROM accounts ORDER BY id");
    Ok(sqlx::query_as::<_, Account>(&sql).fetch_all(pool).await?)
}

pub async fn list_accounts_by_institution(
    pool: &SqlitePool,
    institution_id: Option<i64>,
) -> DbResult<Vec<Account>> {
    let sql = match institution_id {
        Some(_) => format!(
            "SELECT {ACCOUNT_COLUMNS} FROM accounts WHERE institution_id = ?1 ORDER BY id"
        ),
        None => format!(
            "SELECT {ACCOUNT_COLUMNS} FROM accounts WHERE institution_id IS NULL ORDER BY id"
        ),
    };
    let mut q = sqlx::query_as::<_, Account>(&sql);
    if let Some(id) = institution_id {
        q = q.bind(id);
    }
    Ok(q.fetch_all(pool).await?)
}

/// Finds the sole non-archived broker account within the institution.
/// Returns None when 0 or >1 broker accounts exist (ambiguous).
pub async fn find_broker_account_for_institution(
    pool: &SqlitePool,
    institution_id: i64,
) -> DbResult<Option<i64>> {
    let rows: Vec<(i64,)> = sqlx::query_as(
        "SELECT id FROM accounts
          WHERE institution_id = ?1
            AND kind = 'broker'
            AND archived = 0",
    )
    .bind(institution_id)
    .fetch_all(pool)
    .await?;
    if rows.len() == 1 {
        Ok(Some(rows[0].0))
    } else {
        Ok(None)
    }
}

/// Determines the target account for a securities_trades detail row (variant B).
///
/// If `current_account` is itself a broker → `Ok(None)` (no routing needed,
/// `portfolio_*` queries fall back via COALESCE to `tx.account_id`).
/// If `current_account.institution_id` is set AND the institution has exactly one
/// non-archived broker account → `Ok(Some(broker_id))`.
/// Otherwise → `Ok(None)`.
pub async fn resolve_trade_account(
    pool: &SqlitePool,
    current_account: i64,
) -> DbResult<Option<i64>> {
    let acc = get_account(pool, current_account).await?;
    if acc.kind == "broker" {
        return Ok(None);
    }
    let Some(inst_id) = acc.institution_id else {
        return Ok(None);
    };
    find_broker_account_for_institution(pool, inst_id).await
}

/// Finds the sole non-archived non-broker account within the institution —
/// the cash/settlement account through which securities payments flow.
/// Returns None when 0 or >1 such accounts exist (ambiguous).
pub async fn find_cash_settlement_account_for_institution(
    pool: &SqlitePool,
    institution_id: i64,
) -> DbResult<Option<i64>> {
    let rows: Vec<(i64,)> = sqlx::query_as(
        "SELECT id FROM accounts
          WHERE institution_id = ?1
            AND kind != 'broker'
            AND archived = 0",
    )
    .bind(institution_id)
    .fetch_all(pool)
    .await?;
    if rows.len() == 1 {
        Ok(Some(rows[0].0))
    } else {
        Ok(None)
    }
}

/// Symmetric counterpart to `resolve_trade_account`: determines the cash account
/// for a tx from a depot-statement import (Flatex PDFs).
///
/// Background: at Flatex ALL cash movements run through the settlement account;
/// the depot holds only securities/BTC. When the user selects the depot during
/// a PDF import, the tx should be auto-routed to the settlement account — the
/// trade detail row stays on the depot
/// (`securities_trades.account_id` explicitly set by the import flow).
///
/// Returns `Some(cash_id)` when `current_account` is a broker with an
/// institution_id AND the institution has exactly one non-broker account.
/// Otherwise `None` (no re-route; the user chose correctly).
pub async fn resolve_cash_settlement_account(
    pool: &SqlitePool,
    current_account: i64,
) -> DbResult<Option<i64>> {
    let acc = get_account(pool, current_account).await?;
    if acc.kind != "broker" {
        return Ok(None);
    }
    let Some(inst_id) = acc.institution_id else {
        return Ok(None);
    };
    find_cash_settlement_account_for_institution(pool, inst_id).await
}

pub async fn create_account(
    pool: &SqlitePool,
    name: &str,
    kind: &str,
    currency: &str,
    parent_id: Option<i64>,
    iban: Option<&str>,
    institution_id: Option<i64>,
) -> DbResult<Account> {
    let sql = format!(
        "INSERT INTO accounts (name, kind, currency, parent_id, iban, institution_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         RETURNING {ACCOUNT_COLUMNS}"
    );
    Ok(sqlx::query_as::<_, Account>(&sql)
        .bind(name)
        .bind(kind)
        .bind(currency)
        .bind(parent_id)
        .bind(iban)
        .bind(institution_id)
        .fetch_one(pool)
        .await?)
}

pub async fn get_account(pool: &SqlitePool, id: i64) -> DbResult<Account> {
    let sql = format!("SELECT {ACCOUNT_COLUMNS} FROM accounts WHERE id = ?1");
    Ok(sqlx::query_as::<_, Account>(&sql)
        .bind(id)
        .fetch_one(pool)
        .await?)
}

pub async fn update_account(pool: &SqlitePool, account: &Account) -> DbResult<()> {
    sqlx::query(
        "UPDATE accounts SET
            name = ?1,
            kind = ?2,
            currency = ?3,
            icon = ?4,
            color = ?5,
            note = ?6,
            last4 = ?7,
            archived = ?8,
            parent_id = ?9,
            iban = ?10,
            institution_id = ?11
         WHERE id = ?12",
    )
    .bind(&account.name)
    .bind(&account.kind)
    .bind(&account.currency)
    .bind(&account.icon)
    .bind(&account.color)
    .bind(&account.note)
    .bind(&account.last4)
    .bind(account.archived)
    .bind(account.parent_id)
    .bind(&account.iban)
    .bind(account.institution_id)
    .bind(account.id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Returns all account IDs in the subtree (including root). Recursive CTE.
/// Currently only used in tests — `account_balance` and `aggregates.rs` keep
/// the CTE inline (see comments there) so queries stay self-contained.
///
/// # Precondition
/// The `parent_id` graph must be acyclic. Cycles are prevented by
/// `validate_no_cycle` in `update_account` (Task 7). Without that guard, a
/// cycle in the DB would exhaust SQLite's recursion limit and error.
#[cfg(test)]
pub(crate) async fn collect_subtree(pool: &SqlitePool, root: i64) -> DbResult<Vec<i64>> {
    let rows: Vec<(i64,)> = sqlx::query_as(
        "WITH RECURSIVE subtree(id) AS (
            SELECT id FROM accounts WHERE id = ?1
            UNION ALL
            SELECT a.id FROM accounts a JOIN subtree s ON a.parent_id = s.id
         )
         SELECT id FROM subtree",
    )
    .bind(root)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|(id,)| id).collect())
}

pub async fn account_balance(pool: &SqlitePool, id: i64) -> DbResult<i64> {
    // Inline subtree-CTE — keep in sync with `collect_subtree` above.
    let (sum,): (i64,) = sqlx::query_as(
        "WITH RECURSIVE subtree(id) AS (
            SELECT id FROM accounts WHERE id = ?1
            UNION ALL
            SELECT a.id FROM accounts a JOIN subtree s ON a.parent_id = s.id
         )
         SELECT COALESCE(SUM(amount_cents), 0) FROM transactions
         WHERE account_id IN (SELECT id FROM subtree)",
    )
    .bind(id)
    .fetch_one(pool)
    .await?;
    Ok(sum)
}

/// Ensures that `new_parent_id` does not create a cycle with `account_id`.
/// If the ancestor traversal of `new_parent_id` reaches `account_id`,
/// a cycle would exist.
pub async fn validate_no_cycle(
    pool: &SqlitePool,
    account_id: i64,
    new_parent_id: i64,
) -> DbResult<()> {
    if account_id == new_parent_id {
        return Err(DbError::Decode(format!(
            "cycle: account {account_id} cannot be its own parent"
        )));
    }
    let mut current: Option<i64> = Some(new_parent_id);
    let mut steps = 0;
    while let Some(cur) = current {
        if cur == account_id {
            return Err(DbError::Decode(format!(
                "cycle: account {account_id} would become its own ancestor"
            )));
        }
        steps += 1;
        if steps > 10_000 {
            return Err(DbError::Decode("cycle: ancestor traversal too deep".into()));
        }
        let row: Option<(Option<i64>,)> = sqlx::query_as(
            "SELECT parent_id FROM accounts WHERE id = ?1",
        )
        .bind(cur)
        .fetch_optional(pool)
        .await?;
        current = row.and_then(|(p,)| p);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_memory;

    async fn seed_inst(pool: &SqlitePool, name: &str) -> i64 {
        let (id,): (i64,) = sqlx::query_as(
            "INSERT INTO institutions (name) VALUES (?1) RETURNING id"
        ).bind(name).fetch_one(pool).await.unwrap();
        id
    }

    #[tokio::test]
    async fn resolve_cash_settlement_routes_broker_to_sole_sibling_cash_account() {
        let pool = connect_memory().await.unwrap();
        let inst = seed_inst(&pool, "Flatex").await;
        let cash = create_account(&pool, "Flatex Cash", "savings", "EUR", None, None, Some(inst))
            .await.unwrap();
        let broker = create_account(&pool, "Flatex Depot", "broker", "EUR", None, None, Some(inst))
            .await.unwrap();

        let resolved = resolve_cash_settlement_account(&pool, broker.id).await.unwrap();
        assert_eq!(resolved, Some(cash.id),
            "Broker in institution with exactly one cash account must be re-routed");
    }

    #[tokio::test]
    async fn resolve_cash_settlement_returns_none_when_called_on_cash_account() {
        let pool = connect_memory().await.unwrap();
        let inst = seed_inst(&pool, "Flatex").await;
        let cash = create_account(&pool, "Flatex Cash", "savings", "EUR", None, None, Some(inst))
            .await.unwrap();
        let _broker = create_account(&pool, "Flatex Depot", "broker", "EUR", None, None, Some(inst))
            .await.unwrap();

        let resolved = resolve_cash_settlement_account(&pool, cash.id).await.unwrap();
        assert!(resolved.is_none(),
            "Cash account needs no re-route — user already chose correctly");
    }

    #[tokio::test]
    async fn resolve_cash_settlement_returns_none_when_no_institution() {
        let pool = connect_memory().await.unwrap();
        // Broker without institution_id — no sibling lookup possible
        let broker = create_account(&pool, "Standalone-Depot", "broker", "EUR", None, None, None)
            .await.unwrap();
        let resolved = resolve_cash_settlement_account(&pool, broker.id).await.unwrap();
        assert!(resolved.is_none());
    }

    #[tokio::test]
    async fn resolve_cash_settlement_returns_none_when_multiple_cash_siblings() {
        let pool = connect_memory().await.unwrap();
        let inst = seed_inst(&pool, "MultiCash").await;
        // 2 non-broker accounts → ambiguous → no auto-routing
        let _cash1 = create_account(&pool, "Giro", "bank", "EUR", None, None, Some(inst))
            .await.unwrap();
        let _cash2 = create_account(&pool, "Sparkonto", "savings", "EUR", None, None, Some(inst))
            .await.unwrap();
        let broker = create_account(&pool, "Depot", "broker", "EUR", None, None, Some(inst))
            .await.unwrap();
        let resolved = resolve_cash_settlement_account(&pool, broker.id).await.unwrap();
        assert!(resolved.is_none(),
            "Ambiguous (>1 cash accounts) — user must decide");
    }

    #[tokio::test]
    async fn resolve_cash_settlement_ignores_archived_cash_accounts() {
        let pool = connect_memory().await.unwrap();
        let inst = seed_inst(&pool, "Flatex").await;
        let mut old_cash = create_account(&pool, "Alte Verrechnung", "savings", "EUR", None, None, Some(inst))
            .await.unwrap();
        old_cash.archived = true;
        update_account(&pool, &old_cash).await.unwrap();
        let new_cash = create_account(&pool, "Neue Verrechnung", "savings", "EUR", None, None, Some(inst))
            .await.unwrap();
        let broker = create_account(&pool, "Depot", "broker", "EUR", None, None, Some(inst))
            .await.unwrap();

        let resolved = resolve_cash_settlement_account(&pool, broker.id).await.unwrap();
        assert_eq!(resolved, Some(new_cash.id),
            "Archived cash accounts must not be counted");
    }

    #[tokio::test]
    async fn create_then_list_accounts_returns_new_columns_defaulted() {
        let pool = connect_memory().await.unwrap();
        let inserted = create_account(&pool, "TR Verrechnung", "bank", "EUR", None, None, None)
            .await
            .unwrap();
        assert_eq!(inserted.name, "TR Verrechnung");
        assert!(inserted.icon.is_none());
        assert!(inserted.color.is_none());
        assert!(inserted.note.is_none());
        assert!(inserted.last4.is_none());
        assert!(!inserted.archived);

        let listed = list_accounts(&pool).await.unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, inserted.id);
    }

    #[tokio::test]
    async fn get_account_returns_inserted_account() {
        let pool = connect_memory().await.unwrap();
        let inserted = create_account(&pool, "TR", "bank", "EUR", None, None, None).await.unwrap();
        let fetched = get_account(&pool, inserted.id).await.unwrap();
        assert_eq!(fetched.id, inserted.id);
        assert_eq!(fetched.name, "TR");
    }

    #[tokio::test]
    async fn get_account_unknown_id_errors() {
        let pool = connect_memory().await.unwrap();
        let err = get_account(&pool, 9_999).await.unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("no rows"), "got: {msg}");
    }

    #[tokio::test]
    async fn update_account_round_trip_all_fields() {
        let pool = connect_memory().await.unwrap();
        let mut acc = create_account(&pool, "TR", "bank", "EUR", None, None, None).await.unwrap();
        acc.name = "TR Hauptkonto".into();
        acc.kind = "savings".into();
        acc.icon = Some("piggy".into());
        acc.color = Some("oklch(0.55 0.13 230)".into());
        acc.note = Some("Gehalt + Miete".into());
        acc.last4 = Some("4321".into());
        acc.archived = true;
        acc.iban = Some("DE89370400440532013000".into());
        update_account(&pool, &acc).await.unwrap();

        let fetched = get_account(&pool, acc.id).await.unwrap();
        assert_eq!(fetched.name, "TR Hauptkonto");
        assert_eq!(fetched.kind, "savings");
        assert_eq!(fetched.icon.as_deref(), Some("piggy"));
        assert_eq!(fetched.color.as_deref(), Some("oklch(0.55 0.13 230)"));
        assert_eq!(fetched.note.as_deref(), Some("Gehalt + Miete"));
        assert_eq!(fetched.last4.as_deref(), Some("4321"));
        assert!(fetched.archived);
        assert_eq!(fetched.iban.as_deref(), Some("DE89370400440532013000"));
        assert!(fetched.parent_id.is_none());
    }

    #[tokio::test]
    async fn update_account_last4_invalid_rejected_by_check() {
        let pool = connect_memory().await.unwrap();
        let mut acc = create_account(&pool, "TR", "bank", "EUR", None, None, None).await.unwrap();
        acc.last4 = Some("12a4".into());
        let err = update_account(&pool, &acc).await.unwrap_err();
        assert!(err.to_string().to_lowercase().contains("check"), "got: {err}");

        acc.last4 = Some("12345".into());
        let err = update_account(&pool, &acc).await.unwrap_err();
        assert!(err.to_string().to_lowercase().contains("check"), "got: {err}");
    }

    #[tokio::test]
    async fn update_account_last4_none_accepted() {
        let pool = connect_memory().await.unwrap();
        let mut acc = create_account(&pool, "TR", "bank", "EUR", None, None, None).await.unwrap();
        acc.last4 = Some("0000".into());
        update_account(&pool, &acc).await.unwrap();
        acc.last4 = None;
        update_account(&pool, &acc).await.unwrap();
        let fetched = get_account(&pool, acc.id).await.unwrap();
        assert!(fetched.last4.is_none());
    }

    #[tokio::test]
    async fn account_balance_rollup_two_levels() {
        let pool = connect_memory().await.unwrap();
        let parent = create_account(&pool, "p", "bank", "EUR", None, None, None).await.unwrap();
        let child = create_account(&pool, "c", "cash", "EUR", Some(parent.id), None, None).await.unwrap();

        sqlx::query(
            "INSERT INTO transactions
                (account_id, booking_date, amount_cents, currency, counterparty, source)
             VALUES
                (?1, '2026-05-01', 100_000, 'EUR', 'a', 'manual'),
                (?2, '2026-05-02',  50_000, 'EUR', 'b', 'manual')",
        )
        .bind(parent.id)
        .bind(child.id)
        .execute(&pool)
        .await
        .unwrap();

        assert_eq!(account_balance(&pool, parent.id).await.unwrap(), 150_000);
        assert_eq!(account_balance(&pool, child.id).await.unwrap(), 50_000);
    }

    #[tokio::test]
    async fn account_balance_rollup_three_levels() {
        let pool = connect_memory().await.unwrap();
        let a = create_account(&pool, "a", "bank", "EUR", None, None, None).await.unwrap();
        let b = create_account(&pool, "b", "bank", "EUR", Some(a.id), None, None).await.unwrap();
        let c = create_account(&pool, "c", "cash", "EUR", Some(b.id), None, None).await.unwrap();

        sqlx::query(
            "INSERT INTO transactions
                (account_id, booking_date, amount_cents, currency, counterparty, source)
             VALUES
                (?1, '2026-05-01', 100, 'EUR', 'x', 'manual'),
                (?2, '2026-05-01',  20, 'EUR', 'x', 'manual'),
                (?3, '2026-05-01',   3, 'EUR', 'x', 'manual')",
        )
        .bind(a.id).bind(b.id).bind(c.id)
        .execute(&pool).await.unwrap();

        assert_eq!(account_balance(&pool, a.id).await.unwrap(), 123);
        assert_eq!(account_balance(&pool, b.id).await.unwrap(), 23);
        assert_eq!(account_balance(&pool, c.id).await.unwrap(), 3);
    }

    #[tokio::test]
    async fn account_balance_includes_archived_subaccount() {
        let pool = connect_memory().await.unwrap();
        let parent = create_account(&pool, "p", "bank", "EUR", None, None, None).await.unwrap();
        let mut child = create_account(&pool, "c", "cash", "EUR", Some(parent.id), None, None).await.unwrap();
        child.archived = true;
        update_account(&pool, &child).await.unwrap();

        sqlx::query(
            "INSERT INTO transactions
                (account_id, booking_date, amount_cents, currency, counterparty, source)
             VALUES
                (?1, '2026-05-01', 1_000, 'EUR', 'a', 'manual')",
        )
        .bind(child.id).execute(&pool).await.unwrap();

        assert_eq!(account_balance(&pool, parent.id).await.unwrap(), 1_000);
    }

    #[tokio::test]
    async fn account_balance_sums_only_target_account() {
        let pool = connect_memory().await.unwrap();
        let a = create_account(&pool, "A", "bank", "EUR", None, None, None).await.unwrap();
        let b = create_account(&pool, "B", "bank", "EUR", None, None, None).await.unwrap();

        sqlx::query(
            "INSERT INTO transactions
                (account_id, booking_date, amount_cents, currency, counterparty, source)
             VALUES
                (?1, '2026-05-01', 250000, 'EUR', 'in', 'manual'),
                (?1, '2026-05-15', -50000, 'EUR', 'out', 'manual'),
                (?2, '2026-05-15', 99999, 'EUR', 'other', 'manual')",
        )
        .bind(a.id)
        .bind(b.id)
        .execute(&pool)
        .await
        .unwrap();

        assert_eq!(account_balance(&pool, a.id).await.unwrap(), 200_000);
        assert_eq!(account_balance(&pool, b.id).await.unwrap(), 99_999);
    }

    #[tokio::test]
    async fn account_balance_empty_is_zero() {
        let pool = connect_memory().await.unwrap();
        let acc = create_account(&pool, "A", "bank", "EUR", None, None, None).await.unwrap();
        assert_eq!(account_balance(&pool, acc.id).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn account_balance_unknown_id_returns_zero() {
        // Contract: COALESCE returns 0 for missing ID (caller must check existence separately).
        let pool = connect_memory().await.unwrap();
        assert_eq!(account_balance(&pool, 9_999).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn validate_no_cycle_rejects_self_parent() {
        let pool = connect_memory().await.unwrap();
        let a = create_account(&pool, "a", "bank", "EUR", None, None, None).await.unwrap();
        let err = validate_no_cycle(&pool, a.id, a.id).await.unwrap_err();
        assert!(err.to_string().to_lowercase().contains("cycle"), "got: {err}");
    }

    #[tokio::test]
    async fn validate_no_cycle_rejects_indirect_cycle() {
        // A -> B -> C. Attempt: A.parent = C → would create cycle A->B->C->A.
        let pool = connect_memory().await.unwrap();
        let a = create_account(&pool, "a", "bank", "EUR", None, None, None).await.unwrap();
        let b = create_account(&pool, "b", "bank", "EUR", Some(a.id), None, None).await.unwrap();
        let c = create_account(&pool, "c", "bank", "EUR", Some(b.id), None, None).await.unwrap();
        let err = validate_no_cycle(&pool, a.id, c.id).await.unwrap_err();
        assert!(err.to_string().to_lowercase().contains("cycle"), "got: {err}");
    }

    #[tokio::test]
    async fn validate_no_cycle_accepts_valid_move() {
        let pool = connect_memory().await.unwrap();
        let a = create_account(&pool, "a", "bank", "EUR", None, None, None).await.unwrap();
        let b = create_account(&pool, "b", "bank", "EUR", None, None, None).await.unwrap();
        // b.parent = a is fine (no cycle).
        validate_no_cycle(&pool, b.id, a.id).await.unwrap();
    }

    #[tokio::test]
    async fn validate_no_cycle_unknown_parent_is_ok() {
        // Orphan parent (does not exist) → fetch_optional returns None → Ok.
        let pool = connect_memory().await.unwrap();
        let a = create_account(&pool, "a", "bank", "EUR", None, None, None).await.unwrap();
        validate_no_cycle(&pool, a.id, 9_999).await.unwrap();
    }

    #[tokio::test]
    async fn collect_subtree_returns_root_and_all_descendants() {
        let pool = connect_memory().await.unwrap();
        let root = create_account(&pool, "root", "bank", "EUR", None, None, None).await.unwrap();
        let mid = create_account(&pool, "mid", "bank", "EUR", Some(root.id), None, None).await.unwrap();
        let leaf = create_account(&pool, "leaf", "cash", "EUR", Some(mid.id), None, None).await.unwrap();
        let sibling = create_account(&pool, "sibling", "bank", "EUR", None, None, None).await.unwrap();

        let mut subtree = collect_subtree(&pool, root.id).await.unwrap();
        subtree.sort();
        let mut expected = vec![root.id, mid.id, leaf.id];
        expected.sort();
        assert_eq!(subtree, expected);

        let only_sibling = collect_subtree(&pool, sibling.id).await.unwrap();
        assert_eq!(only_sibling, vec![sibling.id]);
    }

    #[tokio::test]
    async fn iban_invalid_format_rejected_by_check() {
        let pool = connect_memory().await.unwrap();
        // Lowercase → CHECK fails (GLOB '[A-Z][A-Z][0-9][0-9]*').
        let err = create_account(&pool, "x", "bank", "EUR", None, Some("de89370400440532013000"), None)
            .await
            .unwrap_err();
        assert!(err.to_string().to_lowercase().contains("check"), "got: {err}");
    }

    #[tokio::test]
    async fn iban_too_short_rejected_by_check() {
        let pool = connect_memory().await.unwrap();
        let err = create_account(&pool, "x", "bank", "EUR", None, Some("DE891234"), None)
            .await
            .unwrap_err();
        assert!(err.to_string().to_lowercase().contains("check"), "got: {err}");
    }

    #[tokio::test]
    async fn iban_valid_accepted() {
        let pool = connect_memory().await.unwrap();
        let acc = create_account(&pool, "x", "bank", "EUR", None, Some("DE89370400440532013000"), None)
            .await
            .unwrap();
        assert_eq!(acc.iban.as_deref(), Some("DE89370400440532013000"));
    }

    #[tokio::test]
    async fn iban_unique_constraint_rejects_duplicate_non_null() {
        let pool = connect_memory().await.unwrap();
        create_account(&pool, "a", "bank", "EUR", None, Some("DE89370400440532013000"), None)
            .await
            .unwrap();
        let err = create_account(&pool, "b", "bank", "EUR", None, Some("DE89370400440532013000"), None)
            .await
            .unwrap_err();
        assert!(err.to_string().to_lowercase().contains("unique"), "got: {err}");
    }

    #[tokio::test]
    async fn iban_unique_constraint_allows_multiple_null() {
        let pool = connect_memory().await.unwrap();
        create_account(&pool, "a", "cash", "EUR", None, None, None).await.unwrap();
        create_account(&pool, "b", "cash", "EUR", None, None, None).await.unwrap();
        create_account(&pool, "c", "cash", "EUR", None, None, None).await.unwrap();
        let listed = list_accounts(&pool).await.unwrap();
        assert_eq!(listed.len(), 3);
    }

    #[tokio::test]
    async fn delete_parent_sets_children_parent_id_to_null() {
        let pool = connect_memory().await.unwrap();
        let parent = create_account(&pool, "p", "bank", "EUR", None, None, None).await.unwrap();
        let child = create_account(&pool, "c", "cash", "EUR", Some(parent.id), None, None).await.unwrap();

        sqlx::query("DELETE FROM accounts WHERE id = ?1")
            .bind(parent.id)
            .execute(&pool)
            .await
            .unwrap();

        let reloaded = get_account(&pool, child.id).await.unwrap();
        assert_eq!(reloaded.parent_id, None);
    }

    #[tokio::test]
    async fn create_account_with_institution_id_persists() {
        let pool = connect_memory().await.unwrap();
        sqlx::query(
            "INSERT INTO institutions (name, icon, color) VALUES ('TestBank', 'bank', NULL)",
        ).execute(&pool).await.unwrap();
        let (inst_id,): (i64,) = sqlx::query_as("SELECT id FROM institutions WHERE name='TestBank'")
            .fetch_one(&pool).await.unwrap();

        let acc = create_account(&pool, "Giro", "bank", "EUR", None, None, Some(inst_id))
            .await.unwrap();
        assert_eq!(acc.institution_id, Some(inst_id));

        let reloaded = get_account(&pool, acc.id).await.unwrap();
        assert_eq!(reloaded.institution_id, Some(inst_id));
    }

    #[tokio::test]
    async fn list_accounts_by_institution_filters_correctly() {
        let pool = connect_memory().await.unwrap();
        sqlx::query("INSERT INTO institutions (name) VALUES ('X')")
            .execute(&pool).await.unwrap();
        let (inst_id,): (i64,) = sqlx::query_as("SELECT id FROM institutions WHERE name='X'")
            .fetch_one(&pool).await.unwrap();
        let a = create_account(&pool, "a", "bank", "EUR", None, None, Some(inst_id)).await.unwrap();
        let _b = create_account(&pool, "b", "bank", "EUR", None, None, None).await.unwrap();

        let listed = list_accounts_by_institution(&pool, Some(inst_id)).await.unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, a.id);
    }

    #[tokio::test]
    async fn find_broker_account_returns_unique_broker() {
        let pool = connect_memory().await.unwrap();
        sqlx::query("INSERT INTO institutions (name) VALUES ('TR')")
            .execute(&pool).await.unwrap();
        let (inst_id,): (i64,) = sqlx::query_as("SELECT id FROM institutions WHERE name='TR'")
            .fetch_one(&pool).await.unwrap();
        let _verrechnung = create_account(&pool, "Verrechnung", "bank", "EUR", None, None, Some(inst_id)).await.unwrap();
        let depot = create_account(&pool, "Depot", "broker", "EUR", None, None, Some(inst_id)).await.unwrap();
        assert_eq!(find_broker_account_for_institution(&pool, inst_id).await.unwrap(), Some(depot.id));
    }

    #[tokio::test]
    async fn find_broker_account_returns_none_when_no_broker() {
        let pool = connect_memory().await.unwrap();
        sqlx::query("INSERT INTO institutions (name) VALUES ('TR')")
            .execute(&pool).await.unwrap();
        let (inst_id,): (i64,) = sqlx::query_as("SELECT id FROM institutions WHERE name='TR'")
            .fetch_one(&pool).await.unwrap();
        let _verrechnung = create_account(&pool, "Verrechnung", "bank", "EUR", None, None, Some(inst_id)).await.unwrap();
        assert_eq!(find_broker_account_for_institution(&pool, inst_id).await.unwrap(), None);
    }

    #[tokio::test]
    async fn find_broker_account_returns_none_when_multiple_brokers() {
        let pool = connect_memory().await.unwrap();
        sqlx::query("INSERT INTO institutions (name) VALUES ('TR')")
            .execute(&pool).await.unwrap();
        let (inst_id,): (i64,) = sqlx::query_as("SELECT id FROM institutions WHERE name='TR'")
            .fetch_one(&pool).await.unwrap();
        let _d1 = create_account(&pool, "Depot1", "broker", "EUR", None, None, Some(inst_id)).await.unwrap();
        let _d2 = create_account(&pool, "Depot2", "broker", "EUR", None, None, Some(inst_id)).await.unwrap();
        assert_eq!(find_broker_account_for_institution(&pool, inst_id).await.unwrap(), None);
    }

    #[tokio::test]
    async fn find_broker_account_ignores_archived_brokers() {
        let pool = connect_memory().await.unwrap();
        sqlx::query("INSERT INTO institutions (name) VALUES ('TR')")
            .execute(&pool).await.unwrap();
        let (inst_id,): (i64,) = sqlx::query_as("SELECT id FROM institutions WHERE name='TR'")
            .fetch_one(&pool).await.unwrap();
        let mut archived = create_account(&pool, "ArchivedDepot", "broker", "EUR", None, None, Some(inst_id)).await.unwrap();
        archived.archived = true;
        update_account(&pool, &archived).await.unwrap();
        let active = create_account(&pool, "ActiveDepot", "broker", "EUR", None, None, Some(inst_id)).await.unwrap();
        assert_eq!(find_broker_account_for_institution(&pool, inst_id).await.unwrap(), Some(active.id));
    }

    #[tokio::test]
    async fn list_accounts_by_institution_none_returns_unassigned() {
        let pool = connect_memory().await.unwrap();
        sqlx::query("INSERT INTO institutions (name) VALUES ('X')")
            .execute(&pool).await.unwrap();
        let (inst_id,): (i64,) = sqlx::query_as("SELECT id FROM institutions WHERE name='X'")
            .fetch_one(&pool).await.unwrap();
        let _a = create_account(&pool, "a", "bank", "EUR", None, None, Some(inst_id)).await.unwrap();
        let b = create_account(&pool, "b", "bank", "EUR", None, None, None).await.unwrap();

        let listed = list_accounts_by_institution(&pool, None).await.unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, b.id);
    }
}
