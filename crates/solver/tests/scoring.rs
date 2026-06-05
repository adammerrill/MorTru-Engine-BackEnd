//! Epic 13 / T13.1 tests — goal scoring.

use solver::*;
use types::{BasisPoints, Cents, GoalMask};

fn solved(id: u64) -> SolvedScenario {
    SolvedScenario {
        id,
        note_rate: BasisPoints(6000),
        apr: BasisPoints(6250),
        monthly_payment: Cents::from_dollars(1_800),
        cash_to_close: Cents::from_dollars(20_000),
        horizon_cost: Cents::from_dollars(95_000),
        lifetime_cost: Cents::from_dollars(347_000),
        lender_fees: Cents::from_dollars(2_500),
        upfront_mi: Cents::from_dollars(5_000),
        total_mi: Cents::from_dollars(12_000),
        equity_at_horizon: Cents::from_dollars(40_000),
        is_fixed_rate: true,
    }
}

#[test]
fn horizon_cost_scores_its_value() {
    let s = StandardScorer
        .score(GoalMask::LOWEST_HORIZON_COST, &solved(1))
        .unwrap();
    assert_eq!(s.value, 9_500_000); // $95k in cents
}

#[test]
fn rate_scores_bps() {
    let s = StandardScorer
        .score(GoalMask::LOWEST_RATE, &solved(1))
        .unwrap();
    assert_eq!(s.value, 6000);
}

#[test]
fn apr_scores_bps() {
    let s = StandardScorer
        .score(GoalMask::LOWEST_APR, &solved(1))
        .unwrap();
    assert_eq!(s.value, 6250);
}

#[test]
fn lower_is_better_for_cost_goals() {
    let cheap = SolvedScenario {
        horizon_cost: Cents::from_dollars(80_000),
        ..solved(1)
    };
    let dear = SolvedScenario {
        horizon_cost: Cents::from_dollars(120_000),
        ..solved(2)
    };
    let cs = StandardScorer
        .score(GoalMask::LOWEST_HORIZON_COST, &cheap)
        .unwrap();
    let ds = StandardScorer
        .score(GoalMask::LOWEST_HORIZON_COST, &dear)
        .unwrap();
    assert!(cs.value < ds.value, "cheaper horizon scores lower (better)");
}

#[test]
fn maximize_equity_is_negated() {
    let s = StandardScorer
        .score(GoalMask::MAX_EQUITY_AT_HORIZON, &solved(1))
        .unwrap();
    // $40k equity → -4,000,000 so more equity ranks lower (better).
    assert_eq!(s.value, -4_000_000);
    let more = SolvedScenario {
        equity_at_horizon: Cents::from_dollars(60_000),
        ..solved(1)
    };
    let ms = StandardScorer
        .score(GoalMask::MAX_EQUITY_AT_HORIZON, &more)
        .unwrap();
    assert!(ms.value < s.value, "more equity scores lower (better)");
}

#[test]
fn arm_shock_none_for_fixed() {
    assert!(StandardScorer
        .score(GoalMask::MINIMIZE_ARM_PAYMENT_SHOCK, &solved(1))
        .is_none());
}

#[test]
fn arm_shock_some_for_adjustable() {
    let arm = SolvedScenario {
        is_fixed_rate: false,
        ..solved(1)
    };
    assert!(StandardScorer
        .score(GoalMask::MINIMIZE_ARM_PAYMENT_SHOCK, &arm)
        .is_some());
}

#[test]
fn payment_stability_fixed_scores_zero() {
    let s = StandardScorer
        .score(GoalMask::HIGHEST_PAYMENT_STABILITY, &solved(1))
        .unwrap();
    assert_eq!(s.value, 0);
    let arm = SolvedScenario {
        is_fixed_rate: false,
        ..solved(1)
    };
    let a = StandardScorer
        .score(GoalMask::HIGHEST_PAYMENT_STABILITY, &arm)
        .unwrap();
    assert!(a.value > s.value, "ARM less stable → higher (worse)");
}

#[test]
fn unsupported_goal_returns_none() {
    // Investor goal needing rental data — not scorable on a bare SolvedScenario.
    assert!(StandardScorer
        .score(GoalMask::MAXIMUM_PURCHASING_POWER, &solved(1))
        .is_none());
}

#[test]
fn score_all_covers_default_consumer() {
    let scores = StandardScorer.score_all(GoalMask::DEFAULT_CONSUMER, &solved(1));
    // DEFAULT_CONSUMER = horizon, payment, ctc, rate, apr → all 5 scorable.
    assert_eq!(scores.len(), 5);
    assert!(scores.contains_key(&GoalMask::LOWEST_RATE.bits()));
}

#[test]
fn score_all_skips_inapplicable() {
    let mask = GoalMask::LOWEST_RATE | GoalMask::MINIMIZE_ARM_PAYMENT_SHOCK;
    // Fixed scenario → ARM shock skipped, only rate scored.
    let scores = StandardScorer.score_all(mask, &solved(1));
    assert_eq!(scores.len(), 1);
}

#[test]
fn scores_are_explainable() {
    let s = StandardScorer
        .score(GoalMask::LOWEST_CASH_TO_CLOSE, &solved(1))
        .unwrap();
    let text = s.explain();
    assert!(text.contains("Source:") && text.contains("ctc="));
}
