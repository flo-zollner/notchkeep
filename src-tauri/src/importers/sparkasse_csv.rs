use super::csv_bank_statement::{parse_csv_bank_statement, CsvBankStatementConfig, CsvEncoding};
use super::{ImportResult, Importer, ParseResult};

/// Reshape preprocessor for non-RFC-compliant Sparkasse George CSV.
///
/// The format exists in at least two variants:
/// - 11-col (current account): Own IBAN, Booking Date, Partner Name, Partner IBAN, BIC/SWIFT,
///   Partner Account Number, Bank code, Amount, Currency, Booking details, Payment Reference
/// - 13-col (savings account): Own account name, Own IBAN, Booking Date, Partner Name, Partner IBAN,
///   BIC/SWIFT, Partner Account Number, Bank code, Amount, Currency, Booking details,
///   Verification of Payee, This IBAN is registered to
///
/// Amount fields with a thousands-comma and Booking-details fields with German cent-commas
/// are UNQUOTED. Naive `,` splitting therefore yields too many fields.
///
/// Strategy: parse the header first, dynamically determine key column positions,
/// then normalise each row using a currency anchor + amount back-resolution.
fn reshape_sparkasse_rows(text: &str) -> String {
    let mut lines = text.split_inclusive('\n');

    // Parse header
    let Some(header_line) = lines.next() else {
        return text.to_string();
    };
    let (header_content, header_terminator) = match header_line.find(['\n', '\r']) {
        Some(i) => (&header_line[..i], &header_line[i..]),
        None => (header_line, ""),
    };
    let header_parts: Vec<&str> = header_content.split(',').collect();

    let normalize = |s: &str| s.trim().to_lowercase();
    let find_col = |name: &str| -> Option<usize> {
        header_parts
            .iter()
            .position(|h| normalize(h) == normalize(name))
    };

    // Locate required columns by name — if any are missing, pass through unchanged
    let Some(partner_name_idx) = find_col("Partner Name") else {
        return text.to_string();
    };
    let Some(amount_idx) = find_col("Amount") else {
        return text.to_string();
    };
    let Some(currency_idx_in_header) = find_col("Currency") else {
        return text.to_string();
    };
    let Some(booking_idx_in_header) = find_col("Booking details") else {
        return text.to_string();
    };

    // Sanity: exactly 4 simple columns between Partner Name and Amount
    // (Partner IBAN, BIC/SWIFT, Partner Account Number, Bank code)
    if amount_idx <= partner_name_idx + 4 || amount_idx - partner_name_idx != 5 {
        return text.to_string();
    }
    // Currency must immediately follow Amount
    if currency_idx_in_header != amount_idx + 1 {
        return text.to_string();
    }
    // Booking details must immediately follow Currency
    if booking_idx_in_header != currency_idx_in_header + 1 {
        return text.to_string();
    }

    let expected_cols = header_parts.len();
    // Number of simple (comma-free) fields after Booking details
    let simple_tail_count = expected_cols - booking_idx_in_header - 1;

    let mut out = String::with_capacity(text.len());
    // Emit header unchanged
    out.push_str(header_content);
    out.push_str(header_terminator);

    // Iterate the rest of the input (after the header)
    for line in lines {
        let (content, terminator) = match line.find(['\n', '\r']) {
            Some(i) => (&line[..i], &line[i..]),
            None => (line, ""),
        };

        if content.is_empty() {
            out.push_str(terminator);
            continue;
        }

        let parts: Vec<&str> = content.split(',').collect();
        let n = parts.len();
        if n <= expected_cols {
            // Clean row or too few fields — pass through (csv reader handles it).
            out.push_str(content);
            out.push_str(terminator);
            continue;
        }

        // 1. Currency anchor: first 3-letter ASCII-alpha field at or after amount_idx
        let currency_idx_found = (amount_idx..n)
            .find(|&i| parts[i].len() == 3 && parts[i].chars().all(|c| c.is_ascii_alphabetic()));
        let Some(currency_idx_found) = currency_idx_found else {
            out.push_str(content);
            out.push_str(terminator);
            continue;
        };
        if currency_idx_found == amount_idx {
            // Amount column empty → bail
            out.push_str(content);
            out.push_str(terminator);
            continue;
        }

        // 2. Amount greedy backward: longest merge of parts[k..currency_idx_found]
        // that satisfies is_valid_amount. k=1..=4 (max 4 parts for X,YYY,YYY,YYY.YY).
        let mut amount_start_idx: Option<usize> = None;
        for k in (1..=4usize).rev() {
            if currency_idx_found < k {
                continue;
            }
            let candidate_start = currency_idx_found - k;
            let candidate = parts[candidate_start..currency_idx_found].join(",");
            if is_valid_amount(&candidate) {
                amount_start_idx = Some(candidate_start);
                break;
            }
        }
        let Some(amount_start_idx) = amount_start_idx else {
            out.push_str(content);
            out.push_str(terminator);
            continue;
        };

        // 3. Head via back-anchor: 4 simple fields directly before amount_start_idx
        // (Partner IBAN, BIC/SWIFT, Partner Account Number, Bank code)
        if amount_start_idx < partner_name_idx + 4 {
            out.push_str(content);
            out.push_str(terminator);
            continue;
        }
        let partner_iban_data_idx = amount_start_idx - 4;

        // Fields before Partner Name (Own account name, Own IBAN, Booking Date, ...) — no commas
        // Partner Name absorbs all excess splits between partner_name_idx and partner_iban_data_idx
        let partner_name = parts[partner_name_idx..partner_iban_data_idx].join(",");
        let partner_iban = parts[partner_iban_data_idx];
        let bic = parts[partner_iban_data_idx + 1];
        let account_num = parts[partner_iban_data_idx + 2];
        let bank_code = parts[partner_iban_data_idx + 3];

        let amount = parts[amount_start_idx..currency_idx_found].join(",");
        let currency = parts[currency_idx_found];

        // 4. Tail: simple_tail_count simple fields at the end; Booking details absorbs the rest
        if n < simple_tail_count + currency_idx_found + 2 {
            // Not enough fields for the tail — bail
            out.push_str(content);
            out.push_str(terminator);
            continue;
        }
        let booking_end = n - simple_tail_count;
        let booking_details = parts[currency_idx_found + 1..booking_end].join(",");
        let tail_simple: Vec<&str> = parts[booking_end..].to_vec();

        // 5. Reassemble the row
        let mut out_fields: Vec<String> = Vec::with_capacity(expected_cols);
        for part in parts.iter().take(partner_name_idx) {
            out_fields.push(part.to_string());
        }
        out_fields.push(partner_name);
        out_fields.push(partner_iban.to_string());
        out_fields.push(bic.to_string());
        out_fields.push(account_num.to_string());
        out_fields.push(bank_code.to_string());
        out_fields.push(amount);
        out_fields.push(currency.to_string());
        out_fields.push(booking_details);
        for t in tail_simple {
            out_fields.push(t.to_string());
        }

        let row_out = out_fields
            .iter()
            .map(|f| quote_csv_field(f))
            .collect::<Vec<_>>()
            .join(",");
        out.push_str(&row_out);
        out.push_str(terminator);
    }
    out
}

