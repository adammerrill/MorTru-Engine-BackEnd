//! `RefDataError` — all error variants for the `ref_data` crate.

use thiserror::Error;

/// Unified error type for reference data operations.
#[derive(Debug, Error)]
pub enum RefDataError {
    /// No record found for this FIPS code and year combination.
    #[error("no {data_type} found for FIPS '{fips}' year {year}")]
    NotFound {
        data_type: &'static str,
        fips: String,
        year: u16,
    },

    /// No USDA income limit found for this FIPS and household size.
    #[error("no USDA income limit for FIPS '{fips}', size {household_size}")]
    UsdaIncomeLimitNotFound { fips: String, household_size: u8 },

    /// Household size out of USDA-supported range (1–8).
    #[error("household size {0} is outside the valid USDA range of 1–8")]
    InvalidHouseholdSize(u8),

    /// Census tract GEOID is not 11 digits.
    #[error("census tract GEOID '{0}' must be 11 digits (state2+county3+tract6)")]
    InvalidGeoid(String),

    /// Storage backend returned an error.
    #[error("storage error: {0}")]
    Storage(String),

    /// Data integrity violation — unexpected null or type mismatch.
    #[error("data integrity error in {table}: {detail}")]
    DataIntegrity { table: &'static str, detail: String },

    /// JSON deserialization from file store failed.
    #[error("JSON error in {file}: {source}")]
    Json {
        file: String,
        #[source]
        source: serde_json::Error,
    },

    /// Version manifest references a data version that no longer exists.
    #[error("version '{0}' referenced in manifest is not present in the store")]
    StaleManifest(String),
}

/// `Result<T, RefDataError>` — crate-wide result alias.
pub type RefDataResult<T> = Result<T, RefDataError>;
