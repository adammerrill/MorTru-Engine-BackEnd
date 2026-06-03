//! Lender profiles and per-lender overlays on agency program rules.
//!
//! **Overlays can only tighten** — a lender may require a higher FICO than the
//! agency minimum, a lower max LTV, or a stricter DTI cap, but never looser.
//! The `apply()` method enforces this invariant.
//!
//! # Updating lender data
//!
//! Add or edit entries in `data/lender_profiles.json`. The file is loaded at
//! store startup; no Rust code changes are needed when lenders are added,
//! removed, or when an overlay changes.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use types::ProgramCode;

use crate::program_rules::ProgramEligibilityRules;

/// A lender's basic identity and active status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LenderProfile {
    pub lender_id: String,
    pub name: String,
    pub nmls_id: Option<String>,
    pub active: bool,
}

/// Per-lender, per-program rule overrides.
///
/// `None` means "no overlay — use agency guideline."
/// Overlays are enforced in `apply()` to only tighten, not loosen.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LenderOverlays {
    pub lender_id: String,
    pub program: ProgramCode,
    /// Override minimum credit score. Must be ≥ agency minimum.
    pub min_credit_score_override: Option<u16>,
    /// Override maximum LTV in basis points. Must be ≤ agency maximum.
    pub max_ltv_bps_override: Option<u32>,
    /// Override maximum front-end DTI in basis points. Must be ≤ agency maximum.
    pub dti_max_bps_override: Option<u32>,
    pub effective_date: NaiveDate,
}

impl LenderOverlays {
    /// Apply this lender's overlays to agency rules, returning a new (tightened) rule set.
    ///
    /// Each field is replaced only if the overlay is stricter. This is the enforcement
    /// point ensuring overlays never loosen agency guidelines.
    #[must_use]
    pub fn apply(&self, agency: &ProgramEligibilityRules) -> ProgramEligibilityRules {
        ProgramEligibilityRules {
            min_credit_score: match self.min_credit_score_override {
                Some(o) if o > agency.min_credit_score => o,
                _ => agency.min_credit_score,
            },
            max_ltv_bps: match self.max_ltv_bps_override {
                Some(o) if o < agency.max_ltv_bps => o,
                _ => agency.max_ltv_bps,
            },
            front_end_dti_max_bps: match self.dti_max_bps_override {
                Some(o) if o < agency.front_end_dti_max_bps => o,
                _ => agency.front_end_dti_max_bps,
            },
            ..*agency
        }
    }
}

/// Top-level shape of `lender_profiles.json`.
#[derive(Debug, Deserialize)]
pub struct LenderProfileFile {
    pub lenders: Vec<LenderProfile>,
    pub overlays: Vec<LenderOverlays>,
}
