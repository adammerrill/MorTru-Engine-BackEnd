//! `IngestionError` — errors that arise while reading rate sheets,
//! MISMO XML files, and RESO property feeds into engine-internal types.

use thiserror::Error;

/// Errors raised during rate-sheet and reference-data ingestion.
///
/// These errors are produced by the `ingest` crate and by the MISMO/RESO
/// parsing layers; they surface at the operator/admin level, not the
/// borrower level.
///
/// # I/O errors
///
/// The [`Self::Io`] variant wraps [`std::io::Error`] via `#[from]`, so any
/// function that opens files or network streams can use `?` without an
/// explicit conversion:
///
/// ```ignore
/// fn load(path: &Path) -> Result<RateSheet, IngestionError> {
///     let bytes = std::fs::read(path)?;   // std::io::Error → IngestionError
///     parse_rate_sheet(&bytes)
/// }
/// ```
#[derive(Debug, Error)]
pub enum IngestionError {
    /// A column or cell in the rate sheet had an unexpected type or format.
    /// `file` is the sheet filename, `expected` is the schema we needed,
    /// `found` is what was actually in the cell.
    #[error("schema mismatch in {file}: expected {expected}, found {found}")]
    SchemaMismatch {
        file: String,
        expected: String,
        found: String,
    },

    /// A rate-sheet cell block could not be mapped to any known rate-table
    /// header pattern. `row` and `col` are 0-based. `hint` is a best-guess
    /// about what the block might be, for human triage.
    #[error(
        "rate sheet block not recognized at row {row}, col {col}: {hint}"
    )]
    UnrecognizedBlock {
        row: usize,
        col: usize,
        hint: String,
    },

    /// A FICO band string was present but could not be parsed to a unique
    /// lower/upper bound pair (e.g. `"680+"` vs `"≥680"` with overlapping
    /// rows). The `input` field contains the raw cell text.
    #[error("ambiguous FICO band string: {input}")]
    AmbiguousFicoBand { input: String },

    /// An LTV band string could not be parsed to a lower/upper percentage
    /// bound. This typically means the sheet uses a non-standard separator
    /// (e.g. `"80.01-85"` when the parser expects `"80.01–85.00"`).
    #[error("malformed LTV band: {input}")]
    MalformedLtvBand { input: String },

    /// An Excel serial date fell outside the plausible range of dates this
    /// engine would ever encounter. Valid serial dates are roughly
    /// `1.0` (1900-01-01) to `109_574.0` (2199-12-31). Values outside this
    /// range — including negative numbers, NaN, and infinity — indicate a
    /// corrupted or mis-mapped cell.
    #[error("excel serial date {serial} out of plausible range")]
    InvalidExcelDate { serial: f64 },

    /// The MISMO 3.4 XML failed schema validation. `0` is the validation
    /// error message produced by the XML parser.
    #[error("MISMO XML validation failed: {0}")]
    MismoValidation(String),

    /// A RESO 2.0 field contained a value not in the field's permitted
    /// lookup set. `field` is the RESO standard field name (e.g.
    /// `"PropertyType"`); `value` is the offending raw value.
    #[error("RESO field {field} has invalid lookup value {value}")]
    InvalidResoLookup { field: String, value: String },

    /// A filesystem or network I/O error occurred while reading the source
    /// data. The [`std::io::Error`] is preserved as the error `source` for
    /// complete diagnostic chains.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ingestion_error_schema_mismatch_display() {
        let err = IngestionError::SchemaMismatch {
            file: "uwm_rate_sheet.xlsx".to_string(),
            expected: "percentage".to_string(),
            found: "abc123".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("uwm_rate_sheet.xlsx"), "{msg}");
        assert!(msg.contains("percentage"), "{msg}");
        assert!(msg.contains("abc123"), "{msg}");
    }

    #[test]
    fn test_ingestion_error_unrecognized_block_display() {
        let err = IngestionError::UnrecognizedBlock {
            row: 4,
            col: 2,
            hint: "looks like LLPA header".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains('4'), "{msg}");
        assert!(msg.contains('2'), "{msg}");
        assert!(msg.contains("looks like LLPA header"), "{msg}");
    }

    #[test]
    fn test_ingestion_error_ambiguous_fico_band_display() {
        let err = IngestionError::AmbiguousFicoBand {
            input: "680+".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("680+"), "{msg}");
    }

    #[test]
    fn test_ingestion_error_malformed_ltv_band_display() {
        let err = IngestionError::MalformedLtvBand {
            input: "80.01~85".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("80.01~85"), "{msg}");
    }

    #[test]
    fn test_ingestion_error_invalid_excel_date_display() {
        let err = IngestionError::InvalidExcelDate { serial: -1.0 };
        let msg = err.to_string();
        assert!(msg.contains("-1"), "{msg}");
    }

    #[test]
    fn test_ingestion_error_mismo_validation_display() {
        let err = IngestionError::MismoValidation(
            "element LoanAmount missing required attribute".to_string(),
        );
        let msg = err.to_string();
        assert!(msg.contains("LoanAmount"), "{msg}");
    }

    #[test]
    fn test_ingestion_error_invalid_reso_lookup_display() {
        let err = IngestionError::InvalidResoLookup {
            field: "PropertyType".to_string(),
            value: "Spaceship".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("PropertyType"), "{msg}");
        assert!(msg.contains("Spaceship"), "{msg}");
    }

    #[test]
    fn test_ingestion_error_io_display() {
        let io_err = std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "rate_sheet.xlsx not found",
        );
        let err = IngestionError::Io(io_err);
        let msg = err.to_string();
        assert!(msg.contains("io error"), "{msg}");
    }
}
