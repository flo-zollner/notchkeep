// Flatex-PDF-Importer (Wertpapier-Belege, AT-Steuerkontext).
//
// Belegstruktur und Sonderfall-Behandlung wurde im Verständnis von
// Portfolio Performance (https://github.com/portfolio-performance/portfolio,
// EPL-2.0) erarbeitet. Diese Datei ist eine Clean-Room-Reimplementation in
// Rust, keine Code-Übernahme aus PP.

use super::{ImportError, ImportResult, Importer, ParseResult, RawTransaction};

pub struct FlatexPdf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BelegTyp {
    KaufFondsZert,
    VerkaufFondsZert,
    Fondsthesaurierung,
    DividendeAuslaendisch,
    SammelabrechnungKrypto,
    SammelabrechnungWertpapier,
    SammelabrechnungSparplan,
    FondsthesaurierungStorno,
    Fusion,
    Kapitalmassnahme,
    Kontoauszug,
    Steuerreport,
}

pub fn detect_beleg_typ(text: &str) -> ImportResult<BelegTyp> {
    if text.contains("Wertpapierabrechnung Kauf") { return Ok(BelegTyp::KaufFondsZert); }
    if text.contains("Wertpapierabrechnung Verkauf") { return Ok(BelegTyp::VerkaufFondsZert); }
    if text.contains("Storno Ertragsmitteilung") {
        return Ok(BelegTyp::FondsthesaurierungStorno);
    }
    if text.contains("Ertragsmitteilung") && text.contains("thesaurierender") {
        return Ok(BelegTyp::Fondsthesaurierung);
    }
    if text.contains("Dividendengutschrift") { return Ok(BelegTyp::DividendeAuslaendisch); }
    if text.contains("Sammelabrechnung") && text.contains("Kryptowerte") {
        return Ok(BelegTyp::SammelabrechnungKrypto);
    }
    if text.contains("Sammelabrechnung aus Sparplan") {
        return Ok(BelegTyp::SammelabrechnungSparplan);
    }
    if text.contains("Sammelabrechnung") && text.contains("Wertpapierkauf") {
        return Ok(BelegTyp::SammelabrechnungWertpapier);
    }
    if text.contains("Fusion in ") || text.contains("im Rahmen einer Fusion") {
        return Ok(BelegTyp::Fusion);
    }
    if text.contains("Kontoauszug Nr") { return Ok(BelegTyp::Kontoauszug); }
    if text.contains("STEUERREPORT") { return Ok(BelegTyp::Steuerreport); }
    if text.contains("Kapitalmaßnahme") || text.contains("Bestandsübertrag") {
        return Ok(BelegTyp::Kapitalmassnahme);
    }
    Err(ImportError::Parse("unbekannter Flatex-Belegtyp".into()))
}

impl Importer for FlatexPdf {
    fn parse(&self, bytes: &[u8]) -> ImportResult<ParseResult> {
        let text = pdf_extract::extract_text_from_mem(bytes)
            .map_err(|e| ImportError::Parse(format!("pdf-extract: {e}")))?;

        let beleg = detect_beleg_typ(&text)?;
        let raws = match beleg {
            BelegTyp::Kontoauszug => return Err(ImportError::Parse(
                "Kontoauszug-PDFs werden derzeit nicht importiert. Bitte nur \
                 Einzelbelege (Kauf, Verkauf, Thesaurierung, Dividende, Krypto) \
                 hochladen. Begründung: Hohes Doppel-Buchungs-Risiko.".into()
            )),
            BelegTyp::Steuerreport => return Err(ImportError::Parse(
                "Steuerreport-PDFs sind noch nicht unterstützt.".into()
            )),
            BelegTyp::Kapitalmassnahme => return Err(ImportError::Parse(
                "Kapitalmaßnahme-Belege sind noch nicht unterstützt — benötigen \
                 echtes Test-Material.".into()
            )),
            BelegTyp::KaufFondsZert => vec![parse_kauf(&text)?],
            BelegTyp::VerkaufFondsZert => vec![parse_verkauf(&text)?],
            BelegTyp::Fondsthesaurierung => vec![parse_thesaurierung(&text)?],
            BelegTyp::FondsthesaurierungStorno => vec![parse_thesaurierung_storno(&text)?],
            BelegTyp::DividendeAuslaendisch => vec![parse_dividende(&text)?],
            BelegTyp::SammelabrechnungKrypto => parse_krypto_sammelabrechnung(&text)?,
            BelegTyp::SammelabrechnungWertpapier => parse_wertpapier_sammelabrechnung(&text)?,
            BelegTyp::SammelabrechnungSparplan => parse_sparplan_sammelabrechnung(&text)?,
            BelegTyp::Fusion => parse_fusion(&text)?,
        };
        Ok(ParseResult { raws, warnings: vec![] })
    }
}

/// ISIN-Pattern: 2 Großbuchstaben + 9 alphanumerisch (uppercase) + 1 Ziffer.
fn is_valid_isin(s: &str) -> bool {
    if s.len() != 12 { return false; }
    let bytes = s.as_bytes();
    if !(bytes[0].is_ascii_uppercase() && bytes[1].is_ascii_uppercase()) {
        return false;
    }
    for &b in &bytes[2..11] {
        if !(b.is_ascii_uppercase() || b.is_ascii_digit()) { return false; }
    }
    bytes[11].is_ascii_digit()
}

/// Sucht in `(ISIN/WKN)`-Klammern nach dem ersten valid-ISIN-Pattern.
/// Robust gegen verschachtelte Klammern (z. B. `LYXOR CORE MSCI WORLD (DR (LU1781541179/LYX0YD)`):
/// prüft alle `(`-Positionen im Text und matched die 12 Zeichen direkt nach `(`
/// gegen das ISIN-Pattern. Returnt das erste Match.
fn extract_isin_from_parens(text: &str) -> Option<String> {
    for (start, _) in text.match_indices('(') {
        let after = start + 1;
        if after + 12 > text.len() { continue; }
        let candidate_bytes = &text.as_bytes()[after..after + 12];
        if !candidate_bytes.iter().all(|b| b.is_ascii()) { continue; }
        let candidate = &text[after..after + 12];
        if is_valid_isin(candidate) {
            return Some(candidate.to_string());
        }
    }
    None
}

use chrono::NaiveDate;

