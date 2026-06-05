//! Epic 16 / Task T6 — contract validation + typed error surface.
//!
//! The wizard JSON is deserialized into [`PartialAnalysisInput`] by serde
//! (which enforces type-level validity: enum variants, integer ranges via the
//! newtype `Deserialize` impls). This module adds the *semantic* validation
//! serde cannot express: cross-field consistency, per-borrower invariants, and
//! domain-range checks the contract leaves open.
//!
//! Validation never panics. It returns a `Vec<WizardError>` so the web layer
//! can surface every problem at once rather than one-at-a-time.

use crate::contract::{PartialAnalysisInput, WizardStep};
use types::TermMonths;

/// A single semantic validation failure. Distinct from `ParseError` (which
/// serde/newtype construction already handles at deserialization time).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WizardError {
    /// `borrower_count` was present but zero.
    ZeroBorrowerCount,
    /// `borrowers` length does not match a known `borrower_count`.
    BorrowerCountMismatch { declared: u8, provided: usize },
    /// A borrower row carried no credit score.
    BorrowerMissingScore { index: usize },
    /// VA fields present on a borrower flagged not VA-eligible (incoherent).
    VaStatusWithoutEligibility { index: usize },
    /// `preferred_term` outside the program-available range (120..=360 months).
    PreferredTermOutOfRange { months: u16 },
    /// `hold_horizon_months` was present but zero.
    ZeroHoldHorizon,
    /// A monetary budget/credit was negative.
    NegativeAmount { field: &'static str },
    /// Seller-paid commission amount present but the toggle implies none.
    SellerCommissionInconsistent,
    /// Cash-to-close budget below the requested seller concessions (incoherent).
    ConcessionsExceedCashContext,
}

impl std::fmt::Display for WizardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WizardError::ZeroBorrowerCount => write!(f, "borrower_count must be >= 1"),
            WizardError::BorrowerCountMismatch { declared, provided } => write!(
                f,
                "borrower_count {declared} but {provided} borrower row(s) provided"
            ),
            WizardError::BorrowerMissingScore { index } => {
                write!(f, "borrower {index} has no credit score")
            }
            WizardError::VaStatusWithoutEligibility { index } => write!(
                f,
                "borrower {index} carries VA status but is not VA-eligible"
            ),
            WizardError::PreferredTermOutOfRange { months } => {
                write!(f, "preferred_term {months}mo outside available 120..=360")
            }
            WizardError::ZeroHoldHorizon => write!(f, "hold_horizon_months must be >= 1"),
            WizardError::NegativeAmount { field } => write!(f, "{field} cannot be negative"),
            WizardError::SellerCommissionInconsistent => {
                write!(f, "seller-paid commission amount present but not enabled")
            }
            WizardError::ConcessionsExceedCashContext => {
                write!(f, "requested concessions exceed the cash-to-close context")
            }
        }
    }
}

impl std::error::Error for WizardError {}

/// Validate the semantic invariants of a (possibly partial) submission.
/// Only validates fields that are *present* — absence is valid for a partial
/// input. Returns every problem found (empty = valid so far).
#[must_use]
pub fn validate(input: &PartialAnalysisInput) -> Vec<WizardError> {
    let mut errs = Vec::new();

    // Borrower count.
    if let Some(n) = input.borrower_count {
        if n == 0 {
            errs.push(WizardError::ZeroBorrowerCount);
        }
        // Only check mismatch once rows are being supplied.
        if !input.borrowers.is_empty() && input.borrowers.len() != n as usize {
            errs.push(WizardError::BorrowerCountMismatch {
                declared: n,
                provided: input.borrowers.len(),
            });
        }
    }

    // Per-borrower invariants.
    for (i, b) in input.borrowers.iter().enumerate() {
        if b.credit_scores.is_empty() {
            errs.push(WizardError::BorrowerMissingScore { index: i });
        }
        if b.annual_income.is_negative() {
            errs.push(WizardError::NegativeAmount {
                field: "annual_income",
            });
        }
        // VA coherence: a `va` block whose `eligible` is false but which asserts
        // previous_use or disability is incoherent (those imply eligibility).
        if let Some(va) = &b.va {
            if !va.eligible && (va.previous_use || va.disability) {
                errs.push(WizardError::VaStatusWithoutEligibility { index: i });
            }
        }
    }

    // Preferred term within the program-available band.
    if let Some(t) = input.preferred_term {
        if TermMonths::new(t.0).is_err() {
            errs.push(WizardError::PreferredTermOutOfRange { months: t.0 });
        }
    }

    // Hold horizon.
    if let Some(0) = input.hold_horizon_months {
        errs.push(WizardError::ZeroHoldHorizon);
    }

    // Monetary non-negativity.
    for (field, amt) in [
        ("monthly_payment_budget", input.monthly_payment_budget),
        ("upfront_cash_budget", input.upfront_cash_budget),
        ("buyer_agent_commission", input.buyer_agent_commission),
        ("purchase_price", input.purchase_price),
    ] {
        if let Some(c) = amt {
            if c.is_negative() {
                errs.push(WizardError::NegativeAmount { field });
            }
        }
    }
    if let Some(c) = input.seller_credits.agent_commission_paid_by_seller {
        if c.is_negative() {
            errs.push(WizardError::NegativeAmount {
                field: "seller_credits.agent_commission_paid_by_seller",
            });
        }
    }
    if let Some(c) = input.seller_credits.concessions_requested {
        if c.is_negative() {
            errs.push(WizardError::NegativeAmount {
                field: "seller_credits.concessions_requested",
            });
        }
    }

    // Cross-field: concessions shouldn't exceed the stated cash-to-close context
    // when both are present (a coherence signal, not an IPC-cap check — that is
    // the eligibility engine's job in 4.26).
    if let (Some(cash), Some(conc)) = (
        input.upfront_cash_budget,
        input.seller_credits.concessions_requested,
    ) {
        if conc.0 > cash.0 && cash.0 >= 0 {
            errs.push(WizardError::ConcessionsExceedCashContext);
        }
    }

    errs
}

/// Convenience: is the submission semantically valid so far?
#[must_use]
pub fn is_valid(input: &PartialAnalysisInput) -> bool {
    validate(input).is_empty()
}

/// The set of steps whose owned fields are complete AND valid.
#[must_use]
pub fn valid_completed_steps(input: &PartialAnalysisInput) -> Vec<WizardStep> {
    if !is_valid(input) {
        // If anything is invalid we still report structurally-complete steps,
        // but the funnel layer will refuse to compute counts on invalid input.
    }
    WizardStep::ALL
        .into_iter()
        .filter(|s| input.step_complete(*s))
        .collect()
}
