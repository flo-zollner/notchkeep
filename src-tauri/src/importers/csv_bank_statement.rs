use std::collections::HashMap;

use super::{ImportError, ImportResult, ParseResult, RawTradeFields, RawTransaction};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CsvEncoding {
    Utf8,
    Utf16Le,
}

/// Signature for TR-specific trade extraction. Called by the generic parser
/// for each row when `cfg.trade_extractor.is_some()`. Returns
/// `(kind_override, trade_fields)` — both optional.
pub type TradeExtractorFn = fn(
    &csv::StringRecord,
    &HashMap<String, usize>,
) -> ImportResult<(Option<String>, Option<RawTradeFields>)>;

/// Fully declarative description of a CSV banking format. Columns are looked
/// up by header NAME (case-insensitive, trimmed) — column reordering,
/// extra columns, and missing optional columns are all permitted.
pub struct CsvBankStatementConfig {
    /// Stored as `transactions.source`.
    pub source: &'static str,
    pub encoding: CsvEncoding,
    pub delimiter: u8,
    /// Required — hard error if not present in the CSV header.
    pub date_field: &'static str,
    /// `chrono::NaiveDate::parse_from_str` format string.
    pub date_format: &'static str,
    /// Required — hard error if not present in the CSV header.
    pub amount_field: &'static str,
    /// Optional. `None` or missing column → `default_currency` is used.
    pub currency_field: Option<&'static str>,
    pub default_currency: &'static str,
    /// First non-empty column from the list becomes `RawTransaction.counterparty`.
    /// Configured columns that are missing from the header produce warnings.
    pub counterparty_fields: &'static [&'static str],
    pub counterparty_iban_field: Option<&'static str>,
    /// First non-empty column → `RawTransaction.purpose`. Warnings if missing.
    pub purpose_fields: &'static [&'static str],
    pub raw_ref_field: Option<&'static str>,
    /// When true, rows with `amount_cents == 0` are silently skipped
    /// (counted neither as `parsed` nor as `skipped`).
    pub skip_zero_amounts: bool,
    /// Optional hook for trade field extraction (TR).
    pub trade_extractor: Option<TradeExtractorFn>,
    /// Optional text preprocessor that runs after `decode_bytes` and before
    /// the CSV reader. Workaround for non-RFC-compliant CSV formats
    /// (e.g. Sparkasse George with unquoted thousands-comma in amount /
    /// booking-details fields). `None` = no preprocessing.
    pub preprocess: Option<fn(&str) -> String>,
}

