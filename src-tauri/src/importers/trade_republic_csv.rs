use std::collections::HashMap;

use super::csv_bank_statement::{CsvBankStatementConfig, CsvEncoding};
use super::{ImportError, ImportResult, ParseResult, RawTradeFields};

/// Importer für den Trade-Republic-„Transaction export"-CSV.
///
/// Format-Annahmen (Stand 2026-05): komma-separiert, Doppelquotes, englische
/// Header, Beträge mit Punkt-Dezimaltrenner und sechs Nachkommastellen.
/// Spalten: datetime, date, account_type, category, type, asset_class, name,
/// symbol, shares, price, amount, fee, tax, currency, original_amount,
/// original_currency, fx_rate, description, transaction_id,
/// counterparty_name, counterparty_iban, payment_reference, mcc_code.
pub struct TradeRepublicCsv;

pub const TR_CONFIG: CsvBankStatementConfig = CsvBankStatementConfig {
    source: "tr_csv",
    encoding: CsvEncoding::Utf8,
    delimiter: b',',
    date_field: "date",
    date_format: "%Y-%m-%d",
    amount_field: "amount",
    currency_field: Some("currency"),
    default_currency: "EUR",
    counterparty_fields: &["name", "counterparty_name", "description"],
    counterparty_iban_field: Some("counterparty_iban"),
    purpose_fields: &["payment_reference"],
    raw_ref_field: Some("transaction_id"),
    skip_zero_amounts: false,
    trade_extractor: Some(tr_extract_trade),
    preprocess: None,
};

impl super::Importer for TradeRepublicCsv {
    fn parse(&self, bytes: &[u8]) -> super::ImportResult<ParseResult> {
        super::csv_bank_statement::parse_csv_bank_statement(bytes, &TR_CONFIG)
    }
}

/// Validiert eine ISIN gegen das Format `[A-Z]{2}[A-Z0-9]{9}[0-9]`.
/// Mirror des CHECK-constraints in der DB.
fn is_valid_isin(s: &str) -> bool {
    if s.len() != 12 { return false; }
    let mut chars = s.chars();
    let c1 = chars.next().unwrap();
    let c2 = chars.next().unwrap();
    if !c1.is_ascii_uppercase() || !c2.is_ascii_uppercase() { return false; }
    let mut middle = chars.by_ref().take(9);
    if !middle.all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()) { return false; }
    let last = chars.next();
    matches!(last, Some(c) if c.is_ascii_digit())
}

/// Extrahiert die erste ISIN-konforme Sequenz aus einem Freitext.
/// Schaut nach `[A-Z]{2}[A-Z0-9]{9}[0-9]` als Whole-Word-Match.
/// Returnt `Some(isin)` wenn gefunden, sonst `None`.
fn extract_isin_from_text(text: &str) -> Option<String> {
    // Split nach Whitespace + Komma/Punkt-trimmed; jeden Token gegen is_valid_isin testen.
    for raw in text.split_whitespace() {
        let token: String = raw.chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .collect();
        // Slide window of length 12 over the token (in case ISIN sits inside a longer string)
        if token.len() >= 12 {
            for start in 0..=token.len() - 12 {
                let candidate = &token[start..start + 12];
                if is_valid_isin(candidate) {
                    return Some(candidate.to_string());
                }
            }
        }
    }
    None
}

/// TR-spezifischer Trade-Extractor — wird vom generischen Parser pro Row
/// aufgerufen. Liefert (kind_override, trade_fields).
fn tr_extract_trade(
    record: &csv::StringRecord,
    header_map: &HashMap<String, usize>,
) -> super::ImportResult<(Option<String>, Option<RawTradeFields>)> {
    let get_by_name = |name: &str| -> &str {
        let normalized = name.trim().to_lowercase();
        header_map
            .get(&normalized)
            .and_then(|idx| record.get(*idx))
            .unwrap_or("")
            .trim()
    };

    let type_raw = get_by_name("type").to_uppercase();
    let symbol_raw = get_by_name("symbol");
    let name_raw = get_by_name("name");
    let asset_class_raw = get_by_name("asset_class").to_uppercase();
    let shares_raw = get_by_name("shares");
    let price_raw = get_by_name("price");
    let fee_raw = get_by_name("fee");
    let tax_raw = get_by_name("tax");
    let fx_raw = get_by_name("fx_rate");
    let original_currency = get_by_name("original_currency");
    let description_raw = get_by_name("description");

    parse_trade_fields(
        &type_raw,
        symbol_raw,
        name_raw,
        &asset_class_raw,
        shares_raw,
        price_raw,
        fee_raw,
        tax_raw,
        fx_raw,
        original_currency,
        description_raw,
    )
}

