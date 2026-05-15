//! Homeowner's Insurance (HOI) estimation rates by state.
//!
//! RESO property data never includes the HOI premium — MLS feeds do not
//! carry insurance cost data. To compute a complete PITIA, the engine
//! estimates HOI using the state average annual premium expressed as a
//! percentage of property value.
//!
//! # Formula
//!
//! `monthly_hoi = purchase_price × annual_rate_bps / 10_000 / 12`
//!
//! The `annual_rate_bps` is stored in basis points (1/100 of 1%).
//! e.g. Texas = 56 bps = 0.56% per year.
//!
//! # Source
//!
//! Rate estimates based on NAIC homeowners insurance data and state
//! insurance department filings. Updated annually.
//!
//! # Precision note
//!
//! HOI estimates are intentionally conservative (rounded up). The engine
//! sets `hoi_estimated = true` in PitiaBreakdown whenever this estimator
//! is used, prompting the borrower to obtain a real quote.

use serde::{Deserialize, Serialize};
use types::Cents;

/// Annual HOI rate for one state, expressed as basis points of property value.
///
/// `annual_rate_bps = 56` means 0.56% per year.
/// A $459,000 property in Texas: `$459,000 × 0.0056 / 12 = $214/mo`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateHoiRate {
    /// 2-letter state abbreviation (e.g. "TX").
    pub state_abbr: String,
    /// Annual rate in basis points of property value (1 bps = 0.01%).
    pub annual_rate_bps: u16,
    pub effective_year: u16,
}

impl StateHoiRate {
    /// Estimate monthly HOI for a property value in Cents.
    ///
    /// Returns `None` if `property_value` is zero.
    #[must_use]
    pub fn monthly_estimate(&self, property_value: Cents) -> Option<Cents> {
        if property_value.0 <= 0 {
            return None;
        }
        // monthly = value × rate_bps / 10_000 / 12
        // Work in i128 to avoid overflow on large values
        let annual = i128::from(property_value.0) * i128::from(self.annual_rate_bps) / 10_000;
        // Ceiling divide by 12 — conservative estimate protects borrower
        let monthly = (annual + 11) / 12;
        Some(Cents(monthly as i64))
    }
}

/// National fallback rate used when no state-specific rate is available.
/// 85 bps (0.85%) is the approximate national average.
pub const NATIONAL_FALLBACK_RATE_BPS: u16 = 85;