/// Findet ein deutsches Datum (`DD.MM.YYYY`) nach einem Label-String.
/// Sucht das erste Datum-Token in derselben Zeile wie der Label.
pub(crate) fn extract_date_after(text: &str, label: &str) -> Option<NaiveDate> {
    let idx = text.find(label)?;
    let tail = &text[idx + label.len()..];
    let line = tail.split('\n').next()?;
    let mut chars = line.chars().enumerate().peekable();
    while let Some(&(i, c)) = chars.peek() {
        if c.is_ascii_digit() {
            let slice = &line[i..];
            if slice.len() >= 10 {
                let candidate = &slice[..10];
                let bytes = candidate.as_bytes();
                let shape_ok = bytes[2] == b'.' && bytes[5] == b'.'
                    && bytes[0..2].iter().all(|b| b.is_ascii_digit())
                    && bytes[3..5].iter().all(|b| b.is_ascii_digit())
                    && bytes[6..10].iter().all(|b| b.is_ascii_digit());
                if shape_ok {
                    if let Ok(d) = NaiveDate::parse_from_str(candidate, "%d.%m.%Y") {
                        return Some(d);
                    }
                }
            }
        }
        chars.next();
    }
    None
}

/// Parst die erste deutschsprachige Zahl (`-1.234,56` oder `1234,56`) nach
/// einem Label und gibt Cents (signiert) zurück. Stoppt am Newline.
pub(crate) fn extract_amount_cents_after(text: &str, label: &str) -> Option<i64> {
    let idx = text.find(label)?;
    let tail = &text[idx + label.len()..];
    parse_de_decimal_to_cents(tail.split('\n').next()?)
}

/// Wie `extract_amount_cents_after`, aber gibt micro-Wert zurück.
pub(crate) fn extract_micro_after(text: &str, label: &str) -> Option<i64> {
    let idx = text.find(label)?;
    let tail = &text[idx + label.len()..];
    parse_de_decimal_to_micro(tail.split('\n').next()?)
}

/// Wie `extract_micro_after`, aber matcht nur wenn dem Label kein
/// ASCII-Buchstabe folgt (verhindert z.B. "Kurs" → "Kurswert").
pub(crate) fn extract_micro_after_word(text: &str, label: &str) -> Option<i64> {
    let text_bytes = text.as_bytes();
    let mut start = 0;
    while start + label.len() <= text_bytes.len() {
        if let Some(rel) = text[start..].find(label) {
            let idx = start + rel;
            let after = idx + label.len();
            // Prüfe: nächstes Zeichen ist kein ASCII-Buchstabe
            if after < text_bytes.len() && text_bytes[after].is_ascii_alphabetic() {
                start = idx + 1;
                continue;
            }
            let tail = &text[after..];
            return parse_de_decimal_to_micro(tail.split('\n').next()?);
        } else {
            break;
        }
    }
    None
}

fn parse_de_decimal_to_cents(line: &str) -> Option<i64> {
    let (int_part, frac_part, neg) = parse_de_number_parts(line)?;
    let frac_padded = format!("{:0<2}", frac_part);
    let frac_cents: i64 = frac_padded[..2].parse().ok()?;
    let cents = int_part.checked_mul(100)?.checked_add(frac_cents)?;
    Some(if neg { -cents } else { cents })
}

fn parse_de_decimal_to_micro(line: &str) -> Option<i64> {
    let (int_part, frac_part, neg) = parse_de_number_parts(line)?;
    let frac_padded = format!("{:0<6}", frac_part);
    let frac_micro: i64 = frac_padded[..6].parse().ok()?;
    let micro = int_part.checked_mul(1_000_000)?.checked_add(frac_micro)?;
    Some(if neg { -micro } else { micro })
}

/// Findet die erste deutsche Dezimalzahl in `line` und gibt
/// `(integer_part, fractional_digits_string, negative)` zurück.
fn parse_de_number_parts(line: &str) -> Option<(i64, String, bool)> {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        let neg = b == b'-';
        let start = if neg { i + 1 } else { i };
        if start < bytes.len() && bytes[start].is_ascii_digit() {
            let mut j = start;
            while j < bytes.len()
                && (bytes[j].is_ascii_digit() || bytes[j] == b'.' || bytes[j] == b',')
            { j += 1; }
            let raw = &line[start..j];
            let normalized: String = raw.replace('.', "");
            let mut parts = normalized.splitn(2, ',');
            let int_str = parts.next()?;
            let frac_str = parts.next().unwrap_or("");
            if !int_str.chars().all(|c| c.is_ascii_digit()) { i += 1; continue; }
            if !frac_str.chars().all(|c| c.is_ascii_digit()) { i += 1; continue; }
            let int_val: i64 = int_str.parse().ok()?;
            return Some((int_val, frac_str.to_string(), neg));
        }
        i += 1;
    }
    None
}

use super::RawTradeFields;

fn parse_kauf(text: &str) -> ImportResult<RawTransaction> {
    parse_wertpapierabrechnung(text, "buy")
}

fn parse_verkauf(text: &str) -> ImportResult<RawTransaction> {
    let mut tx = parse_wertpapierabrechnung(text, "sell")?;
    if let Some(trade) = tx.trade.as_mut() {
        trade.shares_micro = -trade.shares_micro.abs();
    }
    Ok(tx)
}

fn parse_wertpapierabrechnung(text: &str, side: &'static str) -> ImportResult<RawTransaction> {
    let isin = extract_isin_from_parens(text)
        .ok_or_else(|| ImportError::Parse("ISIN nicht gefunden".into()))?;
    let booking_date = extract_date_after(text, "Handelstag")
        .or_else(|| extract_date_after(text, "Auftragsdatum"))
        .ok_or_else(|| ImportError::Parse("Handelstag nicht gefunden".into()))?;
    let amount_cents = extract_amount_cents_after(text, "Endbetrag")
        .ok_or_else(|| ImportError::Parse("Endbetrag nicht gefunden".into()))?;
    let shares_micro = extract_micro_after(text, "Ausgeführt")
        .ok_or_else(|| ImportError::Parse("Ausgeführt (Stücke) nicht gefunden".into()))?;
    let unit_price_micro = extract_micro_after_word(text, "Kurs");
    let provision = extract_amount_cents_after(text, "Provision").unwrap_or(0).abs();
    let fremde_spesen = extract_amount_cents_after(text, "Fremde Spesen").unwrap_or(0).abs();
    let eigene_spesen = extract_amount_cents_after(text, "Eigene Spesen").unwrap_or(0).abs();
    let kest_cents = extract_amount_cents_after(text, "Einbeh. KESt").unwrap_or(0).abs();
    let raw_ref = extract_value_after(text, "Auftragsnummer");
    let name = extract_security_name_inline(text).unwrap_or_else(|| "(Unbekannt)".into());

    Ok(RawTransaction {
        booking_date,
        amount_cents,
        currency: "EUR".to_string(),
        counterparty: Some("flatexDEGIRO".to_string()),
        purpose: Some(name.clone()),
        raw_ref,
        kind: Some(side.to_string()),
        trade: Some(RawTradeFields {
            isin,
            asset_class_raw: "FUND".to_string(),
            name,
            side: side.to_string(),
            shares_micro,
            unit_price_micro,
            fee_cents: provision + fremde_spesen + eigene_spesen,
            kest_cents,
            withholding_tax_cents: 0,
            fx_rate_micro: None,
            fusion_group: None,
        }),
        counterparty_iban: None,
    })
}

