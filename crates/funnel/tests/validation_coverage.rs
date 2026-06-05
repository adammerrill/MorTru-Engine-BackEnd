//! Epic 16 / T6 — coverage-gap tests: exercise every `WizardError` `Display`
//! arm and the `validate()` branches the primary suite doesn't reach.

use funnel::*;
use types::{Cents, CreditScore, Occupancy, TermMonths};

fn score(n: u16) -> CreditScore {
    CreditScore::new(n).unwrap()
}
fn borrower(scores: Vec<CreditScore>, income: i64) -> BorrowerInput {
    BorrowerInput {
        occupancy: Occupancy::PrimaryResidence,
        credit_scores: scores,
        va: None,
        annual_income: Cents::from_dollars(income),
    }
}

// ── Display coverage: every variant must render ─────────────────────────────

#[test]
fn display_renders_all_variants() {
    let variants = [
        WizardError::ZeroBorrowerCount,
        WizardError::BorrowerCountMismatch {
            declared: 2,
            provided: 1,
        },
        WizardError::BorrowerMissingScore { index: 0 },
        WizardError::VaStatusWithoutEligibility { index: 1 },
        WizardError::PreferredTermOutOfRange { months: 999 },
        WizardError::ZeroHoldHorizon,
        WizardError::NegativeAmount {
            field: "monthly_payment_budget",
        },
        WizardError::SellerCommissionInconsistent,
        WizardError::ConcessionsExceedCashContext,
    ];
    for v in &variants {
        let s = format!("{v}");
        assert!(!s.is_empty(), "Display produced empty string for {v:?}");
    }
}

#[test]
fn wizard_error_is_std_error() {
    // Exercise the std::error::Error impl path.
    let e = WizardError::ZeroBorrowerCount;
    let dyn_err: &dyn std::error::Error = &e;
    assert!(!dyn_err.to_string().is_empty());
}

// ── validate() branches not hit by the primary suite ───────────────────────

#[test]
fn per_borrower_negative_income_detected() {
    let mut p = PartialAnalysisInput {
        borrower_count: Some(1),
        ..Default::default()
    };
    p.borrowers.push(borrower(vec![score(700)], -1));
    assert!(validate(&p)
        .iter()
        .any(|e| matches!(e, WizardError::NegativeAmount { field: "annual_income" })));
}

#[test]
fn preferred_term_out_of_range_detected() {
    // TermMonths::new validates 120..=360; construct a raw out-of-range value
    // via the pub tuple field to exercise the validate() range re-check.
    let p = PartialAnalysisInput {
        preferred_term: Some(TermMonths(999)),
        ..Default::default()
    };
    assert!(validate(&p)
        .iter()
        .any(|e| matches!(e, WizardError::PreferredTermOutOfRange { months: 999 })));
}

#[test]
fn valid_preferred_term_passes() {
    let p = PartialAnalysisInput {
        preferred_term: Some(TermMonths(360)),
        ..Default::default()
    };
    assert!(!validate(&p)
        .iter()
        .any(|e| matches!(e, WizardError::PreferredTermOutOfRange { .. })));
}

#[test]
fn seller_commission_negative_detected() {
    let mut p = PartialAnalysisInput::default();
    p.seller_credits.agent_commission_paid_by_seller = Some(Cents(-500));
    assert!(validate(&p).iter().any(|e| matches!(
        e,
        WizardError::NegativeAmount {
            field: "seller_credits.agent_commission_paid_by_seller"
        }
    )));
}

#[test]
fn seller_concessions_negative_detected() {
    let mut p = PartialAnalysisInput::default();
    p.seller_credits.concessions_requested = Some(Cents(-100));
    assert!(validate(&p).iter().any(|e| matches!(
        e,
        WizardError::NegativeAmount {
            field: "seller_credits.concessions_requested"
        }
    )));
}

#[test]
fn negative_purchase_price_and_commission_detected() {
    let p = PartialAnalysisInput {
        purchase_price: Some(Cents(-1)),
        buyer_agent_commission: Some(Cents(-1)),
        upfront_cash_budget: Some(Cents(-1)),
        ..Default::default()
    };
    let errs = validate(&p);
    assert!(errs
        .iter()
        .any(|e| matches!(e, WizardError::NegativeAmount { field: "purchase_price" })));
    assert!(errs
        .iter()
        .any(|e| matches!(e, WizardError::NegativeAmount { field: "buyer_agent_commission" })));
    assert!(errs
        .iter()
        .any(|e| matches!(e, WizardError::NegativeAmount { field: "upfront_cash_budget" })));
}

#[test]
fn concessions_within_cash_is_valid() {
    let mut p = PartialAnalysisInput {
        upfront_cash_budget: Some(Cents::from_dollars(10_000)),
        ..Default::default()
    };
    p.seller_credits.concessions_requested = Some(Cents::from_dollars(3_000));
    assert!(!validate(&p).contains(&WizardError::ConcessionsExceedCashContext));
}

// ── valid_completed_steps on an INVALID input (the !is_valid branch) ────────

#[test]
fn valid_completed_steps_on_invalid_input() {
    // Invalid (zero borrower count) but structurally has steps answered.
    let p = PartialAnalysisInput {
        borrower_count: Some(0),
        property_use: Some(Occupancy::PrimaryResidence),
        ..Default::default()
    };
    assert!(!is_valid(&p));
    // Still returns structurally-complete steps without panicking.
    let _ = valid_completed_steps(&p);
}