fn parse_trade_fields(
    type_raw: &str,
    symbol: &str,
    name: &str,
    asset_class: &str,
    shares: &str,
    price: &str,
    fee: &str,
    tax: &str,
    fx_rate_csv: &str,
    _original_currency: &str,
    description: &str,
) -> ImportResult<(Option<String>, Option<RawTradeFields>)> {
    let side = match type_raw {
        "BUY" => "buy",
        "SELL" => "sell",
        "DIVIDEND" => "dividend",
        "STOCK_SPLIT" | "REVERSE_SPLIT" | "SPIN_OFF" => "corporate_action",
        _ => return Ok((None, None)),
    };

    // ISIN-Resolution: zuerst symbol (gilt für Aktien/ETFs/Bonds wo TR das ISIN ins symbol-
    // Feld schreibt). Wenn symbol kein gültiges ISIN-Format → versuche description
    // (Crypto: TR schreibt dort 'Buy trade XF000BTC0017 Bitcoin, …').
    let isin = if is_valid_isin(symbol) {
        symbol.to_string()
    } else if let Some(extracted) = extract_isin_from_text(description) {
        extracted
    } else {
        // Kein gültiges ISIN-Pattern findbar → diese Row nicht als Trade behandeln.
        // Die Tx landet als generische income/expense.
        return Ok((None, None));
    };

    let shares_abs = parse_micro(shares)?;
    let shares_micro = match side {
        "buy" => shares_abs,
        "sell" => -shares_abs,
        "dividend" => 0,
        "corporate_action" => shares_abs,
        _ => shares_abs,
    };
    let shares_micro = if type_raw == "REVERSE_SPLIT" {
        -shares_micro.abs()
    } else {
        shares_micro
    };

    let unit_price_micro = if price.is_empty() {
        None
    } else {
        Some(parse_micro(price)?)
    };

    // CSV liefert fee/tax signiert; DB will non-negative.
    let fee_cents = super::csv_bank_statement::parse_amount_cents(fee)?.unsigned_abs() as i64;
    let tax_cents = super::csv_bank_statement::parse_amount_cents(tax)?.unsigned_abs() as i64;

    let fx_rate_micro = if fx_rate_csv.is_empty() {
        None
    } else {
        Some(convert_fx_rate(fx_rate_csv)?)
    };

    Ok((Some(side.to_string()), Some(RawTradeFields {
        isin,
        asset_class_raw: asset_class.to_string(),
        name: name.to_string(),
        side: side.to_string(),
        shares_micro,
        unit_price_micro,
        fee_cents,
        kest_cents: tax_cents,              // TR liefert nur eine Steuer-Summe → AT-KESt
        withholding_tax_cents: 0,
        fx_rate_micro,
        fusion_group: None,
    })))
}

/// Parst Decimal-String in micro-Einheit (×1e6). Truncates auf 6 Stellen
/// (TR liefert konsistent 10 Stellen mit Trailing-Nullen).
fn parse_micro(s: &str) -> ImportResult<i64> {
    let s = s.trim();
    if s.is_empty() {
        return Ok(0);
    }
    let negative = s.starts_with('-');
    let unsigned = if negative { &s[1..] } else { s };
    let mut parts = unsigned.splitn(2, '.');
    let int_part = parts.next().unwrap_or("0");
    let frac_part = parts.next().unwrap_or("");

    let int_val: i64 = int_part
        .parse()
        .map_err(|e: std::num::ParseIntError| ImportError::Parse(format!("micro int '{int_part}': {e}")))?;
    let frac_val: i64 = if frac_part.is_empty() {
        0
    } else {
        let padded = format!("{:0<6}", frac_part);
        padded[..6]
            .parse()
            .map_err(|e: std::num::ParseIntError| ImportError::Parse(format!("micro frac '{frac_part}': {e}")))?
    };
    let micro = int_val
        .checked_mul(1_000_000)
        .and_then(|x| x.checked_add(frac_val))
        .ok_or_else(|| ImportError::Parse(format!("micro overflow: '{s}'")))?;
    Ok(if negative { -micro } else { micro })
}

