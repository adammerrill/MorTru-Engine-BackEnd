//! Property MISMO string enumerations.
//!
//! Converts MISMO 3.4 property-related XML strings into `types` crate
//! domain values.
//!
//! `Occupancy` already has `from_mismo()` in the `types` crate and is
//! wrapped here to adapt the error type.
//!
//! `PropertyType` only has `to_mismo()` in the `types` crate, so the
//! inverse mapping is implemented here.

use types::{Occupancy, PropertyType};

// ── PropertyType ─────────────────────────────────────────────────────────────

/// Convert a MISMO 3.4 `GSEProjectClassificationType` or
/// `PropertyStructureType` string to `types::PropertyType`.
///
/// MISMO uses overlapping strings across different elements; this function
/// handles the values most commonly seen in loan files:
///
/// | MISMO value | `PropertyType` |
/// |---|---|
/// | `"Detached"` | `SingleFamilyDetached` |
/// | `"Attached"` | `SingleFamilyAttached` |
/// | `"Townhouse"` | `Townhouse` |
/// | `"Condominium"` | `Condominium` |
/// | `"Cooperative"` | `Cooperative` |
/// | `"PUD"` | `PlannedUnitDevelopment` |
/// | `"ManufacturedHousing"` | `ManufacturedHome` |
/// | `"2-Unit"` | `TwoUnit` |
/// | `"3-Unit"` | `ThreeUnit` |
/// | `"4-Unit"` | `FourUnit` |
///
/// This is the inverse of `PropertyType::to_mismo()`.
///
/// # Errors
/// Returns `MismoError::InvalidEnum` for any unrecognised value.
pub fn try_property_type(s: &str) -> crate::Result<PropertyType> {
    match s.trim() {
        "Detached" => Ok(PropertyType::SingleFamilyDetached),
        "Attached" => Ok(PropertyType::SingleFamilyAttached),
        "Townhouse" => Ok(PropertyType::Townhouse),
        "Condominium" => Ok(PropertyType::Condominium),
        "Cooperative" => Ok(PropertyType::Cooperative),
        "PUD" => Ok(PropertyType::PlannedUnitDevelopment),
        "ManufacturedHousing" => Ok(PropertyType::ManufacturedHome),
        "2-Unit" => Ok(PropertyType::TwoUnit),
        "3-Unit" => Ok(PropertyType::ThreeUnit),
        "4-Unit" => Ok(PropertyType::FourUnit),
        _ => Err(crate::MismoError::InvalidEnum {
            element: "PropertyStructureType",
            value: s.to_owned(),
        }),
    }
}

// ── Occupancy ────────────────────────────────────────────────────────────────

/// Convert a MISMO 3.4 `PropertyUsageType` string to `types::Occupancy`.
///
/// Accepted values: `"PrimaryResidence"`, `"SecondHome"`, `"Investor"`.
///
/// # Errors
/// Returns `MismoError::InvalidEnum` for any unrecognised value.
pub fn try_occupancy(s: &str) -> crate::Result<Occupancy> {
    Occupancy::from_mismo(s).map_err(|_| crate::MismoError::InvalidEnum {
        element: "PropertyUsageType",
        value: s.to_owned(),
    })
}
