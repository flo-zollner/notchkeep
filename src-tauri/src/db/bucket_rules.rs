use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use super::DbResult;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct BucketRule {
    pub id: i64,
    pub priority: i64,
    pub name: String,
    pub counterparty_contains: Option<String>,
    pub min_amount_cents: Option<i64>,
    pub max_amount_cents: Option<i64>,
    pub target_bucket_id: i64,
    pub enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewBucketRulePayload {
    pub priority: i64,
    pub name: String,
    pub counterparty_contains: Option<String>,
    pub min_amount_cents: Option<i64>,
    pub max_amount_cents: Option<i64>,
    pub target_bucket_id: i64,
    pub enabled: bool,
}

pub async fn list_bucket_rules(pool: &SqlitePool) -> DbResult<Vec<BucketRule>> {
    Ok(sqlx::query_as(
        "SELECT id, priority, name, counterparty_contains, min_amount_cents, max_amount_cents,
                target_bucket_id, enabled
           FROM bucket_rules
          ORDER BY priority ASC, id ASC"
    )
    .fetch_all(pool)
    .await?)
}

pub async fn create_bucket_rule(pool: &SqlitePool, payload: &NewBucketRulePayload) -> DbResult<i64> {
    let row: (i64,) = sqlx::query_as(
        "INSERT INTO bucket_rules (priority, name, counterparty_contains, min_amount_cents,
                                    max_amount_cents, target_bucket_id, enabled)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         RETURNING id"
    )
    .bind(payload.priority)
    .bind(&payload.name)
    .bind(&payload.counterparty_contains)
    .bind(payload.min_amount_cents)
    .bind(payload.max_amount_cents)
    .bind(payload.target_bucket_id)
    .bind(payload.enabled as i64)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

pub async fn update_bucket_rule(pool: &SqlitePool, rule: &BucketRule) -> DbResult<()> {
    sqlx::query(
        "UPDATE bucket_rules
            SET priority = ?2, name = ?3, counterparty_contains = ?4,
                min_amount_cents = ?5, max_amount_cents = ?6,
                target_bucket_id = ?7, enabled = ?8
          WHERE id = ?1"
    )
    .bind(rule.id)
    .bind(rule.priority)
    .bind(&rule.name)
    .bind(&rule.counterparty_contains)
    .bind(rule.min_amount_cents)
    .bind(rule.max_amount_cents)
    .bind(rule.target_bucket_id)
    .bind(rule.enabled as i64)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_bucket_rule(pool: &SqlitePool, id: i64) -> DbResult<()> {
    sqlx::query("DELETE FROM bucket_rules WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Wendet alle aktiven Bucket-Regeln auf Income-Tx ohne Bucket aus den letzten
/// `days` Tagen an. Returnt Anzahl der Tx die durch eine Regel einen Topf bekommen
/// haben. First-Match-Wins nach priority ASC.
pub async fn apply_bucket_rules_to_recent_income(
    pool: &SqlitePool,
    days: u32,
) -> DbResult<usize> {
    let rules = list_bucket_rules(pool).await?;
    let active: Vec<_> = rules.into_iter().filter(|r| r.enabled).collect();
    if active.is_empty() {
        return Ok(0);
    }

    let cutoff = chrono::Utc::now().date_naive() - chrono::Duration::days(days as i64);
    let cutoff_str = cutoff.format("%Y-%m-%d").to_string();

    #[derive(sqlx::FromRow)]
    struct Tx {
        id: i64,
        amount_cents: i64,
        counterparty: Option<String>,
    }

    let txs: Vec<Tx> = sqlx::query_as(
        "SELECT id, amount_cents, counterparty
           FROM transactions
          WHERE amount_cents > 0
            AND bucket_id IS NULL
            AND booking_date >= ?1"
    )
    .bind(&cutoff_str)
    .fetch_all(pool)
    .await?;

    let mut updated = 0usize;
    for tx in txs {
        let cp_lower = tx.counterparty.as_deref().unwrap_or("").to_lowercase();
        for rule in &active {
            // Counterparty-Match (substring, case-insensitive — leerer Needle = matched all)
            let needle = rule.counterparty_contains.as_deref().unwrap_or("").to_lowercase();
            if !needle.is_empty() && !cp_lower.contains(&needle) {
                continue;
            }
            // Amount-Range
            if let Some(min) = rule.min_amount_cents {
                if tx.amount_cents < min {
                    continue;
                }
            }
            if let Some(max) = rule.max_amount_cents {
                if tx.amount_cents > max {
                    continue;
                }
            }
            // Match!
            sqlx::query("UPDATE transactions SET bucket_id = ?1 WHERE id = ?2")
                .bind(rule.target_bucket_id)
                .bind(tx.id)
                .execute(pool)
                .await?;
            updated += 1;
            break;  // first match wins
        }
    }
    Ok(updated)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_memory;

    async fn seed_bucket(pool: &SqlitePool, name: &str) -> i64 {
        let row: (i64,) = sqlx::query_as(
            "INSERT INTO buckets (name) VALUES (?1) RETURNING id"
        ).bind(name).fetch_one(pool).await.unwrap();
        row.0
    }

    async fn seed_account(pool: &SqlitePool) -> i64 {
        let row: (i64,) = sqlx::query_as(
            "INSERT INTO accounts (name, kind, currency) VALUES ('Test', 'bank', 'EUR') RETURNING id"
        ).fetch_one(pool).await.unwrap();
        row.0
    }

    async fn seed_income(pool: &SqlitePool, account_id: i64, amount: i64, counterparty: &str, date: &str) -> i64 {
        let row: (i64,) = sqlx::query_as(
            "INSERT INTO transactions (account_id, booking_date, amount_cents, currency, counterparty, source, imported_at)
             VALUES (?1, ?2, ?3, 'EUR', ?4, 'manual', '2026-01-01T00:00:00Z')
             RETURNING id"
        ).bind(account_id).bind(date).bind(amount).bind(counterparty).fetch_one(pool).await.unwrap();
        row.0
    }

    #[tokio::test]
    async fn create_list_delete_roundtrip() {
        let pool = connect_memory().await.unwrap();
        let bid = seed_bucket(&pool, "Reise").await;
        let payload = NewBucketRulePayload {
            priority: 50, name: "Gehalt → Reise".into(),
            counterparty_contains: Some("Arbeitgeber".into()),
            min_amount_cents: Some(100_000), max_amount_cents: None,
            target_bucket_id: bid, enabled: true,
        };
        let id = create_bucket_rule(&pool, &payload).await.unwrap();
        let rules = list_bucket_rules(&pool).await.unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].id, id);

        delete_bucket_rule(&pool, id).await.unwrap();
        let rules = list_bucket_rules(&pool).await.unwrap();
        assert!(rules.is_empty());
    }

    #[tokio::test]
    async fn apply_assigns_matching_tx_to_bucket() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool).await;
        let bid = seed_bucket(&pool, "Reise").await;
        let today = chrono::Utc::now().date_naive().format("%Y-%m-%d").to_string();
        let tx_id = seed_income(&pool, acc, 250_000, "Arbeitgeber GmbH", &today).await;
        create_bucket_rule(&pool, &NewBucketRulePayload {
            priority: 100, name: "Gehalt".into(),
            counterparty_contains: Some("Arbeitgeber".into()),
            min_amount_cents: None, max_amount_cents: None,
            target_bucket_id: bid, enabled: true,
        }).await.unwrap();

        let n = apply_bucket_rules_to_recent_income(&pool, 30).await.unwrap();
        assert_eq!(n, 1);
        let row: (Option<i64>,) = sqlx::query_as("SELECT bucket_id FROM transactions WHERE id = ?")
            .bind(tx_id).fetch_one(&pool).await.unwrap();
        assert_eq!(row.0, Some(bid));
    }

    #[tokio::test]
    async fn apply_respects_amount_range() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool).await;
        let bid = seed_bucket(&pool, "Big").await;
        let today = chrono::Utc::now().date_naive().format("%Y-%m-%d").to_string();
        let small = seed_income(&pool, acc, 5_000, "Refund", &today).await;
        let big = seed_income(&pool, acc, 500_000, "Refund", &today).await;
        create_bucket_rule(&pool, &NewBucketRulePayload {
            priority: 100, name: "Big-Refunds".into(),
            counterparty_contains: Some("Refund".into()),
            min_amount_cents: Some(100_000), max_amount_cents: None,
            target_bucket_id: bid, enabled: true,
        }).await.unwrap();

        apply_bucket_rules_to_recent_income(&pool, 30).await.unwrap();
        let small_b: (Option<i64>,) = sqlx::query_as("SELECT bucket_id FROM transactions WHERE id = ?")
            .bind(small).fetch_one(&pool).await.unwrap();
        let big_b: (Option<i64>,) = sqlx::query_as("SELECT bucket_id FROM transactions WHERE id = ?")
            .bind(big).fetch_one(&pool).await.unwrap();
        assert!(small_b.0.is_none(), "small income below min should not match");
        assert_eq!(big_b.0, Some(bid));
    }

    #[tokio::test]
    async fn apply_first_match_wins_by_priority() {
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool).await;
        let b1 = seed_bucket(&pool, "P50").await;
        let b2 = seed_bucket(&pool, "P100").await;
        let today = chrono::Utc::now().date_naive().format("%Y-%m-%d").to_string();
        let tx = seed_income(&pool, acc, 100_000, "Arbeitgeber", &today).await;
        create_bucket_rule(&pool, &NewBucketRulePayload {
            priority: 100, name: "Lower-Priority".into(),
            counterparty_contains: None, min_amount_cents: None, max_amount_cents: None,
            target_bucket_id: b2, enabled: true,
        }).await.unwrap();
        create_bucket_rule(&pool, &NewBucketRulePayload {
            priority: 50, name: "Higher-Priority".into(),
            counterparty_contains: None, min_amount_cents: None, max_amount_cents: None,
            target_bucket_id: b1, enabled: true,
        }).await.unwrap();

        apply_bucket_rules_to_recent_income(&pool, 30).await.unwrap();
        let row: (Option<i64>,) = sqlx::query_as("SELECT bucket_id FROM transactions WHERE id = ?")
            .bind(tx).fetch_one(&pool).await.unwrap();
        assert_eq!(row.0, Some(b1), "priority 50 wins over 100");
    }

    #[tokio::test]
    async fn bucket_rules_skip_non_income() {
        // apply_bucket_rules_to_recent_income only touches amount_cents > 0.
        // An expense (negative amount) must not receive a bucket even if a rule matches.
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool).await;
        let bid = seed_bucket(&pool, "SomeBucket").await;
        let today = chrono::Utc::now().date_naive().format("%Y-%m-%d").to_string();

        // Insert an expense tx (amount < 0)
        let (expense_id,): (i64,) = sqlx::query_as(
            "INSERT INTO transactions (account_id, booking_date, amount_cents, currency, counterparty, source, imported_at)
             VALUES (?1, ?2, -50_000, 'EUR', 'Shop GmbH', 'manual', '2026-01-01T00:00:00Z')
             RETURNING id"
        ).bind(acc).bind(&today).fetch_one(&pool).await.unwrap();

        create_bucket_rule(&pool, &NewBucketRulePayload {
            priority: 10, name: "Match-all".into(),
            counterparty_contains: None, min_amount_cents: None, max_amount_cents: None,
            target_bucket_id: bid, enabled: true,
        }).await.unwrap();

        let n = apply_bucket_rules_to_recent_income(&pool, 30).await.unwrap();
        assert_eq!(n, 0, "expense tx must not be processed by bucket rules");

        let row: (Option<i64>,) = sqlx::query_as("SELECT bucket_id FROM transactions WHERE id = ?")
            .bind(expense_id).fetch_one(&pool).await.unwrap();
        assert!(row.0.is_none(), "expense tx bucket_id must remain NULL");
    }

    #[tokio::test]
    async fn bucket_rules_disabled_rule_is_skipped() {
        // A disabled rule must not cause any allocation even if it would match.
        let pool = connect_memory().await.unwrap();
        let acc = seed_account(&pool).await;
        let bid = seed_bucket(&pool, "Disabled").await;
        let today = chrono::Utc::now().date_naive().format("%Y-%m-%d").to_string();
        let tx_id = seed_income(&pool, acc, 200_000, "Arbeitgeber", &today).await;

        create_bucket_rule(&pool, &NewBucketRulePayload {
            priority: 10, name: "Disabled-Rule".into(),
            counterparty_contains: None, min_amount_cents: None, max_amount_cents: None,
            target_bucket_id: bid, enabled: false,
        }).await.unwrap();

        let n = apply_bucket_rules_to_recent_income(&pool, 30).await.unwrap();
        assert_eq!(n, 0, "disabled rule must not match");

        let row: (Option<i64>,) = sqlx::query_as("SELECT bucket_id FROM transactions WHERE id = ?")
            .bind(tx_id).fetch_one(&pool).await.unwrap();
        assert!(row.0.is_none());
    }
}
