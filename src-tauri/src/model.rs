use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Account {
    pub id: i64,
    pub name: String,
    pub kind: String,
    pub currency: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub note: Option<String>,
    pub last4: Option<String>,
    pub archived: bool,
    pub parent_id: Option<i64>,
    pub iban: Option<String>,
    pub institution_id: Option<i64>, // NEW
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Category {
    pub id: i64,
    pub parent_id: Option<i64>,
    pub name: String,
    pub color: Option<String>,
    pub icon: Option<String>,
    pub rollover_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Transaction {
    pub id: i64,
    pub account_id: i64,
    pub booking_date: String,
    pub value_date: Option<String>,
    pub amount_cents: i64,
    pub currency: String,
    pub counterparty: Option<String>,
    pub purpose: Option<String>,
    pub raw_ref: Option<String>,
    pub category_id: Option<i64>,
    pub source: String,
    pub source_file_hash: Option<String>,
    pub imported_at: String,
    pub manual_note: Option<String>,
    pub bucket_id: Option<i64>,
    pub kind: String,
    pub counterparty_iban: Option<String>,
    /// Account ID of the associated `securities_trades.account_id`, ONLY for
    /// holding-changing sides (buy/sell/corp_action/fusion_in/fusion_out).
    /// `None` for cash-only Tx and dividends/withholding tax. Set only by the
    /// `list_transactions` path (LEFT JOIN); other queries leave it empty via
    /// `#[sqlx(default)]`. Used by the UI filter that resolves the double-entry
    /// (cash account vs depot).
    #[sqlx(default)]
    pub holding_account_id: Option<i64>,
    /// `securities_trades.side` of the associated trade-detail row. Needed by
    /// the UI to distinguish fusion-out from fusion-in (both have
    /// `kind='corporate_action'`; only side reveals the direction). `None`
    /// for cash-only Tx without a trade link.
    /// Pointer to the paired Tx when it has been identified as an inter-account
    /// transfer (or is itself the automatically created counter-entry).
    /// `None` for ordinary Tx without pairing.
    pub paired_tx_id: Option<i64>,
    #[sqlx(default)]
    pub trade_side: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Bucket {
    pub id: i64,
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub note: Option<String>,
    pub target_cents: Option<i64>,
    pub start_date: Option<String>,
    pub target_date: Option<String>,
    pub archived: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleConditionDto {
    pub field: String,
    pub op: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleDto {
    pub id: i64,
    pub priority: i32,
    pub name: String,
    pub combinator: String,
    pub conditions: Vec<RuleConditionDto>,
    pub target_category_id: i64,
    pub enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewRuleDto {
    pub priority: i32,
    pub name: String,
    pub combinator: String,
    pub conditions: Vec<RuleConditionDto>,
    pub target_category_id: i64,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Security {
    pub id: i64,
    pub isin: String,
    pub symbol: Option<String>,
    pub name: String,
    pub currency: String,
    pub asset_type: String,
    pub country: Option<String>,
    pub sector: Option<String>,
    pub note: Option<String>,
    pub archived: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SecurityBreakdown {
    pub security_id: i64,
    pub dimension: String,
    pub key: String,
    pub weight_bps: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Institution {
    pub id: i64,
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub bic: Option<String>,
    pub country: Option<String>,
    pub note: Option<String>,
    pub archived: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct InstitutionSummary {
    pub id: i64,
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub bic: Option<String>,
    pub country: Option<String>,
    pub note: Option<String>,
    pub archived: bool,
    pub created_at: String,
    pub account_count: i64,
    pub balance_cents: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SecurityTrade {
    pub tx_id: i64,
    pub security_id: i64,
    pub side: String,
    pub shares_micro: i64,
    pub unit_price_micro: Option<i64>,
    pub fee_cents: i64,
    pub tax_cents: i64,
    pub kest_cents: i64,            // NEW
    pub withholding_tax_cents: i64, // NEW
    pub fx_rate_micro: Option<i64>,
    pub account_id: Option<i64>, // NEW — depot account when explicitly routed
}