/// Liest den ersten Token nach einem Label bis zum Whitespace/Newline.
fn extract_value_after(text: &str, label: &str) -> Option<String> {
    let idx = text.find(label)?;
    let tail = &text[idx + label.len()..];
    let line = tail.split('\n').next()?;
    let token = line.trim().trim_start_matches(':').trim();
    let first = token.split_whitespace().next()?;
    if first.is_empty() { None } else { Some(first.to_string()) }
}

/// Sucht die Zeile mit "Nr.xxx Kauf NAME (ISIN/WKN)" und extrahiert NAME.
/// Robust gegen Klammern im Namen (z. B. `X(IE)-MSCI WORLD 1C (IE00BJ0KDQ92/...)`):
/// nutzt die ISIN-Klammer (12-Zeichen-Pattern) als Anker und nimmt den Text
/// zwischen Side-Wort (Kauf/Verkauf) und ISIN-Klammer.
fn extract_security_name_inline(text: &str) -> Option<String> {
    for line in text.lines() {
        // Finde die Position der ISIN-Klammer in dieser Zeile
        let isin_paren_pos: Option<usize> = line.match_indices('(')
            .find_map(|(pos, _)| {
                let after = pos + 1;
                if after + 12 > line.len() { return None; }
                let candidate_bytes = &line.as_bytes()[after..after + 12];
                if !candidate_bytes.iter().all(|b| b.is_ascii()) { return None; }
                let candidate = &line[after..after + 12];
                if is_valid_isin(candidate) { Some(pos) } else { None }
            });
        let Some(paren_pos) = isin_paren_pos else { continue };

        // Vor der ISIN-Klammer: suche das letzte Side-Wort (Kauf oder Verkauf)
        let before = &line[..paren_pos];
        let (side_idx, side_len) = if let Some(idx) = before.rfind("Verkauf") {
            (idx, 7)
        } else if let Some(idx) = before.rfind("Kauf") {
            (idx, 4)
        } else {
            continue;
        };

        let name = before[side_idx + side_len..].trim();
        if !name.is_empty() {
            return Some(name.to_string());
        }
    }
    None
}
fn parse_thesaurierung(text: &str) -> ImportResult<RawTransaction> {
    let isin = extract_isin_from_parens(text)
        .ok_or_else(|| ImportError::Parse("ISIN nicht gefunden".into()))?;
    let booking_date = extract_date_after(text, "Valuta")
        .or_else(|| extract_date_after(text, "Zuflusstag"))
        .ok_or_else(|| ImportError::Parse("Valuta/Zuflusstag nicht gefunden".into()))?;
    let amount_cents = extract_amount_cents_after(text, "Endbetrag")
        .ok_or_else(|| ImportError::Parse("Endbetrag nicht gefunden".into()))?;
    let kest_cents = extract_amount_cents_after(text, "Einbeh. Steuer").unwrap_or(0).abs();
    let raw_ref = extract_value_after(text, "Transaktion-Nr.");
    let name = extract_security_name_thesaurierung(text).unwrap_or_else(|| "(Unbekannt)".into());

    // Thesaurierung = ausschüttungsgleicher Ertrag, der Fonds reinvestiert intern.
    // Cash-Effekt am Verrechnungskonto ist NUR die KESt — kind='tax' und
    // side='tax' (eigene Trade-Side, kein Dividenden-Eintrag).
    Ok(RawTransaction {
        booking_date,
        amount_cents,
        currency: "EUR".to_string(),
        counterparty: Some("flatexDEGIRO".to_string()),
        purpose: Some(name.clone()),
        raw_ref,
        kind: Some("tax".to_string()),
        trade: Some(RawTradeFields {
            isin,
            asset_class_raw: "FUND".to_string(),
            name,
            side: "tax".to_string(),
            shares_micro: 0,
            unit_price_micro: None,
            fee_cents: 0,
            kest_cents,
            withholding_tax_cents: 0,
            fx_rate_micro: None,
            fusion_group: None,
        }),
        counterparty_iban: None,
    })
}

/// Storno einer Thesaurierung: dokumentiert die Rückbuchung eines früheren
/// `Fondsthesaurierung`-Belegs. Der Endbetrag im Storno-Beleg steht weiterhin
/// als negative Zahl (der ursprünglich abgezogene KESt-Betrag), buchhalterisch
/// ist der Effekt aber invers — die Bank erstattet die KESt zurück.
///
/// Mapping: amount_cents wird invertiert (Cash-Eingang statt Belastung),
/// kest_cents = 0 (keine Steuer-Belastung, sondern -Erstattung; die "Last"
/// hebt sich gegen das Original auf Aggregations-Ebene auf).
fn parse_thesaurierung_storno(text: &str) -> ImportResult<RawTransaction> {
    let mut tx = parse_thesaurierung(text)?;
    tx.amount_cents = -tx.amount_cents;
    if let Some(trade) = tx.trade.as_mut() {
        trade.kest_cents = 0;
    }
    // raw_ref distinct halten — Original und Storno haben verschiedene Nr.
    Ok(tx)
}

