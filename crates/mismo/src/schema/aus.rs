//! MISMO 3.4 AUS and qualification schema.
//!
//! Covers:
//! - `AUTOMATED_UNDERWRITING_SYSTEM` — system type and recommendation
//! - `QUALIFICATION` — qualifying rate, housing ratio, total DTI
//!
//! # Document location
//!
//! ```text
//! MESSAGE/DEAL_SETS/DEAL_SET/DEALS/DEAL/LOANS/LOAN/
//!   ├── AUTOMATED_UNDERWRITING_SYSTEMS/
//!   │     └── AUTOMATED_UNDERWRITING_SYSTEM  ← MismoAus
//!   └── QUALIFICATION                        ← MismoQualification
//! ```
//!
//! # AUS systems and their recommendation terminology
//!
//! | System | Approval term | Adverse term |
//! |---|---|---|
//! | DU (Fannie Mae) | Approve/Eligible | Refer / Refer with Caution |
//! | LPA (Freddie Mac) | Accept | Caution |
//! | FHA TOTAL Scorecard | Approve/Eligible (via DU or LPA) | Refer |
//! | GUS (USDA) | Accept/Eligible | Refer |
//!
//! All approval variants map to `AusRecommendation::ApproveEligible`.
//!
//! # Qualifying rate vs note rate
//!
//! For fixed-rate loans: qualifying rate == note rate.
//! For ARMs: qualifying rate is the fully-indexed rate (not the start rate),
//! used for DTI qualification per CFPB ATR rules.

use types::{AusType, BasisPoints, DtiBasisPoints};

use crate::enums::aus::AusRecommendation;

// ── Parsing helpers ───────────────────────────────────────────────────────────

/// Parse a percentage string to `BasisPoints` using the ×1000 scale.
/// `"6.375"` → `BasisPoints(6375)` (interest rate convention).
fn parse_optional_rate_bps(
    opt: &Option<String>,
    element: &'static str,
) -> crate::Result<Option<BasisPoints>> {
    match opt.as_deref() {
        None | Some("") => Ok(None),
        Some(s) => BasisPoints::from_percentage_str(s).map(Some).map_err(|_| {
            crate::MismoError::OutOfRange {
                element,
                detail: format!("'{s}' is not a valid rate percentage"),
            }
        }),
    }
}

/// Parse a DTI percentage string to `DtiBasisPoints`.
/// `"43.0"` → `DtiBasisPoints(4300)` (1 unit = 0.01% DTI).
///
/// Uses the `BasisPoints` ×1000 intermediate, then divides by 10:
/// `"43.0"` → `BasisPoints(43000)` → `DtiBasisPoints(4300)`.
fn parse_optional_dti(
    opt: &Option<String>,
    element: &'static str,
) -> crate::Result<Option<DtiBasisPoints>> {
    match opt.as_deref() {
        None | Some("") => Ok(None),
        Some(s) => {
            let bps =
                BasisPoints::from_percentage_str(s).map_err(|_| crate::MismoError::OutOfRange {
                    element,
                    detail: format!("'{s}' is not a valid DTI percentage"),
                })?;
            Ok(Some(DtiBasisPoints::new(bps.0 / 10)))
        }
    }
}

// ── MismoAus XML struct ───────────────────────────────────────────────────────

/// MISMO 3.4 `AUTOMATED_UNDERWRITING_SYSTEM` element.
///
/// One element per AUS submission. A loan may have been run through multiple
/// systems (e.g. both DU and LPA for comparison); this struct represents one.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename = "AUTOMATED_UNDERWRITING_SYSTEM")]
pub struct MismoAus {
    /// AUS system name.
    /// `"DesktopUnderwriter"` | `"LoanProductAdvisor"` |
    /// `"FHATotalScorecard"` | `"USDARuralHousingGUS"` | `"Manual"`
    #[serde(rename = "AutomatedUnderwritingSystemType")]
    pub system_type: String,

    /// AUS finding / recommendation.
    /// `"Approve"` | `"ApproveEligible"` | `"Accept"` |
    /// `"Refer"` | `"ReferWithCaution"` | `"Ineligible"` | `"OutOfScope"`
    #[serde(
        rename = "AUSRecommendationType",
        skip_serializing_if = "Option::is_none"
    )]
    pub recommendation: Option<String>,

    /// Casefile ID assigned by DU/LPA for reference and re-submission.
    #[serde(rename = "AUSCaseIdentifier", skip_serializing_if = "Option::is_none")]
    pub case_id: Option<String>,
}

