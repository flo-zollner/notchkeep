use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use serde::Deserialize;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use super::{FxProvider, PriceProvider, PricePoint, ProviderError, ProviderResult, Quote};

pub struct YahooProvider {
    client: reqwest::Client,
}

impl YahooProvider {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Notchkeep)")
            .timeout(Duration::from_secs(10))
            .build()
            .expect("reqwest client");
        Self { client }
    }
}

impl Default for YahooProvider {
    fn default() -> Self {
        Self::new()
    }
}

// ───── JSON-Strukturen ─────

#[derive(Deserialize, Debug)]
struct ChartResponse {
    chart: ChartWrapper,
}
#[derive(Deserialize, Debug)]
struct ChartWrapper {
    result: Option<Vec<ChartResult>>,
}
#[derive(Deserialize, Debug)]
struct ChartResult {
    meta: ChartMeta,
    timestamp: Option<Vec<i64>>,
    indicators: Option<ChartIndicators>,
}
#[derive(Deserialize, Debug)]
struct ChartMeta {
    symbol: String,
    #[serde(rename = "regularMarketPrice")]
    regular_market_price: Option<f64>,
    #[serde(rename = "regularMarketTime")]
    regular_market_time: Option<i64>,
    currency: Option<String>,
}
#[derive(Deserialize, Debug)]
struct ChartIndicators {
    quote: Vec<ChartQuote>,
}
#[derive(Deserialize, Debug)]
struct ChartQuote {
    close: Vec<Option<f64>>,
}

#[derive(Deserialize, Debug)]
struct SearchResponse {
    quotes: Option<Vec<SearchQuote>>,
}
#[derive(Deserialize, Debug)]
struct SearchQuote {
    symbol: String,
}

// ───── Helpers ─────

fn to_micro(value: f64) -> i64 {
    (value * 1_000_000.0).round() as i64
}

fn from_unix(ts: i64) -> NaiveDate {
    let dt: DateTime<Utc> = Utc.timestamp_opt(ts, 0).single()
        .unwrap_or_else(|| Utc.timestamp_opt(0, 0).unwrap());
    dt.date_naive()
}

impl PriceProvider for YahooProvider {
    fn fetch_quote<'a>(&'a self, symbol: &'a str)
        -> Pin<Box<dyn Future<Output = ProviderResult<Quote>> + Send + 'a>>
    {
        Box::pin(async move {
            let url = format!(
                "https://query1.finance.yahoo.com/v8/finance/chart/{symbol}?interval=1d&range=5d"
            );
            let resp = self.client.get(&url).send().await
                .map_err(|e| ProviderError::Network(format!("quote {symbol}: {e}")))?;
            let body: ChartResponse = resp.json().await
                .map_err(|e| ProviderError::Parse(format!("quote {symbol}: {e}")))?;
            let result = body.chart.result.and_then(|r| r.into_iter().next())
                .ok_or_else(|| ProviderError::NotFound(format!("no chart for {symbol}")))?;
            let price = result.meta.regular_market_price
                .ok_or_else(|| ProviderError::NotFound(format!("no price for {symbol}")))?;
            let as_of = result.meta.regular_market_time
                .map(from_unix)
                .unwrap_or_else(|| Utc::now().date_naive());
            Ok(Quote {
                symbol: result.meta.symbol,
                price_micro: to_micro(price),
                as_of,
                currency: result.meta.currency.map(|c| c.to_uppercase()),
            })
        })
    }

    fn fetch_history<'a>(&'a self, symbol: &'a str, from: NaiveDate, to: NaiveDate)
        -> Pin<Box<dyn Future<Output = ProviderResult<Vec<PricePoint>>> + Send + 'a>>
    {
        Box::pin(async move {
            let p1 = from.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp();
            let p2 = to.and_hms_opt(23, 59, 59).unwrap().and_utc().timestamp();
            let url = format!(
                "https://query1.finance.yahoo.com/v8/finance/chart/{symbol}?\
                period1={p1}&period2={p2}&interval=1d"
            );
            let resp = self.client.get(&url).send().await
                .map_err(|e| ProviderError::Network(format!("history {symbol}: {e}")))?;
            let body: ChartResponse = resp.json().await
                .map_err(|e| ProviderError::Parse(format!("history {symbol}: {e}")))?;
            let result = body.chart.result.and_then(|r| r.into_iter().next())
                .ok_or_else(|| ProviderError::NotFound(format!("no history for {symbol}")))?;
            let timestamps = result.timestamp.unwrap_or_default();
            let closes = result.indicators
                .and_then(|i| i.quote.into_iter().next())
                .map(|q| q.close)
                .unwrap_or_default();

            let mut out = Vec::new();
            for (i, ts) in timestamps.iter().enumerate() {
                if let Some(Some(close)) = closes.get(i) {
                    out.push(PricePoint {
                        date: from_unix(*ts),
                        close_micro: to_micro(*close),
                    });
                }
            }
            Ok(out)
        })
    }

    fn resolve_symbol<'a>(&'a self, isin: &'a str)
        -> Pin<Box<dyn Future<Output = ProviderResult<Option<String>>> + Send + 'a>>
    {
        Box::pin(async move {
            let url = format!(
                "https://query1.finance.yahoo.com/v1/finance/search?q={isin}"
            );
            let resp = self.client.get(&url).send().await
                .map_err(|e| ProviderError::Network(format!("search {isin}: {e}")))?;
            let body: SearchResponse = resp.json().await
                .map_err(|e| ProviderError::Parse(format!("search {isin}: {e}")))?;
            Ok(body.quotes
                .and_then(|q| q.into_iter().next())
                .map(|q| q.symbol))
        })
    }
}

impl FxProvider for YahooProvider {
    fn fetch_eur_rate<'a>(&'a self, foreign_currency: &'a str)
        -> Pin<Box<dyn Future<Output = ProviderResult<i64>> + Send + 'a>>
    {
        Box::pin(async move {
            let foreign = foreign_currency.to_uppercase();
            if foreign == "EUR" {
                return Ok(1_000_000);
            }
            let symbol = format!("EUR{foreign}=X");
            let q = self.fetch_quote(&symbol).await?;
            if q.price_micro == 0 {
                return Err(ProviderError::Parse(format!("fx zero for {foreign}")));
            }
            // q.price_micro = X * 1e6 (foreign per 1 EUR)
            // rate_micro = 1e12 / q.price_micro = micro_EUR per 1 foreign
            Ok(1_000_000_000_000_i64 / q.price_micro)
        })
    }
}
