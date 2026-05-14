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
    ManufacturedHome,
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
            Self::TwoUnit => "Duplex",
            Self::ThreeUnit => "Triplex",
            Self::FourUnit => "Quadruplex",
        }
    }

    /// Parse from a RESO 2.0 `PropertySubType` lookup string.
    pub fn from_reso_lookup(s: &str) -> Result<Self, ParseError> {
        match s.trim() {
            "Single Family Residence" => Ok(Self::SingleFamilyDetached),
            "Single Family Attached" => Ok(Self::SingleFamilyAttached),
            "Townhouse" | "Town House" => Ok(Self::Townhouse),
            "Condominium" | "Condo" => Ok(Self::Condominium),
            "Stock Cooperative" | "Cooperative" => Ok(Self::Cooperative),
            "Planned Unit Development" | "PUD" => Ok(Self::PlannedUnitDevelopment),
            "Manufactured Home" | "Manufactured Housing" => Ok(Self::ManufacturedHome),
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
    #[must_use]
    pub const fn is_conventional_eligible(self) -> bool {
        !matches!(self, Self::Cooperative)
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
    fn test_property_type_to_mismo() {
        assert_eq!(PropertyType::SingleFamilyDetached.to_mismo(), "Detached");
        assert_eq!(PropertyType::Condominium.to_mismo(), "Condominium");
        assert_eq!(PropertyType::TwoUnit.to_mismo(), "2-Unit");
        assert_eq!(
            PropertyType::ManufacturedHome.to_mismo(),
            "ManufacturedHousing"
        );
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
}