/// Konvertiert TR's `fx_rate` (Float `1 EUR = X foreign`) in `rate_micro`
/// (`1 foreign = N micro_EUR`). Beispiel `1.10` (USD/EUR) → 909_090.
fn convert_fx_rate(csv: &str) -> ImportResult<i64> {
    let csv = csv.trim();
    let micro_per_eur = parse_micro(csv)?;
    if micro_per_eur == 0 {
        return Err(ImportError::Parse(format!("fx_rate cannot be 0: '{csv}'")));
    }
    Ok(1_000_000_000_000_i64 / micro_per_eur)
}


#[cfg(test)]
mod tests {
    use super::*;
    use super::super::Importer as _;
    use chrono::NaiveDate;

    const HEADER: &str = "\"datetime\",\"date\",\"account_type\",\"category\",\"type\",\"asset_class\",\"name\",\"symbol\",\"shares\",\"price\",\"amount\",\"fee\",\"tax\",\"currency\",\"original_amount\",\"original_currency\",\"fx_rate\",\"description\",\"transaction_id\",\"counterparty_name\",\"counterparty_iban\",\"payment_reference\",\"mcc_code\"";

    #[test]
    fn parses_card_transaction_with_merchant_name() {
        let row = "\"2025-05-13T16:42:49.970000Z\",\"2025-05-13\",\"DEFAULT\",\"CASH\",\"CARD_TRANSACTION\",\"\",\"Acme Store 42\",\"\",\"\",\"\",\"-6.300000\",\"\",\"\",\"EUR\",\"\",\"\",\"\",\"TR Card Transaction\",\"00000000-0000-0000-0000-000000000001\",\"\",\"\",\"\",\"5812\"";
        let csv = format!("{HEADER}\n{row}\n");
        let txs = TradeRepublicCsv.parse(csv.as_bytes()).unwrap().raws;
        assert_eq!(txs.len(), 1);
        let t = &txs[0];
        assert_eq!(t.booking_date, NaiveDate::from_ymd_opt(2025, 5, 13).unwrap());
        assert_eq!(t.amount_cents, -630);
        assert_eq!(t.currency, "EUR");
        assert_eq!(t.counterparty.as_deref(), Some("Acme Store 42"));
        assert_eq!(t.purpose, None);
        assert_eq!(t.raw_ref.as_deref(), Some("00000000-0000-0000-0000-000000000001"));
    }

    #[test]
    fn falls_back_to_description_when_name_and_counterparty_empty() {
        // TRANSFER_INSTANT_INBOUND: TR füllt nur description, nicht counterparty_name.
        let row = "\"2025-05-10T19:13:58.970000Z\",\"2025-05-10\",\"DEFAULT\",\"CASH\",\"TRANSFER_INSTANT_INBOUND\",\"\",\"\",\"\",\"\",\"\",\"100.000000\",\"\",\"\",\"EUR\",\"\",\"\",\"\",\"Incoming transfer from Alice Example\",\"00000000-0000-0000-0000-000000000002\",\"\",\"\",\"\",\"\"";
        let csv = format!("{HEADER}\n{row}\n");
        let txs = TradeRepublicCsv.parse(csv.as_bytes()).unwrap().raws;
        let t = &txs[0];
        assert_eq!(t.amount_cents, 10_000);
        assert_eq!(t.counterparty.as_deref(), Some("Incoming transfer from Alice Example"));
    }

