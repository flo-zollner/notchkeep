use chrono::NaiveDate;
use std::collections::HashMap;
use std::pin::Pin;
use std::future::Future;

use super::{FxProvider, PriceProvider, PricePoint, ProviderError, ProviderResult, Quote};

#[derive(Default, Clone)]
pub struct MockProvider {
    pub quotes: HashMap<String, Quote>,
    pub history: HashMap<String, Vec<PricePoint>>,
    pub symbol_for_isin: HashMap<String, String>,
    pub fx_rates: HashMap<String, i64>,
}

impl MockProvider {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_quote(mut self, symbol: &str, price_micro: i64, as_of: NaiveDate) -> Self {
        self.quotes.insert(symbol.to_string(), Quote {
            symbol: symbol.to_string(),
            price_micro,
            as_of,
            currency: None,
        });
        self
    }

    pub fn with_quote_currency(
        mut self, symbol: &str, price_micro: i64, as_of: NaiveDate, currency: &str,
    ) -> Self {
        self.quotes.insert(symbol.to_string(), Quote {
            symbol: symbol.to_string(),
            price_micro,
            as_of,
            currency: Some(currency.to_string()),
        });
        self
    }

    pub fn with_symbol(mut self, isin: &str, symbol: &str) -> Self {
        self.symbol_for_isin.insert(isin.to_string(), symbol.to_string());
        self
    }

    pub fn with_history(mut self, symbol: &str, points: Vec<PricePoint>) -> Self {
        self.history.insert(symbol.to_string(), points);
        self
    }

    pub fn with_fx(mut self, foreign: &str, rate_micro: i64) -> Self {
        self.fx_rates.insert(foreign.to_uppercase(), rate_micro);
        self
    }
}

impl PriceProvider for MockProvider {
    fn fetch_quote<'a>(&'a self, symbol: &'a str)
        -> Pin<Box<dyn Future<Output = ProviderResult<Quote>> + Send + 'a>>
    {
        Box::pin(async move {
            self.quotes.get(symbol).cloned().ok_or_else(||
                ProviderError::NotFound(format!("quote for {symbol}"))
            )
        })
    }

    fn fetch_history<'a>(&'a self, symbol: &'a str, from: NaiveDate, to: NaiveDate)
        -> Pin<Box<dyn Future<Output = ProviderResult<Vec<PricePoint>>> + Send + 'a>>
    {
        Box::pin(async move {
            let all = self.history.get(symbol).cloned().unwrap_or_default();
            Ok(all.into_iter().filter(|p| p.date >= from && p.date <= to).collect())
        })
    }

    fn resolve_symbol<'a>(&'a self, isin: &'a str)
        -> Pin<Box<dyn Future<Output = ProviderResult<Option<String>>> + Send + 'a>>
    {
        Box::pin(async move {
            Ok(self.symbol_for_isin.get(isin).cloned())
        })
    }
}

impl FxProvider for MockProvider {
    fn fetch_eur_rate<'a>(&'a self, foreign_currency: &'a str)
        -> Pin<Box<dyn Future<Output = ProviderResult<i64>> + Send + 'a>>
    {
        Box::pin(async move {
            let key = foreign_currency.to_uppercase();
            self.fx_rates.get(&key).copied().ok_or_else(||
                ProviderError::NotFound(format!("fx rate for {foreign_currency}"))
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    #[tokio::test]
    async fn mock_quote_returns_set_value() {
        let m = MockProvider::new().with_quote("AAPL", 180_500_000, d("2026-05-19"));
        let q = m.fetch_quote("AAPL").await.unwrap();
        assert_eq!(q.symbol, "AAPL");
        assert_eq!(q.price_micro, 180_500_000);
    }

    #[tokio::test]
    async fn mock_quote_not_found() {
        let m = MockProvider::new();
        assert!(m.fetch_quote("UNKNOWN").await.is_err());
    }

    #[tokio::test]
    async fn mock_history_filters_range() {
        let m = MockProvider::new().with_history("AAPL", vec![
            PricePoint { date: d("2026-01-15"), close_micro: 150_000_000 },
            PricePoint { date: d("2026-02-15"), close_micro: 160_000_000 },
            PricePoint { date: d("2026-03-15"), close_micro: 170_000_000 },
        ]);
        let hist = m.fetch_history("AAPL", d("2026-02-01"), d("2026-02-28")).await.unwrap();
        assert_eq!(hist.len(), 1);
        assert_eq!(hist[0].close_micro, 160_000_000);
    }

    #[tokio::test]
    async fn mock_resolve_symbol_lookup() {
        let m = MockProvider::new().with_symbol("US0378331005", "AAPL");
        assert_eq!(m.resolve_symbol("US0378331005").await.unwrap(), Some("AAPL".to_string()));
        assert_eq!(m.resolve_symbol("XX0000000001").await.unwrap(), None);
    }

    #[tokio::test]
    async fn mock_fx_case_insensitive() {
        let m = MockProvider::new().with_fx("USD", 909_100);
        assert_eq!(m.fetch_eur_rate("usd").await.unwrap(), 909_100);
        assert_eq!(m.fetch_eur_rate("USD").await.unwrap(), 909_100);
        assert!(m.fetch_eur_rate("XYZ").await.is_err());
    }
}
