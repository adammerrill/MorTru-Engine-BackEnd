//! Smaller enumerations: `LockPeriod`, `LienPriority`, `BalanceType`,
//! `Tier`, `MiCoverageType`, `AusType`.

use serde::{Deserialize, Serialize};

// ── LockPeriod ────────────────────────────────────────────────────────────────

/// Rate-lock commitment period offered by the lender.
///
/// Lenders typically price each lock period at a different price adjustment;
/// this enum is the key into the lock-period LLPA table.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LockPeriod {
    Day15,
    Day21,
    Day30,
    Day45,
    Day60,
    Day75,
    Day90,
}

impl LockPeriod {
    /// Calendar days in this lock period.
    #[must_use]
    pub const fn days(self) -> u32 {
        match self {
            Self::Day15 => 15,
            Self::Day21 => 21,
            Self::Day30 => 30,
            Self::Day45 => 45,
            Self::Day60 => 60,
            Self::Day75 => 75,
            Self::Day90 => 90,
        }
    }

    /// All lock periods in ascending order, for iteration over a price grid.
    pub const ALL: &'static [Self] = &[
        Self::Day15,
        Self::Day21,
        Self::Day30,
        Self::Day45,
        Self::Day60,
        Self::Day75,
        Self::Day90,
    ];
}

// ── LienPriority ─────────────────────────────────────────────────────────────

/// Position of this lien relative to other recorded liens on the property.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LienPriority {
    First,
    Second,
    Third,
}

impl LienPriority {
    /// MISMO 3.4 `LienPriorityType` value.
    #[must_use]
    pub const fn to_mismo(self) -> &'static str {
        match self {
            Self::First => "FirstLien",
            Self::Second => "SecondLien",
            Self::Third => "ThirdLien",
        }
    }
}

// ── BalanceType ───────────────────────────────────────────────────────────────

/// Whether the loan balance falls within the standard conforming limit,
/// a high-balance (FHFA-designated high-cost area) limit, or is a jumbo.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum BalanceType {
    /// At or below the baseline conforming loan limit.
    Conforming,
    /// Above baseline but at or below the FHFA high-cost-area limit.
    HighBalance,
    /// FHFA's own term for high-balance Freddie Mac loans — same range as
    /// Fannie Mae `HighBalance`.
    SuperConforming,
    /// Exceeds all FHFA conforming limits; not eligible for GSE delivery.
    Jumbo,
}

// ── Tier ─────────────────────────────────────────────────────────────────────

/// Lender-defined quality tier used in some rate-sheet LLPA grids.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum Tier {
    Elite,
    Standard,
}

// ── MiCoverageType ────────────────────────────────────────────────────────────

/// Mortgage insurance structure and who pays the premium.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MiCoverageType {
    /// No MI (e.g., LTV ≤ 80%, or VA/USDA).
    None,
    /// Lender-paid MI (rolled into the rate).
    LenderPaid,
    /// Borrower-paid monthly MI added to the PITI payment.
    BorrowerPaidMonthly,
    /// Borrower-paid single-premium MI (financed or paid at closing).
    BorrowerPaidSingle,
    /// Borrower-paid split premium (partial upfront, partial monthly).
    BorrowerPaidSplit,
    /// FHA: upfront MIP + annual MIP (monthly).
    FhaUpfrontAndAnnual,
    /// VA: upfront funding fee, no monthly MI.
    VaFundingFee,
    /// USDA: upfront guarantee fee + annual fee (monthly).
    UsdaUpfrontAndAnnual,
}

impl MiCoverageType {
    /// True if there is any monthly MI component to include in the PITI.
    #[must_use]
    pub const fn has_monthly_premium(self) -> bool {
        matches!(
            self,
            Self::BorrowerPaidMonthly
                | Self::BorrowerPaidSplit
                | Self::FhaUpfrontAndAnnual
                | Self::UsdaUpfrontAndAnnual
        )
    }

    /// True if there is an upfront premium to include in the closing costs.
    #[must_use]
    pub const fn has_upfront_premium(self) -> bool {
        matches!(
            self,
            Self::BorrowerPaidSingle
                | Self::BorrowerPaidSplit
                | Self::FhaUpfrontAndAnnual
                | Self::VaFundingFee
                | Self::UsdaUpfrontAndAnnual
        )
    }
}

// ── AusType ───────────────────────────────────────────────────────────────────

