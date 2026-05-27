use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

use super::{DbError, DbResult};
use crate::model::{SecurityTrade, Transaction};

const TRADE_COLUMNS: &str = "tx_id, security_id, side, shares_micro, \
     unit_price_micro, fee_cents, tax_cents, kest_cents, withholding_tax_cents, \
     fx_rate_micro, account_id";

const TX_COLUMNS: &str = "id, account_id, booking_date, value_date, amount_cents, currency, \
     counterparty, purpose, raw_ref, category_id, source, source_file_hash, \
     imported_at, manual_note, bucket_id, kind, counterparty_iban, paired_tx_id";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewTradePayload {
    /// transactions.account_id (= Cashkonto). Wo der Cashflow gebucht wird.
    pub account_id: i64,
    pub security_id: i64,
    pub booking_date: String,
    pub side: String,
    pub shares_micro: i64,
    pub unit_price_micro: Option<i64>,
    pub fee_cents: i64,
    pub kest_cents: i64,
    pub withholding_tax_cents: i64,
    pub fx_rate_micro: Option<i64>,
    pub amount_cents: i64,
    pub currency: Option<String>,
    pub counterparty: Option<String>,
    pub manual_note: Option<String>,
    /// Optionaler Override für securities_trades.account_id (= Depot).
    /// Wenn None: Backend leitet das Depot über `resolve_trade_account` aus
    /// `account_id` ab (Auto-Routing innerhalb desselben Instituts).
    #[serde(default)]
    pub holding_account_id: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTradePayload {
    pub shares_micro: Option<i64>,
    pub unit_price_micro: Option<Option<i64>>,
    pub fee_cents: Option<i64>,
    pub tax_cents: Option<i64>,
    pub fx_rate_micro: Option<Option<i64>>,
    // NEU für Depot-Dialog — diese Felder werden vom Multi-Table-Update in
    // einem Folge-Commit verarbeitet (Task 2 des Plans). Bis dahin schreibt
    // `update_trade` sie noch nicht in die DB.
    pub kest_cents: Option<i64>,
    pub withholding_tax_cents: Option<i64>,
    /// `securities_trades.account_id` (= Depot, wenn explizit gesetzt).
    pub account_id: Option<i64>,
    pub security_id: Option<i64>,
    pub amount_cents: Option<i64>,
    /// `transactions.account_id` (= Cashkonto). Disambiguiert vom obigen
    /// `account_id`, das auf die Trade-Detail-Zeile zielt.
    pub tx_account_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeWithTx {
    pub trade: SecurityTrade,
    pub tx: Transaction,
}

const ALLOWED_SIDES: &[&str] = &[
    "buy", "sell", "dividend", "corporate_action", "fusion_out", "fusion_in", "tax",
];

pub async fn create_trade(
    pool: &SqlitePool,
    p: NewTradePayload,
) -> DbResult<TradeWithTx> {
    if !ALLOWED_SIDES.contains(&p.side.as_str()) {
        return Err(DbError::Decode(format!(
            "side must be one of {ALLOWED_SIDES:?}, got {:?}", p.side,
        )));
    }
    if p.fee_cents < 0 {
        return Err(DbError::Decode("fee_cents must be >= 0".into()));
    }
    if p.kest_cents < 0 || p.withholding_tax_cents < 0 {
        return Err(DbError::Decode("tax fields must be >= 0".into()));
    }
    // side='tax' = WP-Steuer-Belastung (z.B. Thesaurierungs-KESt). Hat keine
    // Stück- oder Preis-Komponente und mappt 1:1 auf parent-Tx.kind='tax'.
    if p.side == "tax" {
        if p.shares_micro != 0 {
            return Err(DbError::Decode(
                "side='tax' requires shares_micro=0".into(),
            ));
        }
        if p.unit_price_micro.is_some() {
            return Err(DbError::Decode(
                "side='tax' must not set unit_price_micro".into(),
            ));
        }
    }
    let tax_cents_sum = p.kest_cents + p.withholding_tax_cents;
    // Wenn der Caller das Depot explizit setzt (= aus dem UI-Picker), nimm das.
    // Sonst fällt's auf Auto-Routing via resolve_trade_account zurück.
    let target_account_id = match p.holding_account_id {
        Some(id) => Some(id),
        None => crate::db::accounts::resolve_trade_account(pool, p.account_id).await?,
    };
    let currency = p.currency
        .as_deref()
        .map(|c| c.trim().to_uppercase())
        .filter(|c| !c.is_empty())
        .unwrap_or_else(|| "EUR".to_string());

    let mut sqltx = pool.begin().await?;

    let tx_sql = format!(
        "INSERT INTO transactions
            (account_id, booking_date, amount_cents, currency, counterparty,
             manual_note, source, kind)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'manual', ?7)
         RETURNING {TX_COLUMNS}"
    );
    let tx: Transaction = sqlx::query_as::<_, Transaction>(&tx_sql)
        .bind(p.account_id)
        .bind(&p.booking_date)
        .bind(p.amount_cents)
        .bind(currency)
        .bind(p.counterparty.as_deref())
        .bind(p.manual_note.as_deref())
        .bind(&p.side)
        .fetch_one(&mut *sqltx)
        .await?;

    let trade_sql = format!(
        "INSERT INTO securities_trades
            (tx_id, security_id, side, shares_micro, unit_price_micro,
             fee_cents, tax_cents, kest_cents, withholding_tax_cents, fx_rate_micro, account_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
         RETURNING {TRADE_COLUMNS}"
    );
    let trade: SecurityTrade = sqlx::query_as::<_, SecurityTrade>(&trade_sql)
        .bind(tx.id)
        .bind(p.security_id)
        .bind(&p.side)
        .bind(p.shares_micro)
        .bind(p.unit_price_micro)
        .bind(p.fee_cents)
        .bind(tax_cents_sum)
        .bind(p.kest_cents)
        .bind(p.withholding_tax_cents)
        .bind(p.fx_rate_micro)
        .bind(target_account_id)
        .fetch_one(&mut *sqltx)
        .await?;

    sqltx.commit().await?;
    Ok(TradeWithTx { trade, tx })
}

pub async fn get_trade(pool: &SqlitePool, tx_id: i64) -> DbResult<TradeWithTx> {
    let trade_sql = format!("SELECT {TRADE_COLUMNS} FROM securities_trades WHERE tx_id = ?1");
    let trade = sqlx::query_as::<_, SecurityTrade>(&trade_sql)
        .bind(tx_id).fetch_one(pool).await?;
    let tx_sql = format!("SELECT {TX_COLUMNS} FROM transactions WHERE id = ?1");
    let tx = sqlx::query_as::<_, Transaction>(&tx_sql)
        .bind(tx_id).fetch_one(pool).await?;
    Ok(TradeWithTx { trade, tx })
}

pub async fn list_trades(
    pool: &SqlitePool,
    security_id: Option<i64>,
) -> DbResult<Vec<TradeWithTx>> {
    let sql = "SELECT t.tx_id, t.security_id, t.side, t.shares_micro, t.unit_price_micro,
                      t.fee_cents, t.tax_cents, t.kest_cents, t.withholding_tax_cents,
                      t.fx_rate_micro, t.account_id AS t_account_id,
                      x.id AS x_id, x.account_id, x.booking_date, x.value_date,
                      x.amount_cents, x.currency,
                      x.counterparty, x.purpose, x.raw_ref, x.category_id,
                      x.source, x.source_file_hash, x.imported_at,
                      x.manual_note, x.bucket_id, x.kind, x.counterparty_iban,
                      x.paired_tx_id
                 FROM securities_trades t
                 JOIN transactions x ON x.id = t.tx_id
                WHERE (?1 IS NULL OR t.security_id = ?1)
                ORDER BY x.booking_date DESC, x.id DESC";
    let rows = sqlx::query(sql).bind(security_id).fetch_all(pool).await?;
    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        let trade = SecurityTrade {
            tx_id: r.get("tx_id"),
            security_id: r.get("security_id"),
            side: r.get("side"),
            shares_micro: r.get("shares_micro"),
            unit_price_micro: r.get("unit_price_micro"),
            fee_cents: r.get("fee_cents"),
            tax_cents: r.get("tax_cents"),
            kest_cents: r.get("kest_cents"),
            withholding_tax_cents: r.get("withholding_tax_cents"),
            fx_rate_micro: r.get("fx_rate_micro"),
            account_id: r.get("t_account_id"),
        };
        let tx = Transaction {
            id: r.get("x_id"),
            account_id: r.get("account_id"),
            booking_date: r.get("booking_date"),
            value_date: r.get("value_date"),
            amount_cents: r.get("amount_cents"),
            currency: r.get("currency"),
            counterparty: r.get("counterparty"),
            purpose: r.get("purpose"),
            raw_ref: r.get("raw_ref"),
            category_id: r.get("category_id"),
            source: r.get("source"),
            source_file_hash: r.get("source_file_hash"),
            imported_at: r.get("imported_at"),
            manual_note: r.get("manual_note"),
            bucket_id: r.get("bucket_id"),
            kind: r.get("kind"),
            counterparty_iban: r.get("counterparty_iban"),
            holding_account_id: None,  // nicht relevant in der TradeWithTx-Liste
            paired_tx_id: r.get("paired_tx_id"),
            trade_side: None,
        };
        out.push(TradeWithTx { trade, tx });
    }
    Ok(out)
}

pub async fn delete_trade(pool: &SqlitePool, tx_id: i64) -> DbResult<bool> {
    let res = sqlx::query("DELETE FROM transactions WHERE id = ?1")
        .bind(tx_id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}

pub async fn update_trade(
    pool: &SqlitePool,
    tx_id: i64,
    p: UpdateTradePayload,
) -> DbResult<SecurityTrade> {
    if let Some(f) = p.fee_cents {
        if f < 0 { return Err(DbError::Decode("fee_cents must be >= 0".into())); }
    }
    if let Some(t) = p.tax_cents {
        if t < 0 { return Err(DbError::Decode("tax_cents must be >= 0".into())); }
    }
    if let Some(k) = p.kest_cents {
        if k < 0 { return Err(DbError::Decode("kest_cents must be >= 0".into())); }
    }
    if let Some(w) = p.withholding_tax_cents {
        if w < 0 { return Err(DbError::Decode("withholding_tax_cents must be >= 0".into())); }
    }

    let current_sql = format!("SELECT {TRADE_COLUMNS} FROM securities_trades WHERE tx_id = ?1");
    let current = sqlx::query_as::<_, SecurityTrade>(&current_sql)
        .bind(tx_id).fetch_one(pool).await?;

    let new_kest = p.kest_cents.unwrap_or(current.kest_cents);
    let new_wht  = p.withholding_tax_cents.unwrap_or(current.withholding_tax_cents);
    let new_tax  = p.tax_cents.unwrap_or(new_kest + new_wht);

    let mut tx = pool.begin().await?;

    let st_sql = format!(
        "UPDATE securities_trades SET
            shares_micro          = ?1,
            unit_price_micro      = ?2,
            fee_cents             = ?3,
            tax_cents             = ?4,
            kest_cents            = ?5,
            withholding_tax_cents = ?6,
            fx_rate_micro         = ?7,
            account_id            = ?8,
            security_id           = ?9
         WHERE tx_id = ?10
         RETURNING {TRADE_COLUMNS}"
    );
    let updated_trade: SecurityTrade = sqlx::query_as::<_, SecurityTrade>(&st_sql)
        .bind(p.shares_micro.unwrap_or(current.shares_micro))
        .bind(match p.unit_price_micro {
            Some(v) => v,
            None => current.unit_price_micro,
        })
        .bind(p.fee_cents.unwrap_or(current.fee_cents))
        .bind(new_tax)
        .bind(new_kest)
        .bind(new_wht)
        .bind(match p.fx_rate_micro {
            Some(v) => v,
            None => current.fx_rate_micro,
        })
        .bind(p.account_id.or(current.account_id))
        .bind(p.security_id.unwrap_or(current.security_id))
        .bind(tx_id)
        .fetch_one(&mut *tx)
        .await?;

    if p.amount_cents.is_some() || p.tx_account_id.is_some() {
        let (curr_amount, curr_acc): (i64, i64) = sqlx::query_as(
            "SELECT amount_cents, account_id FROM transactions WHERE id = ?1"
        ).bind(tx_id).fetch_one(&mut *tx).await?;
        sqlx::query("UPDATE transactions SET amount_cents = ?1, account_id = ?2 WHERE id = ?3")
            .bind(p.amount_cents.unwrap_or(curr_amount))
            .bind(p.tx_account_id.unwrap_or(curr_acc))
            .bind(tx_id)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;
    Ok(updated_trade)
}

/// Hängt eine Trade-Zeile an einen bereits existierenden Tx (für CSV-Import-Pfad).
/// Anders als `create_trade`, das den Tx selbst erstellt und atomic ist.
/// `target_account_id`: explizites Depot-Konto; None = Fallback auf tx.account_id (T36 setzt den echten Wert).
#[allow(clippy::too_many_arguments)]
pub async fn insert_trade_row(
    pool: &SqlitePool,
    tx_id: i64,
    security_id: i64,
    side: &str,
    shares_micro: i64,
    unit_price_micro: Option<i64>,
    fee_cents: i64,
    kest_cents: i64,
    withholding_tax_cents: i64,
    fx_rate_micro: Option<i64>,
    target_account_id: Option<i64>,   // None = Fallback auf tx.account_id
    fusion_group: Option<&str>,       // Only set for side='fusion_out'|'fusion_in'
) -> DbResult<()> {
    if !ALLOWED_SIDES.contains(&side) {
        return Err(DbError::Decode(format!(
            "side must be one of {ALLOWED_SIDES:?}, got {side:?}"
        )));
    }
    if fee_cents < 0 {
        return Err(DbError::Decode("fee_cents must be >= 0".into()));
    }
    if kest_cents < 0 || withholding_tax_cents < 0 {
        return Err(DbError::Decode("tax fields must be >= 0".into()));
    }
    let tax_cents_sum = kest_cents + withholding_tax_cents;
    sqlx::query(
        "INSERT INTO securities_trades
            (tx_id, security_id, side, shares_micro, unit_price_micro, fee_cents,
             tax_cents, kest_cents, withholding_tax_cents, fx_rate_micro,
             account_id, fusion_group)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)"
    )
    .bind(tx_id)
    .bind(security_id)
    .bind(side)
    .bind(shares_micro)
    .bind(unit_price_micro)
    .bind(fee_cents)
    .bind(tax_cents_sum)
    .bind(kest_cents)
    .bind(withholding_tax_cents)
    .bind(fx_rate_micro)
    .bind(target_account_id)
    .bind(fusion_group)
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_memory;
    use crate::db::securities::{create_security, NewSecurityPayload};
    use crate::db::accounts::create_account;

    async fn setup_broker_and_security(pool: &sqlx::SqlitePool) -> (i64, i64) {
        let acc = create_account(pool, "Broker", "broker", "EUR", None, None, None).await.unwrap();
        let sec = create_security(pool, NewSecurityPayload {
            isin: "IE00BK5BQT80".into(),
            symbol: Some("VWCE.DE".into()),
            name: "Vanguard FTSE All-World".into(),
            currency: None,
            asset_type: "etf_equity".into(),
            country: None, sector: None, note: None,
        }).await.unwrap();
        (acc.id, sec.id)
    }

    #[tokio::test]
    async fn create_trade_buy_atomic_inserts_tx_and_trade() {
        let pool = connect_memory().await.unwrap();
        let (acc, sec) = setup_broker_and_security(&pool).await;

        let result = create_trade(&pool, NewTradePayload {
            account_id: acc,
            security_id: sec,
            booking_date: "2026-05-18".into(),
            side: "buy".into(),
            shares_micro: 5_000_000,
            unit_price_micro: Some(100_500_000),
            fee_cents: 100,
            kest_cents: 0,
            withholding_tax_cents: 0,
            fx_rate_micro: None,
            amount_cents: -50_350,
            currency: None,
            counterparty: Some("Trade Republic".into()),
            manual_note: None,
            holding_account_id: None,
        }).await.unwrap();

        assert_eq!(result.tx.kind, "buy");
        assert_eq!(result.tx.amount_cents, -50_350);
        assert_eq!(result.tx.account_id, acc);
        assert_eq!(result.trade.security_id, sec);
        assert_eq!(result.trade.side, "buy");
        assert_eq!(result.trade.shares_micro, 5_000_000);
        assert_eq!(result.trade.unit_price_micro, Some(100_500_000));
        assert_eq!(result.trade.fee_cents, 100);
        assert_eq!(result.trade.tx_id, result.tx.id);
    }

    #[tokio::test]
    async fn create_trade_sell_inserts_correctly() {
        let pool = connect_memory().await.unwrap();
        let (acc, sec) = setup_broker_and_security(&pool).await;
        let result = create_trade(&pool, NewTradePayload {
            account_id: acc, security_id: sec,
            booking_date: "2026-06-01".into(),
            side: "sell".into(),
            shares_micro: -3_000_000,
            unit_price_micro: Some(110_000_000),
            fee_cents: 50, kest_cents: 200, withholding_tax_cents: 0,
            fx_rate_micro: None,
            amount_cents: 32_750,
            currency: None, counterparty: None, manual_note: None,
            holding_account_id: None,
        }).await.unwrap();
        assert_eq!(result.trade.side, "sell");
        assert_eq!(result.tx.kind, "sell");
        assert_eq!(result.trade.shares_micro, -3_000_000);
    }

    #[tokio::test]
    async fn create_trade_dividend_with_zero_shares() {
        let pool = connect_memory().await.unwrap();
        let (acc, sec) = setup_broker_and_security(&pool).await;
        let result = create_trade(&pool, NewTradePayload {
            account_id: acc, security_id: sec,
            booking_date: "2026-07-01".into(),
            side: "dividend".into(),
            shares_micro: 0,
            unit_price_micro: None,
            fee_cents: 0, kest_cents: 100, withholding_tax_cents: 0,
            fx_rate_micro: None,
            amount_cents: 1_500,
            currency: None, counterparty: None, manual_note: None,
            holding_account_id: None,
        }).await.unwrap();
        assert_eq!(result.trade.side, "dividend");
        assert_eq!(result.trade.unit_price_micro, None);
        assert_eq!(result.tx.amount_cents, 1_500);
    }

    #[tokio::test]
    async fn create_trade_side_tax_sets_tx_kind_tax() {
        let pool = connect_memory().await.unwrap();
        let (acc, sec) = setup_broker_and_security(&pool).await;
        let result = create_trade(&pool, NewTradePayload {
            account_id: acc, security_id: sec,
            booking_date: "2026-08-01".into(),
            side: "tax".into(),
            shares_micro: 0,
            unit_price_micro: None,
            fee_cents: 0, kest_cents: 250, withholding_tax_cents: 0,
            fx_rate_micro: None,
            amount_cents: -250,
            currency: None, counterparty: None, manual_note: None,
            holding_account_id: None,
        }).await.unwrap();
        assert_eq!(result.tx.kind, "tax");
        assert_eq!(result.trade.side, "tax");
        assert_eq!(result.trade.kest_cents, 250);
    }

    #[tokio::test]
    async fn create_trade_side_tax_rejects_nonzero_shares() {
        let pool = connect_memory().await.unwrap();
        let (acc, sec) = setup_broker_and_security(&pool).await;
        let p = NewTradePayload {
            account_id: acc, security_id: sec,
            booking_date: "2026-08-01".into(),
            side: "tax".into(),
            shares_micro: 1_000_000, unit_price_micro: None,
            fee_cents: 0, kest_cents: 0, withholding_tax_cents: 0, fx_rate_micro: None,
            amount_cents: -250, currency: None, counterparty: None, manual_note: None,
            holding_account_id: None,
        };
        assert!(create_trade(&pool, p).await.is_err());
    }

    #[tokio::test]
    async fn create_trade_rejects_invalid_side() {
        let pool = connect_memory().await.unwrap();
        let (acc, sec) = setup_broker_and_security(&pool).await;
        let p = NewTradePayload {
            account_id: acc, security_id: sec,
            booking_date: "2026-05-18".into(),
            side: "transfer".into(),
            shares_micro: 1_000_000, unit_price_micro: None,
            fee_cents: 0, kest_cents: 0, withholding_tax_cents: 0, fx_rate_micro: None,
            amount_cents: 0, currency: None, counterparty: None, manual_note: None,
            holding_account_id: None,
        };
        assert!(create_trade(&pool, p).await.is_err());
    }

    #[tokio::test]
    async fn create_trade_rejects_negative_fee_or_tax() {
        let pool = connect_memory().await.unwrap();
        let (acc, sec) = setup_broker_and_security(&pool).await;
        let bad = NewTradePayload {
            account_id: acc, security_id: sec,
            booking_date: "2026-05-18".into(),
            side: "buy".into(),
            shares_micro: 1_000_000, unit_price_micro: Some(100_000_000),
            fee_cents: -1, kest_cents: 0, withholding_tax_cents: 0, fx_rate_micro: None,
            amount_cents: -10_000, currency: None, counterparty: None, manual_note: None,
            holding_account_id: None,
        };
        assert!(create_trade(&pool, bad).await.is_err());
    }

    #[tokio::test]
    async fn create_trade_rolls_back_tx_on_trade_insert_failure() {
        let pool = connect_memory().await.unwrap();
        let (acc, _real_sec) = setup_broker_and_security(&pool).await;
        let bad = NewTradePayload {
            account_id: acc, security_id: 99_999,
            booking_date: "2026-05-18".into(),
            side: "buy".into(),
            shares_micro: 1_000_000, unit_price_micro: Some(100_000_000),
            fee_cents: 0, kest_cents: 0, withholding_tax_cents: 0, fx_rate_micro: None,
            amount_cents: -10_000, currency: None, counterparty: None, manual_note: None,
            holding_account_id: None,
        };
        assert!(create_trade(&pool, bad).await.is_err());

        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM transactions")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(count, 0, "Tx must be rolled back");
    }

    #[tokio::test]
    async fn get_trade_returns_trade_with_tx() {
        let pool = connect_memory().await.unwrap();
        let (acc, sec) = setup_broker_and_security(&pool).await;
        let created = create_trade(&pool, NewTradePayload {
            account_id: acc, security_id: sec,
            booking_date: "2026-05-18".into(),
            side: "buy".into(),
            shares_micro: 1_000_000, unit_price_micro: Some(100_000_000),
            fee_cents: 0, kest_cents: 0, withholding_tax_cents: 0, fx_rate_micro: None,
            amount_cents: -10_000, currency: None, counterparty: None, manual_note: None,
            holding_account_id: None,
        }).await.unwrap();

        let fetched = get_trade(&pool, created.tx.id).await.unwrap();
        assert_eq!(fetched.tx.id, created.tx.id);
        assert_eq!(fetched.trade.side, "buy");
    }

    #[tokio::test]
    async fn list_trades_filters_by_security_and_orders_by_date_desc() {
        let pool = connect_memory().await.unwrap();
        let (acc, sec) = setup_broker_and_security(&pool).await;
        let _t1 = create_trade(&pool, NewTradePayload {
            account_id: acc, security_id: sec,
            booking_date: "2026-01-10".into(), side: "buy".into(),
            shares_micro: 1_000_000, unit_price_micro: Some(100_000_000),
            fee_cents: 0, kest_cents: 0, withholding_tax_cents: 0, fx_rate_micro: None,
            amount_cents: -10_000, currency: None, counterparty: None, manual_note: None,
            holding_account_id: None,
        }).await.unwrap();
        let _t2 = create_trade(&pool, NewTradePayload {
            account_id: acc, security_id: sec,
            booking_date: "2026-03-15".into(), side: "buy".into(),
            shares_micro: 1_000_000, unit_price_micro: Some(110_000_000),
            fee_cents: 0, kest_cents: 0, withholding_tax_cents: 0, fx_rate_micro: None,
            amount_cents: -11_000, currency: None, counterparty: None, manual_note: None,
            holding_account_id: None,
        }).await.unwrap();

        let all = list_trades(&pool, None).await.unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].tx.booking_date, "2026-03-15"); // most recent first

        let filtered = list_trades(&pool, Some(sec)).await.unwrap();
        assert_eq!(filtered.len(), 2);
        let filtered_none = list_trades(&pool, Some(99_999)).await.unwrap();
        assert!(filtered_none.is_empty());
    }

    #[tokio::test]
    async fn delete_trade_removes_tx_and_trade() {
        let pool = connect_memory().await.unwrap();
        let (acc, sec) = setup_broker_and_security(&pool).await;
        let created = create_trade(&pool, NewTradePayload {
            account_id: acc, security_id: sec,
            booking_date: "2026-05-18".into(), side: "buy".into(),
            shares_micro: 1_000_000, unit_price_micro: Some(100_000_000),
            fee_cents: 0, kest_cents: 0, withholding_tax_cents: 0, fx_rate_micro: None,
            amount_cents: -10_000, currency: None, counterparty: None, manual_note: None,
            holding_account_id: None,
        }).await.unwrap();
        assert!(delete_trade(&pool, created.tx.id).await.unwrap());

        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM securities_trades")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(count, 0);
        let (tx_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM transactions")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(tx_count, 0);

        assert!(!delete_trade(&pool, 99_999).await.unwrap());
    }

    #[tokio::test]
    async fn update_trade_coalesces_partial() {
        let pool = connect_memory().await.unwrap();
        let (acc, sec) = setup_broker_and_security(&pool).await;
        let created = create_trade(&pool, NewTradePayload {
            account_id: acc, security_id: sec,
            booking_date: "2026-05-18".into(), side: "buy".into(),
            shares_micro: 1_000_000, unit_price_micro: Some(100_000_000),
            fee_cents: 100, kest_cents: 0, withholding_tax_cents: 0, fx_rate_micro: None,
            amount_cents: -10_010, currency: None, counterparty: None, manual_note: None,
            holding_account_id: None,
        }).await.unwrap();

        let updated = update_trade(&pool, created.tx.id, UpdateTradePayload {
            shares_micro: None,
            unit_price_micro: None,
            fee_cents: Some(200),
            tax_cents: None,
            fx_rate_micro: None,
            kest_cents: None,
            withholding_tax_cents: None,
            account_id: None,
            security_id: None,
            amount_cents: None,
            tx_account_id: None,
        }).await.unwrap();
        assert_eq!(updated.fee_cents, 200);
        assert_eq!(updated.shares_micro, 1_000_000);
        assert_eq!(updated.unit_price_micro, Some(100_000_000));
    }

    #[tokio::test]
    async fn insert_trade_row_writes_to_securities_trades() {
        let pool = connect_memory().await.unwrap();
        let (acc_id,): (i64,) = sqlx::query_as(
            "INSERT INTO accounts (name, kind, currency) VALUES ('Broker','broker','EUR') RETURNING id"
        ).fetch_one(&pool).await.unwrap();
        let (sec_id,): (i64,) = sqlx::query_as(
            "INSERT INTO securities (isin, name, currency, asset_type)
             VALUES ('LU0290358497', 'Xtrackers', 'EUR', 'etf_equity') RETURNING id"
        ).fetch_one(&pool).await.unwrap();
        let (tx_id,): (i64,) = sqlx::query_as(
            "INSERT INTO transactions
                (account_id, booking_date, amount_cents, currency, source, kind, imported_at)
             VALUES (?1, '2026-05-13', -1200000, 'EUR', 'tr_csv', 'buy', '2026-05-19T00:00:00Z')
             RETURNING id"
        ).bind(acc_id).fetch_one(&pool).await.unwrap();

        insert_trade_row(
            &pool, tx_id, sec_id, "buy",
            80_473_721, Some(149_117_000), 100, 0, 0, None, None, None,
        ).await.unwrap();

        let (count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM securities_trades WHERE tx_id = ?"
        ).bind(tx_id).fetch_one(&pool).await.unwrap();
        assert_eq!(count, 1);

        let (shares, side): (i64, String) = sqlx::query_as(
            "SELECT shares_micro, side FROM securities_trades WHERE tx_id = ?"
        ).bind(tx_id).fetch_one(&pool).await.unwrap();
        assert_eq!(shares, 80_473_721);
        assert_eq!(side, "buy");
    }

    #[tokio::test]
    async fn insert_trade_row_rejects_invalid_side() {
        let pool = connect_memory().await.unwrap();
        let err = insert_trade_row(&pool, 999, 999, "weirdside", 0, None, 0, 0, 0, None, None, None).await;
        assert!(err.is_err());
        let msg = format!("{:?}", err.err().unwrap());
        assert!(msg.contains("side must be one of"), "got: {msg}");
    }

    #[tokio::test]
    async fn create_trade_routes_to_depot_when_account_at_verrechnung() {
        // Variante B: User legt manuell einen Buy am Verrechnungskonto an.
        // securities_trades.account_id sollte automatisch aufs Depot des Instituts zeigen.
        let pool = connect_memory().await.unwrap();
        sqlx::query("INSERT INTO institutions (name) VALUES ('TR')")
            .execute(&pool).await.unwrap();
        let (inst_id,): (i64,) = sqlx::query_as("SELECT id FROM institutions WHERE name='TR'")
            .fetch_one(&pool).await.unwrap();
        let verrechnung = create_account(&pool, "V", "bank", "EUR", None, None, Some(inst_id)).await.unwrap();
        let depot = create_account(&pool, "D", "broker", "EUR", None, None, Some(inst_id)).await.unwrap();
        let sec = crate::db::securities::create_security(&pool, crate::db::securities::NewSecurityPayload {
            isin: "IE00BK5BQT82".into(), symbol: None, name: "X".into(),
            currency: None, asset_type: "etf_equity".into(),
            country: None, sector: None, note: None,
        }).await.unwrap();

        let result = create_trade(&pool, NewTradePayload {
            account_id: verrechnung.id,
            security_id: sec.id,
            booking_date: "2026-05-18".into(),
            side: "buy".into(),
            shares_micro: 5_000_000,
            unit_price_micro: Some(10_000_000),
            fee_cents: 0,
            kest_cents: 0,
            withholding_tax_cents: 0,
            fx_rate_micro: None,
            amount_cents: -50_000,
            currency: None,
            counterparty: None,
            manual_note: None,
            holding_account_id: None,
        }).await.unwrap();

        // Tx hängt am Verrechnung
        assert_eq!(result.tx.account_id, verrechnung.id);
        // securities_trades.account_id zeigt aufs Depot
        assert_eq!(result.trade.account_id, Some(depot.id));
    }

    #[tokio::test]
    async fn create_trade_leaves_account_null_when_directly_at_broker() {
        // User legt direkt am Depot an → kein Routing nötig, account_id NULL (Fallback).
        let pool = connect_memory().await.unwrap();
        let (acc, sec) = setup_broker_and_security(&pool).await;
        let result = create_trade(&pool, NewTradePayload {
            account_id: acc,
            security_id: sec,
            booking_date: "2026-05-18".into(),
            side: "buy".into(),
            shares_micro: 5_000_000,
            unit_price_micro: Some(10_000_000),
            fee_cents: 0, kest_cents: 0, withholding_tax_cents: 0, fx_rate_micro: None,
            amount_cents: -50_000,
            currency: None, counterparty: None, manual_note: None,
            holding_account_id: None,
        }).await.unwrap();
        assert_eq!(result.trade.account_id, None);
    }

    #[tokio::test]
    async fn create_trade_leaves_account_null_when_no_broker_in_institution() {
        // Bank-Konto im Institut ohne Broker → kein Routing möglich.
        let pool = connect_memory().await.unwrap();
        sqlx::query("INSERT INTO institutions (name) VALUES ('BankOnly')")
            .execute(&pool).await.unwrap();
        let (inst_id,): (i64,) = sqlx::query_as("SELECT id FROM institutions WHERE name='BankOnly'")
            .fetch_one(&pool).await.unwrap();
        let bank = create_account(&pool, "B", "bank", "EUR", None, None, Some(inst_id)).await.unwrap();
        let sec = crate::db::securities::create_security(&pool, crate::db::securities::NewSecurityPayload {
            isin: "IE00BK5BQT83".into(), symbol: None, name: "X".into(),
            currency: None, asset_type: "etf_equity".into(),
            country: None, sector: None, note: None,
        }).await.unwrap();
        let result = create_trade(&pool, NewTradePayload {
            account_id: bank.id, security_id: sec.id,
            booking_date: "2026-05-18".into(),
            side: "buy".into(),
            shares_micro: 5_000_000,
            unit_price_micro: Some(10_000_000),
            fee_cents: 0, kest_cents: 0, withholding_tax_cents: 0, fx_rate_micro: None,
            amount_cents: -50_000,
            currency: None, counterparty: None, manual_note: None,
            holding_account_id: None,
        }).await.unwrap();
        assert_eq!(result.trade.account_id, None);
    }

    #[tokio::test]
    async fn update_trade_handles_kest_and_wht_separately() {
        let pool = connect_memory().await.unwrap();
        let (acc, sec) = setup_broker_and_security(&pool).await;
        sqlx::query(
            "INSERT INTO transactions
                (account_id, booking_date, amount_cents, currency, source, kind, imported_at)
             VALUES (?1, '2026-05-15', -10000, 'EUR', 'manual', 'buy', '2026-05-15T00:00:00Z')"
        ).bind(acc).execute(&pool).await.unwrap();
        let (tx_id,): (i64,) = sqlx::query_as("SELECT MAX(id) FROM transactions")
            .fetch_one(&pool).await.unwrap();
        insert_trade_row(&pool, tx_id, sec, "buy", 1_000_000, Some(10_000_000), 100, 50, 30, None, None, None)
            .await.unwrap();

        let payload = UpdateTradePayload {
            shares_micro: None, unit_price_micro: None, fee_cents: None,
            tax_cents: None, fx_rate_micro: None,
            kest_cents: Some(80), withholding_tax_cents: Some(40),
            account_id: None, security_id: None, amount_cents: None,
            tx_account_id: None,
        };
        let updated = update_trade(&pool, tx_id, payload).await.unwrap();
        assert_eq!(updated.kest_cents, 80);
        assert_eq!(updated.withholding_tax_cents, 40);
        assert_eq!(updated.tax_cents, 120, "tax_cents = kest + wht (legacy-Summe)");
    }

    #[tokio::test]
    async fn update_trade_updates_account_id_and_amount_cents() {
        let pool = connect_memory().await.unwrap();
        let cash = create_account(&pool, "Cash", "savings", "EUR", None, None, None).await.unwrap();
        let depot = create_account(&pool, "Depot", "broker", "EUR", None, None, None).await.unwrap();
        let depot2 = create_account(&pool, "Depot2", "broker", "EUR", None, None, None).await.unwrap();
        let sec = create_security(&pool, NewSecurityPayload {
            isin: "LU0290358497".into(), symbol: None, name: "X".into(),
            currency: None, asset_type: "etf_equity".into(),
            country: None, sector: None, note: None,
        }).await.unwrap();
        sqlx::query(
            "INSERT INTO transactions
                (account_id, booking_date, amount_cents, currency, source, kind, imported_at)
             VALUES (?1, '2026-05-15', -10000, 'EUR', 'manual', 'buy', '2026-05-15T00:00:00Z')"
        ).bind(cash.id).execute(&pool).await.unwrap();
        let (tx_id,): (i64,) = sqlx::query_as("SELECT MAX(id) FROM transactions")
            .fetch_one(&pool).await.unwrap();
        insert_trade_row(&pool, tx_id, sec.id, "buy", 1_000_000, Some(10_000_000), 0, 0, 0, None, Some(depot.id), None)
            .await.unwrap();

        let payload = UpdateTradePayload {
            shares_micro: None, unit_price_micro: None, fee_cents: None,
            tax_cents: None, fx_rate_micro: None, kest_cents: None,
            withholding_tax_cents: None, security_id: None,
            account_id: Some(depot2.id),
            amount_cents: Some(-12000),
            tx_account_id: None,
        };
        let updated = update_trade(&pool, tx_id, payload).await.unwrap();
        assert_eq!(updated.account_id, Some(depot2.id));

        let (new_amount,): (i64,) = sqlx::query_as(
            "SELECT amount_cents FROM transactions WHERE id = ?1"
        ).bind(tx_id).fetch_one(&pool).await.unwrap();
        assert_eq!(new_amount, -12000);
    }

    #[tokio::test]
    async fn update_trade_can_swap_security_id() {
        let pool = connect_memory().await.unwrap();
        let (acc, sec_a) = setup_broker_and_security(&pool).await;
        let sec_b = create_security(&pool, NewSecurityPayload {
            isin: "IE00B4L5Y983".into(), symbol: None, name: "Other".into(),
            currency: None, asset_type: "etf_equity".into(),
            country: None, sector: None, note: None,
        }).await.unwrap();
        sqlx::query(
            "INSERT INTO transactions
                (account_id, booking_date, amount_cents, currency, source, kind, imported_at)
             VALUES (?1, '2026-05-15', -10000, 'EUR', 'manual', 'buy', '2026-05-15T00:00:00Z')"
        ).bind(acc).execute(&pool).await.unwrap();
        let (tx_id,): (i64,) = sqlx::query_as("SELECT MAX(id) FROM transactions")
            .fetch_one(&pool).await.unwrap();
        insert_trade_row(&pool, tx_id, sec_a, "buy", 1_000_000, Some(10_000_000), 0, 0, 0, None, None, None)
            .await.unwrap();

        let payload = UpdateTradePayload {
            shares_micro: None, unit_price_micro: None, fee_cents: None,
            tax_cents: None, fx_rate_micro: None, kest_cents: None,
            withholding_tax_cents: None, account_id: None,
            security_id: Some(sec_b.id),
            amount_cents: None, tx_account_id: None,
        };
        let updated = update_trade(&pool, tx_id, payload).await.unwrap();
        assert_eq!(updated.security_id, sec_b.id);
    }

    #[tokio::test]
    async fn update_trade_rollback_on_invalid_security() {
        let pool = connect_memory().await.unwrap();
        let (acc, sec) = setup_broker_and_security(&pool).await;
        sqlx::query(
            "INSERT INTO transactions
                (account_id, booking_date, amount_cents, currency, source, kind, imported_at)
             VALUES (?1, '2026-05-15', -10000, 'EUR', 'manual', 'buy', '2026-05-15T00:00:00Z')"
        ).bind(acc).execute(&pool).await.unwrap();
        let (tx_id,): (i64,) = sqlx::query_as("SELECT MAX(id) FROM transactions")
            .fetch_one(&pool).await.unwrap();
        insert_trade_row(&pool, tx_id, sec, "buy", 1_000_000, Some(10_000_000), 0, 0, 0, None, None, None)
            .await.unwrap();

        let payload = UpdateTradePayload {
            shares_micro: None, unit_price_micro: None, fee_cents: None,
            tax_cents: None, fx_rate_micro: None, kest_cents: None,
            withholding_tax_cents: None, account_id: None,
            security_id: Some(99_999),
            amount_cents: Some(-9999),
            tx_account_id: None,
        };
        let res = update_trade(&pool, tx_id, payload).await;
        assert!(res.is_err(), "FK-Violation muss Fehler werfen");

        let (amt,): (i64,) = sqlx::query_as(
            "SELECT amount_cents FROM transactions WHERE id = ?1"
        ).bind(tx_id).fetch_one(&pool).await.unwrap();
        assert_eq!(amt, -10000, "Rollback: amount_cents unverändert");
    }
}
