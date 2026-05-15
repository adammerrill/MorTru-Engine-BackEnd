//! FCC Census Geocoder client — lat/lon → FIPS code.
//!
//! # API
//!
//! FCC Census Block Finder (public, no auth required):
//! ```text
//! GET https://geo.fcc.gov/api/census/block/find
//!     ?latitude={lat}&longitude={lon}&censusYear=2020&showall=false&format=json
//! ```
//!
//! # Response structure
//!
//! ```json
//! {
//!   "status": "OK",
//!   "County": { "FIPS": "48209", "name": "Hays" },
//!   "State":  { "FIPS": "48",    "code": "TX", "name": "Texas" },
//!   "Block":  { "FIPS": "482090109053009" }
//! }
//! ```
//!
//! Block.FIPS is 15 digits: state(2) + county(3) + tract(6) + block(4).
//! The first 11 digits are the census tract GEOID used by `ref_data`.
//!
//! # Usage
//!
//! In Epic 5 (geo enrichment pipeline) the `FccClient` is called with
//! coordinates from `PropertyReso::validated_coordinates()`. Task 3.7
//! delivers the response parsing and data types; the HTTP call is wired
//! into the async enrichment pipeline in Epic 5.
//!
//! # Testing
//!
//! All tests use `parse_fcc_response()` directly — no live HTTP calls.
//! The Kyle TX reference coordinates `(30.0394, -97.8772)` map to
//! FIPS `48209` (Hays County, TX).

use std::str::FromStr;

use serde::Deserialize;

use crate::error::{ResoError, ResoResult};

// ── Public types ──────────────────────────────────────────────────────────────

/// Result of a successful FCC FIPS resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FipsResolution {
    /// 5-digit county FIPS code (state 2 + county 3).
    pub fips_code: types::FipsCode,
    /// County name as returned by FCC (e.g. "Hays").
    pub county_name: String,
    /// State abbreviation (e.g. `StateCode::TX`).
    pub state_code: types::StateCode,
    /// 11-digit census tract GEOID (first 11 of Block.FIPS), if present.
    pub tract_geoid: Option<String>,
}

/// FCC geocoder client.
///
/// The base URL is configurable so tests can point to a mock server.
/// In production always use `FccClient::new()`.
#[derive(Debug)]
pub struct FccClient {
    base_url: String,
}

impl FccClient {
    /// Production FCC Census Block Finder URL.
    pub const PRODUCTION_URL: &'static str = "https://geo.fcc.gov/api/census/block/find";

    /// Create a client pointing at the production FCC endpoint.
    #[must_use]
    pub fn new() -> Self {
        Self {
            base_url: Self::PRODUCTION_URL.to_owned(),
        }
    }

    /// Create a client with a custom base URL (for integration tests or mocks).
    #[must_use]
    pub fn with_base_url(url: &str) -> Self {
        Self {
            base_url: url.to_owned(),
        }
    }

    /// Base URL this client is configured to use.
    #[must_use]
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Validate coordinates before issuing an API call.
    ///
    /// Returns `Err` for out-of-bounds WGS 84 values or null island `(0,0)`.
    pub fn validate_coordinates(lat: f64, lon: f64) -> ResoResult<()> {
        if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
            return Err(ResoError::InvalidCoordinate { lat, lon });
        }
        if lat == 0.0 && lon == 0.0 {
            return Err(ResoError::InvalidCoordinate { lat, lon });
        }
        Ok(())
    }

    /// Resolve `(lat, lon)` to a `FipsResolution`.
    ///
    /// **HTTP implementation is delivered in Epic 5** (geo enrichment pipeline),
    /// where the async runtime is established. This stub validates inputs and
    /// documents the intended contract; the body is replaced in Epic 5.
    ///
    /// In tests, use `parse_fcc_response()` directly with fixture JSON.
    #[allow(unused_variables)]
    pub fn resolve(&self, lat: f64, lon: f64) -> ResoResult<FipsResolution> {
        Self::validate_coordinates(lat, lon)?;
        // HTTP call — implemented in Epic 5
        Err(ResoError::FccApiError {
            message: "HTTP client not yet wired — use parse_fcc_response() in tests".into(),
        })
    }
}

impl Default for FccClient {
    fn default() -> Self {
        Self::new()
    }
}

// ── Response parsing ──────────────────────────────────────────────────────────

/// Parse a raw FCC Census Block Finder JSON response into `FipsResolution`.
///
/// This is the testable pure-function core of the FCC client — all tests
/// call this directly with fixture JSON rather than making live HTTP requests.
pub fn parse_fcc_response(json: &str) -> ResoResult<FipsResolution> {
    let body: FccResponseBody = serde_json::from_str(json).map_err(|e| ResoError::FccApiError {
        message: format!("invalid FCC JSON: {e}"),
    })?;

    if body.status.as_deref() != Some("OK") {
        return Err(ResoError::FccApiError {
            message: format!(
                "FCC returned status '{}'",
                body.status.as_deref().unwrap_or("(none)")
            ),
        });
    }

    let county = body.county.ok_or_else(|| ResoError::FccApiError {
        message: "FCC response missing County object".into(),
    })?;

    let state = body.state.ok_or_else(|| ResoError::FccApiError {
        message: "FCC response missing State object".into(),
    })?;

    // County.FIPS is the 5-digit county FIPS (e.g. "48209")
    let fips_code =
        types::FipsCode::from_str(&county.fips).map_err(|_| ResoError::FccApiError {
            message: format!("FCC returned invalid county FIPS '{}'", county.fips),
        })?;

    // State.code is the 2-letter abbreviation (e.g. "TX")
    let state_code =
        types::StateCode::from_str(&state.code).map_err(|_| ResoError::FccApiError {
            message: format!("FCC returned unknown state code '{}'", state.code),
        })?;

    // Block.FIPS is 15 digits; first 11 = census tract GEOID
    let tract_geoid = body.block.as_ref().and_then(|b| {
        let digits: String = b.fips.chars().filter(|c| c.is_ascii_digit()).collect();
        if digits.len() >= 11 {
            Some(digits[..11].to_owned())
        } else {
            None
        }
    });

    Ok(FipsResolution {
        fips_code,
        county_name: county.name,
        state_code,
        tract_geoid,
    })
}

// ── Serde shapes ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct FccResponseBody {
    #[serde(rename = "status")]
    status: Option<String>,
    #[serde(rename = "County")]
    county: Option<FccCounty>,
    #[serde(rename = "State")]
    state: Option<FccState>,
    #[serde(rename = "Block")]
    block: Option<FccBlock>,
}

#[derive(Debug, Deserialize)]
struct FccCounty {
    #[serde(rename = "FIPS")]
    fips: String,
    #[serde(rename = "name")]
    name: String,
}

#[derive(Debug, Deserialize)]
struct FccState {
    #[serde(rename = "code")]
    code: String,
}

#[derive(Debug, Deserialize)]
struct FccBlock {
    #[serde(rename = "FIPS")]
    fips: String,
}