    #[test]
    fn prefers_counterparty_name_over_description() {
        // SEPA-Überweisung: counterparty_name gesetzt, soll Vorrang vor description haben.
        let row = "\"2025-06-01T10:00:00.000000Z\",\"2025-06-01\",\"DEFAULT\",\"CASH\",\"TRANSFER_INBOUND\",\"\",\"\",\"\",\"\",\"\",\"500.000000\",\"\",\"\",\"EUR\",\"\",\"\",\"\",\"SEPA-Transfer\",\"00000000-0000-0000-0000-000000000003\",\"Bob Example\",\"DE00000000000000000000\",\"Miete Juni\",\"\"";
        let csv = format!("{HEADER}\n{row}\n");
        let txs = TradeRepublicCsv.parse(csv.as_bytes()).unwrap().raws;
        let t = &txs[0];
        assert_eq!(t.counterparty.as_deref(), Some("Bob Example"));
        assert_eq!(t.purpose.as_deref(), Some("Miete Juni"));
    }

    /// Spot-Check gegen eine echte TR-Export-Datei. Lokal mit
    /// `cargo test --lib -- --ignored` ausführbar; die Datei selbst ist
    /// per `.gitignore` aus dem Repo ausgeschlossen.
    #[test]
    #[ignore]
    fn smoke_test_real_export_file() {
        let path = "../test-material/Transaction export.csv";
        let bytes = std::fs::read(path).expect("real TR-CSV in test-material/");
        let txs = TradeRepublicCsv.parse(&bytes).expect("parse").raws;
        assert!(!txs.is_empty(), "no rows parsed");
        eprintln!("→ {} rows", txs.len());
        for t in txs.iter().take(3) {
            eprintln!("  {} | {:>8}¢ | {} | cp={:?} purp={:?}",
                t.booking_date, t.amount_cents, t.currency, t.counterparty, t.purpose);
        }
    }

