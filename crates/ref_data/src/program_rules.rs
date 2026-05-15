//! Program eligibility rules — the guardrails evaluated before geographic checks.
//!
//! These are agency/GSE guidelines, not geographic data. Every scenario
//! runs through program rules first; geographic checks only run if the
//! program rules pass.
//!
//! # Evaluation order
//!
//! 1. Credit score ≥ minimum               → else CREDIT_SCORE_BELOW_MINIMUM
//! 2. LTV ≤ maximum for credit tier        → else LTV_EXCEEDS_PROGRAM_MAXIMUM
//! 3. Front-end DTI ≤ maximum              → else FRONT_END_DTI_EXCEEDED
//! 4. Geographic checks (FHA limit, USDA)  → GeoEligibility
//!
//! # VA note
//!
//! VA does not use front-end DTI. The engine sets `front_end_dti_max_bps`
//! to 9999 (99.99%) for VA so the DTI check always passes. VA eligibility
//! is determined by residual income, which requires a licensed loan
//! officer and is out of scope.

use serde::{Deserialize, Serialize};
use types::{CreditScore, DtiBasisPoints, LtvBasisPoints, ProgramCode};

use crate::error::{RefDataError, RefDataResult};

// ── ProgramEligibilityRules ───────────────────────────────────────────────────

/// Agency / GSE eligibility rules for one program.
///
/// These are the standard guidelines. Lender overlays (Task 4.16) can
/// tighten (but not loosen) these requirements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramEligibilityRules {
    /// The program these rules apply to.
    pub program: ProgramCode,

    // ── Credit score minimums ─────────────────────────────────────────────
    /// Standard minimum representative credit score.
    pub min_credit_score: u16,
    /// Some programs (FHA only) allow a lower score with a higher down payment.
    /// `None` for programs with a single fixed minimum.
    pub min_credit_score_alt: Option<u16>,
    /// Down payment threshold (as LTV bps, inverted) to qualify for the
    /// alternative credit minimum. E.g. FHA: 1000 bps = 10% down.
    /// `None` if no alt tier.
    pub alt_credit_min_down_payment_bps: Option<u16>,

    // ── LTV maximums ──────────────────────────────────────────────────────
    /// Maximum LTV for standard credit tier (in basis points; 9650 = 96.50%).
    pub max_ltv_bps: u32,
    /// Maximum LTV when using the alternative (lower) credit minimum.
    /// E.g. FHA 500–579 credit: 9000 bps (90% LTV).
    pub max_ltv_bps_alt_credit: Option<u32>,
    /// Maximum LTV for high-balance loans (above standard national conforming limit).
    /// `None` if the program doesn't distinguish high-balance.
    pub max_ltv_bps_high_balance: Option<u32>,

    // ── Front-end DTI ─────────────────────────────────────────────────────
    /// Maximum front-end DTI in basis points (3100 = 31.00%).
    ///
    /// VA is set to 9999 because VA uses residual income, not DTI.
    /// This means the DTI check always passes for VA; the loan officer
    /// will verify residual income separately.
    pub front_end_dti_max_bps: u32,

    // ── Program-specific flags ────────────────────────────────────────────
    /// True if the program requires primary residence occupancy.
    pub requires_primary_residence: bool,
    /// True if at least one borrower must be a first-time homebuyer.
    pub requires_first_time_buyer: bool,
    /// True if the program requires VA entitlement.
    pub requires_va_entitlement: bool,
    /// True if the program requires USDA geographic + income eligibility.
    pub requires_usda_eligibility: bool,
    /// True if the program requires income ≤ 80% AMI (HomeReady/HP).
    pub requires_ami_income_check: bool,

    /// Effective date of these guidelines.
    pub effective_date: chrono::NaiveDate,
}

impl ProgramEligibilityRules {
    /// Minimum credit score for this program given the proposed down payment.
    ///
    /// FHA allows a lower minimum (500) with ≥10% down.
    /// All other programs have a single minimum.
    #[must_use]
    pub fn min_credit_score_for_down_payment(&self, down_payment_bps: u32) -> u16 {
        if let (Some(alt_min), Some(alt_dp)) = (
            self.min_credit_score_alt,
            self.alt_credit_min_down_payment_bps,
        ) {
            if down_payment_bps >= u32::from(alt_dp) {
                return alt_min;
            }
        }
        self.min_credit_score
    }

    /// Maximum LTV for this program given the credit score and high-balance flag.
    #[must_use]
    pub fn max_ltv_for(&self, credit_score: CreditScore, is_high_balance: bool) -> LtvBasisPoints {
        // High-balance override first
        if is_high_balance {
            if let Some(hb_max) = self.max_ltv_bps_high_balance {
                return LtvBasisPoints(hb_max);
            }
        }
        // Alt credit tier (FHA 500-579 → 90% max)
        if let (Some(alt_min), Some(alt_max)) =
            (self.min_credit_score_alt, self.max_ltv_bps_alt_credit)
        {
            if credit_score.0 < self.min_credit_score && credit_score.0 >= alt_min {
                return LtvBasisPoints(alt_max);
            }
        }
        LtvBasisPoints(self.max_ltv_bps)
    }

    /// Front-end DTI limit for this program.
    #[must_use]
    pub fn front_end_dti_limit(&self) -> DtiBasisPoints {
        DtiBasisPoints::new(self.front_end_dti_max_bps)
    }

    /// True if `credit_score` meets the minimum for the given down payment.
    #[must_use]
    pub fn credit_score_eligible(&self, score: CreditScore, down_payment_bps: u32) -> bool {
        let min = self.min_credit_score_for_down_payment(down_payment_bps);
        score.0 >= min
    }

    /// True if `ltv` is within the maximum for the given credit score and
    /// high-balance status.
    #[must_use]
    pub fn ltv_eligible(
        &self,
        ltv: LtvBasisPoints,
        credit_score: CreditScore,
        is_high_balance: bool,
    ) -> bool {
        ltv <= self.max_ltv_for(credit_score, is_high_balance)
    }

    /// True if `front_end_dti` is within the program's maximum.
    #[must_use]
    pub fn dti_eligible(&self, front_end_dti: DtiBasisPoints) -> bool {
        front_end_dti <= self.front_end_dti_limit()
    }
}

// ── AllProgramRules ───────────────────────────────────────────────────────────

/// Collection of rules for all programs, used as a single store lookup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllProgramRules(pub Vec<ProgramEligibilityRules>);

impl AllProgramRules {
    /// Find rules for a specific program.
    pub fn for_program(&self, program: ProgramCode) -> RefDataResult<&ProgramEligibilityRules> {
        self.0
            .iter()
            .find(|r| r.program == program)
            .ok_or_else(|| RefDataError::NotFound {
                data_type: "ProgramEligibilityRules",
                fips: program_code_str(program).to_owned(),
                year: 0,
            })
    }
}

fn program_code_str(p: ProgramCode) -> &'static str {
    match p {
        ProgramCode::Conventional => "Conventional",
        ProgramCode::HomeReady => "HomeReady",
        ProgramCode::HomePossible => "HomePossible",
        ProgramCode::HomeOne => "HomeOne",
        ProgramCode::Fha => "Fha",
        ProgramCode::FhaDpa => "FhaDpa",
        ProgramCode::Va => "Va",
        ProgramCode::VaJumbo => "VaJumbo",
        ProgramCode::Usda => "Usda",
        ProgramCode::Bond => "Bond",
        ProgramCode::Jumbo => "Jumbo",
        ProgramCode::NonQm => "NonQm",
    }
}
