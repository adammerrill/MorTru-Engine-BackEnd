//! Epic 13 / T13.2 tests — bisection convergence solver.

use solver::*;
use types::{BasisPoints, Cents, LtvBasisPoints};

/// A linear fixture pricer: CTC = balance * ctc_factor / 10000, monotone in
/// balance. Models the smooth (no-tier) case for convergence verification.
struct LinearPricer {
    lo: Cents,
    hi: Cents,
    /// CTC as a fraction of balance, in basis points (e.g. 1000 = 10%).
    ctc_bps: i64,
}

impl ScenarioPricer for LinearPricer {
    fn price_at(&self, balance: Cents) -> Option<PricedPoint> {
        if balance.0 < self.lo.0 || balance.0 > self.hi.0 {
            return None;
        }
        let ctc = Cents(balance.0 * self.ctc_bps / 10_000);
        Some(PricedPoint {
            balance,
            ltv: LtvBasisPoints(8000),
            mi: Cents::ZERO,
            llpa_bps: 0,
            note_rate: BasisPoints(6000),
            monthly_payment: Cents(balance.0 / 200), // arbitrary monotone proxy
            cash_to_close: ctc,
            horizon_cost: Cents(balance.0 / 2),
        })
    }
    fn balance_bounds(&self) -> (Cents, Cents) {
        (self.lo, self.hi)
    }
}

fn pricer() -> LinearPricer {
    LinearPricer {
        lo: Cents::from_dollars(100_000),
        hi: Cents::from_dollars(1_000_000),
        ctc_bps: 1000, // CTC = 10% of balance
    }
}

#[test]
fn converges_on_cash_to_close_target() {
    // Want CTC = $50,000 → balance = $500,000 (10%).
    let r = solve(
        &pricer(),
        SolveTarget::CashToClose(Cents::from_dollars(50_000)),
        SolverConfig::default(),
    )
    .expect("should converge");
    // Within tolerance of $500k balance.
    assert!((r.value.balance.0 - Cents::from_dollars(500_000).0).abs() <= 20_000);
    assert!((r.value.cash_to_close.0 - Cents::from_dollars(50_000).0).abs() <= 100);
}

#[test]
fn converges_within_iteration_budget() {
    let r = solve(
        &pricer(),
        SolveTarget::CashToClose(Cents::from_dollars(70_000)),
        SolverConfig::default(),
    )
    .expect("converge");
    // bisection over a $900k range at $1 tol ≤ 20 iters.
    assert!(r.steps.len() <= 20);
}

#[test]
fn records_iteration_provenance() {
    let r = solve(
        &pricer(),
        SolveTarget::CashToClose(Cents::from_dollars(60_000)),
        SolverConfig::default(),
    )
    .unwrap();
    assert!(!r.steps.is_empty());
    let text = r.explain();
    assert!(text.contains("solver_iter") && text.contains("Source:"));
}

#[test]
fn infeasible_high_target_reports_nonconvergent() {
    // CTC target $200k → balance $2M, above the $1M bound. Bracket collapses.
    let err = solve(
        &pricer(),
        SolveTarget::CashToClose(Cents::from_dollars(200_000)),
        SolverConfig::default(),
    )
    .unwrap_err();
    assert!(matches!(
        err.reason,
        NonConvergeReason::GoalInfeasibleInBounds | NonConvergeReason::MaxIters
    ));
    // best attempt is the closest priceable point (the high bound).
    assert!(err.best_attempt.balance.0 <= Cents::from_dollars(1_000_000).0);
}

#[test]
fn payment_target_converges() {
    // monthly_payment = balance/200; want $2,500 → balance $500,000.
    let r = solve(
        &pricer(),
        SolveTarget::MonthlyPayment(Cents::from_dollars(2_500)),
        SolverConfig::default(),
    )
    .expect("converge");
    assert!((r.value.balance.0 - Cents::from_dollars(500_000).0).abs() <= 20_000);
}

#[test]
fn horizon_target_converges() {
    // horizon_cost = balance/2; want $250,000 → balance $500,000.
    let r = solve(
        &pricer(),
        SolveTarget::HorizonCost(Cents::from_dollars(250_000)),
        SolverConfig::default(),
    )
    .expect("converge");
    assert!((r.value.balance.0 - Cents::from_dollars(500_000).0).abs() <= 20_000);
}

#[test]
fn unpriceable_bounds_report_no_eligible_balance() {
    struct Dead;
    impl ScenarioPricer for Dead {
        fn price_at(&self, _: Cents) -> Option<PricedPoint> {
            None
        }
        fn balance_bounds(&self) -> (Cents, Cents) {
            (Cents::from_dollars(100_000), Cents::from_dollars(200_000))
        }
    }
    let err = solve(
        &Dead,
        SolveTarget::CashToClose(Cents::from_dollars(10_000)),
        SolverConfig::default(),
    )
    .unwrap_err();
    assert_eq!(err.reason, NonConvergeReason::NoEligibleBalance);
}

#[test]
fn tighter_tolerance_still_converges() {
    let cfg = SolverConfig {
        max_iters: 40,
        tolerance: Cents(1),
    };
    let r = solve(
        &pricer(),
        SolveTarget::CashToClose(Cents::from_dollars(50_000)),
        cfg,
    )
    .expect("converge");
    assert!((r.value.cash_to_close.0 - Cents::from_dollars(50_000).0).abs() <= 1);
}

#[test]
fn solve_target_value_accessor() {
    assert_eq!(
        SolveTarget::CashToClose(Cents::from_dollars(5)).value(),
        Cents::from_dollars(5)
    );
}

#[test]
fn priced_point_realized_matches_target_kind() {
    let p = pricer().price_at(Cents::from_dollars(500_000)).unwrap();
    assert_eq!(
        p.realized(SolveTarget::CashToClose(Cents::ZERO)),
        p.cash_to_close
    );
    assert_eq!(
        p.realized(SolveTarget::MonthlyPayment(Cents::ZERO)),
        p.monthly_payment
    );
    assert_eq!(
        p.realized(SolveTarget::HorizonCost(Cents::ZERO)),
        p.horizon_cost
    );
}
