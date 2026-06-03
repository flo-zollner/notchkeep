use tauri::State;

use crate::categorization::rules::{Combinator, MatchField, MatchOp, Rule, RuleCondition};
use crate::commands::accounts::{CommandError, DbState};
use crate::db::rules::{
    delete_rule as db_delete_rule, insert_rule as db_insert_rule, list_rules as db_list_rules,
    restore_rule as db_restore_rule, update_rule as db_update_rule, NewRule,
};
use crate::import_flow::{
    bulk_assign_category_by_rule as bulk_assign_impl, count_matching_transactions,
};
use crate::model::{NewRuleDto, RuleConditionDto, RuleDto};

#[tauri::command]
pub async fn list_rules(state: State<'_, DbState>) -> Result<Vec<RuleDto>, CommandError> {
    let rules = db_list_rules(&state.pool()).await?;
    Ok(rules.into_iter().map(rule_to_dto).collect())
}

#[tauri::command]
pub async fn create_rule(
    state: State<'_, DbState>,
    rule: NewRuleDto,
) -> Result<RuleDto, CommandError> {
    let new_rule = dto_to_new_rule(rule)?;
    let pool = state.pool();
    let id = db_insert_rule(&pool, &new_rule).await?;
    let dto = db_list_rules(&pool)
        .await?
        .into_iter()
        .find(|r| r.id == id)
        .map(rule_to_dto)
        .ok_or_else(|| CommandError {
            message: format!("rule {id} disappeared after insert"),
        })?;
    Ok(dto)
}

#[tauri::command]
pub async fn update_rule(
    state: State<'_, DbState>,
    rule: RuleDto,
) -> Result<RuleDto, CommandError> {
    let id = rule.id;
    let domain = dto_to_rule(rule)?;
    let pool = state.pool();
    db_update_rule(&pool, &domain).await?;
    let dto = db_list_rules(&pool)
        .await?
        .into_iter()
        .find(|r| r.id == id)
        .map(rule_to_dto)
        .ok_or_else(|| CommandError {
            message: format!("rule {id} not found after update"),
        })?;
    Ok(dto)
}

#[tauri::command]
pub async fn delete_rule(state: State<'_, DbState>, id: i64) -> Result<(), CommandError> {
    db_delete_rule(&state.pool(), id).await?;
    Ok(())
}

#[tauri::command]
pub async fn restore_rule(state: State<'_, DbState>, id: i64) -> Result<bool, CommandError> {
    Ok(db_restore_rule(&state.pool(), id).await?)
}

#[tauri::command]
pub async fn apply_rule_to_existing(
    state: State<'_, DbState>,
    rule_id: i64,
) -> Result<usize, CommandError> {
    let n = bulk_assign_impl(&state.pool(), rule_id).await?;
    Ok(n)
}

/// Match-preview counter for unsaved rules: takes a rule definition that has
/// not yet been persisted and returns how many existing transactions it would
/// match.
#[tauri::command]
pub async fn preview_rule_match(
    state: State<'_, DbState>,
    rule: NewRuleDto,
) -> Result<usize, CommandError> {
    let new_rule = dto_to_new_rule(rule)?;
    let synthetic = Rule {
        id: 0,
        priority: new_rule.priority,
        name: new_rule.name,
        combinator: new_rule.combinator,
        conditions: new_rule.conditions,
        target_category_id: new_rule.target_category_id,
        enabled: new_rule.enabled,
    };
    let n = count_matching_transactions(&state.pool(), &synthetic).await?;
    Ok(n)
}

// ─── Serializer/Deserializer between domain and DTO ───

fn rule_to_dto(rule: Rule) -> RuleDto {
    RuleDto {
        id: rule.id,
        priority: rule.priority,
        name: rule.name,
        combinator: combinator_to_str(&rule.combinator).into(),
        conditions: rule.conditions.into_iter().map(condition_to_dto).collect(),
        target_category_id: rule.target_category_id,
        enabled: rule.enabled,
    }
}

