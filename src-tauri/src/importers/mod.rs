use chrono::NaiveDate;
use sha2::{Digest, Sha256};

pub mod csv_bank_statement;
pub mod trade_republic_csv;
pub mod flatex_pdf;
pub mod sparkasse_csv;

#[derive(Debug, Clone, PartialEq)]
pub struct RawTransaction {
    pub booking_date: NaiveDate,
    pub amount_cents: i64,
    pub currency: String,
    pub counterparty: Option<String>,
    pub purpose: Option<String>,
    pub raw_ref: Option<String>,
    /// Optional override for `transactions.kind`. `None` ⇒ caller determines
    /// `'income'`/`'expense'` from the sign of `amount_cents` (existing behaviour).
    pub kind: Option<String>,
    /// Trade data when the row represents a securities transaction.
    pub trade: Option<RawTradeFields>,
    /// IBAN of the counterparty (from TR-CSV column 20). `None` if unknown.
    pub counterparty_iban: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RawTradeFields {
    pub isin: String,
    /// Raw CSV value before mapping (e.g. "FUND"). Mapping happens at insert time.
    pub asset_class_raw: String,
    pub name: String,
    pub side: String,  // 'buy' | 'sell' | 'dividend' | 'corporate_action' | 'fusion_out' | 'fusion_in'
    pub shares_micro: i64,
    pub unit_price_micro: Option<i64>,
    pub fee_cents: i64,
    pub kest_cents: i64,
    pub withholding_tax_cents: i64,
    pub fx_rate_micro: Option<i64>,
    /// Pairing identifier for fusion trades (same value on source and
    /// destination row). `None` for all non-fusion sides.
    pub fusion_group: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("csv: {0}")]
    Csv(#[from] csv::Error),
    #[error("parse: {0}")]
    Parse(String),
}

pub type ImportResult<T> = Result<T, ImportError>;

/// Result of a parser run: the parsed rows plus non-fatal warnings
/// (e.g. missing optional columns). Errors for fatal conditions (such as
/// a missing required column or file decode failure) are still returned as
/// `ImportError`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ParseResult {
    pub raws: Vec<RawTransaction>,
    pub warnings: Vec<String>,
}

pub trait Importer {
    fn parse(&self, bytes: &[u8]) -> ImportResult<ParseResult>;
}

/// SHA-256 of the file contents as a hex string — stored as `source_file_hash`
/// in the `transactions` table and forms part of the dedup index.
pub fn source_file_hash(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(64);
    for b in digest {
        use std::fmt::Write;
        let _ = write!(out, "{b:02x}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_bytes_produce_same_hash() {
        let a = source_file_hash(b"hello world");
        let b = source_file_hash(b"hello world");
        assert_eq!(a, b);
    }

    #[test]
    fn different_bytes_produce_different_hash() {
        let a = source_file_hash(b"hello world");
        let b = source_file_hash(b"hello world!");
        assert_ne!(a, b);
    }

    #[test]
    fn hash_is_64_hex_chars() {
        let h = source_file_hash(b"anything");
        assert_eq!(h.len(), 64);
        assert!(h.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
    }
}
