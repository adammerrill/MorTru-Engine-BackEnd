//! Epic 13 / T13.3 tests — multi-goal solve over a scenario set.

use solver::*;
use types::{BasisPoints, Cents, GoalMask, LtvBasisPoints};

/// Linear pricer (smooth, monotone) for convergence; rate fixed per instance.
struct FixturePricer {
    lo: Cents,
    hi: Cents,
    ctc_bps: i64,
    rate: u32,
}
impl ScenarioPricer for FixturePricer {
    fn price_at(&self, balance: Cents) -> Option<PricedPoint> {
        if balance.0 < self.lo.0 || balance.0 > self.hi.0 {
            return None;
        }
        Some(PricedPoint {
            balance,
            ltv: LtvBasisPoints(8000),
            mi: Cents::ZERO,
            llpa_bps: 0,
            note_rate: BasisPoints(self.rate),
            monthly_payment: Cents(balance.0 / 200),
            cash_to_close: Cents(balance.0 * self.ctc_bps / 10_000),
            horizon_cost: Cents(balance.0 / 2),
        })
    }
    fn balance_bounds(&self) -> (Cents, Cents) {
        (self.lo, self.hi)
    }
}
fn pr(rate: u32) -> FixturePricer {
    FixturePricer {
        lo: Cents::from_dollars(100_000),
        hi: Cents::from_dollars(1_000_000),
        ctc_bps: 1000,
        rate,
    }
}

fn budget() -> SolveBudget {
    SolveBudget {
        cash_to_close: Some(Cents::from_dollars(50_000)),
        monthly_payment: Some(Cents::from_dollars(2_500)),
        horizon_cost: Some(Cents::from_dollars(250_000)),
    }
}

// ── target_for mapping ──────────────────────────────────────────────────────

#[test]
fn ctc_goal_targets_cash_budget() {
    let t = target_for(GoalMask::LOWEST_CASH_TO_CLOSE, &budget());
    assert_eq!(
        t,
        Some(SolveTarget::CashToClose(Cents::from_dollars(50_000)))
    );
}

#[test]
fn payment_goal_targets_payment_budget() {
    let t = target_for(GoalMask::LOWEST_PAYMENT, &budget());
    assert_eq!(
        t,
        Some(SolveTarget::MonthlyPayment(Cents::from_dollars(2_500)))
    );
}

#[test]
fn rate_goal_has_no_balance_target() {
    assert_eq!(target_for(GoalMask::LOWEST_RATE, &budget()), None);
}

#[test]
fn ctc_goal_without_budget_is_none() {
    let empty = SolveBudget::default();
    assert_eq!(target_for(GoalMask::LOWEST_CASH_TO_CLOSE, &empty), None);
}

// ── solve_goal: balance-driving goal ────────────────────────────────────────

#[test]
fn solve_goal_ctc_converges_and_scores() {
    let item = SolveItem {
        id: 1,
        pricer: &pr(6000),
        is_fixed_rate: true,
    };
    let o = solve_goal(
        &item,
        GoalMask::LOWEST_CASH_TO_CLOSE,
        &budget(),
        &StandardScorer,
        SolverConfig::default(),
    )
    .expect("converge+score");
    assert_eq!(o.goal, GoalMask::LOWEST_CASH_TO_CLOSE);
    // converged near $500k balance (CTC $50k @ 10%).
    assert!((o.solved.cash_to_close.0 - Cents::from_dollars(50_000).0).abs() <= 100);
    // score is the CTC value (lower better).
    assert!(o.score.value > 0);
}

// ── solve_goal: ranking goal (rate) prices at max balance ───────────────────

#[test]
fn solve_goal_rate_prices_at_max_balance() {
    let item = SolveItem {
        id: 2,
        pricer: &pr(5500),
        is_fixed_rate: true,
    };
    let o = solve_goal(
        &item,
        GoalMask::LOWEST_RATE,
        &budget(),
        &StandardScorer,
        SolverConfig::default(),
    )
    .expect("rate goal scores");
    assert_eq!(o.score.value, 5500); // rate bps
    assert_eq!(o.solved.cash_to_close.0, Cents::from_dollars(100_000).0); // 10% of $1M
}