// ── MismoQualification XML struct ─────────────────────────────────────────────

/// MISMO 3.4 `QUALIFICATION` element — qualifying ratios.
///
/// These ratios are computed by the AUS or the underwriter and stored here
/// for disclosure on the Loan Estimate and Closing Disclosure.
///
/// # Housing ratio (front-end DTI)
/// `housing_expense / gross_monthly_income`
/// Includes: P&I + taxes/12 + HOI/12 + MI/month + HOA/month
///
/// # Total DTI (back-end DTI)
/// `(housing_expense + all_monthly_liabilities) / gross_monthly_income`
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename = "QUALIFICATION")]
pub struct MismoQualification {
    /// Qualifying interest rate as a percentage string.
    /// For fixed: equals note rate. For ARM: fully-indexed rate per ATR.
    /// e.g. `"6.375"` → `BasisPoints(6375)`.
    #[serde(
        rename = "QualifyingRatePercent",
        skip_serializing_if = "Option::is_none"
    )]
    pub qualifying_rate: Option<String>,

    /// Front-end (housing expense) ratio as a percentage.
    /// e.g. `"28.50"` → `DtiBasisPoints(2850)`.
    #[serde(
        rename = "HousingExpenseRatio",
        skip_serializing_if = "Option::is_none"
    )]
    pub housing_ratio: Option<String>,

    /// Back-end (total debt) ratio as a percentage.
    /// e.g. `"43.00"` → `DtiBasisPoints(4300)`.
    #[serde(
        rename = "TotalDebtExpenseRatio",
        skip_serializing_if = "Option::is_none"
    )]
    pub total_dti: Option<String>,
}

// ── Parsed output types ───────────────────────────────────────────────────────

/// Typed AUS submission result — output of [`MismoAus::parse`].
#[derive(Debug, Clone)]
pub struct AusParsed {
    /// Which AUS system was used.
    pub system: AusType,
    /// AUS recommendation, or `None` if the submission has not yet returned.
    pub recommendation: Option<AusRecommendation>,
    /// Casefile ID for re-submission or audit trail.
    pub case_id: Option<String>,
}

impl AusParsed {
    /// True if the AUS recommendation allows the loan to proceed to closing.
    #[must_use]
    pub fn is_approvable(&self) -> bool {
        self.recommendation
            .map(|r| r.is_approvable())
            .unwrap_or(false)
    }
}

/// Typed qualifying ratios — output of [`MismoQualification::parse`].
#[derive(Debug, Clone)]
pub struct QualificationParsed {
    /// Qualifying rate (fixed = note rate; ARM = fully-indexed rate).
    pub qualifying_rate: Option<BasisPoints>,
    /// Front-end (housing expense) DTI ratio.
    pub housing_ratio: Option<DtiBasisPoints>,
    /// Back-end (total debt) DTI ratio.
    pub total_dti: Option<DtiBasisPoints>,
}

// ── Parse implementations ─────────────────────────────────────────────────────

impl MismoAus {
    /// Parse raw XML strings into a typed [`AusParsed`].
    ///
    /// # Errors
    /// Returns [`crate::MismoError::InvalidEnum`] for unknown system types or
    /// unknown recommendation strings.
    pub fn parse(&self) -> crate::Result<AusParsed> {
        let system = crate::enums::aus::try_aus_type(&self.system_type)?;
        let recommendation = self
            .recommendation
            .as_deref()
            .filter(|s| !s.is_empty())
            .map(AusRecommendation::try_from_str)
            .transpose()?;
        Ok(AusParsed {
            system,
            recommendation,
            case_id: self.case_id.clone(),
        })
    }
}

impl MismoQualification {
    /// Parse raw XML strings into a typed [`QualificationParsed`].
    ///
    /// # Errors
    /// Returns [`crate::MismoError::OutOfRange`] for non-numeric rate or
    /// ratio fields.
    pub fn parse(&self) -> crate::Result<QualificationParsed> {
        Ok(QualificationParsed {
            qualifying_rate: parse_optional_rate_bps(
                &self.qualifying_rate,
                "QualifyingRatePercent",
            )?,
            housing_ratio: parse_optional_dti(&self.housing_ratio, "HousingExpenseRatio")?,
            total_dti: parse_optional_dti(&self.total_dti, "TotalDebtExpenseRatio")?,
        })
    }
}
