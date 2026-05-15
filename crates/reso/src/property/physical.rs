//! Physical dimension, construction, and lot helpers — Categories 7–9.
//!
//! All methods operate on already-deserialized `PropertyReso` fields and
//! return typed Rust values. No I/O, no network, no allocations beyond
//! what the conversions require.
//!
//! # Unit conventions
//!
//! - All area values are returned as integer square feet (`u32`).
//! - `LivingArea` is stored as `Decimal` in the struct; truncated to `u32`
//!   here (sub-square-foot precision is irrelevant for underwriting).
//! - Lot sizes: the engine uses square feet internally. Acre inputs are
//!   converted using the exact factor 43,560 sq ft/acre via `Decimal`
//!   arithmetic to avoid floating-point drift.
//! - Year values are returned as `u16` — years fit trivially.
//!
//! # Year built validation
//!
//! `year_built()` validates the range 1800–2040. Values outside this window
//! indicate data entry errors in the MLS feed. `effective_year_built()` is
//! not validated — it may legitimately be equal to the current year for a
//! newly renovated property.

use rust_decimal::{prelude::ToPrimitive, Decimal};

use crate::{
    error::{ResoError, ResoResult},
    property::PropertyReso,
};

/// Earliest plausible year a US residential structure was built.
const MIN_YEAR_BUILT: u16 = 1800;
/// Latest plausible year for pre-publication data (new construction contracts).
const MAX_YEAR_BUILT: u16 = 2040;

impl PropertyReso {
    // ── Category 7: Physical Dimensions ──────────────────────────────────────

    /// Finished livable square footage (GLA) as an integer.
    ///
    /// Truncates any fractional part. Returns `None` if `LivingArea` is absent.
    pub fn living_area_sqft(&self) -> Option<u32> {
        self.living_area.as_ref().and_then(|d| d.to_u32())
    }

    /// Finished above-grade area in square feet.
    pub fn above_grade_sqft(&self) -> Option<u32> {
        self.above_grade_finished_area
            .as_ref()
            .and_then(|d| d.to_u32())
    }

    /// Finished below-grade (basement) area in square feet.
    pub fn below_grade_finished_sqft(&self) -> Option<u32> {
        self.below_grade_finished_area
            .as_ref()
            .and_then(|d| d.to_u32())
    }

    /// Total finished area — above grade + finished basement.
    ///
    /// Returns `None` if `LivingArea` is absent. Adds `BelowGradeFinishedArea`
    /// when present.
    pub fn total_area_sqft(&self) -> Option<u32> {
        let above = self.living_area_sqft()?;
        let below = self.below_grade_finished_sqft().unwrap_or(0);
        above.checked_add(below)
    }

    /// Number of stories, rounded to the nearest integer.
    ///
    /// RESO stores `StoriesTotal` as `Decimal` to allow "1.5" for split-levels.
    /// Returns `None` if absent.
    pub fn stories(&self) -> Option<u8> {
        self.stories_total.as_ref().and_then(|d| {
            // Round 1.5 → 2, not truncate → 1
            let rounded = d.round_dp(0);
            rounded.to_u8()
        })
    }

    // ── Category 8: Construction / Age ────────────────────────────────────────

    /// Year the structure was originally completed, validated.
    ///
    /// Accepts values in the range 1800–2040. Values outside this window
    /// indicate a data error in the MLS feed and return `Err(ParseError)`.
    /// Returns `Ok(None)` if the field is absent.
    pub fn year_built(&self) -> ResoResult<Option<u16>> {
        let Some(y) = self.year_built else {
            return Ok(None);
        };
        let y = y as u16;
        if y < MIN_YEAR_BUILT || y > MAX_YEAR_BUILT {
            return Err(ResoError::ParseError {
                field: "YearBuilt",
                detail: format!("{y} is outside the valid range {MIN_YEAR_BUILT}–{MAX_YEAR_BUILT}"),
            });
        }
        Ok(Some(y))
    }

    /// Effective year built — accounts for major renovations.
    ///
    /// Returns `YearBuiltEffective` when present, otherwise falls back to
    /// `YearBuilt`. Not range-validated (renovation years are always plausible).
    /// Returns `None` if both fields are absent.
    pub fn effective_year_built(&self) -> Option<u16> {
        self.year_built_effective
            .map(|y| y as u16)
            .or_else(|| self.year_built.map(|y| y as u16))
    }

    /// True if `NewConstructionYN` is explicitly `true`.
    ///
    /// Returns `false` when the field is absent (conservative — most listings
    /// are not new construction, and a missing field is not a positive signal).
    pub fn is_new_construction(&self) -> bool {
        self.new_construction_yn.unwrap_or(false)
    }

    /// True if the property shares a wall with another unit.
    ///
    /// Uses `PropertyAttachedYN`. Returns `false` when absent.
    pub fn is_attached(&self) -> bool {
        self.property_attached_yn.unwrap_or(false)
    }

    /// True if the foundation is a slab.
    ///
    /// Checks `FoundationDetails` collection for the string "Slab".
    pub fn is_on_slab(&self) -> bool {
        self.foundation_details
            .as_ref()
            .map(|v| v.iter().any(|s| s.eq_ignore_ascii_case("slab")))
            .unwrap_or(false)
    }

    // ── Category 9: Lot / Land ────────────────────────────────────────────────

    /// Lot size in square feet (integer).
    ///
    /// Priority: `LotSizeSquareFeet` → `LotSizeAcres × 43,560`.
    /// Returns `None` if neither field is present.
    pub fn lot_size_sqft(&self) -> Option<u32> {
        if let Some(sqft) = self.lot_size_square_feet.as_ref() {
            return sqft.to_u32();
        }
        // Convert from acres: 1 acre = 43,560 sq ft exactly
        if let Some(acres) = self.lot_size_acres.as_ref() {
            let sqft = acres * Decimal::from(43_560u32);
            return sqft.to_u32();
        }
        // Fall through to LotSizeArea with known units
        if let Some(area) = self.lot_size_area.as_ref() {
            let units = self.lot_size_units.as_deref().unwrap_or("SquareFeet");
            return match units {
                u if u.eq_ignore_ascii_case("SquareFeet") => area.to_u32(),
                u if u.eq_ignore_ascii_case("Acres") => (area * Decimal::from(43_560u32)).to_u32(),
                _ => None,
            };
        }
        None
    }

    /// Lot size in acres as a `Decimal`.
    ///
    /// Priority: `LotSizeAcres` → `LotSizeSquareFeet ÷ 43,560`.
    pub fn lot_size_acres(&self) -> Option<Decimal> {
        if let Some(a) = self.lot_size_acres {
            return Some(a);
        }
        if let Some(sqft) = self.lot_size_square_feet {
            let acres = sqft / Decimal::from(43_560u32);
            return Some(acres.round_dp(6));
        }
        None
    }

    /// True if `LandLeaseYN` is `true` — the land is leased, not owned.
    ///
    /// Land leases affect underwriting (most agencies have restrictions on
    /// properties with land leases less than the loan term + 5 years).
    pub fn has_land_lease(&self) -> bool {
        self.land_lease_yn.unwrap_or(false)
    }
}
