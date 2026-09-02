//! PAY by square encoding — the Slovak payment-QR standard (UC-ACC-05.8).
//!
//! Implements the by square PAY v1.2.0 container so SK banking apps can scan
//! an invoice and prefill the transfer (amount, IBAN, variable symbol — the
//! same keys EPIC-ACC-08.6 auto-match pairs on). The sibling SPAYD string
//! (`routes::invoices::hooks::payment_spayd` in accounting-server) covers the
//! CZ "QR platba" convention; this module covers PAY by square.
//!
//! Container format (verified against the reference implementation,
//! `github.com/xseman/bysquare`, Go + TypeScript):
//!
//! 1. Payment data serialized as TAB-separated fields (v1.2.0 field order,
//!    single payment-order payment; tabs inside values become spaces).
//! 2. CRC32 (IEEE) of the serialized string, 4 bytes little-endian,
//!    **prepended** to the payload bytes.
//! 3. LZMA1-compressed (lc=3, lp=0, pb=2 — props byte 0x5D); the 13-byte
//!    `.lzma` header is stripped, only the raw stream body is kept.
//! 4. Assembled: 2-byte bysquare header (type=0x0 PAY, version=0x2 (1.2.0),
//!    doc-type=0x0, reserved=0x0 — one nibble each) + 2-byte little-endian
//!    length of the checksummed payload + LZMA body.
//! 5. Base32Hex-encoded (RFC 4648 extended-hex alphabet, no padding).
//!
//! The decoder reverses the container (reconstructing the LZMA header with the
//! spec's 2^17 dictionary) — used by round-trip tests and validated against a
//! reference QR string produced by the upstream implementation.

use chrono::NaiveDate;
use data_encoding::BASE32HEX_NOPAD;
use rust_decimal::Decimal;

/// bysquare header: type=PAY(0x0), version=1.2.0(0x2), doctype=0x0, reserved=0x0.
const HEADER: [u8; 2] = [0x02, 0x00];
/// `.lzma` props byte for lc=3, lp=0, pb=2.
const LZMA_PROPS: u8 = 0x5D;
/// Spec dictionary size (2^17) for the reconstructed decode header.
const LZMA_DICT_SIZE: u32 = 0x0002_0000;

/// A single payment-order request (the only payment type ACC documents need).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PayBySquare {
    /// Free-text document reference (serialized `InvoiceID`).
    pub invoice_id: Option<String>,
    /// Amount due. Zero/negative amounts are rejected by [`encode`].
    pub amount: Decimal,
    /// ISO-4217 code, e.g. `EUR`.
    pub currency: String,
    pub due_date: Option<NaiveDate>,
    pub variable_symbol: Option<String>,
    pub constant_symbol: Option<String>,
    pub specific_symbol: Option<String>,
    /// Free-text note shown to the payer.
    pub note: Option<String>,
    pub iban: String,
    pub bic: Option<String>,
    /// Beneficiary (supplier) name — required by spec v1.2.0.
    pub beneficiary_name: String,
}

/// Encoding failure.
#[derive(Debug)]
pub enum BySquareError {
    /// Nothing is owed — the QR must be omitted (UC-ACC-05.8).
    NothingOwed,
    /// IBAN or beneficiary missing.
    MissingField(&'static str),
    /// LZMA / container failure (should not happen for well-formed input).
    Encoding(String),
}

impl std::fmt::Display for BySquareError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BySquareError::NothingOwed => write!(f, "amount due is zero or negative"),
            BySquareError::MissingField(field) => write!(f, "missing required field: {field}"),
            BySquareError::Encoding(e) => write!(f, "bysquare encoding failed: {e}"),
        }
    }
}

/// Tabs are the field separator — values may not contain them (spec §3.8).
fn sanitize(value: &str) -> String {
    value.replace('\t', " ")
}

/// Spec number format: decimal string with trailing zeros trimmed
/// (`100.50` → `100.5`, `100.00` → `100`).
fn format_amount(amount: Decimal) -> String {
    amount.normalize().to_string()
}

