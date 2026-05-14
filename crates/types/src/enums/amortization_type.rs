//! `AmortizationType` — how the loan balance amortises over time.

use serde::{Deserialize, Serialize};

use crate::ParseError;

/// Payment structure / amortisation method.
///
/// Only `Fixed` and `Arm` are eligible for Qualified Mortgage status under
/// Reg Z § 1026.43(e). The others are annotated accordingly.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AmortizationType {
    Fixed,
    Arm,
    /// Interest-only loans are **not QM-eligible**.
    InterestOnly,
    /// Graduated-payment mortgages are **not QM-eligible**.
    GraduatedPayment,
    /// Option-ARM / negative-amortisation products are **not QM-eligible**.
    PaymentOption,
}

impl AmortizationType {
    /// MISMO 3.4 `AmortizationType` value.
    #[must_use]
    pub const fn to_mismo(self) -> &'static str {
        match self {
            Self::Fixed => "Fixed",
            Self::Arm => "AdjustableRate",
            Self::InterestOnly => "InterestOnly",
            Self::GraduatedPayment => "GraduatedPayment",
            Self::PaymentOption => "NegativeAmortization",
        }
    }

    /// Parse from a MISMO 3.4 `AmortizationType` string.
    pub fn from_mismo(s: &str) -> Result<Self, ParseError> {
        match s.trim() {
            "Fixed" => Ok(Self::Fixed),
            "AdjustableRate" | "ARM" => Ok(Self::Arm),
            "InterestOnly" => Ok(Self::InterestOnly),
            "GraduatedPayment" | "GraduatedPaymentMortgage" => Ok(Self::GraduatedPayment),
            "NegativeAmortization" | "PaymentOption" => Ok(Self::PaymentOption),
            other => Err(ParseError::InvalidStateCode(format!(
                "unknown MISMO AmortizationType: `{other}`"
            ))),
        }
    }

    /// True if this amortisation type is eligible for QM safe harbor
    /// under Reg Z § 1026.43(e).
    #[must_use]
    pub const fn is_qm_eligible(self) -> bool {
        matches!(self, Self::Fixed | Self::Arm)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_amortization_type_to_mismo() {
        assert_eq!(AmortizationType::Fixed.to_mismo(), "Fixed");
        assert_eq!(AmortizationType::Arm.to_mismo(), "AdjustableRate");
        assert_eq!(AmortizationType::InterestOnly.to_mismo(), "InterestOnly");
        assert_eq!(
            AmortizationType::GraduatedPayment.to_mismo(),
            "GraduatedPayment"
        );
        assert_eq!(
            AmortizationType::PaymentOption.to_mismo(),
            "NegativeAmortization"
        );
    }

    #[test]
    fn test_amortization_type_from_mismo() {
        assert_eq!(
            AmortizationType::from_mismo("Fixed").unwrap(),
            AmortizationType::Fixed
        );
        assert_eq!(
            AmortizationType::from_mismo("AdjustableRate").unwrap(),
            AmortizationType::Arm
        );
        assert_eq!(
            AmortizationType::from_mismo("ARM").unwrap(),
            AmortizationType::Arm
        );
        assert_eq!(
            AmortizationType::from_mismo("InterestOnly").unwrap(),
            AmortizationType::InterestOnly
        );
        assert_eq!(
            AmortizationType::from_mismo("NegativeAmortization").unwrap(),
            AmortizationType::PaymentOption
        );
    }

    #[test]
    fn test_amortization_type_is_qm_eligible() {
        assert!(AmortizationType::Fixed.is_qm_eligible());
        assert!(AmortizationType::Arm.is_qm_eligible());
        assert!(!AmortizationType::InterestOnly.is_qm_eligible());
        assert!(!AmortizationType::GraduatedPayment.is_qm_eligible());
        assert!(!AmortizationType::PaymentOption.is_qm_eligible());
    }

    #[test]
    fn test_amortization_type_serde_json() {
        let a = AmortizationType::Fixed;
        let json = serde_json::to_string(&a).unwrap();
        assert_eq!(json, "\"fixed\"");
        let back: AmortizationType = serde_json::from_str(&json).unwrap();
        assert_eq!(back, a);
    }
}