fn is_valid_amount(s: &str) -> bool {
    let body = s.strip_prefix('-').unwrap_or(s);
    let Some(dot_idx) = body.find('.') else {
        return false;
    };
    let int_part = &body[..dot_idx];
    let frac_part = &body[dot_idx + 1..];
    if int_part.is_empty() || frac_part.is_empty() {
        return false;
    }
    if !frac_part.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    // int_part: pure digits, OR thousand-sep groups (first 1-3 digits, rest exactly 3)
    if int_part.chars().all(|c| c.is_ascii_digit()) {
        return true;
    }
    let groups: Vec<&str> = int_part.split(',').collect();
    if groups.len() < 2 {
        return false;
    }
    if groups[0].is_empty() || groups[0].len() > 3 || !groups[0].chars().all(|c| c.is_ascii_digit())
    {
        return false;
    }
    groups[1..]
        .iter()
        .all(|g| g.len() == 3 && g.chars().all(|c| c.is_ascii_digit()))
}

fn quote_csv_field(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        let escaped = s.replace('"', "\"\"");
        format!("\"{}\"", escaped)
    } else {
        s.to_string()
    }
}

pub const SPARKASSE_CONFIG: CsvBankStatementConfig = CsvBankStatementConfig {
    source: "sparkasse_csv",
    encoding: CsvEncoding::Utf16Le,
    delimiter: b',',
    date_field: "Booking Date",
    date_format: "%d.%m.%Y",
    amount_field: "Amount",
    currency_field: Some("Currency"),
    default_currency: "EUR",
    counterparty_fields: &["Partner Name"],
    counterparty_iban_field: Some("Partner IBAN"),
    purpose_fields: &["Payment Reference", "Booking details"],
    raw_ref_field: None,
    skip_zero_amounts: true,
    trade_extractor: None,
    preprocess: Some(reshape_sparkasse_rows),
};

