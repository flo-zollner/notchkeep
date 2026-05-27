use regex::Regex;

use crate::importers::RawTransaction;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchField {
    Counterparty,
    Description,
    Amount,
    Account,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MatchOp {
    Contains(String),
    Equals(String),
    StartsWith(String),
    EndsWith(String),
    Regex(String),
    /// Amounts in cents, inclusive on both sides.
    Range { min_cents: i64, max_cents: i64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Combinator {
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuleCondition {
    pub field: MatchField,
    pub op: MatchOp,
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub id: i64,
    pub priority: i32,
    pub name: String,
    pub combinator: Combinator,
    pub conditions: Vec<RuleCondition>,
    pub target_category_id: i64,
    pub enabled: bool,
}

/// Context for rule matching. `tx.purpose` carries the human-readable description
/// for the UI; callers from the import flow set `purpose` from the CSV field,
/// bulk-reassign sets `manual_note ?? purpose` from the database.
#[derive(Debug, Clone, Copy)]
pub struct MatchContext<'a> {
    pub tx: &'a RawTransaction,
    pub account_id: Option<i64>,
}

impl<'a> MatchContext<'a> {
    pub fn new(tx: &'a RawTransaction, account_id: Option<i64>) -> Self {
        Self { tx, account_id }
    }
}

/// Finds the matching rule with the lowest `priority` from a rule set
/// (lower number wins). Disabled rules are skipped.
pub fn first_matching_rule<'a>(rules: &'a [Rule], ctx: &MatchContext) -> Option<&'a Rule> {
    rules
        .iter()
        .filter(|r| r.enabled && match_rule(r, ctx))
        .min_by_key(|r| r.priority)
}

pub fn match_rule(rule: &Rule, ctx: &MatchContext) -> bool {
    if rule.conditions.is_empty() {
        return false;
    }
    let predicate = |c: &RuleCondition| match_condition(c, ctx);
    match rule.combinator {
        Combinator::And => rule.conditions.iter().all(predicate),
        Combinator::Or => rule.conditions.iter().any(predicate),
    }
}

fn match_condition(cond: &RuleCondition, ctx: &MatchContext) -> bool {
    match cond.field {
        MatchField::Counterparty => match_string(&cond.op, ctx.tx.counterparty.as_deref().unwrap_or("")),
        MatchField::Description => match_string(&cond.op, ctx.tx.purpose.as_deref().unwrap_or("")),
        MatchField::Amount => match_amount(&cond.op, ctx.tx.amount_cents),
        MatchField::Account => match_account(&cond.op, ctx.account_id),
    }
}

fn match_string(op: &MatchOp, haystack: &str) -> bool {
    // Contains/Equals/StartsWith/EndsWith are case-insensitive — expected behaviour
    // for bank counterparties (e.g. "Spar Dankt" vs "SPAR DANKT").
    // Regex remains case-sensitive; the user can prepend `(?i)` themselves.
    match op {
        MatchOp::Contains(needle) => {
            haystack.to_lowercase().contains(&needle.to_lowercase())
        }
        MatchOp::Equals(value) => {
            haystack.to_lowercase() == value.to_lowercase()
        }
        MatchOp::StartsWith(prefix) => {
            haystack.to_lowercase().starts_with(&prefix.to_lowercase())
        }
        MatchOp::EndsWith(suffix) => {
            haystack.to_lowercase().ends_with(&suffix.to_lowercase())
        }
        MatchOp::Regex(pattern) => Regex::new(pattern)
            .map(|re| re.is_match(haystack))
            .unwrap_or(false),
        MatchOp::Range { .. } => false,
    }
}

fn match_amount(op: &MatchOp, amount_cents: i64) -> bool {
    match op {
        MatchOp::Range { min_cents, max_cents } => {
            amount_cents >= *min_cents && amount_cents <= *max_cents
        }
        MatchOp::Equals(v) => v.parse::<i64>().map(|n| n == amount_cents).unwrap_or(false),
        _ => false,
    }
}

fn match_account(op: &MatchOp, account_id: Option<i64>) -> bool {
    let Some(aid) = account_id else { return false };
    match op {
        MatchOp::Equals(v) => v.parse::<i64>().map(|n| n == aid).unwrap_or(false),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn tx_with(counterparty: Option<&str>, purpose: Option<&str>, amount_cents: i64) -> RawTransaction {
        RawTransaction {
            booking_date: NaiveDate::from_ymd_opt(2026, 5, 10).unwrap(),
            amount_cents,
            currency: "EUR".to_string(),
            counterparty: counterparty.map(str::to_string),
            purpose: purpose.map(str::to_string),
            raw_ref: None,
            kind: None,
            trade: None,
            counterparty_iban: None,
        }
    }

    fn rule_single(field: MatchField, op: MatchOp) -> Rule {
        Rule {
            id: 1,
            priority: 100,
            name: "test".to_string(),
            combinator: Combinator::And,
            conditions: vec![RuleCondition { field, op }],
            target_category_id: 1,
            enabled: true,
        }
    }

    fn ctx_with<'a>(tx: &'a RawTransaction, account_id: Option<i64>) -> MatchContext<'a> {
        MatchContext::new(tx, account_id)
    }

    #[test]
    fn contains_on_counterparty_matches_substring() {
        let rule = rule_single(MatchField::Counterparty, MatchOp::Contains("REWE".into()));
        let tx = tx_with(Some("REWE Markt Berlin"), None, -1299);
        assert!(match_rule(&rule, &ctx_with(&tx, None)));
    }

    #[test]
    fn equals_on_description_requires_exact_match() {
        let rule = rule_single(MatchField::Description, MatchOp::Equals("Gehalt Mai".into()));
        let yes = tx_with(None, Some("Gehalt Mai"), 150_000);
        let no = tx_with(None, Some("Gehalt April"), 150_000);
        assert!(match_rule(&rule, &ctx_with(&yes, None)));
        assert!(!match_rule(&rule, &ctx_with(&no, None)));
    }

    #[test]
    fn starts_and_ends_with_on_description() {
        let starts = rule_single(MatchField::Description, MatchOp::StartsWith("Gehalt".into()));
        let ends = rule_single(MatchField::Description, MatchOp::EndsWith("Mai".into()));
        let tx = tx_with(None, Some("Gehalt Mai"), 150_000);
        assert!(match_rule(&starts, &ctx_with(&tx, None)));
        assert!(match_rule(&ends, &ctx_with(&tx, None)));
    }

    #[test]
    fn regex_on_counterparty_matches_pattern() {
        let rule = rule_single(MatchField::Counterparty, MatchOp::Regex(r"(?i)^rewe\b.*".into()));
        let yes = tx_with(Some("REWE Markt"), None, -1299);
        let no = tx_with(Some("Edeka"), None, -1299);
        assert!(match_rule(&rule, &ctx_with(&yes, None)));
        assert!(!match_rule(&rule, &ctx_with(&no, None)));
    }

    #[test]
    fn range_on_amount_is_inclusive() {
        let rule = rule_single(
            MatchField::Amount,
            MatchOp::Range { min_cents: -2000, max_cents: -1000 },
        );
        for amt in [-2000, -1299, -1000] {
            assert!(match_rule(&rule, &ctx_with(&tx_with(None, None, amt), None)));
        }
        for amt in [-999, -2001] {
            assert!(!match_rule(&rule, &ctx_with(&tx_with(None, None, amt), None)));
        }
    }

    #[test]
    fn account_equals_matches_id() {
        let rule = rule_single(MatchField::Account, MatchOp::Equals("42".into()));
        let tx = tx_with(None, None, -100);
        assert!(match_rule(&rule, &ctx_with(&tx, Some(42))));
        assert!(!match_rule(&rule, &ctx_with(&tx, Some(7))));
        assert!(!match_rule(&rule, &ctx_with(&tx, None)));
    }

    #[test]
    fn and_combinator_requires_all_conditions() {
        let rule = Rule {
            id: 1,
            priority: 100,
            name: "REWE groß".into(),
            combinator: Combinator::And,
            conditions: vec![
                RuleCondition {
                    field: MatchField::Counterparty,
                    op: MatchOp::Contains("REWE".into()),
                },
                RuleCondition {
                    field: MatchField::Amount,
                    op: MatchOp::Range { min_cents: -5000, max_cents: -2000 },
                },
            ],
            target_category_id: 1,
            enabled: true,
        };
        let big = tx_with(Some("REWE City"), None, -3000);
        let small = tx_with(Some("REWE City"), None, -500);
        let other = tx_with(Some("EDEKA"), None, -3000);
        assert!(match_rule(&rule, &ctx_with(&big, None)));
        assert!(!match_rule(&rule, &ctx_with(&small, None)));
        assert!(!match_rule(&rule, &ctx_with(&other, None)));
    }

    #[test]
    fn or_combinator_matches_any_condition() {
        let rule = Rule {
            id: 1,
            priority: 100,
            name: "Supermarkt".into(),
            combinator: Combinator::Or,
            conditions: vec![
                RuleCondition {
                    field: MatchField::Counterparty,
                    op: MatchOp::Contains("REWE".into()),
                },
                RuleCondition {
                    field: MatchField::Counterparty,
                    op: MatchOp::Contains("EDEKA".into()),
                },
            ],
            target_category_id: 1,
            enabled: true,
        };
        assert!(match_rule(&rule, &ctx_with(&tx_with(Some("REWE Markt"), None, -1), None)));
        assert!(match_rule(&rule, &ctx_with(&tx_with(Some("EDEKA City"), None, -1), None)));
        assert!(!match_rule(&rule, &ctx_with(&tx_with(Some("LIDL"), None, -1), None)));
    }

    #[test]
    fn empty_conditions_never_match() {
        let rule = Rule {
            id: 1,
            priority: 100,
            name: "leer".into(),
            combinator: Combinator::And,
            conditions: vec![],
            target_category_id: 1,
            enabled: true,
        };
        let tx = tx_with(Some("any"), None, -1);
        assert!(!match_rule(&rule, &ctx_with(&tx, None)));
    }

    #[test]
    fn first_matching_rule_picks_lowest_priority() {
        let tx = tx_with(Some("REWE Markt"), None, -1299);
        let mut generic = rule_single(MatchField::Counterparty, MatchOp::Contains("REWE".into()));
        generic.priority = 100;
        generic.name = "generic".into();
        let mut specific =
            rule_single(MatchField::Counterparty, MatchOp::Contains("REWE Markt".into()));
        specific.priority = 10;
        specific.name = "specific".into();
        let rules = vec![generic, specific];
        let winner = first_matching_rule(&rules, &ctx_with(&tx, None)).expect("some rule matches");
        assert_eq!(winner.name, "specific");
    }

    #[test]
    fn first_matching_rule_skips_disabled_rules() {
        let tx = tx_with(Some("REWE Markt"), None, -1299);
        let mut disabled =
            rule_single(MatchField::Counterparty, MatchOp::Contains("REWE".into()));
        disabled.enabled = false;
        disabled.priority = 1;
        disabled.name = "disabled".into();
        let mut active =
            rule_single(MatchField::Counterparty, MatchOp::Contains("REWE".into()));
        active.priority = 100;
        active.name = "active".into();
        let rules = vec![disabled, active];
        let winner = first_matching_rule(&rules, &ctx_with(&tx, None)).unwrap();
        assert_eq!(winner.name, "active");
    }

    #[test]
    fn first_matching_rule_returns_none_if_no_match() {
        let tx = tx_with(Some("Edeka"), None, -1299);
        let rule = rule_single(MatchField::Counterparty, MatchOp::Contains("REWE".into()));
        assert!(first_matching_rule(std::slice::from_ref(&rule), &ctx_with(&tx, None)).is_none());
    }

    #[test]
    fn contains_is_case_insensitive() {
        let rule = rule_single(MatchField::Counterparty, MatchOp::Contains("Spar".into()));
        let upper = tx_with(Some("SPAR DANKT 5266"), None, -1500);
        let lower = tx_with(Some("spar markt"), None, -1500);
        let mixed = tx_with(Some("Spar Dankt 5266"), None, -1500);
        assert!(match_rule(&rule, &ctx_with(&upper, None)));
        assert!(match_rule(&rule, &ctx_with(&lower, None)));
        assert!(match_rule(&rule, &ctx_with(&mixed, None)));
    }

    #[test]
    fn equals_is_case_insensitive() {
        let rule = rule_single(MatchField::Description, MatchOp::Equals("Gehalt Mai".into()));
        let upper = tx_with(None, Some("GEHALT MAI"), 150_000);
        let lower = tx_with(None, Some("gehalt mai"), 150_000);
        assert!(match_rule(&rule, &ctx_with(&upper, None)));
        assert!(match_rule(&rule, &ctx_with(&lower, None)));
    }

    #[test]
    fn starts_with_is_case_insensitive() {
        let rule = rule_single(MatchField::Counterparty, MatchOp::StartsWith("Inter".into()));
        let upper = tx_with(Some("INTERSPAR DANKT 8440"), None, -2500);
        assert!(match_rule(&rule, &ctx_with(&upper, None)));
    }

    #[test]
    fn ends_with_is_case_insensitive() {
        let rule = rule_single(MatchField::Counterparty, MatchOp::EndsWith("Markt".into()));
        let upper = tx_with(Some("EDEKA MARKT"), None, -1299);
        assert!(match_rule(&rule, &ctx_with(&upper, None)));
    }
}