/// Automated underwriting system used for this loan recommendation.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AusType {
    /// Fannie Mae Desktop Underwriter.
    DesktopUnderwriter,
    /// Freddie Mac Loan Product Advisor.
    LoanProductAdvisor,
    /// FHA TOTAL Mortgage Scorecard.
    Got,
    /// USDA Guaranteed Underwriting System.
    Gus,
    /// No AUS run — underwriting by a human underwriter.
    Manual,
}

impl AusType {
    /// MISMO 3.4 `UnderwritingSystemType` value.
    #[must_use]
    pub const fn to_mismo(self) -> &'static str {
        match self {
            Self::DesktopUnderwriter => "DesktopUnderwriter",
            Self::LoanProductAdvisor => "LoanProductAdvisor",
            Self::Got => "FHATotalScorecard",
            Self::Gus => "USDARuralHousingGUS",
            Self::Manual => "Manual",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lock_period_days() {
        assert_eq!(LockPeriod::Day15.days(), 15);
        assert_eq!(LockPeriod::Day30.days(), 30);
        assert_eq!(LockPeriod::Day90.days(), 90);
    }

    #[test]
    fn test_lock_period_all_ascending() {
        let days: Vec<u32> = LockPeriod::ALL.iter().map(|l| l.days()).collect();
        let mut sorted = days.clone();
        sorted.sort_unstable();
        assert_eq!(days, sorted, "ALL must be in ascending day order");
        assert_eq!(LockPeriod::ALL.len(), 7);
    }

    #[test]
    fn test_lien_priority_to_mismo() {
        assert_eq!(LienPriority::First.to_mismo(), "FirstLien");
        assert_eq!(LienPriority::Second.to_mismo(), "SecondLien");
        assert_eq!(LienPriority::Third.to_mismo(), "ThirdLien");
    }

    #[test]
    fn test_mi_coverage_type_monthly() {
        assert!(MiCoverageType::BorrowerPaidMonthly.has_monthly_premium());
        assert!(MiCoverageType::FhaUpfrontAndAnnual.has_monthly_premium());
        assert!(MiCoverageType::UsdaUpfrontAndAnnual.has_monthly_premium());
        assert!(!MiCoverageType::None.has_monthly_premium());
        assert!(!MiCoverageType::LenderPaid.has_monthly_premium());
        assert!(!MiCoverageType::VaFundingFee.has_monthly_premium());
    }

    #[test]
    fn test_mi_coverage_type_upfront() {
        assert!(MiCoverageType::BorrowerPaidSingle.has_upfront_premium());
        assert!(MiCoverageType::FhaUpfrontAndAnnual.has_upfront_premium());
        assert!(MiCoverageType::VaFundingFee.has_upfront_premium());
        assert!(MiCoverageType::UsdaUpfrontAndAnnual.has_upfront_premium());
        assert!(!MiCoverageType::None.has_upfront_premium());
        assert!(!MiCoverageType::BorrowerPaidMonthly.has_upfront_premium());
    }

    #[test]
    fn test_aus_type_to_mismo() {
        assert_eq!(AusType::DesktopUnderwriter.to_mismo(), "DesktopUnderwriter");
        assert_eq!(AusType::LoanProductAdvisor.to_mismo(), "LoanProductAdvisor");
        assert_eq!(AusType::Got.to_mismo(), "FHATotalScorecard");
        assert_eq!(AusType::Gus.to_mismo(), "USDARuralHousingGUS");
        assert_eq!(AusType::Manual.to_mismo(), "Manual");
    }

    #[test]
    fn test_misc_enums_serde_json() {
        macro_rules! serde_rt {
            ($val:expr, $expected_json:expr) => {{
                let json = serde_json::to_string(&$val).unwrap();
                assert_eq!(json, $expected_json, "serialization mismatch for {:?}", $val);
                let back = serde_json::from_str(&json).unwrap();
                assert_eq!($val, back, "deserialization roundtrip failed for {:?}", $val);
            }};
        }
        serde_rt!(LockPeriod::Day30, "\"day30\"");
        serde_rt!(LienPriority::First, "\"first\"");
        serde_rt!(BalanceType::HighBalance, "\"high_balance\"");
        serde_rt!(Tier::Elite, "\"elite\"");
        serde_rt!(MiCoverageType::BorrowerPaidMonthly, "\"borrower_paid_monthly\"");
        serde_rt!(AusType::DesktopUnderwriter, "\"desktop_underwriter\"");
    }
}
