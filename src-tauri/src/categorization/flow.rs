use crate::categorization::fuzzy::{suggest_category_from_history_scored, HistoryEntry};
use crate::categorization::rules::{first_matching_rule, MatchContext, Rule};

#[derive(Debug, Clone, PartialEq)]
pub enum CategorizationOutcome {
    Rule { rule_id: i64, category_id: i64 },
    Fuzzy { category_id: i64, score: f64 },
    None,
}

/// Tries the rule engine first; falls back to fuzzy matching against counterparty history.
pub fn categorize(
    ctx: &MatchContext,
    rules: &[Rule],
    history: &[HistoryEntry],
    fuzzy_threshold: f64,
) -> CategorizationOutcome {
    if let Some(rule) = first_matching_rule(rules, ctx) {
        return CategorizationOutcome::Rule {
            rule_id: rule.id,
            category_id: rule.target_category_id,
        };
    }
    if let Some(cp) = ctx.tx.counterparty.as_deref() {
        if let Some((category_id, score)) =
            suggest_category_from_history_scored(cp, history, fuzzy_threshold)
        {
            return CategorizationOutcome::Fuzzy { category_id, score };
        }
    }
    CategorizationOutcome::None
}

#[cfg(test)]
mod tests {
    use crate::categorization::flow::{categorize, CategorizationOutcome};
    use crate::categorization::fuzzy::HistoryEntry;
    use crate::categorization::rules::{
        Combinator, MatchContext, MatchField, MatchOp, Rule, RuleCondition,
    };
    use crate::importers::RawTransaction;
    use chrono::NaiveDate;

    fn tx(counterparty: Option<&str>, amount_cents: i64) -> RawTransaction {
        RawTransaction {
            booking_date: NaiveDate::from_ymd_opt(2026, 5, 10).unwrap(),
            amount_cents,
            currency: "EUR".into(),
            counterparty: counterparty.map(str::to_string),
            purpose: None,
            raw_ref: None,
            kind: None,
            trade: None,
            counterparty_iban: None,
        }
    }

    fn rule_contains_cp(id: i64, needle: &str, target: i64, priority: i32) -> Rule {
        Rule {
            id,
            priority,
            name: format!("rule-{id}"),
            combinator: Combinator::And,
            conditions: vec![RuleCondition {
                field: MatchField::Counterparty,
                op: MatchOp::Contains(needle.into()),
            }],
            target_category_id: target,
            enabled: true,
        }
    }

    #[test]
    fn rule_match_wins_over_fuzzy() {
        let t = tx(Some("REWE Markt Berlin"), -1299);
        let rules = vec![rule_contains_cp(7, "REWE", 42, 10)];
        let history = vec![HistoryEntry {
            counterparty: "REWE Markt Berlin".into(),
            category_id: 99,
        }];
        let outcome = categorize(&MatchContext::new(&t, None), &rules, &history, 0.5);
        assert!(matches!(
            outcome,
            CategorizationOutcome::Rule {
                rule_id: 7,
                category_id: 42
            }
        ));
    }

    #[test]
    fn fuzzy_fires_when_no_rule_matches() {
        let t = tx(Some("REWE Markt Hamburg"), -500);
        let rules: Vec<Rule> = vec![];
        let history = vec![HistoryEntry {
            counterparty: "REWE Markt Berlin".into(),
            category_id: 7,
        }];
        match categorize(&MatchContext::new(&t, None), &rules, &history, 0.8) {
            CategorizationOutcome::Fuzzy { category_id, score } => {
                assert_eq!(category_id, 7);
                assert!(score >= 0.8);
            }
            other => panic!("expected Fuzzy, got {other:?}"),
        }
    }

    #[test]
    fn none_when_no_rule_and_no_fuzzy() {
        let t = tx(Some("Wholly Unknown GmbH"), -1000);
        let history = vec![HistoryEntry {
            counterparty: "REWE".into(),
            category_id: 1,
        }];
        let outcome = categorize(&MatchContext::new(&t, None), &[], &history, 0.95);
        assert!(matches!(outcome, CategorizationOutcome::None));
    }

    #[test]
    fn none_when_no_counterparty_and_no_rule() {
        let t = tx(None, -1000);
        let history = vec![HistoryEntry {
            counterparty: "REWE".into(),
            category_id: 1,
        }];
        let outcome = categorize(&MatchContext::new(&t, None), &[], &history, 0.5);
        assert!(matches!(outcome, CategorizationOutcome::None));
    }

    #[test]
    fn rule_fires_on_amount_without_counterparty() {
        let t = tx(None, -1500);
        let r = Rule {
            id: 3,
            priority: 50,
            name: "Kleinbeträge".into(),
            combinator: Combinator::And,
            conditions: vec![RuleCondition {
                field: MatchField::Amount,
                op: MatchOp::Range {
                    min_cents: -2000,
                    max_cents: -1000,
                },
            }],
            target_category_id: 11,
            enabled: true,
        };
        let outcome = categorize(
            &MatchContext::new(&t, None),
            std::slice::from_ref(&r),
            &[],
            0.5,
        );
        assert!(matches!(
            outcome,
            CategorizationOutcome::Rule {
                rule_id: 3,
                category_id: 11
            }
        ));
    }

    #[test]
    fn rule_with_account_condition_fires() {
        let t = tx(Some("REWE"), -1000);
        let r = Rule {
            id: 9,
            priority: 50,
            name: "TR-Lebensmittel".into(),
            combinator: Combinator::And,
            conditions: vec![
                RuleCondition {
                    field: MatchField::Counterparty,
                    op: MatchOp::Contains("REWE".into()),
                },
                RuleCondition {
                    field: MatchField::Account,
                    op: MatchOp::Equals("42".into()),
                },
            ],
            target_category_id: 5,
            enabled: true,
        };
        let outcome = categorize(
            &MatchContext::new(&t, Some(42)),
            std::slice::from_ref(&r),
            &[],
            0.5,
        );
        assert!(matches!(
            outcome,
            CategorizationOutcome::Rule {
                rule_id: 9,
                category_id: 5
            }
        ));

        // Different account → no match
        let outcome2 = categorize(
            &MatchContext::new(&t, Some(7)),
            std::slice::from_ref(&r),
            &[],
            0.5,
        );
        assert!(matches!(outcome2, CategorizationOutcome::None));
    }
}
