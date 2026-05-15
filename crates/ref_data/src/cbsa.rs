//! CBSA (Core Based Statistical Area) crosswalk — county FIPS → MSA classification.
//!
//! Maps a 5-digit county FIPS code to its CBSA code and designation
//! (Metropolitan, Micropolitan, or Rural). Used for:
//!
//! - USDA rural determination near MSA boundaries (Micropolitan counties
//!   often have rural tracts even when adjacent to a large metro)
//! - MSA-level income limit lookups
//! - Analytics and reporting
//!
//! Source: Census Bureau OMB Metropolitan and Micropolitan Statistical
//! Area delineation files. Updated every 3–5 years.

use serde::{Deserialize, Serialize};

/// Metropolitan / Micropolitan / Rural classification per OMB.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CbsaDesignation {
    /// Metropolitan Statistical Area — principal city ≥ 50,000 population.
    Metropolitan,
    /// Micropolitan Statistical Area — urban cluster of 10,000–49,999.
    Micropolitan,
    /// No CBSA — county is outside any metropolitan or micropolitan area.
    Rural,
}

impl CbsaDesignation {
    /// True for Metropolitan areas only.
    #[must_use]
    pub const fn is_metro(self) -> bool {
        matches!(self, Self::Metropolitan)
    }

    /// True for any urban designation (Metro or Micro).
    #[must_use]
    pub const fn is_urban(self) -> bool {
        matches!(self, Self::Metropolitan | Self::Micropolitan)
    }
}

/// CBSA assignment for one county.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CbsaEntry {
    /// 5-digit county FIPS code.
    pub fips_code: String,
    /// 5-digit CBSA code. `None` for rural counties (no CBSA assigned).
    pub cbsa_code: Option<String>,
    /// Human-readable CBSA name (e.g. "Austin-Round Rock-Georgetown, TX").
    pub cbsa_name: Option<String>,
    /// OMB designation for this county.
    pub designation: CbsaDesignation,
    /// True for Metropolitan counties only. Convenience alias.
    pub is_metro: bool,
}

impl CbsaEntry {
    /// True if this county is Micropolitan but not Metropolitan.
    /// Relevant for USDA: Micropolitan counties can still have rural tracts.
    #[must_use]
    pub fn is_micro(&self) -> bool {
        self.designation == CbsaDesignation::Micropolitan
    }

    /// True if this county has no CBSA at all.
    #[must_use]
    pub fn is_rural(&self) -> bool {
        self.designation == CbsaDesignation::Rural
    }
}
