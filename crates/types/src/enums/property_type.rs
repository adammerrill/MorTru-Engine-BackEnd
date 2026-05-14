//! `PropertyType` — physical property classification.
//!
//! Maps to both MISMO `PropertyUsageType` / `AttachmentType` and
//! RESO 2.0 `PropertySubType`.

use serde::{Deserialize, Serialize};

use crate::ParseError;

/// Physical classification of the collateral property.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PropertyType {
    SingleFamilyDetached,
    SingleFamilyAttached,
    Townhouse,
    Condominium,
    Cooperative,
    PlannedUnitDevelopment,
    /// Factory-built on a permanent chassis, HUD-code post-1976.
    /// Eligible for FHA, VA, USDA, and conventional (with restrictions).
    ManufacturedHome,
    /// Factory-built in sections, placed on a permanent foundation.
    /// Treated as site-built for most agency purposes; eligible for all programs.
    Modular,
    /// Pre-HUD (pre-1976) or title not converted to real property.
    /// Ineligible for conventional, FHA, VA, and USDA as real property.
    MobileHome,
    TwoUnit,
    ThreeUnit,
    FourUnit,
}

impl PropertyType {
    /// RESO 2.0 `PropertySubType` lookup value.
    #[must_use]
    pub const fn to_reso_lookup(self) -> &'static str {
        match self {
            Self::SingleFamilyDetached => "Single Family Residence",
            Self::SingleFamilyAttached => "Single Family Attached",
            Self::Townhouse => "Townhouse",
            Self::Condominium => "Condominium",
            Self::Cooperative => "Stock Cooperative",
            Self::PlannedUnitDevelopment => "Planned Unit Development",
            Self::ManufacturedHome => "Manufactured Home",
            Self::Modular => "Modular",
            Self::MobileHome => "Mobile Home",
            Self::TwoUnit => "Duplex",
            Self::ThreeUnit => "Triplex",
            Self::FourUnit => "Quadruplex",
        }
    }

    /// Parse from a RESO 2.0 `PropertySubType` lookup string.
    pub fn from_reso_lookup(s: &str) -> Result<Self, ParseError> {
        match s.trim() {
            // Standard RESO 2.0 strings
            "Single Family Residence" => Ok(Self::SingleFamilyDetached),
            // ABOR / Austin Board of Realtors feed uses this exact string
            "Single Family Resi" => Ok(Self::SingleFamilyDetached),
            "Single Family Attached" => Ok(Self::SingleFamilyAttached),
            "Townhouse" | "Town House" => Ok(Self::Townhouse),
            "Condominium" | "Condo" => Ok(Self::Condominium),
            "Stock Cooperative" | "Cooperative" => Ok(Self::Cooperative),
            "Planned Unit Development" | "PUD" => Ok(Self::PlannedUnitDevelopment),
            "Manufactured Home" | "Manufactured Housing" => Ok(Self::ManufacturedHome),
            // Factory-built on permanent foundation — treated as site-built
            "Modular" | "Modular Home" => Ok(Self::Modular),
            // Pre-HUD or personal property title — universally ineligible
            "Mobile Home" => Ok(Self::MobileHome),
            "Duplex" => Ok(Self::TwoUnit),
            "Triplex" => Ok(Self::ThreeUnit),
            "Quadruplex" | "Fourplex" => Ok(Self::FourUnit),
            other => Err(ParseError::IdentifierInvalidChars {
                kind: "PropertyType",
                value: other.to_string(),
            }),
        }
    }

    /// MISMO 3.4 `GsePropType` string (used in the AUS interface).
    #[must_use]
    pub const fn to_mismo(self) -> &'static str {
        match self {
            Self::SingleFamilyDetached => "Detached",
            Self::SingleFamilyAttached => "Attached",
            Self::Townhouse => "Attached",
            Self::Condominium => "Condominium",
            Self::Cooperative => "Cooperative",
            Self::PlannedUnitDevelopment => "PUD",
            Self::ManufacturedHome => "ManufacturedHousing",
            // MISMO treats modular as site-built; no dedicated code
            Self::Modular => "Detached",
            // No standard MISMO code; ineligible flag set in eligibility crate
            Self::MobileHome => "ManufacturedHousing",
            Self::TwoUnit => "2-Unit",
            Self::ThreeUnit => "3-Unit",
            Self::FourUnit => "4-Unit",
        }
    }

    /// True if this is a 2–4 unit income property.
    #[must_use]
    pub const fn is_multi_unit(self) -> bool {
        matches!(self, Self::TwoUnit | Self::ThreeUnit | Self::FourUnit)
    }

    /// True if this property type is eligible for standard conventional pricing.
    /// Co-ops are not accepted by Fannie/Freddie outside NYC co-op programs.
    /// Mobile homes are ineligible as personal property.
    #[must_use]
    pub const fn is_conventional_eligible(self) -> bool {
        !matches!(self, Self::Cooperative | Self::MobileHome)
    }

    /// True if this type is ineligible for ALL agency programs (conventional,
    /// FHA, VA, USDA) when titled as personal property.
    ///
    /// Mobile homes pre-dating the 1976 HUD Manufactured Housing Standards, or
    /// any unit whose title has not been converted to real property, are always
    /// ineligible. This flag triggers a hard rejection in the eligibility crate.
    #[must_use]
    pub const fn is_ineligible_personal_property(self) -> bool {
        matches!(self, Self::MobileHome)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_property_type_to_reso_lookup() {
        assert_eq!(
            PropertyType::SingleFamilyDetached.to_reso_lookup(),
            "Single Family Residence"
        );
        assert_eq!(PropertyType::Condominium.to_reso_lookup(), "Condominium");
        assert_eq!(PropertyType::Townhouse.to_reso_lookup(), "Townhouse");
        assert_eq!(PropertyType::TwoUnit.to_reso_lookup(), "Duplex");
        assert_eq!(PropertyType::ThreeUnit.to_reso_lookup(), "Triplex");
        assert_eq!(PropertyType::FourUnit.to_reso_lookup(), "Quadruplex");
        assert_eq!(
            PropertyType::ManufacturedHome.to_reso_lookup(),
            "Manufactured Home"
        );
        assert_eq!(
            PropertyType::PlannedUnitDevelopment.to_reso_lookup(),
            "Planned Unit Development"
        );
        assert_eq!(PropertyType::Modular.to_reso_lookup(), "Modular");
        assert_eq!(PropertyType::MobileHome.to_reso_lookup(), "Mobile Home");
    }

    #[test]
    fn test_property_type_from_reso_lookup() {
        assert_eq!(
            PropertyType::from_reso_lookup("Single Family Residence").unwrap(),
            PropertyType::SingleFamilyDetached
        );
        assert_eq!(
            PropertyType::from_reso_lookup("Condominium").unwrap(),
            PropertyType::Condominium
        );
        assert_eq!(
            PropertyType::from_reso_lookup("Condo").unwrap(),
            PropertyType::Condominium
        );
        assert_eq!(
            PropertyType::from_reso_lookup("PUD").unwrap(),
            PropertyType::PlannedUnitDevelopment
        );
        assert_eq!(
            PropertyType::from_reso_lookup("Duplex").unwrap(),
            PropertyType::TwoUnit
        );
        assert_eq!(
            PropertyType::from_reso_lookup("Fourplex").unwrap(),
            PropertyType::FourUnit
        );
        assert!(PropertyType::from_reso_lookup("Spaceship").is_err());
    }

    #[test]
    fn test_property_type_abor_single_family_resi_string() {
        // ABOR (Austin Board of Realtors) feed uses "Single Family Resi"
        assert_eq!(
            PropertyType::from_reso_lookup("Single Family Resi").unwrap(),
            PropertyType::SingleFamilyDetached
        );
    }

    #[test]
    fn test_property_type_modular_from_reso() {
        assert_eq!(
            PropertyType::from_reso_lookup("Modular").unwrap(),
            PropertyType::Modular
        );
        assert_eq!(
            PropertyType::from_reso_lookup("Modular Home").unwrap(),
            PropertyType::Modular
        );
    }

    #[test]
    fn test_property_type_mobile_home_from_reso() {
        assert_eq!(
            PropertyType::from_reso_lookup("Mobile Home").unwrap(),
            PropertyType::MobileHome
        );
    }

    #[test]
    fn test_property_type_mobile_home_is_ineligible() {
        assert!(PropertyType::MobileHome.is_ineligible_personal_property());
        assert!(!PropertyType::ManufacturedHome.is_ineligible_personal_property());
        assert!(!PropertyType::Modular.is_ineligible_personal_property());
        assert!(!PropertyType::SingleFamilyDetached.is_ineligible_personal_property());
    }

    #[test]
    fn test_property_type_modular_is_conventional_eligible() {
        // Modular treated as site-built — eligible for all programs
        assert!(PropertyType::Modular.is_conventional_eligible());
        // Mobile home ineligible as personal property
        assert!(!PropertyType::MobileHome.is_conventional_eligible());
    }

    #[test]
    fn test_property_type_to_mismo() {
        assert_eq!(PropertyType::SingleFamilyDetached.to_mismo(), "Detached");
        assert_eq!(PropertyType::Condominium.to_mismo(), "Condominium");
        assert_eq!(PropertyType::TwoUnit.to_mismo(), "2-Unit");
        assert_eq!(
            PropertyType::ManufacturedHome.to_mismo(),
            "ManufacturedHousing"
        );
        // Modular → Detached (MISMO has no distinct modular code)
        assert_eq!(PropertyType::Modular.to_mismo(), "Detached");
    }

    #[test]
    fn test_property_type_is_multi_unit() {
        assert!(PropertyType::TwoUnit.is_multi_unit());
        assert!(PropertyType::ThreeUnit.is_multi_unit());
        assert!(PropertyType::FourUnit.is_multi_unit());
        assert!(!PropertyType::SingleFamilyDetached.is_multi_unit());
        assert!(!PropertyType::Condominium.is_multi_unit());
    }

    #[test]
    fn test_property_type_serde_json() {
        let pt = PropertyType::SingleFamilyDetached;
        let json = serde_json::to_string(&pt).unwrap();
        assert_eq!(json, "\"single_family_detached\"");
        let back: PropertyType = serde_json::from_str(&json).unwrap();
        assert_eq!(back, pt);
    }

    #[test]
    fn test_property_type_modular_serde_json() {
        let json = serde_json::to_string(&PropertyType::Modular).unwrap();
        assert_eq!(json, "\"modular\"");
        let back: PropertyType = serde_json::from_str(&json).unwrap();
        assert_eq!(back, PropertyType::Modular);
    }

    #[test]
    fn test_property_type_mobile_home_serde_json() {
        let json = serde_json::to_string(&PropertyType::MobileHome).unwrap();
        assert_eq!(json, "\"mobile_home\"");
        let back: PropertyType = serde_json::from_str(&json).unwrap();
        assert_eq!(back, PropertyType::MobileHome);
    }
}
