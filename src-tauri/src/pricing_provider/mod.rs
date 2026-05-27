use chrono::NaiveDate;

pub mod mock;
pub mod yahoo;

#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("network: {0}")]
    Network(String),
    #[error("parse: {0}")]
    Parse(String),
    #[error("not_found: {0}")]
    NotFound(String),
}

pub type ProviderResult<T> = Result<T, ProviderError>;

#[derive(Debug, Clone, PartialEq)]
pub struct Quote {
    pub symbol: String,
    pub price_micro: i64,
    pub as_of: NaiveDate,
    /// Trading currency as reported by the provider (e.g. "EUR", "USD", "HKD").
    /// `None` when the provider does not supply it.
    pub currency: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PricePoint {
    pub date: NaiveDate,
    pub close_micro: i64,
}

/// Abstracts the price source (Yahoo, Mock, ...).
pub trait PriceProvider: Send + Sync {
    fn fetch_quote<'a>(&'a self, symbol: &'a str)
        -> std::pin::Pin<Box<dyn std::future::Future<Output = ProviderResult<Quote>> + Send + 'a>>;

    fn fetch_history<'a>(
        &'a self,
        symbol: &'a str,
        from: NaiveDate,
        to: NaiveDate,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ProviderResult<Vec<PricePoint>>> + Send + 'a>>;

    fn resolve_symbol<'a>(&'a self, isin: &'a str)
        -> std::pin::Pin<Box<dyn std::future::Future<Output = ProviderResult<Option<String>>> + Send + 'a>>;
}

pub trait FxProvider: Send + Sync {
    fn fetch_eur_rate<'a>(&'a self, foreign_currency: &'a str)
        -> std::pin::Pin<Box<dyn std::future::Future<Output = ProviderResult<i64>> + Send + 'a>>;
}

/// Convenience combined trait for state storage as `Box<dyn CombinedProvider>`.
pub trait CombinedProvider: PriceProvider + FxProvider {}
impl<T: PriceProvider + FxProvider + ?Sized> CombinedProvider for T {}