/// Serialize to the v1.2.0 TAB-joined field sequence (single payment order,
/// single bank account, no standing-order/direct-debit extensions).
fn serialize(data: &PayBySquare) -> String {
    let opt = |v: &Option<String>| sanitize(v.as_deref().unwrap_or(""));
    let parts: Vec<String> = vec![
        opt(&data.invoice_id),
        "1".to_string(), // payment count
        "1".to_string(), // type: payment order
        format_amount(data.amount),
        sanitize(&data.currency),
        data.due_date
            .map(|d| d.format("%Y%m%d").to_string())
            .unwrap_or_default(),
        opt(&data.variable_symbol),
        opt(&data.constant_symbol),
        opt(&data.specific_symbol),
        String::new(), // originator's reference (unused when symbols are set)
        opt(&data.note),
        "1".to_string(), // bank-account count
        sanitize(&data.iban),
        opt(&data.bic),
        "0".to_string(), // no standing-order extension
        "0".to_string(), // no direct-debit extension
        sanitize(&data.beneficiary_name),
        String::new(), // beneficiary street
        String::new(), // beneficiary city
    ];
    parts.join("\t")
}

/// Encode to the PAY by square QR string (the text a QR generator encodes).
pub fn encode(data: &PayBySquare) -> Result<String, BySquareError> {
    if data.amount <= Decimal::ZERO {
        return Err(BySquareError::NothingOwed);
    }
    if data.iban.trim().is_empty() {
        return Err(BySquareError::MissingField("iban"));
    }
    if data.beneficiary_name.trim().is_empty() {
        return Err(BySquareError::MissingField("beneficiary_name"));
    }

    let payload = serialize(data);

    // CRC32 (IEEE) little-endian, PREPENDED (spec §3.10).
    let crc = crc32fast::hash(payload.as_bytes());
    let mut checked = Vec::with_capacity(4 + payload.len());
    checked.extend_from_slice(&crc.to_le_bytes());
    checked.extend_from_slice(payload.as_bytes());

    // LZMA1-compress, then strip the 13-byte `.lzma` header — the container
    // carries the parameters implicitly (props 0x5D, dict 2^17) and the length
    // in its own 2-byte field.
    let mut compressed = Vec::new();
    lzma_rs::lzma_compress(&mut &checked[..], &mut compressed)
        .map_err(|e| BySquareError::Encoding(e.to_string()))?;
    if compressed.len() < 13 {
        return Err(BySquareError::Encoding(
            "compressed stream too short".into(),
        ));
    }
    let body = &compressed[13..];

    let mut out = Vec::with_capacity(4 + body.len());
    out.extend_from_slice(&HEADER);
    out.extend_from_slice(&(checked.len() as u16).to_le_bytes());
    out.extend_from_slice(body);

    Ok(BASE32HEX_NOPAD.encode(&out))
}

