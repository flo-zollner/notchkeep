#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub counterparty: String,
    pub category_id: i64,
}

/// Searches the history for the counterparty with the highest Jaro-Winkler
/// similarity to the given `counterparty`. Returns the associated
/// `category_id` when the score meets or exceeds the threshold.
pub fn suggest_category_from_history(
    counterparty: &str,
    history: &[HistoryEntry],
    threshold: f64,
) -> Option<i64> {
    suggest_category_from_history_scored(counterparty, history, threshold).map(|(id, _)| id)
}

/// Like [`suggest_category_from_history`], but also returns the Jaro-Winkler
/// score of the match.
pub fn suggest_category_from_history_scored(
    counterparty: &str,
    history: &[HistoryEntry],
    threshold: f64,
) -> Option<(i64, f64)> {
    history
        .iter()
        .map(|h| (h, strsim::jaro_winkler(counterparty, &h.counterparty)))
        .filter(|(_, score)| *score >= threshold)
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(h, score)| (h.category_id, score))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(cp: &str, cat: i64) -> HistoryEntry {
        HistoryEntry {
            counterparty: cp.to_string(),
            category_id: cat,
        }
    }

    #[test]
    fn suggests_category_of_most_similar_counterparty() {
        let history = vec![
            entry("REWE Markt Berlin", 1),
            entry("Edeka", 2),
            entry("DM Drogerie", 3),
        ];
        assert_eq!(
            suggest_category_from_history("REWE Markt Hamburg", &history, 0.8),
            Some(1)
        );
    }

    #[test]
    fn returns_none_when_below_threshold() {
        let history = vec![entry("Edeka", 2)];
        assert!(suggest_category_from_history("Bank of America", &history, 0.8).is_none());
    }

    #[test]
    fn returns_none_for_empty_history() {
        assert!(suggest_category_from_history("REWE", &[], 0.5).is_none());
    }
}
