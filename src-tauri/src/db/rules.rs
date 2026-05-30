use sqlx::SqlitePool;

use crate::categorization::rules::{Combinator, MatchField, MatchOp, Rule, RuleCondition};

use super::{DbError, DbResult};

#[derive(Debug, Clone)]
pub struct NewRule {
    pub priority: i32,
    pub name: String,
    pub combinator: Combinator,
    pub conditions: Vec<RuleCondition>,
    pub target_category_id: i64,
    pub enabled: bool,
}

const FIELD_COUNTERPARTY: &str = "counterparty";
const FIELD_DESCRIPTION: &str = "description";
const FIELD_AMOUNT: &str = "amount";
const FIELD_ACCOUNT: &str = "account";

const OP_CONTAINS: &str = "contains";
const OP_EQUALS: &str = "equals";
const OP_STARTS_WITH: &str = "starts_with";
const OP_ENDS_WITH: &str = "ends_with";
const OP_REGEX: &str = "regex";
const OP_RANGE: &str = "range";

const COMB_AND: &str = "and";
const COMB_OR: &str = "or";

fn field_to_str(f: &MatchField) -> &'static str {
    match f {
        MatchField::Counterparty => FIELD_COUNTERPARTY,
        MatchField::Description => FIELD_DESCRIPTION,
        MatchField::Amount => FIELD_AMOUNT,
        MatchField::Account => FIELD_ACCOUNT,
    }
}

fn field_from_str(s: &str) -> DbResult<MatchField> {
    match s {
        FIELD_COUNTERPARTY => Ok(MatchField::Counterparty),
        FIELD_DESCRIPTION => Ok(MatchField::Description),
        FIELD_AMOUNT => Ok(MatchField::Amount),
        FIELD_ACCOUNT => Ok(MatchField::Account),
        other => Err(DbError::Decode(format!("unknown match_field '{other}'"))),
    }
}

fn combinator_to_str(c: &Combinator) -> &'static str {
    match c {
        Combinator::And => COMB_AND,
        Combinator::Or => COMB_OR,
    }
}

fn combinator_from_str(s: &str) -> DbResult<Combinator> {
    match s {
        COMB_AND => Ok(Combinator::And),
        COMB_OR => Ok(Combinator::Or),
        other => Err(DbError::Decode(format!("unknown combinator '{other}'"))),
    }
}

/// Returns `(op_string, value_string)` for the DB columns `op` / `value`.
/// Range is encoded as `"min..max"` (cents as integer) in `value`.
fn op_to_db(op: &MatchOp) -> (&'static str, String) {
    match op {
        MatchOp::Contains(v) => (OP_CONTAINS, v.clone()),
        MatchOp::Equals(v) => (OP_EQUALS, v.clone()),
        MatchOp::StartsWith(v) => (OP_STARTS_WITH, v.clone()),
        MatchOp::EndsWith(v) => (OP_ENDS_WITH, v.clone()),
        MatchOp::Regex(v) => (OP_REGEX, v.clone()),
        MatchOp::Range {
            min_cents,
            max_cents,
        } => (OP_RANGE, format!("{min_cents}..{max_cents}")),
    }
}

fn op_from_db(op: &str, value: &str) -> DbResult<MatchOp> {
    match op {
        OP_CONTAINS => Ok(MatchOp::Contains(value.to_string())),
        OP_EQUALS => Ok(MatchOp::Equals(value.to_string())),
        OP_STARTS_WITH => Ok(MatchOp::StartsWith(value.to_string())),
        OP_ENDS_WITH => Ok(MatchOp::EndsWith(value.to_string())),
        OP_REGEX => Ok(MatchOp::Regex(value.to_string())),
        OP_RANGE => {
            let (min, max) = value
                .split_once("..")
                .ok_or_else(|| DbError::Decode(format!("bad range value '{value}'")))?;
            let min_cents = min
                .parse::<i64>()
                .map_err(|e| DbError::Decode(format!("bad range min '{min}': {e}")))?;
            let max_cents = max
                .parse::<i64>()
                .map_err(|e| DbError::Decode(format!("bad range max '{max}': {e}")))?;
            Ok(MatchOp::Range {
                min_cents,
                max_cents,
            })
        }
        other => Err(DbError::Decode(format!("unknown match_op '{other}'"))),
    }
}