/// Wertpapier-Name in der Thesaurierungs-Zeile: "Nr.xxx NAME (ISIN/WKN)".
fn extract_security_name_thesaurierung(text: &str) -> Option<String> {
    for line in text.lines() {
        if !line.contains("Nr.") { continue; }
        if line.contains("Auftragsnummer") { continue; }
        if line.contains("Transaktion") { continue; }
        if line.contains("Konto Nr") { continue; }
        if let Some(paren) = line.find('(') {
            if let Some(nr_idx) = line.find("Nr.") {
                let after_nr = &line[nr_idx + 3..paren];
                // Skip the number token (digits), then collect the name
                let rest: String = after_nr.chars().skip_while(|c| !c.is_whitespace()).collect();
                let name = rest.trim();
                if !name.is_empty() {
                    return Some(name.to_string());
                }
            }
        }
    }
    None
}
fn parse_dividende(text: &str) -> ImportResult<RawTransaction> {
    let isin = extract_isin_from_parens(text)
        .ok_or_else(|| ImportError::Parse("ISIN nicht gefunden".into()))?;
    let booking_date = extract_date_after(text, "Zahlungstag")
        .or_else(|| extract_date_after(text, "Valuta"))
        .ok_or_else(|| ImportError::Parse("Zahlungstag nicht gefunden".into()))?;
    let amount_cents = extract_amount_cents_after(text, "Endbetrag")
        .ok_or_else(|| ImportError::Parse("Endbetrag nicht gefunden".into()))?;
    let kest_cents = extract_amount_cents_after(text, "Einbeh. Steuer").unwrap_or(0).abs();
    let raw_ref = extract_value_after(text, "Transaktion-Nr.");
    let name = extract_security_name_thesaurierung(text).unwrap_or_else(|| "(Unbekannt)".into());

    // Quellensteuer in Original-Währung × Devisenkurs → EUR.
    // Beleg-Konvention: Devisenkurs = original_currency pro EUR
    // Beispiel: HKD/EUR = 9,1735 → 1,56 HKD ÷ 9,1735 = 0,17 EUR.
    let quellenst_orig_micro = extract_micro_after(text, "Gez. Quellenst.").unwrap_or(0).abs();
    let fx_orig_per_eur_micro = extract_micro_after(text, "Devisenkurs").unwrap_or(1_000_000);
    let withholding_tax_cents = if fx_orig_per_eur_micro > 0 && quellenst_orig_micro > 0 {
        // quellenst_orig_micro (micro orig-currency)
        // ÷ fx_orig_per_eur_micro (micro orig per EUR) = micro EUR
        // EUR-cents = micro_eur / 10_000
        let withholding_eur_micro = (quellenst_orig_micro as i128 * 1_000_000i128
            / fx_orig_per_eur_micro as i128) as i64;
        (withholding_eur_micro + 5_000) / 10_000   // rounding
    } else {
        0
    };

    Ok(RawTransaction {
        booking_date,
        amount_cents,
        currency: "EUR".to_string(),
        counterparty: Some("flatexDEGIRO".to_string()),
        purpose: Some(name.clone()),
        raw_ref,
        kind: Some("dividend".to_string()),
        trade: Some(RawTradeFields {
            isin,
            asset_class_raw: "STOCK".to_string(),
            name,
            side: "dividend".to_string(),
            shares_micro: 0,
            unit_price_micro: None,
            fee_cents: 0,
            kest_cents,
            withholding_tax_cents,
            fx_rate_micro: Some(fx_orig_per_eur_micro),
            fusion_group: None,
        }),
        counterparty_iban: None,
    })
}
/// Bekannte Mappings für Crypto-Pseudo-ISINs (kompatibel mit TR-Pseudo-ISINs).
fn known_crypto_isin(name_upper: &str) -> Option<&'static str> {
    match name_upper {
        "BITCOIN" | "BTC" => Some("XF000BTC0017"),
        "ETHEREUM" | "ETH" => Some("XF000ETH0017"),
        _ => None,
    }
}

/// Generiert eine deterministische synthetische ISIN für einen Crypto-Namen.
/// Format `XF` + 9 alphanum + 1 digit (matched ISIN-Pattern).
fn synth_crypto_isin(name_upper: &str) -> String {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(name_upper.as_bytes());
    let hex = format!("{hash:x}");   // 64 lowercase hex chars
    let upper = hex.to_uppercase();
    // 9 alphanum + 1 digit
    let mut core: String = upper.chars().take(9).collect();
    let last_digit_char = upper.chars().find(|c| c.is_ascii_digit()).unwrap_or('0');
    core.push(last_digit_char);
    format!("XF{core}")
}

/// Sucht in einem mehrzeiligen Text den ersten numerischen Token nach einem Label,
/// wobei der Wert auf einer anderen Zeile (oder nach `:`) stehen darf.
/// Gibt Cents zurück.
fn extract_amount_cents_multiline(block: &str, label: &str) -> Option<i64> {
    let idx = block.find(label)?;
    let tail = &block[idx + label.len()..];
    // Suche in den nächsten paar Tokens/Zeilen nach einer Zahl
    for line in tail.lines().take(6) {
        if let Some(val) = parse_de_decimal_to_cents(line) {
            return Some(val);
        }
    }
    None
}

/// Wie `extract_amount_cents_multiline` aber gibt Mikrowert zurück.
fn extract_micro_multiline(block: &str, label: &str) -> Option<i64> {
    let idx = block.find(label)?;
    let tail = &block[idx + label.len()..];
    for line in tail.lines().take(6) {
        if let Some(val) = parse_de_decimal_to_micro(line) {
            return Some(val);
        }
    }
    None
}

/// Wie `extract_date_after` aber sucht auch in den nächsten paar Zeilen.
fn extract_date_multiline(block: &str, label: &str) -> Option<NaiveDate> {
    let idx = block.find(label)?;
    let tail = &block[idx + label.len()..];
    for line in tail.lines().take(4) {
        if let Some(d) = extract_date_after(line, "") {
            return Some(d);
        }
        // Versuche direkt in der Zeile
        let trimmed = line.trim().trim_start_matches(':').trim();
        if trimmed.len() >= 10 {
            if let Ok(d) = NaiveDate::parse_from_str(&trimmed[..10], "%d.%m.%Y") {
                return Some(d);
            }
        }
    }
    None
}

