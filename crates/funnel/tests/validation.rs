//! Epic 16 / T6 — validation tests.

use funnel::*;
use types::{Cents, CreditScore, Occupancy};

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

#[test]
fn empty_input_is_valid() {
    let p = PartialAnalysisInput::default();
    assert!(is_valid(&p), "{:?}", validate(&p));
}

#[test]
fn zero_borrower_count_rejected() {
    let p = PartialAnalysisInput {
        borrower_count: Some(0),
        ..Default::default()
    };
    assert!(validate(&p).contains(&WizardError::ZeroBorrowerCount));
}

#[test]
fn borrower_count_mismatch_detected() {
    let mut p = PartialAnalysisInput {
        borrower_count: Some(2),
        ..Default::default()
    };
    p.borrowers.push(borrower(vec![score(740)], 90_000));
    let errs = validate(&p);
    assert!(errs.iter().any(|e| matches!(
        e,
        WizardError::BorrowerCountMismatch {
            declared: 2,
            provided: 1
        }
    )));
}

#[test]
fn matching_count_is_valid() {
    let mut p = PartialAnalysisInput {
        borrower_count: Some(1),
        ..Default::default()
    };
    p.borrowers.push(borrower(vec![score(740)], 90_000));
    assert!(is_valid(&p), "{:?}", validate(&p));
}

#[test]
fn borrower_missing_score_detected() {
    let mut p = PartialAnalysisInput {
        borrower_count: Some(1),
        ..Default::default()
    };
    p.borrowers.push(borrower(vec![], 90_000));
    assert!(validate(&p).contains(&WizardError::BorrowerMissingScore { index: 0 }));
}

#[test]
fn va_status_without_eligibility_detected() {
    let mut p = PartialAnalysisInput {
        borrower_count: Some(1),
        ..Default::default()
    };
    let mut b = borrower(vec![score(700)], 80_000);
    b.va = Some(VaStatus {
        eligible: false,
        previous_use: true,
        disability: false,
    });
    p.borrowers.push(b);
    assert!(validate(&p).contains(&WizardError::VaStatusWithoutEligibility { index: 0 }));
}

#[test]
fn coherent_va_status_is_valid() {
    let mut p = PartialAnalysisInput {
        borrower_count: Some(1),
        ..Default::default()
    };
    let mut b = borrower(vec![score(700)], 80_000);
    b.va = Some(VaStatus {
        eligible: true,
        previous_use: true,
        disability: true,
    });
    p.borrowers.push(b);
    assert!(is_valid(&p), "{:?}", validate(&p));
}

#[test]
fn zero_hold_horizon_rejected() {
    let p = PartialAnalysisInput {
        hold_horizon_months: Some(0),
        ..Default::default()
    };
    assert!(validate(&p).contains(&WizardError::ZeroHoldHorizon));
}

#[test]
fn negative_budget_rejected() {
    let p = PartialAnalysisInput {
        monthly_payment_budget: Some(Cents(-100)),
        ..Default::default()
    };
    assert!(validate(&p).iter().any(|e| matches!(
        e,
        WizardError::NegativeAmount {
            field: "monthly_payment_budget"
        }
    )));
}

#[test]
fn concessions_exceeding_cash_detected() {
    let mut p = PartialAnalysisInput {
        upfront_cash_budget: Some(Cents::from_dollars(5_000)),
        ..Default::default()
    };
    p.seller_credits.concessions_requested = Some(Cents::from_dollars(10_000));
    assert!(validate(&p).contains(&WizardError::ConcessionsExceedCashContext));
}

#[test]
fn multiple_errors_all_reported() {
    let mut p = PartialAnalysisInput {
        borrower_count: Some(0),
        hold_horizon_months: Some(0),
        ..Default::default()
    };
    p.borrowers.push(borrower(vec![], -5));
    let errs = validate(&p);
    // zero count, missing score, negative income, zero horizon = at least 4
    assert!(errs.len() >= 4, "expected >=4 errors, got {errs:?}");
}

#[test]
fn valid_completed_steps_tracks_progress() {
    let mut p = PartialAnalysisInput {
        borrower_count: Some(1),
        property_use: Some(Occupancy::PrimaryResidence),
        ..Default::default()
    };
    p.borrowers.push(borrower(vec![score(760)], 100_000));
    let steps = valid_completed_steps(&p);
    assert!(steps.contains(&WizardStep::BorrowerCount));
    assert!(steps.contains(&WizardStep::PropertyUse));
    assert!(steps.contains(&WizardStep::BorrowerDetails));
}