async fn insert_conditions(
    pool: &SqlitePool,
    rule_id: i64,
    conditions: &[RuleCondition],
) -> DbResult<()> {
    for (idx, cond) in conditions.iter().enumerate() {
        let (op_str, value_str) = op_to_db(&cond.op);
        sqlx::query(
            "INSERT INTO rule_conditions (rule_id, position, field, op, value)
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )
        .bind(rule_id)
        .bind(idx as i64)
        .bind(field_to_str(&cond.field))
        .bind(op_str)
        .bind(&value_str)
        .execute(pool)
        .await?;
    }
    Ok(())
}

pub async fn insert_rule(pool: &SqlitePool, rule: &NewRule) -> DbResult<i64> {
    if rule.conditions.is_empty() {
        return Err(DbError::Decode(
            "rule must have at least one condition".into(),
        ));
    }
    let (id,): (i64,) = sqlx::query_as(
        "INSERT INTO rules
            (priority, name, combinator, target_category_id, enabled)
         VALUES (?1, ?2, ?3, ?4, ?5)
         RETURNING id",
    )
    .bind(rule.priority)
    .bind(&rule.name)
    .bind(combinator_to_str(&rule.combinator))
    .bind(rule.target_category_id)
    .bind(rule.enabled as i32)
    .fetch_one(pool)
    .await?;
    insert_conditions(pool, id, &rule.conditions).await?;
    Ok(id)
}

pub async fn list_rules(pool: &SqlitePool) -> DbResult<Vec<Rule>> {
    let rule_rows: Vec<(i64, i32, String, String, i64, i32)> = sqlx::query_as(
        "SELECT id, priority, name, combinator, target_category_id, enabled
         FROM rules
         ORDER BY priority, id",
    )
    .fetch_all(pool)
    .await?;

    let cond_rows: Vec<(i64, i64, String, String, String)> = sqlx::query_as(
        "SELECT rule_id, position, field, op, value
         FROM rule_conditions
         ORDER BY rule_id, position",
    )
    .fetch_all(pool)
    .await?;

    let mut rules: Vec<Rule> = rule_rows
        .into_iter()
        .map(|(id, priority, name, comb, target, enabled)| {
            Ok(Rule {
                id,
                priority,
                name,
                combinator: combinator_from_str(&comb)?,
                conditions: Vec::new(),
                target_category_id: target,
                enabled: enabled != 0,
            })
        })
        .collect::<DbResult<Vec<Rule>>>()?;

    for (rule_id, _pos, field, op, value) in cond_rows {
        let cond = RuleCondition {
            field: field_from_str(&field)?,
            op: op_from_db(&op, &value)?,
        };
        if let Some(rule) = rules.iter_mut().find(|r| r.id == rule_id) {
            rule.conditions.push(cond);
        }
    }

    Ok(rules)
}

