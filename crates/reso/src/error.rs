//! `ResoError` — all error variants for the `reso` crate.

use thiserror::Error;

/// Unified error type for RESO parsing, validation, and enrichment.
#[derive(Debug, Error)]
pub enum ResoError {
    /// A required field was absent from the RESO payload.
    #[error("required RESO field '{field}' is missing")]
    MissingField { field: &'static str },

    /// A Lookup field contained a value not in the RESO 2.0 standard.
    #[error("unknown RESO lookup value '{value}' for field '{field}'")]
    InvalidLookup { field: &'static str, value: String },

    /// A numeric field contained a non-numeric string.
    #[error("field '{field}' is not a valid number: '{value}'")]
    InvalidNumeric { field: &'static str, value: String },

    /// Latitude/longitude coordinates are outside the valid range.
    #[error("coordinates ({lat}, {lon}) are not valid WGS 84 decimal degrees")]
    InvalidCoordinate { lat: f64, lon: f64 },

    /// FCC Census Geocoder API call failed.
    #[error("FCC geocoding API error: {message}")]
    FccApiError { message: String },

    /// General parse/conversion failure with field context.
    #[error("failed to parse field '{field}': {detail}")]
    ParseError {
        field: &'static str,
        detail: String,
    },

    /// PropertyType value not in the RESO 2.0 lookup table.
    #[error("unknown RESO PropertyType: '{value}'")]
    UnknownPropertyType { value: String },

    /// PropertySubType value not in the RESO 2.0 lookup table.
    #[error("unknown RESO PropertySubType: '{value}'")]
    UnknownPropertySubType { value: String },

    /// JSON deserialization error from the RESO Web API payload.
    #[error("RESO JSON deserialization error: {0}")]
    Json(#[from] serde_json::Error),
}

/// `Result<T, ResoError>` — crate-wide result alias.
pub type ResoResult<T> = Result<T, ResoError>;
