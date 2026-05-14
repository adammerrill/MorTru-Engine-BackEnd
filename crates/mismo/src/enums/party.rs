//! Party/borrower MISMO enumeration types.
//!
//! Defines the VA funding fee tier — a computed value derived from first-use
//! status and LTV — and the affordable lending program classification used for
//! HomeReady/HomePossible/HomeOne eligibility.

use types::{BasisPoints, LtvBasisPoints};

// ── VA Funding Fee Tier ───────────────────────────────────────────────────────

/// VA funding fee tier, computed from first-use status and down payment %.
///
/// | Tier | Rate |
/// |---|---|
/// | `FirstUseBelow5Pct` | 2.15% |
/// | `FirstUse5To10Pct` | 1.50% |
/// | `FirstUseAbove10Pct` | 1.25% |
/// | `SubsequentBelow5Pct` | 3.30% |
/// | `Subsequent5To10Pct` | 1.50% |
/// | `SubsequentAbove10Pct` | 1.25% |
/// | `CashOutRefiFirstUse` | 2.15% |
/// | `CashOutRefiSubsequent` | 3.30% |
/// | `Irrrl` | 0.50% |
/// | `Exempt` | 0.00% — 10%+ service-connected disability |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VaFundingFeeTier {
    /// First-time VA use, LTV > 95% (< 5% down).
    FirstUseBelow5Pct,
    /// First-time VA use, LTV 90.01–95% (5–9.99% down).
    FirstUse5To10Pct,
    /// First-time VA use, LTV <= 90% (10%+ down).
    FirstUseAbove10Pct,
    /// Subsequent VA use, LTV > 95% (< 5% down).
    SubsequentBelow5Pct,
    /// Subsequent VA use, LTV 90.01–95% (5–9.99% down).
    Subsequent5To10Pct,
    /// Subsequent VA use, LTV <= 90% (10%+ down).
    SubsequentAbove10Pct,
    /// Cash-out refinance, first-time VA use.
    CashOutRefiFirstUse,
    /// Cash-out refinance, subsequent VA use.
    CashOutRefiSubsequent,
    /// Interest Rate Reduction Refinance Loan (VA streamline).
    Irrrl,
    /// Veteran has 10%+ service-connected disability — fee waived.
    Exempt,
}

impl VaFundingFeeTier {
    /// Returns the upfront funding fee as basis points of the base loan amount.
    #[must_use]
    pub const fn rate_bps(self) -> BasisPoints {
        match self {
            Self::FirstUseBelow5Pct => BasisPoints(215),
            Self::FirstUse5To10Pct => BasisPoints(150),
            Self::FirstUseAbove10Pct => BasisPoints(125),
            Self::SubsequentBelow5Pct => BasisPoints(330),
            Self::Subsequent5To10Pct => BasisPoints(150),
            Self::SubsequentAbove10Pct => BasisPoints(125),
            Self::CashOutRefiFirstUse => BasisPoints(215),
            Self::CashOutRefiSubsequent => BasisPoints(330),
            Self::Irrrl => BasisPoints(50),
            Self::Exempt => BasisPoints(0),
        }
    }

    /// Derive the correct tier from borrower inputs.
    ///
    /// # Parameters
    /// - `first_use` — true if this is the borrower's first VA use
    /// - `ltv` — loan-to-value at origination (in LtvBasisPoints)
    /// - `is_cash_out_refi` — true for cash-out refinance
    /// - `is_irrrl` — true for VA streamline refinance
    /// - `exempt` — true for 10%+ service-connected disability
    #[must_use]
    pub fn from_inputs(
        first_use: bool,
        ltv: LtvBasisPoints,
        is_cash_out_refi: bool,
        is_irrrl: bool,
        exempt: bool,
    ) -> Self {
        if exempt {
            return Self::Exempt;
        }
        if is_irrrl {
            return Self::Irrrl;
        }
        if is_cash_out_refi {
            return if first_use {
                Self::CashOutRefiFirstUse
            } else {
                Self::CashOutRefiSubsequent
            };
        }
        // LTV > 9500 bps (> 95%) = < 5% down
        // LTV 9001–9500 bps (90.01–95%) = 5–9.99% down
        // LTV <= 9000 bps (<= 90%) = >= 10% down
        if first_use {
            if ltv.0 > 9500 {
                Self::FirstUseBelow5Pct
            } else if ltv.0 > 9000 {
                Self::FirstUse5To10Pct
            } else {
                Self::FirstUseAbove10Pct
            }
        } else if ltv.0 > 9500 {
            Self::SubsequentBelow5Pct
        } else if ltv.0 > 9000 {
            Self::Subsequent5To10Pct
        } else {
            Self::SubsequentAbove10Pct
        }
    }
}

// ── Affordable Lending Program ────────────────────────────────────────────────

/// Affordable lending program classification.
///
/// HomeReady (Fannie Mae) and HomePossible (Freddie Mac) require the
/// borrower's income to be <= 80% AMI for the property census tract.
/// HomeOne (Freddie Mac) is first-time-buyer only with no income limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AffordableLendingProgram {
    /// Fannie Mae HomeReady — 97% LTV, 3% min down, income <= 80% AMI.
    HomeReady,
    /// Freddie Mac Home Possible — 97% LTV, 3% min down, income <= 80% AMI.
    HomePossible,
    /// Freddie Mac HomeOne — first-time buyer only, no income limit, 3% down.
    HomeOne,
    /// No affordable lending program applies.
    None,
}

impl AffordableLendingProgram {
    /// Parse from a MISMO extension string.
    ///
    /// # Errors
    /// Returns `MismoError::InvalidEnum` for unrecognised values.
    pub fn try_from_str(s: &str) -> crate::Result<Self> {
        match s.trim() {
            "HomeReady" => Ok(Self::HomeReady),
            "HomePossible" => Ok(Self::HomePossible),
            "HomeOne" => Ok(Self::HomeOne),
            "None" | "" => Ok(Self::None),
            _ => Err(crate::MismoError::InvalidEnum {
                element: "AffordableLendingProgramType",
                value: s.to_owned(),
            }),
        }
    }
}