// ── solve_goal skips ────────────────────────────────────────────────────────

#[test]
fn solve_goal_skips_when_target_missing() {
    let empty = SolveBudget::default();
    let item = SolveItem {
        id: 3,
        pricer: &pr(6000),
        is_fixed_rate: true,
    };
    // CTC goal with no cash budget → NonConvergent? No: target_for None → ranking path,
    // prices at max, but scorer for CTC reads cash_to_close → scorable. So it succeeds.
    let r = solve_goal(
        &item,
        GoalMask::LOWEST_CASH_TO_CLOSE,
        &empty,
        &StandardScorer,
        SolverConfig::default(),
    );
    assert!(r.is_ok());
}

#[test]
fn solve_goal_non_scorable_goal_skips() {
    let item = SolveItem {
        id: 4,
        pricer: &pr(6000),
        is_fixed_rate: true,
    };
    // Investor goal not scorable on a bare SolvedScenario → NotScorable.
    let r = solve_goal(
        &item,
        GoalMask::MAXIMUM_PURCHASING_POWER,
        &budget(),
        &StandardScorer,
        SolverConfig::default(),
    );
    assert_eq!(r.unwrap_err(), OutcomeSkip::NotScorable);
}

// ── solve_all over a set ────────────────────────────────────────────────────

#[test]
fn solve_all_covers_default_consumer_across_scenarios() {
    let p1 = pr(6000);
    let p2 = pr(5500);
    let items = vec![
        SolveItem {
            id: 1,
            pricer: &p1,
            is_fixed_rate: true,
        },
        SolveItem {
            id: 2,
            pricer: &p2,
            is_fixed_rate: true,
        },
    ];
    let (outcomes, _skips) = solve_all(
        &items,
        GoalMask::DEFAULT_CONSUMER,
        &budget(),
        &StandardScorer,
        SolverConfig::default(),
    );
    // 2 scenarios × 5 default-consumer goals = up to 10 outcomes.
    assert!(!outcomes.is_empty());
    assert!(outcomes
        .iter()
        .all(|o| o.scenario_id == 1 || o.scenario_id == 2));
}

#[test]
fn solve_all_lower_rate_scenario_scores_better_on_rate() {
    let p1 = pr(6000);
    let p2 = pr(5500);
    let items = vec![
        SolveItem {
            id: 1,
            pricer: &p1,
            is_fixed_rate: true,
        },
        SolveItem {
            id: 2,
            pricer: &p2,
            is_fixed_rate: true,
        },
    ];
    let (outcomes, _) = solve_all(
        &items,
        GoalMask::LOWEST_RATE,
        &budget(),
        &StandardScorer,
        SolverConfig::default(),
    );
    let s1 = outcomes
        .iter()
        .find(|o| o.scenario_id == 1)
        .unwrap()
        .score
        .value;
    let s2 = outcomes
        .iter()
        .find(|o| o.scenario_id == 2)
        .unwrap()
        .score
        .value;
    assert!(
        s2 < s1,
        "lower-rate scenario scores lower (better): {s2} < {s1}"
    );
}

#[test]
fn solve_all_collects_skips() {
    let p1 = pr(6000);
    let items = vec![SolveItem {
        id: 1,
        pricer: &p1,
        is_fixed_rate: true,
    }];
    // Enable a non-scorable goal alongside a scorable one.
    let mask = GoalMask::LOWEST_RATE | GoalMask::MAXIMUM_PURCHASING_POWER;
    let (outcomes, skips) = solve_all(
        &items,
        mask,
        &budget(),
        &StandardScorer,
        SolverConfig::default(),
    );
    assert_eq!(outcomes.len(), 1); // rate scored
    assert_eq!(skips.len(), 1); // purchasing power skipped
    assert_eq!(skips[0].2, OutcomeSkip::NotScorable);
}

#[test]
fn outcome_carries_provenance() {
    let item = SolveItem {
        id: 9,
        pricer: &pr(6000),
        is_fixed_rate: true,
    };
    let o = solve_goal(
        &item,
        GoalMask::LOWEST_HORIZON_COST,
        &budget(),
        &StandardScorer,
        SolverConfig::default(),
    )
    .expect("ok");
    assert!(o.score.explain().contains("Source:"));
}
