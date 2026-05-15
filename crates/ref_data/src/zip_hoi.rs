//! ZIP-level homeowner's insurance premium lookup.
//!
//! Provides a more precise HOI rate than the state average from
//! [`crate::hoi_rates::StateHoiRate`] for areas where ZIP-level data
//! is available. Currently covers Texas (TDOI data).
//!
//! # Lookup fallthrough
//!
//! The analysis engine queries in this order:
//! 1. ZIP-level (`zip_hoi_rate`) — precise, TX only for now
//! 2. State average (`state_hoi_rate`) — all 50 states + DC
//! 3. National fallback (`NATIONAL_FALLBACK_RATE_BPS = 85`)
//!
//! Source: Texas Department of Insurance (TDOI) annual homeowners
//! insurance premium data, published by ZIP code.

use serde::{Deserialize, Serialize};
use types::Cents;

/// Annual HOI rate for one ZIP code.
///
/// `annual_rate_bps = 62` means 0.62% of property value per year.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZipHoiRate {
    /// 5-digit ZIP code.
    pub zip5: String,
    /// 2-letter state abbreviation (e.g. "TX").
    pub state_abbr: String,
    /// Annual premium expressed as basis points of property value.
    pub annual_rate_bps: u16,
    /// Median annual premium in dollars from source data.
    /// Used as a cross-check; the engine uses `annual_rate_bps` for estimates.
    pub median_annual_premium_cents: Option<Cents>,
    /// Number of policies in the TDOI sample for this ZIP.
    pub sample_size: Option<u32>,
    pub effective_year: u16,
}

impl ZipHoiRate {
    /// Estimate monthly HOI for a property value in Cents.
    /// Uses ceiling division — conservative.
    #[must_use]
    pub fn monthly_estimate(&self, property_value: Cents) -> Option<Cents> {
        if property_value.0 <= 0 {
            return None;
        }
        let annual = i128::from(property_value.0) * i128::from(self.annual_rate_bps) / 10_000;
        let monthly = (annual + 11) / 12;
        Some(Cents(monthly as i64))
    }
}