fn parse_krypto_sammelabrechnung(text: &str) -> ImportResult<Vec<RawTransaction>> {
    // Im pdf-extract-Output liegt die Nr.-Zeile als ein langer String mit Spaces:
    // "Nr.327516865/1    Kauf                           BITCOIN"
    // Wir suchen alle solchen Zeilen und segmentieren den Text in Blöcke.
    let lines: Vec<&str> = text.lines().collect();
    let mut position_starts: Vec<usize> = Vec::new();  // Zeilennummern der "Nr."-Zeilen

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        // Match "Nr.digits/digits..." (Order-Referenz-Zeile)
        if !trimmed.starts_with("Nr.") { continue; }
        let after_nr = &trimmed[3..];
        // Ersten Token (bis Whitespace) extrahieren und prüfen
        let first_token = after_nr.split_whitespace().next().unwrap_or("");
        if let Some(slash_pos) = first_token.find('/') {
            let before_slash = &first_token[..slash_pos];
            let after_slash = &first_token[slash_pos+1..];
            // Prüfe: NNNN/D — Zahlen vor und nach Slash
            if before_slash.chars().all(|c| c.is_ascii_digit())
                && !before_slash.is_empty()
                && after_slash.chars().next().map_or(false, |c| c.is_ascii_digit())
            {
                // Prüfe: Zeile enthält "Kauf" oder "Verkauf" (es ist eine Positions-Zeile)
                let tokens: Vec<&str> = trimmed.split_whitespace().collect();
                if tokens.iter().any(|t| *t == "Kauf" || *t == "Verkauf") {
                    position_starts.push(i);
                }
            }
        }
    }

    if position_starts.is_empty() {
        return Err(ImportError::Parse(
            "Krypto-Sammelabrechnung: keine Positionen erkannt (Nr.X/Y nicht gefunden)".into()
        ));
    }

    let mut out = Vec::new();

    for (pos_idx, &start_line) in position_starts.iter().enumerate() {
        let end_line = position_starts.get(pos_idx + 1).copied().unwrap_or(lines.len());
        let block_lines = &lines[start_line..end_line];
        let block = block_lines.join("\n");

        // Tokens aus der Nr.-Zeile parsen: "Nr.327516865/1    Kauf    BITCOIN"
        let nr_line = lines[start_line].trim();
        let tokens: Vec<&str> = nr_line.split_whitespace().collect();

        // Token[0] = "Nr.327516865/1" → raw_ref = "327516865/1"
        let raw_ref = tokens[0].trim_start_matches("Nr.").to_string();
        let raw_ref_display = format!("Nr.{}", raw_ref);

        // Side: "Kauf" oder "Verkauf" in den Tokens
        let side = tokens.iter()
            .find_map(|t| {
                if *t == "Kauf" { Some("buy") }
                else if *t == "Verkauf" { Some("sell") }
                else { None }
            })
            .ok_or_else(|| ImportError::Parse(
                format!("Krypto-Beleg ({raw_ref_display}): Kauf/Verkauf nicht erkannt")
            ))?;

        // Asset-Name: alle Tokens nach "Kauf"/"Verkauf" die nur Großbuchstaben sind
        let side_word = if side == "buy" { "Kauf" } else { "Verkauf" };
        let name_upper = {
            let side_idx = tokens.iter().position(|t| *t == side_word).unwrap();
            let name_tokens: Vec<&str> = tokens[side_idx+1..].iter()
                .take_while(|t| t.chars().all(|c| c.is_ascii_uppercase() || c == '-'))
                .copied()
                .collect();

            if !name_tokens.is_empty() {
                name_tokens.join(" ")
            } else {
                // Fallback: suche in den Blockzeilen nach einer All-Caps-Zeile
                block_lines.iter()
                    .skip(1)
                    .take(20)
                    .find_map(|l| {
                        let t = l.trim();
                        if t.is_empty() || t == "Kauf" || t == "Verkauf" { return None; }
                        if t.len() >= 2
                            && t.chars().all(|c| c.is_ascii_uppercase() || c == ' ' || c == '-')
                            && t.chars().any(|c| c.is_ascii_uppercase())
                        {
                            Some(t.to_string())
                        } else {
                            None
                        }
                    })
                    .ok_or_else(|| ImportError::Parse(
                        format!("Krypto-Beleg ({raw_ref_display}): Asset-Name nicht gefunden")
                    ))?
            }
        };

        // Datum: "Schlusstag" oder "Valuta"
        let booking_date = extract_date_multiline(&block, "Schlusstag")
            .or_else(|| extract_date_multiline(&block, "Valuta"))
            .ok_or_else(|| ImportError::Parse(
                format!("Krypto-Beleg ({raw_ref_display}): Datum nicht gefunden")
            ))?;

        // Endbetrag: letzter numerischer Wert vor "Transaktion-Nr." oder Ende
        let amount_cents = extract_amount_cents_multiline(&block, "Endbetrag")
            .ok_or_else(|| ImportError::Parse(
                format!("Krypto-Beleg ({raw_ref_display}): Endbetrag nicht gefunden")
            ))?;

        // Stücke: "davon ausgef." — auf gleicher Zeile
        let shares_micro_abs = extract_micro_after(&block, "davon ausgef.")
            .or_else(|| extract_micro_after(&block, "davon ausgef"))
            .or_else(|| extract_micro_multiline(&block, "Ordervolumen"))
            .ok_or_else(|| ImportError::Parse(
                format!("Krypto-Beleg ({raw_ref_display}): Stücke nicht gefunden")
            ))?;
        let shares_micro = if side == "sell" {
            -shares_micro_abs.abs()
        } else {
            shares_micro_abs
        };

        // Kurs: auf der nächsten Zeile nach "Kurs\n: value"
        let unit_price_micro = extract_micro_multiline(&block, "Kurs\n");

        // Provision: suche nach dem Wert unter "Provision"
        let provision = extract_amount_cents_multiline(&block, "Provision")
            .unwrap_or(0).abs();

        // Einbeh. Steuer
        let kest_cents = extract_amount_cents_multiline(&block, "Einbeh. Steuer")
            .unwrap_or(0).abs();

        let isin = known_crypto_isin(&name_upper)
            .map(String::from)
            .unwrap_or_else(|| synth_crypto_isin(&name_upper));

        out.push(RawTransaction {
            booking_date,
            amount_cents,
            currency: "EUR".to_string(),
            counterparty: Some("flatexDEGIRO".to_string()),
            purpose: Some(name_upper.clone()),
            raw_ref: Some(raw_ref_display),
            kind: Some(side.to_string()),
            trade: Some(RawTradeFields {
                isin,
                asset_class_raw: "CRYPTO".to_string(),
                name: name_upper,
                side: side.to_string(),
                shares_micro,
                unit_price_micro,
                fee_cents: provision,
                kest_cents,
                withholding_tax_cents: 0,
                fx_rate_micro: None,
                fusion_group: None,
            }),
            counterparty_iban: None,
        });
    }

    if out.is_empty() {
        return Err(ImportError::Parse(
            "Krypto-Sammelabrechnung: keine Positionen erkannt".into()
        ));
    }
    Ok(out)
}

