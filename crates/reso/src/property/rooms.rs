//! Room, parking, and systems helpers — Categories 10–12.
//!
//! # Collection field searches
//!
//! RESO `Collection` type fields (Heating, Cooling, Sewer, WaterSource, etc.)
//! arrive as `Vec<String>`. All `contains_*` predicates do case-insensitive
//! substring matching so that minor spelling variations between MLS boards
//! ("CentralAir", "Central Air", "Central A/C") all resolve correctly.

use rust_decimal::prelude::ToPrimitive;

use crate::property::PropertyReso;

/// Returns `true` if `haystack` contains `needle` (case-insensitive substring).
fn contains_ci(haystack: &[String], needle: &str) -> bool {
    let needle_lower = needle.to_ascii_lowercase();
    haystack
        .iter()
        .any(|s| s.to_ascii_lowercase().contains(&needle_lower))
}

impl PropertyReso {
    // ── Category 10: Rooms ────────────────────────────────────────────────────

    /// Total bedrooms as `u8`.
    pub fn bedrooms(&self) -> Option<u8> {
        self.bedrooms_total.and_then(|n| {
            if n >= 0 && n <= u8::MAX as i32 {
                Some(n as u8)
            } else {
                None
            }
        })
    }

    /// Total bathrooms as a `Decimal` (e.g. `2.5` for two full + one half).
    pub fn bathrooms_decimal(&self) -> Option<rust_decimal::Decimal> {
        self.bathrooms_total_decimal
    }

    /// Full bathrooms only (toilet + sink + tub or shower).
    pub fn bathrooms_full(&self) -> Option<u8> {
        self.bathrooms_full.and_then(|n| {
            if n >= 0 && n <= u8::MAX as i32 {
                Some(n as u8)
            } else {
                None
            }
        })
    }

    /// Half bathrooms only (toilet + sink, no tub or shower).
    pub fn bathrooms_half(&self) -> Option<u8> {
        self.bathrooms_half.and_then(|n| {
            if n >= 0 && n <= u8::MAX as i32 {
                Some(n as u8)
            } else {
                None
            }
        })
    }

    /// True if any basement/below-grade area exists.
    ///
    /// Checks `BasementYN` first, then falls back to checking whether
    /// `Basement` collection is non-empty and does not only contain "None".
    pub fn has_basement(&self) -> bool {
        if let Some(yn) = self.basement_yn {
            return yn;
        }
        self.basement
            .as_ref()
            .map(|v| !v.is_empty() && !v.iter().all(|s| s.eq_ignore_ascii_case("none")))
            .unwrap_or(false)
    }

    /// True if the basement has finished living area.
    ///
    /// Checks `Basement` collection for "Finished" or "PartiallyFinished".
    pub fn is_basement_finished(&self) -> bool {
        self.basement
            .as_ref()
            .map(|v| {
                v.iter().any(|s| {
                    s.eq_ignore_ascii_case("finished")
                        || s.eq_ignore_ascii_case("partiallyfinished")
                        || s.eq_ignore_ascii_case("partially finished")
                })
            })
            .unwrap_or(false)
    }

    // ── Category 11: Parking ──────────────────────────────────────────────────

    /// Total parking spaces of all types.
    pub fn parking_total(&self) -> Option<u8> {
        self.parking_total.and_then(|n| {
            if n >= 0 && n <= u8::MAX as i32 {
                Some(n as u8)
            } else {
                None
            }
        })
    }

    /// Enclosed garage spaces (rounded from Decimal).
    pub fn garage_spaces(&self) -> Option<u8> {
        self.garage_spaces.as_ref().and_then(|d| d.to_u8())
    }

    /// True if any garage exists — `GarageYN` or `GarageSpaces` > 0.
    pub fn has_garage(&self) -> bool {
        if let Some(yn) = self.garage_yn {
            return yn;
        }
        self.garage_spaces
            .as_ref()
            .map(|d| d.to_u8().unwrap_or(0) > 0)
            .unwrap_or(false)
    }

    /// True if the garage is physically attached to the dwelling.
    pub fn has_attached_garage(&self) -> bool {
        self.attached_garage_yn.unwrap_or(false)
    }

    // ── Category 12: Systems and Utilities ───────────────────────────────────

    /// True if central air conditioning is present.
    ///
    /// Checks `Cooling` collection for "CentralAir" (case-insensitive).
    pub fn has_central_ac(&self) -> bool {
        self.cooling
            .as_ref()
            .map(|v| contains_ci(v, "central"))
            .unwrap_or(false)
    }

    /// True if forced-air heating is present.
    ///
    /// Checks `Heating` collection for "ForcedAir" (case-insensitive).
    pub fn has_forced_air_heat(&self) -> bool {
        self.heating
            .as_ref()
            .map(|v| contains_ci(v, "forced"))
            .unwrap_or(false)
    }

    /// True if connected to a public sewer system.
    ///
    /// Checks `Sewer` collection for "Public" (covers "PublicSewer" and
    /// "Public Sewer" variants used by different MLS boards).
    pub fn is_on_public_sewer(&self) -> bool {
        self.sewer
            .as_ref()
            .map(|v| contains_ci(v, "public"))
            .unwrap_or(false)
    }

    /// True if connected to a public water supply.
    ///
    /// Checks `WaterSource` collection for "Public".
    pub fn is_on_public_water(&self) -> bool {
        self.water_source
            .as_ref()
            .map(|v| contains_ci(v, "public"))
            .unwrap_or(false)
    }

    /// True if a private pool is on the property.
    pub fn has_pool(&self) -> bool {
        self.pool_private_yn.unwrap_or(false)
    }

    /// True if solar panels are installed.
    ///
    /// Checks `SolarPanels` boolean field first, then `GreenEnergyGeneration`
    /// collection for "Solar".
    pub fn has_solar(&self) -> bool {
        if let Some(yn) = self.solar_panels {
            return yn;
        }
        self.green_energy_generation
            .as_ref()
            .map(|v| contains_ci(v, "solar"))
            .unwrap_or(false)
    }

    /// True if any fireplace exists.
    ///
    /// Checks `FireplaceYN` first, then `FireplacesTotal` > 0.
    pub fn has_fireplace(&self) -> bool {
        if let Some(yn) = self.fireplace_yn {
            return yn;
        }
        self.fireplaces_total.map(|n| n > 0).unwrap_or(false)
    }

    /// Number of fireplaces.
    pub fn fireplaces(&self) -> u8 {
        self.fireplaces_total
            .map(|n| if n >= 0 { n as u8 } else { 0 })
            .unwrap_or(0)
    }
}