/// Main entry point. Reads CSV bytes, validates the header against the config,
/// iterates over all data rows, and builds `RawTransaction` instances.
///
/// Errors:
///   - Encoding decode failure
///   - Required column (`date_field`, `amount_field`) not found in the CSV header
///   - Parse failure in a data row (date or amount)
///
/// Warnings (non-fatal, returned in `ParseResult.warnings`):
///   - Configured optional column not found in the CSV header
pub fn parse_csv_bank_statement(
    bytes: &[u8],
    cfg: &CsvBankStatementConfig,
) -> ImportResult<ParseResult> {
    let text = decode_bytes(bytes, cfg.encoding)?;
    let text = match cfg.preprocess {
        Some(pp) => pp(&text),
        None => text,
    };
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(cfg.delimiter)
        .has_headers(true)
        .from_reader(text.as_bytes());

    let headers = rdr.headers()?.clone();
    let header_map = build_header_map(headers.iter());

    let mut warnings: Vec<String> = Vec::new();

    // Check required columns
    for required in [cfg.date_field, cfg.amount_field] {
        if !header_map.contains_key(&normalize_header(required)) {
            return Err(ImportError::Parse(format!(
                "required column '{required}' not found in CSV header (got: {:?})",
                headers.iter().collect::<Vec<_>>()
            )));
        }
    }

    // Optional configured columns: missing → warning
    let optional_singles: &[Option<&'static str>] = &[
        cfg.currency_field,
        cfg.counterparty_iban_field,
        cfg.raw_ref_field,
    ];
    for field in optional_singles.iter().flatten() {
        if !header_map.contains_key(&normalize_header(field)) {
            warnings.push(format!(
                "configured column '{field}' not found in CSV header — falling back to default/None"
            ));
        }
    }
    for field in cfg.counterparty_fields {
        if !header_map.contains_key(&normalize_header(field)) {
            warnings.push(format!(
                "counterparty fallback column '{field}' not found in CSV header — skipping in chain"
            ));
        }
    }
    for field in cfg.purpose_fields {
        if !header_map.contains_key(&normalize_header(field)) {
            warnings.push(format!(
                "purpose fallback column '{field}' not found in CSV header — skipping in chain"
            ));
        }
    }

    let mut raws: Vec<RawTransaction> = Vec::new();

    for record in rdr.records() {
        let record = record?;

        let date_str = lookup_field(&record, &header_map, cfg.date_field)
            .unwrap_or("")
            .trim();
        let booking_date = chrono::NaiveDate::parse_from_str(date_str, cfg.date_format)
            .map_err(|e| ImportError::Parse(format!("date '{date_str}': {e}")))?;

        let amount_str = lookup_field(&record, &header_map, cfg.amount_field).unwrap_or("");
        let amount_cents = parse_amount_cents(amount_str)?;

        if cfg.skip_zero_amounts && amount_cents == 0 {
            continue;
        }

        let currency = cfg
            .currency_field
            .and_then(|f| lookup_field(&record, &header_map, f))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| cfg.default_currency.to_string());

        let counterparty = pick_first_non_empty(&record, &header_map, cfg.counterparty_fields);
        let counterparty_iban = cfg
            .counterparty_iban_field
            .and_then(|f| lookup_field(&record, &header_map, f))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let purpose = pick_first_non_empty(&record, &header_map, cfg.purpose_fields);
        let raw_ref = cfg
            .raw_ref_field
            .and_then(|f| lookup_field(&record, &header_map, f))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let (kind, trade) = if let Some(extractor) = cfg.trade_extractor {
            extractor(&record, &header_map)?
        } else {
            (None, None)
        };

        raws.push(RawTransaction {
            booking_date,
            amount_cents,
            currency,
            counterparty,
            counterparty_iban,
            purpose,
            raw_ref,
            kind,
            trade,
        });
    }

    Ok(ParseResult { raws, warnings })
}

fn pick_first_non_empty(
    record: &csv::StringRecord,
    header_map: &HashMap<String, usize>,
    fields: &[&str],
) -> Option<String> {
    for field in fields {
        if let Some(v) = lookup_field(record, header_map, field) {
            let v = v.trim();
            if !v.is_empty() {
                return Some(v.to_string());
            }
        }
    }
    None
}

fn normalize_header(s: &str) -> String {
    s.trim().to_lowercase()
}

fn build_header_map<'a, I: IntoIterator<Item = &'a str>>(headers: I) -> HashMap<String, usize> {
    let mut map = HashMap::new();
    for (i, h) in headers.into_iter().enumerate() {
        map.insert(normalize_header(h), i);
    }
    map
}

fn lookup_field<'a>(
    record: &'a csv::StringRecord,
    header_map: &HashMap<String, usize>,
    field: &str,
) -> Option<&'a str> {
    let idx = *header_map.get(&normalize_header(field))?;
    record.get(idx)
}

