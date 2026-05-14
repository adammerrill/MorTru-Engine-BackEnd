//! AUS (Automated Underwriting System) MISMO enumeration types.
//!
//! `AusType` already exists in the `types` crate with `to_mismo()` but
//! without `from_mismo()`. This module provides both the inverse parse
//! function and the `AusRecommendation` outcome type.

use types::AusType;

// ── AusType ───────────────────────────────────────────────────────────────────

/// Convert a MISMO 3.4 `AutomatedUnderwritingSystemType` string to
/// `types::AusType`.
///
/// | MISMO value | `AusType` |
/// |---|---|
/// | `"DesktopUnderwriter"` | `DesktopUnderwriter` |
/// | `"LoanProductAdvisor"` | `LoanProductAdvisor` |
/// | `"FHATotalScorecard"` | `Got` |
/// | `"USDARuralHousingGUS"` | `Gus` |
/// | `"Manual"` | `Manual` |
///
/// This is the inverse of `AusType::to_mismo()`.
///
/// # Errors
/// Returns `MismoError::InvalidEnum` for unrecognised values.
pub fn try_aus_type(s: &str) -> crate::Result<AusType> {
    match s.trim() {
        "DesktopUnderwriter" => Ok(AusType::DesktopUnderwriter),
        "LoanProductAdvisor" => Ok(AusType::LoanProductAdvisor),
        "FHATotalScorecard" => Ok(AusType::Got),
        "USDARuralHousingGUS" => Ok(AusType::Gus),
        "Manual" => Ok(AusType::Manual),
        _ => Err(crate::MismoError::InvalidEnum {
            element: "AutomatedUnderwritingSystemType",
            value: s.to_owned(),
        }),
    }
}

// ── AUS Recommendation ───────────────────────────────────────────────────────

/// The AUS recommendation outcome for a loan submission.
///
/// DU (Fannie Mae) and LPA (Freddie Mac) use slightly different terminology
/// for the same outcomes; this enum normalises them.
///
/// | DU term | LPA term | Variant |
/// |---|---|---|
/// | Approve/Eligible | Accept | `ApproveEligible` |
/// | Approve/Ineligible | — | `ApproveIneligible` |
/// | Refer | Caution | `Refer` |
/// | Refer with Caution | — | `ReferWithCaution` |
/// | Out of Scope | Ineligible | `Ineligible` |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AusRecommendation {
    /// DU: Approve/Eligible — LPA: Accept.
    /// Loan meets guidelines; eligible for sale to GSE. Clear to close.
    ApproveEligible,
    /// DU: Approve/Ineligible.
    /// Creditworthy borrower but the loan does not meet GSE eligibility
    /// (e.g. exceeds conforming limit). Eligible for portfolio lending.
    ApproveIneligible,
    /// DU: Refer — LPA: Caution.
    /// Does not meet AUS guidelines; requires manual underwriting.
    Refer,
    /// DU: Refer with Caution.
    /// High-risk; manual underwriting with additional documentation.
    ReferWithCaution,
    /// DU: Out of Scope — LPA: Ineligible.
    /// Does not meet program requirements at all.
    Ineligible,
}

impl AusRecommendation {
    /// Parse from a MISMO `AUSRecommendationType` string.
    ///
    /// # Errors
    /// Returns `MismoError::InvalidEnum` for unrecognised values.
    pub fn try_from_str(s: &str) -> crate::Result<Self> {
        match s.trim() {
            // DU approval — slash form used in live MISMO XML
            "Approve/Eligible"  |
            // DU approval — camelCase form
            "Approve"           |
            "ApproveEligible"   |
            // LPA approval
            "Accept"            |
            "AcceptEligible"    => Ok(Self::ApproveEligible),

            "ApproveIneligible" => Ok(Self::ApproveIneligible),

            // DU / LPA refer
            "Refer"             |
            "Caution"           => Ok(Self::Refer),

            "ReferWithCaution"  => Ok(Self::ReferWithCaution),

            // Out of scope / ineligible
            "OutOfScope"        |
            "Ineligible"        => Ok(Self::Ineligible),

            _ => Err(crate::MismoError::InvalidEnum {
                element: "AUSRecommendationType",
                value:   s.to_owned(),
            }),
        }
    }

    /// Returns true if this recommendation allows the loan to close.
    #[must_use]
    pub const fn is_approvable(self) -> bool {
        matches!(self, Self::ApproveEligible | Self::ApproveIneligible)
    }
}
