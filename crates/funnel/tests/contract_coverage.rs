//! Epic 16 / T1 — direct coverage for `BorrowerInput::is_complete`, exercising
//! all three branches (no score, negative income, fully complete).

use funnel::*;
use types::{Cents, CreditScore, Occupancy};

fn b(scores: Vec<CreditScore>, income: i64) -> BorrowerInput {
    BorrowerInput {
        occupancy: Occupancy::PrimaryResidence,
        credit_scores: scores,
        va: None,
        annual_income: Cents::from_dollars(income),
    }
}

#[test]
fn is_complete_true_when_scored_and_nonnegative() {
    assert!(b(vec![CreditScore::new(740).unwrap()], 90_000).is_complete());
}

#[test]
fn is_complete_false_without_score() {
    assert!(!b(vec![], 90_000).is_complete());
}

#[test]
fn is_complete_false_with_negative_income() {
    let bi = BorrowerInput {
        occupancy: Occupancy::PrimaryResidence,
        credit_scores: vec![CreditScore::new(700).unwrap()],
        va: None,
        annual_income: Cents(-1),
    };
    assert!(!bi.is_complete());
}

#[test]
fn is_complete_true_with_zero_income() {
    // zero is non-negative → complete (income gate is non-negativity, not >0)
    assert!(b(vec![CreditScore::new(700).unwrap()], 0).is_complete());
}