/// Sammelabrechnung (Wertpapierkauf/-verkauf) — strukturell identisch zur
/// Krypto-Sammelabrechnung, aber pro Position eine echte ISIN in der `(ISIN/WKN)`-
/// Klammer und Fees aus Provision + Fremde Spesen + Eigene Spesen.
fn parse_wertpapier_sammelabrechnung(text: &str) -> ImportResult<Vec<RawTransaction>> {
    let lines: Vec<&str> = text.lines().collect();
    let mut position_starts: Vec<usize> = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if !trimmed.starts_with("Nr.") { continue; }
        let after_nr = &trimmed[3..];
        let first_token = after_nr.split_whitespace().next().unwrap_or("");
        if let Some(slash_pos) = first_token.find('/') {
            let before_slash = &first_token[..slash_pos];
            let after_slash = &first_token[slash_pos + 1..];
            if before_slash.chars().all(|c| c.is_ascii_digit())
                && !before_slash.is_empty()
                && after_slash.chars().next().map_or(false, |c| c.is_ascii_digit())
            {
                let tokens: Vec<&str> = trimmed.split_whitespace().collect();
                if tokens.iter().any(|t| *t == "Kauf" || *t == "Verkauf") {
                    position_starts.push(i);
                }
            }
        }
    }

    if position_starts.is_empty() {
        return Err(ImportError::Parse(
            "Wertpapier-Sammelabrechnung: keine Positionen erkannt".into()
        ));
    }

    let mut out = Vec::new();
    for (pos_idx, &start_line) in position_starts.iter().enumerate() {
        let end_line = position_starts.get(pos_idx + 1).copied().unwrap_or(lines.len());
        let block_lines = &lines[start_line..end_line];
        let block = block_lines.join("\n");

        let nr_line = lines[start_line].trim();
        let tokens: Vec<&str> = nr_line.split_whitespace().collect();
        let raw_ref = tokens[0].trim_start_matches("Nr.").to_string();
        let raw_ref_display = format!("Nr.{raw_ref}");

        let side = tokens.iter()
            .find_map(|t| match *t {
                "Kauf" => Some("buy"),
                "Verkauf" => Some("sell"),
                _ => None,
            })
            .ok_or_else(|| ImportError::Parse(
                format!("WP-Sammel ({raw_ref_display}): Kauf/Verkauf nicht erkannt")
            ))?;

        // Echte ISIN aus dem Block (Klammer-Pattern)
        let isin = extract_isin_from_parens(&block)
            .ok_or_else(|| ImportError::Parse(
                format!("WP-Sammel ({raw_ref_display}): ISIN nicht gefunden")
            ))?;

        // Asset-Name: Tokens zwischen "Kauf"/"Verkauf" und der `(`-Klammer in der Nr.-Zeile
        let side_word = if side == "buy" { "Kauf" } else { "Verkauf" };
        let name = {
            let nr_line_str = lines[start_line];
            let side_idx = nr_line_str.find(side_word).unwrap_or(0);
            let after_side = &nr_line_str[side_idx + side_word.len()..];
            let until_paren = after_side.split('(').next().unwrap_or("").trim();
            if until_paren.is_empty() {
                format!("(ISIN {isin})")
            } else {
                until_paren.to_string()
            }
        };

        // Datum
        let booking_date = extract_date_multiline(&block, "Schlusstag")
            .or_else(|| extract_date_multiline(&block, "Valuta"))
            .or_else(|| extract_date_multiline(&block, "Handelstag"))
            .ok_or_else(|| ImportError::Parse(
                format!("WP-Sammel ({raw_ref_display}): Datum nicht gefunden")
            ))?;

        let amount_cents = extract_amount_cents_multiline(&block, "Endbetrag")
            .ok_or_else(|| ImportError::Parse(
                format!("WP-Sammel ({raw_ref_display}): Endbetrag nicht gefunden")
            ))?;

        let shares_micro_abs = extract_micro_after(&block, "davon ausgef.")
            .or_else(|| extract_micro_after(&block, "davon ausgef"))
            .or_else(|| extract_micro_multiline(&block, "Ordervolumen"))
            .or_else(|| extract_micro_after(&block, "Ausgeführt"))
            .ok_or_else(|| ImportError::Parse(
                format!("WP-Sammel ({raw_ref_display}): Stücke nicht gefunden")
            ))?;
        let shares_micro = if side == "sell" {
            -shares_micro_abs.abs()
        } else {
            shares_micro_abs
        };

        let unit_price_micro = extract_micro_multiline(&block, "Kurs\n")
            .or_else(|| extract_micro_after_word(&block, "Kurs"));

        let provision = extract_amount_cents_multiline(&block, "Provision")
            .unwrap_or(0).abs();
        let fremde_spesen = extract_amount_cents_multiline(&block, "Fremde Spesen")
            .unwrap_or(0).abs();
        let eigene_spesen = extract_amount_cents_multiline(&block, "Eigene Spesen")
            .unwrap_or(0).abs();
        let kest_cents = extract_amount_cents_multiline(&block, "Einbeh. Steuer")
            .unwrap_or(0).abs();

        out.push(RawTransaction {
            booking_date,
            amount_cents,
            currency: "EUR".to_string(),
            counterparty: Some("flatexDEGIRO".to_string()),
            purpose: Some(name.clone()),
            raw_ref: Some(raw_ref_display),
            kind: Some(side.to_string()),
            trade: Some(RawTradeFields {
                isin,
                asset_class_raw: "FUND".to_string(),
                name,
                side: side.to_string(),
                shares_micro,
                unit_price_micro,
                fee_cents: provision + fremde_spesen + eigene_spesen,
                kest_cents,
                withholding_tax_cents: 0,
                fx_rate_micro: None,
                fusion_group: None,
            }),
            counterparty_iban: None,
        });
    }

    Ok(out)
}

/// Sammelabrechnung aus Sparplan — eine ISIN, mehrere Käufe in einer Tabelle.
/// Jede Tabellen-Zeile beginnt mit `Kauf` (oder `Verkauf`), gefolgt von
/// 2 Daten (Buchtag, Valuta), Stücke, Kurs, Betrag.
fn parse_sparplan_sammelabrechnung(text: &str) -> ImportResult<Vec<RawTransaction>> {
    let auftrag = extract_value_after(text, "Auftrags-Nr")
        .ok_or_else(|| ImportError::Parse("Sparplan: Auftrags-Nr nicht gefunden".into()))?;
    let isin = extract_value_after(text, "ISIN")
        .filter(|s| is_valid_isin(s))
        .ok_or_else(|| ImportError::Parse("Sparplan: ISIN nicht gefunden / ungültig".into()))?;
    let name = extract_value_line_after(text, "Bezeichnung")
        .unwrap_or_else(|| "(Sparplan)".to_string());

    let mut out = Vec::new();
    let mut idx: usize = 0;
    for line in text.lines() {
        let trimmed = line.trim();
        let tokens: Vec<&str> = trimmed.split_whitespace().collect();
        if tokens.len() < 6 { continue; }
        let side = match tokens[0] {
            "Kauf" => "buy",
            "Verkauf" => "sell",
            _ => continue,
        };
        // Token[1] = Buchtag dd.mm.yyyy, Token[2] = Valuta dd.mm.yyyy (ignore)
        let buchtag = tokens[1];
        if buchtag.len() != 10 || buchtag.as_bytes().get(2) != Some(&b'.') {
            continue;   // keine Daten-Zeile
        }
        let booking_date = match chrono::NaiveDate::parse_from_str(buchtag, "%d.%m.%Y") {
            Ok(d) => d,
            Err(_) => continue,
        };
        // Token[3] = Stücke (deutsch dezimal, kann Komma + 6 Nachkommastellen haben)
        // Token[4] = Kurs ("13,1977")
        // Token[5] = "EUR"
        // Token[6] = Betrag ("100,00")
        // Token[7] = "EUR"
        let shares_micro = parse_de_decimal_to_micro(tokens[3])
            .ok_or_else(|| ImportError::Parse(
                format!("Sparplan-Zeile {idx}: Stücke nicht parsbar: {:?}", tokens[3])
            ))?;
        let unit_price_micro = parse_de_decimal_to_micro(tokens[4])
            .ok_or_else(|| ImportError::Parse(
                format!("Sparplan-Zeile {idx}: Kurs nicht parsbar: {:?}", tokens[4])
            ))?;
        // Betrag suchen: Token nach "EUR" (Token 5) → Token 6
        let amount_abs = parse_de_decimal_to_cents(tokens.get(6).unwrap_or(&""))
            .ok_or_else(|| ImportError::Parse(
                format!("Sparplan-Zeile {idx}: Betrag nicht parsbar: {:?}", tokens.get(6))
            ))?;
        let amount_cents = if side == "buy" { -amount_abs } else { amount_abs };

        out.push(RawTransaction {
            booking_date,
            amount_cents,
            currency: "EUR".to_string(),
            counterparty: Some("flatexDEGIRO".to_string()),
            purpose: Some(name.clone()),
            // Eindeutiger raw_ref pro Position: Sparplan-Nr + Index
            raw_ref: Some(format!("SP{auftrag}-{idx}")),
            kind: Some(side.to_string()),
            trade: Some(RawTradeFields {
                isin: isin.clone(),
                asset_class_raw: "FUND".to_string(),
                name: name.clone(),
                side: side.to_string(),
                shares_micro: if side == "sell" { -shares_micro.abs() } else { shares_micro },
                unit_price_micro: Some(unit_price_micro),
                fee_cents: 0,
                kest_cents: 0,
                withholding_tax_cents: 0,
                fx_rate_micro: None,
                fusion_group: None,
            }),
            counterparty_iban: None,
        });
        idx += 1;
    }

    if out.is_empty() {
        return Err(ImportError::Parse(
            "Sparplan: keine Tabellen-Zeilen erkannt".into()
        ));
    }
    Ok(out)
}