pub async fn update_rule(pool: &SqlitePool, rule: &Rule) -> DbResult<()> {
    if rule.conditions.is_empty() {
        return Err(DbError::Decode(
            "rule must have at least one condition".into(),
        ));
    }
    // UPDATE + DELETE + re-INSERT of the conditions must be atomic: if the
    // re-insert failed mid-way the rule would be left with no conditions
    // (silently breaking categorization). Wrap all three in one transaction.
    let mut tx = pool.begin().await?;
    sqlx::query(
        "UPDATE rules
         SET priority = ?1, name = ?2, combinator = ?3,
             target_category_id = ?4, enabled = ?5
         WHERE id = ?6",
    )
    .bind(rule.priority)
    .bind(&rule.name)
    .bind(combinator_to_str(&rule.combinator))
    .bind(rule.target_category_id)
    .bind(rule.enabled as i32)
    .bind(rule.id)
    .execute(&mut *tx)
    .await?;

    sqlx::query("DELETE FROM rule_conditions WHERE rule_id = ?1")
        .bind(rule.id)
        .execute(&mut *tx)
        .await?;

    for (idx, cond) in rule.conditions.iter().enumerate() {
        let (op_str, value_str) = op_to_db(&cond.op);
        sqlx::query(
            "INSERT INTO rule_conditions (rule_id, position, field, op, value)
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )
        .bind(rule.id)
        .bind(idx as i64)
        .bind(field_to_str(&cond.field))
        .bind(op_str)
        .bind(&value_str)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

pub async fn delete_rule(pool: &SqlitePool, id: i64) -> DbResult<()> {
    sqlx::query("DELETE FROM rules WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::categorization::rules::{Combinator, MatchField, MatchOp, Rule, RuleCondition};
    use crate::db::connect_memory;
    use crate::db::rules::{delete_rule, insert_rule, list_rules, update_rule, NewRule};
    use sqlx::SqlitePool;

    async fn seed_category(pool: &SqlitePool, name: &str) -> i64 {
        let (id,): (i64,) =
            sqlx::query_as("INSERT INTO categories (name) VALUES (?1) RETURNING id")
                .bind(name)
                .fetch_one(pool)
                .await
                .unwrap();
        id
    }

    fn new_single_rule(cat: i64, field: MatchField, op: MatchOp, name: &str) -> NewRule {
        NewRule {
            priority: 100,
            name: name.into(),
            combinator: Combinator::And,
            conditions: vec![RuleCondition { field, op }],
            target_category_id: cat,
            enabled: true,
        }
    }

    #[tokio::test]
    async fn insert_then_list_rule() {
        let pool = connect_memory().await.unwrap();
        let cat = seed_category(&pool, "Lebensmittel").await;
        let id = insert_rule(
            &pool,
            &new_single_rule(
                cat,
                MatchField::Counterparty,
                MatchOp::Contains("REWE".into()),
                "REWE",
            ),
        )
        .await
        .unwrap();
        assert!(id > 0);

        let rules = list_rules(&pool).await.unwrap();
        assert_eq!(rules.len(), 1);
        let r = &rules[0];
        assert_eq!(r.id, id);
        assert_eq!(r.name, "REWE");
        assert_eq!(r.target_category_id, cat);
        assert!(r.enabled);
        assert_eq!(r.combinator, Combinator::And);
        assert_eq!(r.conditions.len(), 1);
        assert_eq!(r.conditions[0].field, MatchField::Counterparty);
        assert!(matches!(&r.conditions[0].op, MatchOp::Contains(v) if v == "REWE"));
    }

    #[tokio::test]
    async fn insert_rejects_empty_conditions() {
        let pool = connect_memory().await.unwrap();
        let cat = seed_category(&pool, "Cat").await;
        let res = insert_rule(
            &pool,
            &NewRule {
                priority: 1,
                name: "empty".into(),
                combinator: Combinator::And,
                conditions: vec![],
                target_category_id: cat,
                enabled: true,
            },
        )
        .await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn roundtrip_all_match_ops_and_fields() {
        let pool = connect_memory().await.unwrap();
        let cat = seed_category(&pool, "Misc").await;

        let rule = NewRule {
            priority: 10,
            name: "mixed".into(),
            combinator: Combinator::Or,
            conditions: vec![
                RuleCondition {
                    field: MatchField::Counterparty,
                    op: MatchOp::Contains("REWE".into()),
                },
                RuleCondition {
                    field: MatchField::Description,
                    op: MatchOp::StartsWith("Gehalt".into()),
                },
                RuleCondition {
                    field: MatchField::Description,
                    op: MatchOp::EndsWith("Mai".into()),
                },
                RuleCondition {
                    field: MatchField::Description,
                    op: MatchOp::Equals("Bar".into()),
                },
                RuleCondition {
                    field: MatchField::Counterparty,
                    op: MatchOp::Regex(r"^EDEKA".into()),
                },
                RuleCondition {
                    field: MatchField::Amount,
                    op: MatchOp::Range {
                        min_cents: -5000,
                        max_cents: -1000,
                    },
                },
                RuleCondition {
                    field: MatchField::Account,
                    op: MatchOp::Equals("42".into()),
                },
            ],
            target_category_id: cat,
            enabled: false,
        };
        insert_rule(&pool, &rule).await.unwrap();

        let rules = list_rules(&pool).await.unwrap();
        assert_eq!(rules.len(), 1);
        let r = &rules[0];
        assert_eq!(r.combinator, Combinator::Or);
        assert_eq!(r.conditions.len(), 7);
        assert!(matches!(&r.conditions[0].op, MatchOp::Contains(v) if v == "REWE"));
        assert!(matches!(&r.conditions[1].op, MatchOp::StartsWith(v) if v == "Gehalt"));
        assert!(matches!(&r.conditions[2].op, MatchOp::EndsWith(v) if v == "Mai"));
        assert!(matches!(&r.conditions[3].op, MatchOp::Equals(v) if v == "Bar"));
        assert!(matches!(&r.conditions[4].op, MatchOp::Regex(v) if v == "^EDEKA"));
        assert!(matches!(
            &r.conditions[5].op,
            MatchOp::Range {
                min_cents: -5000,
                max_cents: -1000
            }
        ));
        assert_eq!(r.conditions[5].field, MatchField::Amount);
        assert_eq!(r.conditions[6].field, MatchField::Account);
        assert!(matches!(&r.conditions[6].op, MatchOp::Equals(v) if v == "42"));
        assert!(!r.enabled);
    }

    #[tokio::test]
    async fn update_replaces_conditions() {
        let pool = connect_memory().await.unwrap();
        let cat = seed_category(&pool, "Cat").await;
        let id = insert_rule(
            &pool,
            &new_single_rule(
                cat,
                MatchField::Counterparty,
                MatchOp::Contains("REWE".into()),
                "REWE",
            ),
        )
        .await
        .unwrap();

        let updated = Rule {
            id,
            priority: 5,
            name: "REWE specific".into(),
            combinator: Combinator::And,
            conditions: vec![
                RuleCondition {
                    field: MatchField::Counterparty,
                    op: MatchOp::Equals("REWE Markt".into()),
                },
                RuleCondition {
                    field: MatchField::Amount,
                    op: MatchOp::Range {
                        min_cents: -10_000,
                        max_cents: -1,
                    },
                },
            ],
            target_category_id: cat,
            enabled: false,
        };
        update_rule(&pool, &updated).await.unwrap();

        let rules = list_rules(&pool).await.unwrap();
        assert_eq!(rules.len(), 1);
        let r = &rules[0];
        assert_eq!(r.priority, 5);
        assert_eq!(r.name, "REWE specific");
        assert_eq!(r.conditions.len(), 2);
        assert!(matches!(&r.conditions[0].op, MatchOp::Equals(v) if v == "REWE Markt"));
        assert!(matches!(
            &r.conditions[1].op,
            MatchOp::Range {
                min_cents: -10_000,
                max_cents: -1
            }
        ));
        assert!(!r.enabled);
    }

    #[tokio::test]
    async fn delete_removes_rule_and_conditions() {
        let pool = connect_memory().await.unwrap();
        let cat = seed_category(&pool, "Cat").await;
        let id = insert_rule(
            &pool,
            &new_single_rule(
                cat,
                MatchField::Counterparty,
                MatchOp::Contains("REWE".into()),
                "REWE",
            ),
        )
        .await
        .unwrap();

        delete_rule(&pool, id).await.unwrap();
        assert!(list_rules(&pool).await.unwrap().is_empty());

        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM rule_conditions")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 0);
    }
}