/// Parses a monetary string into integer cents. Accepts English format with
/// `.` as the decimal separator and an optional `,` thousands separator
/// (stripped before parsing). The fractional part is right-padded with `0`
/// to at least 2 digits and truncated to 2 digits (no rounding — TR supplies
/// six decimal places whose last four are always 0).
///
/// Examples:
/// - `"100.000000"` (TR) → 10_000
/// - `"8,946.90"` (Sparkasse) → 894_690
/// - `"-5,000.00"` → -500_000
/// - `""` → 0
pub(crate) fn parse_amount_cents(s: &str) -> ImportResult<i64> {
    let s = s.trim();
    if s.is_empty() {
        return Ok(0);
    }
    // Strip thousands-comma
    let stripped: String = s.chars().filter(|c| *c != ',').collect();

    let negative = stripped.starts_with('-');
    let unsigned = if negative {
        &stripped[1..]
    } else {
        stripped.as_str()
    };

    let mut parts = unsigned.splitn(2, '.');
    let int_part = parts.next().unwrap_or("0");
    let frac_part = parts.next().unwrap_or("");

    if int_part.is_empty() || !int_part.chars().all(|c| c.is_ascii_digit()) {
        return Err(ImportError::Parse(String::from("amount integer part: invalid digits")));
    }
    if !frac_part.chars().all(|c| c.is_ascii_digit()) {
        return Err(ImportError::Parse(String::from("amount fraction part: invalid digits")));
    }
    if unsigned.split('.').count() > 2 {
        return Err(ImportError::Parse(String::from("amount: multiple decimal points")));
    }

    let int_val: i64 = int_part.parse().map_err(|_: std::num::ParseIntError| {
        ImportError::Parse(String::from("amount integer part: out of range"))
    })?;
    let frac_val: i64 = if frac_part.is_empty() {
        0
    } else {
        let padded = format!("{:0<2}", frac_part);
        padded[..2].parse().map_err(|_: std::num::ParseIntError| {
            ImportError::Parse(String::from("amount fraction part: out of range"))
        })?
    };
    let cents = int_val
        .checked_mul(100)
        .and_then(|x| x.checked_add(frac_val))
        .ok_or_else(|| ImportError::Parse(String::from("amount: value out of range")))?;
    Ok(if negative { -cents } else { cents })
}