/// Helper: extrahiert den restlichen Zeilen-Inhalt nach `Label : ` (alles bis Newline,
/// gestrippt). Nützlich für Felder wie `Bezeichnung : LYXOR CORE MSCI WORLD (DR`.
fn extract_value_line_after(text: &str, label: &str) -> Option<String> {
    let idx = text.find(label)?;
    let tail = &text[idx + label.len()..];
    let line = tail.split('\n').next()?;
    let value = line.trim().trim_start_matches(':').trim();
    if value.is_empty() { None } else { Some(value.to_string()) }
}

/// Fusionsbeleg: ein Wertpapier wird ausgebucht und ein anderes (Ziel-Fonds)
/// gleichzeitig eingebucht. Kein Cash-Flow.
///
/// Format pdf-extract:
///   WKN     ISIN          Wertpapierbezeichnung           Anzahl
///   LYX0YD  LU1781541179  AMUNDI MSCI WORLD V ETF         212,571745
///   ...
///   im Rahmen einer Fusion ... Valuta DD.MM.YYYY ... eingebucht:
///   WKN     ISIN          Wertpapierbezeichnung                             Anzahl
///   ETF146  IE000BI8OT95  AMUNDI MSCI WORLD ETF                      30,846924
///
/// Erzeugt 2 RawTransactions mit kind='corporate_action', amount_cents=0:
///   - Quelle: shares_micro negativ (Ausbuchung)
///   - Ziel:   shares_micro positiv (Einbuchung)
fn parse_fusion(text: &str) -> ImportResult<Vec<RawTransaction>> {
    // Datum: bevorzugt "Valuta DD.MM.YYYY" aus dem Fusions-Text, sonst Brief-Datum.
    let booking_date = extract_valuta_after_fusion(text)
        .or_else(|| extract_date_after(text, "Graz,"))
        .ok_or_else(|| ImportError::Parse("Fusion: Datum nicht gefunden".into()))?;

    let rows = extract_fusion_isin_rows(text);
    if rows.len() < 2 {
        return Err(ImportError::Parse(format!(
            "Fusion: erwartet ≥2 ISIN-Zeilen (Quelle + Ziel), gefunden {}", rows.len()
        )));
    }
    let (out_isin, out_name, out_shares_micro) = &rows[0];
    let (in_isin, in_name, in_shares_micro) = &rows[1];

    // Transaktion-Nr. eindeutig pro Tx machen
    let tx_nr = extract_value_after(text, "Transaktion-Nr.")
        .unwrap_or_else(|| "FUSION".to_string());

    // Pair-Identifier: identische Value für Quell- und Ziel-Row, damit der
    // FIFO-Solver die Cost-Basis von Quelle nach Ziel übertragen kann.
    let fusion_group = format!("FUSION-{tx_nr}");

    let make_tx = |isin: &str, name: &str, shares_micro: i64, suffix: &str, side: &str| RawTransaction {
        booking_date,
        amount_cents: 0,
        currency: "EUR".to_string(),
        // counterparty muss pro Tx unique sein (dedup-Index: account_id+date+amount+
        // counterparty+hash). Bei amount_cents=0 sind beide Fusion-Tx sonst identisch.
        counterparty: Some(format!("flatexDEGIRO · Fusion {isin}")),
        purpose: Some(format!("Fusion: {name}")),
        raw_ref: Some(format!("{fusion_group}-{suffix}")),
        kind: Some("corporate_action".to_string()),
        trade: Some(RawTradeFields {
            isin: isin.to_string(),
            asset_class_raw: "FUND".to_string(),
            name: name.to_string(),
            side: side.to_string(),
            shares_micro,
            unit_price_micro: None,
            fee_cents: 0,
            kest_cents: 0,
            withholding_tax_cents: 0,
            fx_rate_micro: None,
            fusion_group: Some(fusion_group.clone()),
        }),
        counterparty_iban: None,
    };

    Ok(vec![
        make_tx(out_isin, out_name, -out_shares_micro.abs(), "out", "fusion_out"),
        make_tx(in_isin,  in_name,   in_shares_micro.abs(),  "in",  "fusion_in"),
    ])
}

/// Liest "Valuta DD.MM.YYYY" aus dem Fusions-Beschreibungs-Text.
/// Suche im Tail nach "im Rahmen einer Fusion" das erste DD.MM.YYYY-Token.
fn extract_valuta_after_fusion(text: &str) -> Option<chrono::NaiveDate> {
    let idx = text.find("im Rahmen einer Fusion")?;
    let tail = &text[idx..];
    // Char-boundary-safe Iteration: nutze char_indices.
    for (start, _) in tail.char_indices() {
        if start + 10 > tail.len() { break; }
        if !tail.is_char_boundary(start + 10) { continue; }
        let candidate = &tail[start..start + 10];
        if candidate.len() != 10 { continue; }
        let b = candidate.as_bytes();
        if b.len() != 10 { continue; }
        if b[2] == b'.' && b[5] == b'.'
            && b[0..2].iter().all(|c| c.is_ascii_digit())
            && b[3..5].iter().all(|c| c.is_ascii_digit())
            && b[6..10].iter().all(|c| c.is_ascii_digit())
        {
            if let Ok(d) = chrono::NaiveDate::parse_from_str(candidate, "%d.%m.%Y") {
                return Some(d);
            }
        }
    }
    None
}

