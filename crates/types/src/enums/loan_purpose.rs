//! `LoanPurpose` — why the borrower is taking out the loan.

use serde::{Deserialize, Serialize};

use crate::ParseError;

/// Purpose of the loan transaction. Drives eligible programs, LTV limits,
/// and required documentation.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoanPurpose {
    Purchase,
    RateAndTermRefinance,
    CashOutRefinance,
    Construction,
    ConstructionToPermanent,
}

impl LoanPurpose {
    /// MISMO 3.4 `LoanPurposeType` value.
    #[must_use]
    pub const fn to_mismo(self) -> &'static str {
        match self {
            Self::Purchase => "Purchase",
            Self::RateAndTermRefinance => "Refinance",
            Self::CashOutRefinance => "CashOutRefinance",
            Self::Construction => "Construction",
            Self::ConstructionToPermanent => "ConstructionToPermanent",
        }
    }

    /// Parse from a MISMO 3.4 `LoanPurposeType` string.
    pub fn from_mismo(s: &str) -> Result<Self, ParseError> {
        match s.trim() {
            "Purchase" => Ok(Self::Purchase),
            "Refinance" | "RateTermRefinance" | "LimitedCashOutRefinance" => {
                Ok(Self::RateAndTermRefinance)
            }
            "CashOutRefinance" | "CashOut" => Ok(Self::CashOutRefinance),
            "Construction" => Ok(Self::Construction),
            "ConstructionToPermanent" => Ok(Self::ConstructionToPermanent),
            other => Err(ParseError::InvalidStateCode(format!(
                "unknown MISMO LoanPurposeType: `{other}`"
            ))),
        }
    }

    /// True if this is any form of refinance.
    #[must_use]
    pub const fn is_refinance(self) -> bool {
        matches!(self, Self::RateAndTermRefinance | Self::CashOutRefinance)
    }

    /// True if this is a construction loan (or construction-to-permanent).
    #[must_use]
    pub const fn is_construction(self) -> bool {
        matches!(self, Self::Construction | Self::ConstructionToPermanent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loan_purpose_to_mismo() {
        assert_eq!(LoanPurpose::Purchase.to_mismo(), "Purchase");
        assert_eq!(LoanPurpose::RateAndTermRefinance.to_mismo(), "Refinance");
        assert_eq!(LoanPurpose::CashOutRefinance.to_mismo(), "CashOutRefinance");
        assert_eq!(LoanPurpose::Construction.to_mismo(), "Construction");
        assert_eq!(
            LoanPurpose::ConstructionToPermanent.to_mismo(),
            "ConstructionToPermanent"
        );
    }

    #[test]
    fn test_loan_purpose_from_mismo_known() {
        assert_eq!(
            LoanPurpose::from_mismo("Purchase").unwrap(),
            LoanPurpose::Purchase
        );
        assert_eq!(
            LoanPurpose::from_mismo("Refinance").unwrap(),
            LoanPurpose::RateAndTermRefinance
        );
        assert_eq!(
            LoanPurpose::from_mismo("LimitedCashOutRefinance").unwrap(),
            LoanPurpose::RateAndTermRefinance
        );
        assert_eq!(
            LoanPurpose::from_mismo("CashOutRefinance").unwrap(),
            LoanPurpose::CashOutRefinance
        );
        assert_eq!(
            LoanPurpose::from_mismo("CashOut").unwrap(),
            LoanPurpose::CashOutRefinance
        );
    }

    #[test]
    fn test_loan_purpose_from_mismo_unknown_returns_error() {
        assert!(LoanPurpose::from_mismo("").is_err());
        assert!(LoanPurpose::from_mismo("purchase").is_err());
    }

    #[test]
    fn test_loan_purpose_is_refinance() {
        assert!(LoanPurpose::RateAndTermRefinance.is_refinance());
        assert!(LoanPurpose::CashOutRefinance.is_refinance());
        assert!(!LoanPurpose::Purchase.is_refinance());
        assert!(!LoanPurpose::Construction.is_refinance());
    }

    #[test]
    fn test_loan_purpose_serde_json() {
        let p = LoanPurpose::CashOutRefinance;
        let json = serde_json::to_string(&p).unwrap();
        assert_eq!(json, "\"cash_out_refinance\"");
        let back: LoanPurpose = serde_json::from_str(&json).unwrap();
        assert_eq!(back, p);
    }
}