    /// Vollverifikation gegen die echte TR-Datei. Werte sind unabhängig per
    /// Python aus der Rohdatei ermittelt (siehe Konversation 2026-05-16).
    /// Schlägt fehl, sobald das echte Format vom Parser abweicht.
    #[test]
    #[ignore]
    fn full_verification_against_real_export() {
        let path = "../test-material/Transaction export.csv";
        let bytes = std::fs::read(path).expect("real TR-CSV in test-material/");
        let txs = TradeRepublicCsv.parse(&bytes).expect("parse").raws;

        // Anzahl Datenzeilen.
        assert_eq!(txs.len(), 328, "row count");

        // Summe aller amount_cents.
        let total: i64 = txs.iter().map(|t| t.amount_cents).sum();
        assert_eq!(total, 469_939, "sum amount_cents");

        // Datums-Range.
        let dates: Vec<_> = txs.iter().map(|t| t.booking_date).collect();
        assert_eq!(dates.iter().min().unwrap().to_string(), "2025-05-10");
        assert_eq!(dates.iter().max().unwrap().to_string(), "2026-05-16");

        // Währung: ausschließlich EUR.
        for t in &txs {
            assert_eq!(t.currency, "EUR", "non-EUR currency: {:?}", t);
        }

        // Counterparty ist immer gesetzt (description ist Last-Resort-Fallback).
        let with_cp = txs.iter().filter(|t| t.counterparty.is_some()).count();
        assert_eq!(with_cp, 328, "counterparty present");

        // Purpose ist nie gesetzt (payment_reference durchgängig leer).
        let with_purpose = txs.iter().filter(|t| t.purpose.is_some()).count();
        assert_eq!(with_purpose, 0, "purpose absent");

        // raw_ref (transaction_id) ist immer gesetzt.
        let with_raw = txs.iter().filter(|t| t.raw_ref.is_some()).count();
        assert_eq!(with_raw, 328, "raw_ref present");

        // raw_ref muss eindeutig sein (UUIDs).
        let mut ids: Vec<_> = txs.iter().filter_map(|t| t.raw_ref.as_deref()).collect();
        ids.sort();
        let before = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), before, "transaction_ids not unique");

        // 6d: Trade-Rows zählen + Sample-Validierung
        let trades: Vec<_> = txs.iter().filter(|t| t.trade.is_some()).collect();
        assert!(!trades.is_empty(), "expected at least 1 trade in real fixture");
        eprintln!("→ {} trade rows", trades.len());

        let buy_count = trades.iter()
            .filter(|t| t.kind.as_deref() == Some("buy"))
            .count();
        assert!(buy_count > 0, "expected at least 1 buy");

        // Sample-Check: erste Trade-Row muss eine ISIN haben.
        let first = trades.first().unwrap();
        let t_fields = first.trade.as_ref().unwrap();
        assert_eq!(t_fields.isin.len(), 12, "ISIN should be 12 chars");
        assert!(t_fields.isin.chars().all(|c| c.is_ascii_alphanumeric()), "ISIN alphanumeric");

        eprintln!("✓ 328 rows, sum=469_939¢ (4699,39 €), unique ids, all EUR, all with counterparty");
    }

    #[test]
    fn parses_buy_row_as_trade() {
        let row = "\"2026-05-13T07:27:12.647Z\",\"2026-05-13\",\"DEFAULT\",\"TRADING\",\"BUY\",\"FUND\",\"Xtrackers II EUR Overnight\",\"LU0290358497\",\"80.4737210000\",\"149.1170000000\",\"-12000.00\",\"-1.00\",\"\",\"EUR\",\"\",\"\",\"\",\"Buy trade\",\"506e2ed2-2e39-4a92-bfce-4bae5ca20eb7\",\"\",\"\",\"\",\"\"";
        let csv = format!("{HEADER}\n{row}\n");
        let txs = TradeRepublicCsv.parse(csv.as_bytes()).unwrap().raws;
        assert_eq!(txs.len(), 1);
        let t = &txs[0];
        assert_eq!(t.kind.as_deref(), Some("buy"));
        let trade = t.trade.as_ref().expect("trade fields set");
        assert_eq!(trade.isin, "LU0290358497");
        assert_eq!(trade.asset_class_raw, "FUND");
        assert_eq!(trade.name, "Xtrackers II EUR Overnight");
        assert_eq!(trade.side, "buy");
        assert_eq!(trade.shares_micro, 80_473_721);
        assert_eq!(trade.unit_price_micro, Some(149_117_000));
        assert_eq!(trade.fee_cents, 100);
        assert_eq!(trade.kest_cents, 0);
        assert_eq!(trade.withholding_tax_cents, 0);
        assert!(trade.fx_rate_micro.is_none());
    }

    #[test]
    fn parses_sell_row_as_trade() {
        let row = "\"2026-06-01T10:00:00Z\",\"2026-06-01\",\"DEFAULT\",\"TRADING\",\"SELL\",\"STOCK\",\"Apple Inc.\",\"US0378331005\",\"10.0000000000\",\"180.5000000000\",\"1805.00\",\"-1.00\",\"\",\"EUR\",\"\",\"\",\"\",\"Sell trade\",\"id-sell-1\",\"\",\"\",\"\",\"\"";
        let csv = format!("{HEADER}\n{row}\n");
        let txs = TradeRepublicCsv.parse(csv.as_bytes()).unwrap().raws;
        let t = &txs[0];
        assert_eq!(t.amount_cents, 180_500);
        assert_eq!(t.kind.as_deref(), Some("sell"));
        let trade = t.trade.as_ref().unwrap();
        assert_eq!(trade.side, "sell");
        assert_eq!(trade.shares_micro, -10_000_000);
        assert_eq!(trade.isin, "US0378331005");
        assert_eq!(trade.asset_class_raw, "STOCK");
        assert_eq!(trade.fee_cents, 100);
    }

    #[test]
    fn parses_dividend_row_as_trade() {
        let row = "\"2026-04-15T10:00:00Z\",\"2026-04-15\",\"DEFAULT\",\"DIVIDEND\",\"DIVIDEND\",\"STOCK\",\"Apple Inc.\",\"US0378331005\",\"\",\"\",\"5.00\",\"\",\"-0.75\",\"EUR\",\"\",\"\",\"\",\"Dividend payment\",\"id-div-1\",\"\",\"\",\"\",\"\"";
        let csv = format!("{HEADER}\n{row}\n");
        let txs = TradeRepublicCsv.parse(csv.as_bytes()).unwrap().raws;
        let t = &txs[0];
        assert_eq!(t.amount_cents, 500);
        assert_eq!(t.kind.as_deref(), Some("dividend"));
        let trade = t.trade.as_ref().unwrap();
        assert_eq!(trade.side, "dividend");
        assert_eq!(trade.shares_micro, 0);
        assert_eq!(trade.kest_cents, 75);
        assert_eq!(trade.withholding_tax_cents, 0);
    }

    #[test]
    fn card_transaction_does_not_become_trade() {
        let row = "\"2025-05-13T16:42:49.970000Z\",\"2025-05-13\",\"DEFAULT\",\"CASH\",\"CARD_TRANSACTION\",\"\",\"Acme Store 42\",\"\",\"\",\"\",\"-6.300000\",\"\",\"\",\"EUR\",\"\",\"\",\"\",\"TR Card Transaction\",\"id-card-1\",\"\",\"\",\"\",\"5812\"";
        let csv = format!("{HEADER}\n{row}\n");
        let t = &TradeRepublicCsv.parse(csv.as_bytes()).unwrap().raws[0];
        assert!(t.kind.is_none(), "card-tx hat kein kind-override");
        assert!(t.trade.is_none(), "card-tx hat keine trade-fields");
    }

    #[test]
    fn buy_with_fx_rate_converts_correctly() {
        let row = "\"2026-04-15T10:00:00Z\",\"2026-04-15\",\"DEFAULT\",\"TRADING\",\"BUY\",\"STOCK\",\"Apple Inc.\",\"US0378331005\",\"10.0000000000\",\"180.0000000000\",\"-1636.36\",\"-0.91\",\"\",\"EUR\",\"-1800.00\",\"USD\",\"1.10\",\"Buy USD\",\"id-fx-1\",\"\",\"\",\"\",\"\"";
        let csv = format!("{HEADER}\n{row}\n");
        let t = &TradeRepublicCsv.parse(csv.as_bytes()).unwrap().raws[0];
        let trade = t.trade.as_ref().unwrap();
        assert_eq!(trade.fx_rate_micro, Some(909_090));
    }

    #[test]
    fn parse_micro_truncates_to_six_decimals() {
        assert_eq!(parse_micro("80.4737210000").unwrap(), 80_473_721);
        assert_eq!(parse_micro("0").unwrap(), 0);
        assert_eq!(parse_micro("").unwrap(), 0);
        assert_eq!(parse_micro("-2.5").unwrap(), -2_500_000);
        assert_eq!(parse_micro("100").unwrap(), 100_000_000);
    }

    #[test]
    fn convert_fx_rate_basic() {
        assert_eq!(convert_fx_rate("1.10").unwrap(), 909_090);
        assert_eq!(convert_fx_rate("1.00").unwrap(), 1_000_000);
    }

    #[test]
    fn is_valid_isin_accepts_typical_isins() {
        assert!(is_valid_isin("LU0290358497"));  // XEON
        assert!(is_valid_isin("IE00BK5BQT80"));  // VWCE
        assert!(is_valid_isin("XF000BTC0017"));  // TR-Crypto-ISIN für Bitcoin
        assert!(!is_valid_isin("DE000A1B2C3D")); // letztes Char muss Ziffer sein — ungültig
    }

    #[test]
    fn is_valid_isin_rejects_short_or_malformed() {
        assert!(!is_valid_isin("BTC"));
        assert!(!is_valid_isin(""));
        assert!(!is_valid_isin("LU029035849"));    // 11 Zeichen
        assert!(!is_valid_isin("LU02903584977"));  // 13 Zeichen
        assert!(!is_valid_isin("lu0290358497"));   // lowercase
        assert!(!is_valid_isin("LU029035849X"));   // letztes Char kein digit
    }

    #[test]
    fn extract_isin_from_text_finds_crypto_isin() {
        let desc = "Buy trade XF000BTC0017 Bitcoin, quantity: 0.004154";
        assert_eq!(extract_isin_from_text(desc), Some("XF000BTC0017".to_string()));
    }

    #[test]
    fn extract_isin_from_text_returns_none_when_no_match() {
        assert_eq!(extract_isin_from_text("Some random freetext"), None);
        assert_eq!(extract_isin_from_text(""), None);
    }

    #[test]
    fn extract_isin_from_text_handles_punctuation() {
        // ISIN am Wortende mit Komma
        let desc = "Sell XF000BTC0017, see notes";
        assert_eq!(extract_isin_from_text(desc), Some("XF000BTC0017".to_string()));
    }

    #[test]
    fn parses_crypto_row_uses_isin_from_description() {
        // Echte BTC-Row aus dem User-Bug-Report (asset_class=CRYPTO, symbol=BTC,
        // ISIN ist nur in description).
        let row = "\"2026-05-22T15:43:33.644Z\",\"2026-05-22\",\"DEFAULT\",\"TRADING\",\"BUY\",\"CRYPTO\",\"Bitcoin\",\"BTC\",\"0.0041540000\",\"66677.8800000000\",\"-276.98\",\"-1.00\",\"\",\"EUR\",\"\",\"\",\"\",\"Buy trade XF000BTC0017 Bitcoin, quantity: 0.004154\",\"87f39bba-4d1b-40a6-81cc-c0a6e9e2f305\",\"\",\"\",\"\",\"\"";
        let csv = format!("{HEADER}\n{row}");
        let txs = TradeRepublicCsv.parse(csv.as_bytes()).expect("parse").raws;
        assert_eq!(txs.len(), 1);
        let t = &txs[0];
        assert_eq!(t.kind.as_deref(), Some("buy"));
        let trade = t.trade.as_ref().expect("trade fields set");
        assert_eq!(trade.isin, "XF000BTC0017");
        assert_eq!(trade.asset_class_raw, "CRYPTO");
        assert_eq!(trade.shares_micro, 4154);   // 0.0041540000 × 1_000_000 ≈ 4154
    }

    #[test]
    fn parses_buy_with_invalid_symbol_and_no_isin_in_description_is_not_trade() {
        // Falls weder symbol noch description ISIN-Pattern matchen → keine Trade-Klassifikation.
        let row = "\"2026-05-22T15:43:33.644Z\",\"2026-05-22\",\"DEFAULT\",\"TRADING\",\"BUY\",\"OTHER\",\"Some Asset\",\"\",\"0.0041540000\",\"66677.88\",\"-276.98\",\"-1.00\",\"\",\"EUR\",\"\",\"\",\"\",\"no isin here\",\"unique-id\",\"\",\"\",\"\",\"\"";
        let csv = format!("{HEADER}\n{row}");
        let txs = TradeRepublicCsv.parse(csv.as_bytes()).expect("parse").raws;
        assert_eq!(txs.len(), 1);
        let t = &txs[0];
        assert!(t.kind.is_none(), "kind should be None when no valid ISIN found");
        assert!(t.trade.is_none(), "trade should be None when no valid ISIN found");
    }

    #[test]
    fn parses_multiple_rows() {
        let row1 = "\"2025-05-13T16:42:49Z\",\"2025-05-13\",\"DEFAULT\",\"CASH\",\"CARD_TRANSACTION\",\"\",\"Acme Store\",\"\",\"\",\"\",\"-6.300000\",\"\",\"\",\"EUR\",\"\",\"\",\"\",\"TR Card Transaction\",\"id-1\",\"\",\"\",\"\",\"5812\"";
        let row2 = "\"2025-05-14T08:00:00Z\",\"2025-05-14\",\"DEFAULT\",\"CASH\",\"CARD_TRANSACTION\",\"\",\"Bagel Bar\",\"\",\"\",\"\",\"-4.200000\",\"\",\"\",\"EUR\",\"\",\"\",\"\",\"TR Card Transaction\",\"id-2\",\"\",\"\",\"\",\"5812\"";
        let csv = format!("{HEADER}\n{row1}\n{row2}\n");
        let txs = TradeRepublicCsv.parse(csv.as_bytes()).unwrap().raws;
        assert_eq!(txs.len(), 2);
        assert_eq!(txs[0].raw_ref.as_deref(), Some("id-1"));
        assert_eq!(txs[1].raw_ref.as_deref(), Some("id-2"));
    }
}