/// Findet alle Zeilen im Format `WKN ISIN NAME ANZAHL` (Whitespace-getrennt).
/// Returnt Tupel (isin, name, shares_micro). Reihenfolge wie im Text.
fn extract_fusion_isin_rows(text: &str) -> Vec<(String, String, i64)> {
    let mut out = Vec::new();
    for line in text.lines() {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.len() < 4 { continue; }
        // Token[1] muss ISIN-Pattern matchen
        if !is_valid_isin(tokens[1]) { continue; }
        // Letztes Token muss eine deutsche Dezimalzahl sein
        let last = tokens[tokens.len() - 1];
        let shares = match parse_de_decimal_to_micro(last) {
            Some(v) => v,
            None => continue,
        };
        // Name = alles zwischen ISIN und letzter Spalte
        let name = tokens[2..tokens.len() - 1].join(" ");
        out.push((tokens[1].to_string(), name, shares));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_beleg_typ_recognizes_kauf() {
        assert_eq!(
            detect_beleg_typ("... Wertpapierabrechnung Kauf Fonds/Zertifikate ...").unwrap(),
            BelegTyp::KaufFondsZert
        );
    }

    #[test]
    fn detect_beleg_typ_recognizes_thesaurierung() {
        assert_eq!(
            detect_beleg_typ("Ertragsmitteilung - thesaurierender transparenter Fonds").unwrap(),
            BelegTyp::Fondsthesaurierung
        );
    }

    #[test]
    fn detect_beleg_typ_recognizes_dividende() {
        assert_eq!(
            detect_beleg_typ("Dividendengutschrift für ausländische Wertpapiere").unwrap(),
            BelegTyp::DividendeAuslaendisch
        );
    }

    #[test]
    fn detect_beleg_typ_recognizes_krypto() {
        assert_eq!(
            detect_beleg_typ("Sammelabrechnung (Kauf/-verkauf Kryptowerte)").unwrap(),
            BelegTyp::SammelabrechnungKrypto
        );
    }

    #[test]
    fn detect_beleg_typ_recognizes_kontoauszug() {
        assert_eq!(
            detect_beleg_typ("Kontoauszug Nr: 001/2025 ...").unwrap(),
            BelegTyp::Kontoauszug
        );
    }

    #[test]
    fn detect_beleg_typ_recognizes_steuerreport() {
        assert_eq!(
            detect_beleg_typ("STEUERREPORT 01.10.2025 - 31.12.2025").unwrap(),
            BelegTyp::Steuerreport
        );
    }

    #[test]
    fn detect_beleg_typ_unknown_errors() {
        let err = detect_beleg_typ("Random Text").unwrap_err();
        assert!(err.to_string().contains("unbekannt"));
    }

    #[test]
    fn extract_isin_from_parens_finds_isin() {
        let text = "Nr.326003142/1 Kauf ISHARES MSCI WORLD SMALL (IE00BF4RFH31/A2DWBY)";
        assert_eq!(extract_isin_from_parens(text), Some("IE00BF4RFH31".to_string()));
    }

    #[test]
    fn extract_isin_from_parens_returns_none_when_no_parens() {
        assert_eq!(extract_isin_from_parens("no parens here"), None);
    }

    #[test]
    fn extract_isin_from_parens_ignores_invalid_pattern() {
        // Klammer ohne ISIN-konformes Pattern
        assert_eq!(extract_isin_from_parens("Foo (xyz/abc)"), None);
    }

    #[test]
    fn extract_isin_from_parens_handles_nested_parens() {
        // Alter Flatex-Beleg: Name enthält "(DR" als Distributor-Suffix ohne schließende Klammer,
        // dann kommt die echte ISIN-Klammer.
        let text = "LYXOR CORE MSCI WORLD (DR (LU1781541179/LYX0YD)";
        assert_eq!(extract_isin_from_parens(text), Some("LU1781541179".to_string()));
    }

    #[test]
    fn extract_isin_from_parens_handles_multiple_picks_first() {
        let text = "(IE00BF4RFH31/A2DWBY) ... (CNE100006WS8/A418NB)";
        assert_eq!(extract_isin_from_parens(text), Some("IE00BF4RFH31".to_string()));
    }

    #[test]
    fn extract_date_after_finds_label() {
        let text = "Handelstag                  11.09.2025\nValuta              15.09.2025";
        let d = extract_date_after(text, "Handelstag").unwrap();
        assert_eq!(d.to_string(), "2025-09-11");
    }

    #[test]
    fn extract_date_after_finds_label_with_dot_separator() {
        let text = "Schlusstag    : 25.10.2025, 21:45 Uhr";
        let d = extract_date_after(text, "Schlusstag").unwrap();
        assert_eq!(d.to_string(), "2025-10-25");
    }

    #[test]
    fn extract_date_after_returns_none_when_missing() {
        assert!(extract_date_after("nothing", "Foo").is_none());
    }

    #[test]
    fn extract_amount_cents_after_parses_negative() {
        let text = "Endbetrag                          :                            -395,08 EUR";
        assert_eq!(extract_amount_cents_after(text, "Endbetrag"), Some(-39508));
    }

    #[test]
    fn extract_amount_cents_after_parses_thousands_separator() {
        let text = "Endbetrag         :       -1.757,87 EUR";
        assert_eq!(extract_amount_cents_after(text, "Endbetrag"), Some(-175787));
    }

    #[test]
    fn extract_amount_cents_after_parses_positive() {
        let text = "Endbetrag         :        1,25 EUR";
        assert_eq!(extract_amount_cents_after(text, "Endbetrag"), Some(125));
    }

    #[test]
    fn extract_amount_cents_after_zero() {
        let text = "Einbeh. KESt                     :                                0,00 EUR";
        assert_eq!(extract_amount_cents_after(text, "Einbeh. KESt"), Some(0));
    }

    #[test]
    fn extract_amount_cents_after_returns_none_when_label_missing() {
        assert!(extract_amount_cents_after("nothing", "Foo").is_none());
    }

    #[test]
    fn extract_micro_after_parses_decimal() {
        let text = "Kurs           :                        7,436000 EUR";
        assert_eq!(extract_micro_after(text, "Kurs"), Some(7_436_000));
    }

    #[test]
    fn extract_micro_after_parses_integer_stueck() {
        let text = "Ausgeführt     :                              52 St.";
        assert_eq!(extract_micro_after(text, "Ausgeführt"), Some(52_000_000));
    }

    #[test]
    fn extract_micro_after_parses_thousands() {
        let text = "Kurs           :                        96.057,4900 EUR";
        assert_eq!(extract_micro_after(text, "Kurs"), Some(96_057_490_000));
    }
}
