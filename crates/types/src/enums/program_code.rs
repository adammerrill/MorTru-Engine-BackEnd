//! `ProgramCode` — top-level loan program classification.

use serde::{Deserialize, Serialize};

use crate::ParseError;

/// Top-level loan program. Drives which guidelines, MI rules, and
/// rate-sheet columns apply to a scenario.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProgramCode {
    Conventional,
    HomeReady,
    HomePossible,
    HomeOne,
    Fha,
    /// FHA with Down Payment Assistance (subordinate lien from an HFA).
    FhaDpa,
    Va,
    VaJumbo,
    /// USDA Section 502 Guaranteed Rural Housing.
    Usda,
    /// State HFA bond/down-payment-assistance programs.
    Bond,
    Jumbo,
    NonQm,
}

impl ProgramCode {
    /// MISMO 3.4 `MortgageType` enumeration value.
    ///
    /// MISMO uses a small set of top-level mortgage types; several of our
    /// program codes collapse onto the same MISMO value.
    #[must_use]
    pub const fn to_mismo_mortgage_type(self) -> &'static str {
        match self {
            Self::Conventional
            | Self::HomeReady
            | Self::HomePossible
            | Self::HomeOne
            | Self::Bond
            | Self::Jumbo
            | Self::NonQm => "Conventional",
            Self::Fha | Self::FhaDpa => "FHA",
            Self::Va | Self::VaJumbo => "VA",
            Self::Usda => "USDARuralDevelopment",
        }
    }

    /// Parse from a MISMO 3.4 `MortgageType` string.
    ///
    /// Returns the most-general program code for each MISMO type.
    /// Callers that need finer granularity (e.g. HomeReady vs plain
    /// Conventional) should use product-specific eligibility logic after
    /// parsing.
    pub fn from_mismo_mortgage_type(s: &str) -> Result<Self, ParseError> {
        match s.trim() {
            "Conventional" => Ok(Self::Conventional),
            "FHA" => Ok(Self::Fha),
            "VA" => Ok(Self::Va),
            "USDARuralDevelopment" | "USDA" => Ok(Self::Usda),
            other => Err(ParseError::InvalidPercentageString(format!(
                "unknown MISMO MortgageType: `{other}`"
            ))),
        }
    }

    /// True for the three GSE / agency programs that use DU or LPA.
    #[must_use]
    pub const fn is_agency(self) -> bool {
        matches!(
            self,
            Self::Conventional | Self::HomeReady | Self::HomePossible | Self::HomeOne
        )
    }

    /// True for government-insured / guaranteed programs.
    #[must_use]
    pub const fn is_government(self) -> bool {
        matches!(
            self,
            Self::Fha | Self::FhaDpa | Self::Va | Self::VaJumbo | Self::Usda
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_program_code_to_mismo_mortgage_type() {
        assert_eq!(
            ProgramCode::Conventional.to_mismo_mortgage_type(),
            "Conventional"
        );
        assert_eq!(
            ProgramCode::HomeReady.to_mismo_mortgage_type(),
            "Conventional"
        );
        assert_eq!(
            ProgramCode::HomePossible.to_mismo_mortgage_type(),
            "Conventional"
        );
        assert_eq!(ProgramCode::Jumbo.to_mismo_mortgage_type(), "Conventional");
        assert_eq!(ProgramCode::NonQm.to_mismo_mortgage_type(), "Conventional");
        assert_eq!(ProgramCode::Fha.to_mismo_mortgage_type(), "FHA");
        assert_eq!(ProgramCode::FhaDpa.to_mismo_mortgage_type(), "FHA");
        assert_eq!(ProgramCode::Va.to_mismo_mortgage_type(), "VA");
        assert_eq!(ProgramCode::VaJumbo.to_mismo_mortgage_type(), "VA");
        assert_eq!(
            ProgramCode::Usda.to_mismo_mortgage_type(),
            "USDARuralDevelopment"
        );
        assert_eq!(ProgramCode::Bond.to_mismo_mortgage_type(), "Conventional");
    }

    #[test]
    fn test_program_code_from_mismo_known() {
        assert_eq!(
            ProgramCode::from_mismo_mortgage_type("Conventional").unwrap(),
            ProgramCode::Conventional
        );
        assert_eq!(
            ProgramCode::from_mismo_mortgage_type("FHA").unwrap(),
            ProgramCode::Fha
        );
        assert_eq!(
            ProgramCode::from_mismo_mortgage_type("VA").unwrap(),
            ProgramCode::Va
        );
        assert_eq!(
            ProgramCode::from_mismo_mortgage_type("USDARuralDevelopment").unwrap(),
            ProgramCode::Usda
        );
        assert_eq!(
            ProgramCode::from_mismo_mortgage_type("USDA").unwrap(),
            ProgramCode::Usda
        );
    }

    #[test]
    fn test_program_code_from_mismo_unknown_returns_error() {
        assert!(ProgramCode::from_mismo_mortgage_type("Jumbo").is_err());
        assert!(ProgramCode::from_mismo_mortgage_type("").is_err());
        assert!(ProgramCode::from_mismo_mortgage_type("conventional").is_err()); // case-sensitive
        assert!(ProgramCode::from_mismo_mortgage_type("fha").is_err());
    }

    #[test]
    fn test_program_code_is_agency() {
        assert!(ProgramCode::Conventional.is_agency());
        assert!(ProgramCode::HomeReady.is_agency());
        assert!(ProgramCode::HomePossible.is_agency());
        assert!(ProgramCode::HomeOne.is_agency());
        assert!(!ProgramCode::Fha.is_agency());
        assert!(!ProgramCode::Va.is_agency());
        assert!(!ProgramCode::Jumbo.is_agency());
    }

    #[test]
    fn test_program_code_is_government() {
        assert!(ProgramCode::Fha.is_government());
        assert!(ProgramCode::FhaDpa.is_government());
        assert!(ProgramCode::Va.is_government());
        assert!(ProgramCode::VaJumbo.is_government());
        assert!(ProgramCode::Usda.is_government());
        assert!(!ProgramCode::Conventional.is_government());
        assert!(!ProgramCode::Jumbo.is_government());
    }

    #[test]
    fn test_program_code_serde_json() {
        let p = ProgramCode::HomeReady;
        let json = serde_json::to_string(&p).unwrap();
        assert_eq!(json, "\"home_ready\"");
        let back: ProgramCode = serde_json::from_str(&json).unwrap();
        assert_eq!(back, p);
    }
}