fn condition_to_dto(c: RuleCondition) -> RuleConditionDto {
    let (op, value) = op_to_dto(&c.op);
    RuleConditionDto {
        field: field_to_str(&c.field).into(),
        op: op.into(),
        value,
    }
}

fn dto_to_new_rule(dto: NewRuleDto) -> Result<NewRule, CommandError> {
    let conditions = dto
        .conditions
        .into_iter()
        .map(dto_to_condition)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(NewRule {
        priority: dto.priority,
        name: dto.name,
        combinator: combinator_from_str(&dto.combinator)?,
        conditions,
        target_category_id: dto.target_category_id,
        enabled: dto.enabled,
    })
}

fn dto_to_rule(dto: RuleDto) -> Result<Rule, CommandError> {
    let conditions = dto
        .conditions
        .into_iter()
        .map(dto_to_condition)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Rule {
        id: dto.id,
        priority: dto.priority,
        name: dto.name,
        combinator: combinator_from_str(&dto.combinator)?,
        conditions,
        target_category_id: dto.target_category_id,
        enabled: dto.enabled,
    })
}

fn dto_to_condition(d: RuleConditionDto) -> Result<RuleCondition, CommandError> {
    Ok(RuleCondition {
        field: field_from_str(&d.field)?,
        op: op_from_dto(&d.op, &d.value)?,
    })
}

fn field_to_str(f: &MatchField) -> &'static str {
    match f {
        MatchField::Counterparty => "counterparty",
        MatchField::Description => "description",
        MatchField::Amount => "amount",
        MatchField::Account => "account",
    }
}

fn field_from_str(s: &str) -> Result<MatchField, CommandError> {
    match s {
        "counterparty" => Ok(MatchField::Counterparty),
        "description" => Ok(MatchField::Description),
        "amount" => Ok(MatchField::Amount),
        "account" => Ok(MatchField::Account),
        other => Err(CommandError {
            message: format!("unknown match field '{other}'"),
        }),
    }
}

fn combinator_to_str(c: &Combinator) -> &'static str {
    match c {
        Combinator::And => "and",
        Combinator::Or => "or",
    }
}

fn combinator_from_str(s: &str) -> Result<Combinator, CommandError> {
    match s {
        "and" => Ok(Combinator::And),
        "or" => Ok(Combinator::Or),
        other => Err(CommandError {
            message: format!("unknown combinator '{other}'"),
        }),
    }
}

fn op_to_dto(op: &MatchOp) -> (&'static str, String) {
    match op {
        MatchOp::Contains(v) => ("contains", v.clone()),
        MatchOp::Equals(v) => ("equals", v.clone()),
        MatchOp::StartsWith(v) => ("starts_with", v.clone()),
        MatchOp::EndsWith(v) => ("ends_with", v.clone()),
        MatchOp::Regex(v) => ("regex", v.clone()),
        MatchOp::Range {
            min_cents,
            max_cents,
        } => ("range", format!("{min_cents}..{max_cents}")),
    }
}

fn op_from_dto(op: &str, value: &str) -> Result<MatchOp, CommandError> {
    match op {
        "contains" => Ok(MatchOp::Contains(value.to_string())),
        "equals" => Ok(MatchOp::Equals(value.to_string())),
        "starts_with" => Ok(MatchOp::StartsWith(value.to_string())),
        "ends_with" => Ok(MatchOp::EndsWith(value.to_string())),
        "regex" => Ok(MatchOp::Regex(value.to_string())),
        "range" => {
            let (min, max) = value.split_once("..").ok_or_else(|| CommandError {
                message: format!("bad range value '{value}'"),
            })?;
            let min_cents = min.parse::<i64>().map_err(|e| CommandError {
                message: format!("bad range min '{min}': {e}"),
            })?;
            let max_cents = max.parse::<i64>().map_err(|e| CommandError {
                message: format!("bad range max '{max}': {e}"),
            })?;
            Ok(MatchOp::Range {
                min_cents,
                max_cents,
            })
        }
        other => Err(CommandError {
            message: format!("unknown match op '{other}'"),
        }),
    }
}
