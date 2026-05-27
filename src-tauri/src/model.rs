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
    pub institution_id: Option<i64>,    // NEW
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Goal {
    pub id: i64,
    pub name: String,
    pub category_id: i64,
    pub target_cents: i64,
    pub start_date: String,
    pub target_date: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub note: Option<String>,
    pub archived: bool,
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
    /// Account-ID der zugehörigen `securities_trades.account_id`, NUR für
    /// bestandsändernde Sides (buy/sell/corp_action/fusion_in/fusion_out).
    /// `None` für Cash-only Tx und Dividenden/KESt. Wird nur vom
    /// `list_transactions`-Pfad gesetzt (LEFT JOIN); andere Queries lassen
    /// es leer via `#[sqlx(default)]`. Für UI-Filter, das die Doppelbuchung
    /// (Cash-Konto vs Depot) auflöst.
    #[sqlx(default)]
    pub holding_account_id: Option<i64>,
    /// `securities_trades.side` der zugehörigen Trade-Detail-Zeile. Wird vom
    /// UI gebraucht um Fusion-Out vs Fusion-In zu unterscheiden (beide haben
    /// `kind='corporate_action'`, nur side erkennt die Richtung). `None`
    /// für Cash-only Tx ohne Trade-Verknüpfung.
    /// Pointer auf die gepaarte Tx wenn diese als Inter-Account-Transfer
    /// erkannt wurde (oder die automatisch erzeugte Gegenbuchung selbst ist).
    /// `None` für normale Tx ohne Pairing.
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
    pub kest_cents: i64,              // NEU
    pub withholding_tax_cents: i64,   // NEU
    pub fx_rate_micro: Option<i64>,
    pub account_id: Option<i64>,    // NEU — depot-Konto wenn explizit geroutet
}