/// Decode a PAY by square QR string back to the raw TAB-separated fields.
/// Verifies the container CRC. Public mainly for tests and the expense-inbox
/// ingest path (scanning a supplier's QR — EPIC-ACC-07).
pub fn decode_fields(qr: &str) -> Result<Vec<String>, BySquareError> {
    let bin = BASE32HEX_NOPAD
        .decode(qr.trim().to_uppercase().as_bytes())
        .map_err(|e| BySquareError::Encoding(format!("base32hex: {e}")))?;
    if bin.len() < 5 {
        return Err(BySquareError::Encoding("container too short".into()));
    }
    let declared_len = u16::from_le_bytes([bin[2], bin[3]]) as usize;

    // Reconstruct the 13-byte `.lzma` header the encoder stripped.
    let mut stream = Vec::with_capacity(13 + bin.len() - 4);
    stream.push(LZMA_PROPS);
    stream.extend_from_slice(&LZMA_DICT_SIZE.to_le_bytes());
    stream.extend_from_slice(&(declared_len as u64).to_le_bytes());
    stream.extend_from_slice(&bin[4..]);

    let mut decompressed = Vec::new();
    lzma_rs::lzma_decompress(&mut &stream[..], &mut decompressed)
        .map_err(|e| BySquareError::Encoding(format!("lzma: {e}")))?;
    decompressed.truncate(declared_len);
    if decompressed.len() < 4 {
        return Err(BySquareError::Encoding("payload too short".into()));
    }

    let crc_stored = u32::from_le_bytes([
        decompressed[0],
        decompressed[1],
        decompressed[2],
        decompressed[3],
    ]);
    let payload = &decompressed[4..];
    if crc32fast::hash(payload) != crc_stored {
        return Err(BySquareError::Encoding("CRC mismatch".into()));
    }

    let text =
        String::from_utf8(payload.to_vec()).map_err(|e| BySquareError::Encoding(e.to_string()))?;
    Ok(text.split('\t').map(str::to_string).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn sample() -> PayBySquare {
        PayBySquare {
            invoice_id: Some("2026/001".into()),
            amount: dec!(123.45),
            currency: "EUR".into(),
            due_date: NaiveDate::from_ymd_opt(2026, 8, 15),
            variable_symbol: Some("2026001".into()),
            iban: "SK9611000000002918599669".into(),
            beneficiary_name: "ACME s.r.o.".into(),
            ..Default::default()
        }
    }

    #[test]
    fn serializes_v120_field_order() {
        let s = serialize(&sample());
        let fields: Vec<&str> = s.split('\t').collect();
        assert_eq!(fields[0], "2026/001"); // invoice id
        assert_eq!(fields[1], "1"); // payment count
        assert_eq!(fields[2], "1"); // payment order
        assert_eq!(fields[3], "123.45");
        assert_eq!(fields[4], "EUR");
        assert_eq!(fields[5], "20260815");
        assert_eq!(fields[6], "2026001"); // VS
        assert_eq!(fields[11], "1"); // bank account count
        assert_eq!(fields[12], "SK9611000000002918599669");
        assert_eq!(fields[14], "0"); // no standing order
        assert_eq!(fields[15], "0"); // no direct debit
        assert_eq!(fields[16], "ACME s.r.o."); // beneficiary name
        assert_eq!(fields.len(), 19);
    }

    #[test]
    fn amount_formatting_trims_trailing_zeros() {
        assert_eq!(format_amount(dec!(100.50)), "100.5");
        assert_eq!(format_amount(dec!(100.00)), "100");
        assert_eq!(format_amount(dec!(0.10)), "0.1");
    }

    #[test]
    fn tabs_in_values_become_spaces() {
        let mut data = sample();
        data.note = Some("line\tbreak".into());
        let s = serialize(&data);
        assert!(s.contains("line break"));
        assert_eq!(
            s.split('\t').count(),
            19,
            "tab injection must not add fields"
        );
    }

    #[test]
    fn encode_decode_round_trip() {
        let qr = encode(&sample()).expect("encode");
        // Base32hex alphabet only (0-9, A-V).
        assert!(qr
            .chars()
            .all(|c| c.is_ascii_digit() || ('A'..='V').contains(&c)));
        let fields = decode_fields(&qr).expect("decode");
        assert_eq!(fields[0], "2026/001");
        assert_eq!(fields[3], "123.45");
        assert_eq!(fields[12], "SK9611000000002918599669");
        assert_eq!(fields[16], "ACME s.r.o.");
    }

    /// Reference vector produced by the upstream implementation
    /// (github.com/xseman/bysquare, pay/decode_test.go): our decoder must read
    /// containers produced by other spec-compliant encoders.
    #[test]
    fn decodes_reference_vector_from_upstream_impl() {
        let qr = "0804Q000AEM958SPQK31JJFA00H0OBFGMH6PKV0OQSNQPQK5K2BATU8DV6PA0G2P9U05QCF640MRVMTLLI3OJ8CEGOUEP5GR3LIJ4C0A8ERUI3JHM3VTNG00";
        let fields = decode_fields(qr).expect("decode reference vector");
        assert_eq!(fields[0], "random-id");
    }

    #[test]
    fn zero_amount_is_rejected() {
        let mut data = sample();
        data.amount = Decimal::ZERO;
        assert!(matches!(encode(&data), Err(BySquareError::NothingOwed)));
    }

    #[test]
    fn missing_iban_and_beneficiary_are_rejected() {
        let mut data = sample();
        data.iban = "  ".into();
        assert!(matches!(
            encode(&data),
            Err(BySquareError::MissingField("iban"))
        ));
        let mut data = sample();
        data.beneficiary_name = String::new();
        assert!(matches!(
            encode(&data),
            Err(BySquareError::MissingField("beneficiary_name"))
        ));
    }
}