pub struct SparkasseCsv;

impl Importer for SparkasseCsv {
    fn parse(&self, bytes: &[u8]) -> ImportResult<ParseResult> {
        parse_csv_bank_statement(bytes, &SPARKASSE_CONFIG)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reshape_passes_through_well_formed_row() {
        let input = "Own IBAN,Booking Date,Partner Name,Partner IBAN,BIC/SWIFT,Partner Account Number,Bank code,Amount,Currency,Booking details,Payment Reference\nAT01,01.05.2026,X,,,,, -13.03,EUR,note,\n";
        let out = reshape_sparkasse_rows(input);
        assert!(out.contains("-13.03"));
        assert_eq!(out.lines().count(), 2);
    }

    #[test]
    fn reshape_fixes_unquoted_thousand_separator_amount() {
        let header = "Own IBAN,Booking Date,Partner Name,Partner IBAN,BIC/SWIFT,Partner Account Number,Bank code,Amount,Currency,Booking details,Payment Reference\n";
        let row = "AT01,11.05.2026,Max Mustermann,AT37,BICTESTX,99999999,20815,2,000.00,EUR,,Own transfer\n";
        let input = format!("{}{}", header, row);
        let out = reshape_sparkasse_rows(&input);
        // Amount should appear as quoted "2,000.00" in the output
        assert!(
            out.contains(r#""2,000.00""#),
            "expected quoted amount, got: {}",
            out
        );
    }

    #[test]
    fn reshape_fixes_unquoted_comma_in_booking_details() {
        let header = "Own IBAN,Booking Date,Partner Name,Partner IBAN,BIC/SWIFT,Partner Account Number,Bank code,Amount,Currency,Booking details,Payment Reference\n";
        let row = "AT01,03.05.2026,CONTINENTALE,DE95,,,,-19.54,EUR,544323411 BU 19,54,\n";
        let input = format!("{}{}", header, row);
        let out = reshape_sparkasse_rows(&input);
        // booking details "544323411 BU 19,54" should be quoted
        assert!(
            out.contains(r#""544323411 BU 19,54""#),
            "expected quoted booking details, got: {}",
            out
        );
    }

    #[test]
    fn reshape_handles_amount_and_booking_both_with_commas() {
        let header = "Own IBAN,Booking Date,Partner Name,Partner IBAN,BIC/SWIFT,Partner Account Number,Bank code,Amount,Currency,Booking details,Payment Reference\n";
        let row = "AT01,11.05.2026,X,AT02,,,,5,000.00,EUR,note with 1,234 stuff,memo\n";
        let input = format!("{}{}", header, row);
        let out = reshape_sparkasse_rows(&input);
        assert!(out.contains(r#""5,000.00""#));
        assert!(out.contains(r#""note with 1,234 stuff""#));
        assert!(out.contains("memo"));
    }

    #[test]
    fn is_valid_amount_accepts_simple_and_thousand_sep() {
        assert!(is_valid_amount("13.03"));
        assert!(is_valid_amount("-13.03"));
        assert!(is_valid_amount("0.00"));
        assert!(is_valid_amount("8,946.90"));
        assert!(is_valid_amount("-5,000.00"));
        assert!(is_valid_amount("1,234,567.89"));
    }

    #[test]
    fn is_valid_amount_rejects_malformed() {
        assert!(!is_valid_amount(""));
        assert!(!is_valid_amount("abc"));
        assert!(!is_valid_amount("12")); // no decimal
        assert!(!is_valid_amount("12.")); // no frac
        assert!(!is_valid_amount(".12")); // no int
        assert!(!is_valid_amount("1,23.45")); // wrong group size
        assert!(!is_valid_amount("12,345.6.7")); // multiple dots
        assert!(!is_valid_amount("38000-102.42")); // the actual bug pattern
    }

    #[test]
    fn reshape_fixes_partner_name_with_address_commas() {
        let header = "Own IBAN,Booking Date,Partner Name,Partner IBAN,BIC/SWIFT,Partner Account Number,Bank code,Amount,Currency,Booking details,Payment Reference\n";
        // Row 239: partner name with embedded commas (address-like)
        let row = "AT000000000000000001,23.06.2023,Versicherung AG, Musterstr. 1, 1010 Wien,AT010000000000000000,BICTESTY,,38000,-102.42,EUR,1234567890,\n";
        let input = format!("{}{}", header, row);
        let out = reshape_sparkasse_rows(&input);
        // Partner Name should be re-joined as one quoted field
        assert!(
            out.contains(r#""Versicherung AG, Musterstr. 1, 1010 Wien""#),
            "expected quoted partner name, got: {}",
            out
        );
        // Amount should be -102.42 (unquoted, no commas)
        assert!(
            out.contains(",-102.42,"),
            "expected raw amount -102.42, got: {}",
            out
        );
        // Verify it parses end-to-end correctly (encode the raw input, not the reshaped
        // output — parse_csv_bank_statement calls reshape internally via preprocess hook)
        let parsed =
            parse_csv_bank_statement(&encode_to_utf16_le_with_bom(&input), &SPARKASSE_CONFIG)
                .expect("parse must succeed");
        assert_eq!(parsed.raws.len(), 1);
        assert_eq!(parsed.raws[0].amount_cents, -10242);
        assert_eq!(
            parsed.raws[0].counterparty.as_deref(),
            Some("Versicherung AG, Musterstr. 1, 1010 Wien")
        );
        assert_eq!(
            parsed.raws[0].counterparty_iban.as_deref(),
            Some("AT010000000000000000")
        );
    }

    #[test]
    fn reshape_handles_partner_name_and_booking_details_both_with_commas() {
        // Row 1939: Partner Name OK, but booking details has multiple commas (3 extra)
        let header = "Own IBAN,Booking Date,Partner Name,Partner IBAN,BIC/SWIFT,Partner Account Number,Bank code,Amount,Currency,Booking details,Payment Reference\n";
        let row = "AT000000000000000001,09.02.2026,Gesundheitskasse AG,AT020000000000000000,BICTESTZ,,60000,119.92,EUR,1234567890 WAH RB 160,00 Info 37,84 1234567890 WAZ RB 490,00 Info,\n";
        let input = format!("{}{}", header, row);
        let out = reshape_sparkasse_rows(&input);
        assert!(
            out.contains(",119.92,"),
            "amount should pass through cleanly: {}",
            out
        );
        assert!(out.contains("Gesundheitskasse AG"));
        // Booking details should be quoted as one field
        assert!(out.contains(r#""1234567890 WAH RB 160,00 Info"#));
    }

    #[test]
    fn reshape_handles_13_col_variant_with_own_account_name_and_verification_columns() {
        let header = "Own account name,Own IBAN,Booking Date,Partner Name,Partner IBAN,BIC/SWIFT,Partner Account Number,Bank code,Amount,Currency,Booking details,Verification of Payee,This IBAN is registered to\n";
        // Clean row, 13 cols → passthrough
        let row1 = "Smart Sparen,AT000000000000000002,02.06.2025,Max Mustermann,AT000000000000000001,BICTESTX,00000001,20815,220.00,EUR,,,\n";
        let input = format!("{}{}", header, row1);
        let out = reshape_sparkasse_rows(&input);
        // Should still have 13 fields per row (count commas)
        let header_commas = header.trim().matches(',').count();
        for line in out.lines().skip(1) {
            // crude check: csv-parse each line and count fields
            let mut rdr = csv::ReaderBuilder::new()
                .has_headers(false)
                .from_reader(line.as_bytes());
            if let Some(Ok(rec)) = rdr.records().next() {
                assert_eq!(
                    rec.len(),
                    header_commas + 1,
                    "row '{}' has {} fields, expected {}",
                    line,
                    rec.len(),
                    header_commas + 1
                );
            }
        }
    }

    #[test]
    fn reshape_13_col_fixes_unquoted_booking_details_with_commas() {
        let header = "Own account name,Own IBAN,Booking Date,Partner Name,Partner IBAN,BIC/SWIFT,Partner Account Number,Bank code,Amount,Currency,Booking details,Verification of Payee,This IBAN is registered to\n";
        // Real over-split: booking details contains 'Habenzinsen 1,00000% ab ...'
        let row = "Smart Sparen,AT000000000000000002,14.05.2025,,,,,,0.00,EUR,Smart Sparen Aktion ab 14.05.25 bis 13.05.26 Habenzinsen 1,00000% ab 14.05.25,,\n";
        let input = format!("{}{}", header, row);
        let out = reshape_sparkasse_rows(&input);
        // Booking details should be reassembled and quoted
        assert!(
            out.contains(
                r#""Smart Sparen Aktion ab 14.05.25 bis 13.05.26 Habenzinsen 1,00000% ab 14.05.25""#
            ),
            "expected quoted booking details, got: {}",
            out
        );
    }

    #[test]
    fn reshape_13_col_full_roundtrip_via_parser() {
        let header = "Own account name,Own IBAN,Booking Date,Partner Name,Partner IBAN,BIC/SWIFT,Partner Account Number,Bank code,Amount,Currency,Booking details,Verification of Payee,This IBAN is registered to\n";
        let rows = "Smart Sparen,AT000000000000000002,02.06.2025,Max Mustermann,AT000000000000000001,BICTESTX,00000001,20815,220.00,EUR,Test payment,,\nSmart Sparen,AT000000000000000002,14.05.2025,,,,,,0.00,EUR,Habenzinsen 1,00000% Info,,\n";
        let input = format!("{}{}", header, rows);
        let bytes = encode_to_utf16_le_with_bom(&input);
        let parsed = parse_csv_bank_statement(&bytes, &SPARKASSE_CONFIG).expect("parse");
        // The zero-amount row gets skipped (skip_zero_amounts=true), leaving 1
        assert_eq!(parsed.raws.len(), 1);
        assert_eq!(parsed.raws[0].amount_cents, 22000);
        assert_eq!(
            parsed.raws[0].counterparty.as_deref(),
            Some("Max Mustermann")
        );
    }

    #[test]
    fn reshape_falls_through_when_header_missing_required_columns() {
        // No "Amount" column → reshape can't anchor → passthrough → generic parser errors with required-column message
        let bad_header = "Foo,Bar,Baz\n";
        let row = "a,b,c\n";
        let input = format!("{}{}", bad_header, row);
        let out = reshape_sparkasse_rows(&input);
        assert_eq!(
            out, input,
            "unknown header should be passed through unchanged"
        );
    }

    fn encode_to_utf16_le_with_bom(s: &str) -> Vec<u8> {
        let mut out = vec![0xFF, 0xFE];
        for u in s.encode_utf16() {
            out.extend_from_slice(&u.to_le_bytes());
        }
        out
    }
}