/// Decodes a byte slice to a string according to the configured encoding.
/// UTF-8: optional BOM (EF BB BF) is stripped. UTF-16 LE: BOM (FF FE) is
/// REQUIRED; missing BOM is an error (to prevent UTF-8 bytes from being
/// accidentally interpreted as UTF-16).
pub(crate) fn decode_bytes(bytes: &[u8], encoding: CsvEncoding) -> ImportResult<String> {
    match encoding {
        CsvEncoding::Utf8 => {
            let stripped = if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
                &bytes[3..]
            } else {
                bytes
            };
            String::from_utf8(stripped.to_vec())
                .map_err(|e| ImportError::Parse(format!("utf-8 decode: {e}")))
        }
        CsvEncoding::Utf16Le => {
            if bytes.len() < 2 || bytes[0] != 0xFF || bytes[1] != 0xFE {
                return Err(ImportError::Parse(
                    "expected UTF-16 LE BOM (FF FE) at start of file".into(),
                ));
            }
            let payload = &bytes[2..];
            if !payload.len().is_multiple_of(2) {
                return Err(ImportError::Parse(
                    "UTF-16 LE payload has odd byte length".into(),
                ));
            }
            let units: Vec<u16> = payload
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            Ok(String::from_utf16_lossy(&units))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL_CONFIG: CsvBankStatementConfig = CsvBankStatementConfig {
        source: "test_csv",
        encoding: CsvEncoding::Utf8,
        delimiter: b',',
        date_field: "date",
        date_format: "%Y-%m-%d",
        amount_field: "amount",
        currency_field: Some("currency"),
        default_currency: "EUR",
        counterparty_fields: &["partner"],
        counterparty_iban_field: Some("iban"),
        purpose_fields: &["purpose"],
        raw_ref_field: Some("ref"),
        skip_zero_amounts: false,
        trade_extractor: None,
        preprocess: None,
    };

    #[test]
    fn parse_happy_path_one_row() {
        let csv = "date,amount,currency,partner,iban,purpose,ref\n2026-05-01,-13.03,EUR,SPAR,AT01,coffee,xyz\n";
        let result = parse_csv_bank_statement(csv.as_bytes(), &MINIMAL_CONFIG).unwrap();
        assert!(result.warnings.is_empty());
        assert_eq!(result.raws.len(), 1);
        let r = &result.raws[0];
        assert_eq!(
            r.booking_date,
            chrono::NaiveDate::from_ymd_opt(2026, 5, 1).unwrap()
        );
        assert_eq!(r.amount_cents, -1303);
        assert_eq!(r.currency, "EUR");
        assert_eq!(r.counterparty.as_deref(), Some("SPAR"));
        assert_eq!(r.counterparty_iban.as_deref(), Some("AT01"));
        assert_eq!(r.purpose.as_deref(), Some("coffee"));
        assert_eq!(r.raw_ref.as_deref(), Some("xyz"));
        assert!(r.trade.is_none());
        assert!(r.kind.is_none());
    }

    #[test]
    fn parse_required_amount_column_missing_errors() {
        // amount column missing from header → hard error.
        let csv = "date,currency,partner\n2026-05-01,EUR,SPAR\n";
        let err = parse_csv_bank_statement(csv.as_bytes(), &MINIMAL_CONFIG).unwrap_err();
        assert!(
            matches!(err, ImportError::Parse(msg) if msg.contains("amount") && msg.contains("required"))
        );
    }

    #[test]
    fn parse_required_date_column_missing_errors() {
        let csv = "amount,currency,partner\n-13.03,EUR,SPAR\n";
        let err = parse_csv_bank_statement(csv.as_bytes(), &MINIMAL_CONFIG).unwrap_err();
        assert!(
            matches!(err, ImportError::Parse(msg) if msg.contains("date") && msg.contains("required"))
        );
    }

    #[test]
    fn parse_invalid_date_in_row_errors() {
        let csv = "date,amount,currency,partner,iban,purpose,ref\nNOTADATE,-13.03,EUR,SPAR,AT01,coffee,xyz\n";
        let err = parse_csv_bank_statement(csv.as_bytes(), &MINIMAL_CONFIG).unwrap_err();
        assert!(matches!(err, ImportError::Parse(_)));
    }

    #[test]
    fn parse_invalid_amount_in_row_errors() {
        let csv = "date,amount,currency,partner,iban,purpose,ref\n2026-05-01,NOTANUMBER,EUR,SPAR,AT01,coffee,xyz\n";
        let err = parse_csv_bank_statement(csv.as_bytes(), &MINIMAL_CONFIG).unwrap_err();
        assert!(matches!(err, ImportError::Parse(_)));
    }

    #[test]
    fn parse_amount_cents_rejects_overflow_instead_of_wrapping() {
        // 17-digit integer part: int_val * 100 overflows i64. Must be a parse
        // error, not a silently wrapped (corrupt) cent value.
        let err = parse_amount_cents("99999999999999999.00").unwrap_err();
        assert!(matches!(err, ImportError::Parse(msg) if msg.contains("out of range")));
    }

    #[test]
    fn parse_amount_cents_error_does_not_echo_field_value() {
        // A misaligned column could carry PII into the amount field; the error
        // must never contain the raw value.
        let err = parse_amount_cents("DE00000000000000000000").unwrap_err();
        let ImportError::Parse(msg) = err else {
            panic!("expected Parse")
        };
        assert!(!msg.contains("DE00000000000000000000"));
    }

    #[test]
    fn decode_utf8_passthrough_no_bom() {
        let bytes = b"hello,world\n";
        let s = decode_bytes(bytes, CsvEncoding::Utf8).unwrap();
        assert_eq!(s, "hello,world\n");
    }

    #[test]
    fn decode_utf8_strips_bom_if_present() {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(b"hello");
        let s = decode_bytes(&bytes, CsvEncoding::Utf8).unwrap();
        assert_eq!(s, "hello");
    }

    #[test]
    fn decode_utf16_le_with_bom() {
        // "hi" as UTF-16 LE: 0xFF 0xFE 'h' 0x00 'i' 0x00
        let bytes = &[0xFF, 0xFE, b'h', 0x00, b'i', 0x00];
        let s = decode_bytes(bytes, CsvEncoding::Utf16Le).unwrap();
        assert_eq!(s, "hi");
    }

    #[test]
    fn decode_utf16_le_rejects_missing_bom() {
        let bytes = &[b'h', 0x00, b'i', 0x00];
        let err = decode_bytes(bytes, CsvEncoding::Utf16Le).unwrap_err();
        assert!(matches!(err, ImportError::Parse(msg) if msg.contains("BOM")));
    }

    #[test]
    fn decode_utf16_le_rejects_odd_byte_length() {
        let bytes = &[0xFF, 0xFE, b'h', 0x00, b'i'];
        let err = decode_bytes(bytes, CsvEncoding::Utf16Le).unwrap_err();
        assert!(matches!(err, ImportError::Parse(_)));
    }

    #[test]
    fn amount_no_thousands_english_decimal() {
        assert_eq!(parse_amount_cents("100.000000").unwrap(), 10_000);
        assert_eq!(parse_amount_cents("-6.300000").unwrap(), -630);
        assert_eq!(parse_amount_cents("13.03").unwrap(), 1303);
        assert_eq!(parse_amount_cents("-1234.50").unwrap(), -123_450);
    }

    #[test]
    fn amount_with_thousands_separator() {
        assert_eq!(parse_amount_cents("8,946.90").unwrap(), 894_690);
        assert_eq!(parse_amount_cents("-5,000.00").unwrap(), -500_000);
        assert_eq!(parse_amount_cents("1,234,567.89").unwrap(), 123_456_789);
    }

    #[test]
    fn amount_empty_string_returns_zero() {
        assert_eq!(parse_amount_cents("").unwrap(), 0);
        assert_eq!(parse_amount_cents("   ").unwrap(), 0);
    }

    #[test]
    fn amount_zero_in_various_forms() {
        assert_eq!(parse_amount_cents("0").unwrap(), 0);
        assert_eq!(parse_amount_cents("0.00").unwrap(), 0);
        assert_eq!(parse_amount_cents("-0.00").unwrap(), 0);
    }

    #[test]
    fn amount_no_decimal_part() {
        assert_eq!(parse_amount_cents("42").unwrap(), 4200);
        assert_eq!(parse_amount_cents("-7").unwrap(), -700);
    }

    #[test]
    fn amount_invalid_input_errors() {
        assert!(parse_amount_cents("abc").is_err());
        assert!(parse_amount_cents("1.2.3").is_err());
    }

    #[test]
    fn header_normalize_lowercases_and_trims() {
        assert_eq!(normalize_header("  Booking Date  "), "booking date");
        assert_eq!(normalize_header("AMOUNT"), "amount");
        assert_eq!(normalize_header("BIC/SWIFT"), "bic/swift");
    }

    #[test]
    fn build_header_map_indexes_by_normalized_name() {
        let headers = ["Booking Date", " Amount ", "Currency"];
        let map = build_header_map(headers.iter().copied());
        assert_eq!(map.get("booking date"), Some(&0));
        assert_eq!(map.get("amount"), Some(&1));
        assert_eq!(map.get("currency"), Some(&2));
        assert_eq!(map.get("missing"), None);
    }

    #[test]
    fn build_header_map_last_wins_on_duplicate_normalized_name() {
        let headers = ["Currency", "currency", "CURRENCY"];
        let map = build_header_map(headers.iter().copied());
        // Last entry wins — deterministic behaviour on duplicates.
        assert_eq!(map.get("currency"), Some(&2));
    }

    #[test]
    fn lookup_field_returns_value_at_column() {
        let record = csv::StringRecord::from(vec!["a", "b", "c"]);
        let mut map = HashMap::new();
        map.insert("col1".to_string(), 1usize);
        assert_eq!(lookup_field(&record, &map, "col1"), Some("b"));
        assert_eq!(lookup_field(&record, &map, "missing"), None);
    }

    #[test]
    fn parse_reordered_columns_work_via_name_lookup() {
        // Columns in a different order than expected by MINIMAL_CONFIG.
        let csv = "ref,purpose,iban,partner,currency,amount,date\nxyz,coffee,AT01,SPAR,EUR,-13.03,2026-05-01\n";
        let result = parse_csv_bank_statement(csv.as_bytes(), &MINIMAL_CONFIG).unwrap();
        assert!(result.warnings.is_empty());
        assert_eq!(result.raws.len(), 1);
        assert_eq!(result.raws[0].counterparty.as_deref(), Some("SPAR"));
        assert_eq!(result.raws[0].amount_cents, -1303);
    }

    #[test]
    fn parse_extra_unknown_columns_are_silently_ignored() {
        let csv = "date,amount,currency,partner,iban,purpose,ref,extra1,extra2\n2026-05-01,-13.03,EUR,SPAR,AT01,coffee,xyz,foo,bar\n";
        let result = parse_csv_bank_statement(csv.as_bytes(), &MINIMAL_CONFIG).unwrap();
        assert!(
            result.warnings.is_empty(),
            "extra columns must be silent: {:?}",
            result.warnings
        );
        assert_eq!(result.raws.len(), 1);
    }

    #[test]
    fn parse_optional_currency_column_missing_warns_and_defaults() {
        // currency column missing → warning + default_currency.
        let csv = "date,amount,partner,iban,purpose,ref\n2026-05-01,-13.03,SPAR,AT01,coffee,xyz\n";
        let result = parse_csv_bank_statement(csv.as_bytes(), &MINIMAL_CONFIG).unwrap();
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains("currency"));
        assert_eq!(result.raws[0].currency, "EUR");
    }

    #[test]
    fn parse_skip_zero_amount_excludes_row() {
        let cfg_skip = CsvBankStatementConfig {
            skip_zero_amounts: true,
            ..MINIMAL_CONFIG
        };
        let csv = "date,amount,currency,partner,iban,purpose,ref\n2026-05-01,0.00,EUR,X,,note,\n2026-05-02,-13.03,EUR,SPAR,,,\n";
        let result = parse_csv_bank_statement(csv.as_bytes(), &cfg_skip).unwrap();
        assert_eq!(result.raws.len(), 1);
        assert_eq!(result.raws[0].amount_cents, -1303);
    }

    #[test]
    fn parse_zero_amount_kept_when_skip_disabled() {
        // MINIMAL_CONFIG has skip_zero_amounts=false
        let csv = "date,amount,currency,partner,iban,purpose,ref\n2026-05-01,0.00,EUR,X,,note,\n";
        let result = parse_csv_bank_statement(csv.as_bytes(), &MINIMAL_CONFIG).unwrap();
        assert_eq!(result.raws.len(), 1);
        assert_eq!(result.raws[0].amount_cents, 0);
    }

    #[test]
    fn parse_counterparty_fallback_picks_first_non_empty() {
        let cfg_multi = CsvBankStatementConfig {
            counterparty_fields: &["preferred", "fallback1", "fallback2"],
            ..MINIMAL_CONFIG
        };
        let csv = "date,amount,currency,iban,purpose,ref,preferred,fallback1,fallback2\n2026-05-01,-13.03,EUR,AT01,coffee,xyz,,SECOND,THIRD\n";
        let result = parse_csv_bank_statement(csv.as_bytes(), &cfg_multi).unwrap();
        assert_eq!(result.raws[0].counterparty.as_deref(), Some("SECOND"));
    }

    #[test]
    fn parse_counterparty_all_chain_empty_returns_none() {
        let cfg_multi = CsvBankStatementConfig {
            counterparty_fields: &["preferred", "fallback1"],
            ..MINIMAL_CONFIG
        };
        let csv = "date,amount,currency,iban,purpose,ref,preferred,fallback1\n2026-05-01,-13.03,EUR,AT01,coffee,xyz,,\n";
        let result = parse_csv_bank_statement(csv.as_bytes(), &cfg_multi).unwrap();
        assert!(result.raws[0].counterparty.is_none());
    }

    #[test]
    fn parse_purpose_picks_first_non_empty_with_chain() {
        let cfg_purpose = CsvBankStatementConfig {
            purpose_fields: &["primary", "secondary"],
            ..MINIMAL_CONFIG
        };
        let csv = "date,amount,currency,partner,iban,ref,primary,secondary\n2026-05-01,-13.03,EUR,SPAR,AT01,xyz,,fallback-text\n";
        let result = parse_csv_bank_statement(csv.as_bytes(), &cfg_purpose).unwrap();
        assert_eq!(result.raws[0].purpose.as_deref(), Some("fallback-text"));
    }
}
