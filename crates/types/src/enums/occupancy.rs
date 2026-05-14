//! `Occupancy` — borrower's intended use of the property.

use serde::{Deserialize, Serialize};

use crate::ParseError;

/// Borrower's intended occupancy of the collateral property.
///
/// Maps to MISMO 3.4 `PropertyUsageType` and to the RESO 2.0
/// `OccupantType` lookup.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Occupancy {
    PrimaryResidence,
    SecondHome,
    Investment,
}

impl Occupancy {
    /// MISMO 3.4 `PropertyUsageType` value.
    #[must_use]
    pub const fn to_mismo(self) -> &'static str {
        match self {
            Self::PrimaryResidence => "PrimaryResidence",
            Self::SecondHome => "SecondHome",
            Self::Investment => "Investor",
        }
    }

    /// Parse from a MISMO 3.4 `PropertyUsageType` string.
    pub fn from_mismo(s: &str) -> Result<Self, ParseError> {
        match s.trim() {
            "PrimaryResidence" => Ok(Self::PrimaryResidence),
            "SecondHome" | "Second Home" => Ok(Self::SecondHome),
            "Investor" | "Investment" | "NonOwnerOccupied" => Ok(Self::Investment),
            other => Err(ParseError::InvalidStateCode(format!(
                "unknown MISMO PropertyUsageType: `{other}`"
            ))),
        }
    }

    /// RESO 2.0 `OccupantType` lookup value.
    #[must_use]
    pub const fn to_reso_lookup(self) -> &'static str {
        match self {
            Self::PrimaryResidence => "Owner",
            Self::SecondHome => "Owner",
            Self::Investment => "Tenant",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_occupancy_round_trip_mismo() {
        for occ in [
            Occupancy::PrimaryResidence,
            Occupancy::SecondHome,
            Occupancy::Investment,
        ] {
            let mismo = occ.to_mismo();
            let back = Occupancy::from_mismo(mismo).unwrap_or_else(|_| {
                panic!("from_mismo failed for {mismo} (from {occ:?})")
            });
            assert_eq!(back, occ, "MISMO roundtrip failed for {occ:?}");
        }
    }

    #[test]
    fn test_occupancy_from_mismo_known() {
        assert_eq!(Occupancy::from_mismo("PrimaryResidence").unwrap(), Occupancy::PrimaryResidence);
        assert_eq!(Occupancy::from_mismo("SecondHome").unwrap(), Occupancy::SecondHome);
        assert_eq!(Occupancy::from_mismo("Second Home").unwrap(), Occupancy::SecondHome);
        assert_eq!(Occupancy::from_mismo("Investor").unwrap(), Occupancy::Investment);
        assert_eq!(Occupancy::from_mismo("Investment").unwrap(), Occupancy::Investment);
        assert_eq!(Occupancy::from_mismo("NonOwnerOccupied").unwrap(), Occupancy::Investment);
    }

    #[test]
    fn test_occupancy_from_mismo_unknown_returns_error() {
        assert!(Occupancy::from_mismo("").is_err());
        assert!(Occupancy::from_mismo("Rental").is_err());
        assert!(Occupancy::from_mismo("primaryresidence").is_err());
    }

    #[test]
    fn test_occupancy_serde_json() {
        let o = Occupancy::PrimaryResidence;
        let json = serde_json::to_string(&o).unwrap();
        assert_eq!(json, "\"primary_residence\"");
        let back: Occupancy = serde_json::from_str(&json).unwrap();
        assert_eq!(back, o);
    }
}
