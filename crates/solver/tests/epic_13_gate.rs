//! Epic 13 / T13.4 (config) + T13.5 (capstone gate) tests.

use solver::*;
use types::{BasisPoints, Cents, GoalMask, LtvBasisPoints};

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

// ── T13.4 config ────────────────────────────────────────────────────────────

#[test]
fn consumer_config_uses_default_consumer_goals() {
    let c = AnalysisConfig::consumer(budget());
    assert_eq!(c.goals, GoalMask::DEFAULT_CONSUMER);
    assert_eq!(c.goal_count(), 5);
}

#[test]
fn investor_config_uses_default_investor_goals() {
    let c = AnalysisConfig::investor(budget());
    assert_eq!(c.goals, GoalMask::DEFAULT_INVESTOR);
}

#[test]
fn with_and_without_goal_toggle() {
    let c = AnalysisConfig::consumer(budget())
        .with_goal(GoalMask::LOWEST_MI_COST)
        .without_goal(GoalMask::LOWEST_APR);
    assert!(c.goals.contains(GoalMask::LOWEST_MI_COST));
    assert!(!c.goals.contains(GoalMask::LOWEST_APR));
}

#[test]
fn run_analysis_reports_optimized_goals() {
    let p1 = pr(6000);
    let items = vec![SolveItem {
        id: 1,
        pricer: &p1,
        is_fixed_rate: true,
    }];
    let cfg = AnalysisConfig::consumer(budget());
    let res = run_analysis(&items, &cfg, &StandardScorer);
    // DEFAULT_CONSUMER's 5 goals are all consumer-scorable → optimized.
    assert!(!res.outcomes.is_empty());
    assert!(res.coverage.optimized.contains(&GoalMask::LOWEST_RATE));
    assert_eq!(res.coverage.total_outcomes, res.outcomes.len());
}

#[test]
fn run_analysis_flags_unmet_goal() {
    let p1 = pr(6000);
    let items = vec![SolveItem {
        id: 1,
        pricer: &p1,
        is_fixed_rate: true,
    }];
    // Enable an investor goal not scorable on a bare SolvedScenario.
    let cfg = AnalysisConfig::consumer(budget()).with_goal(GoalMask::HIGHEST_CASH_ON_CASH_RETURN);
    let res = run_analysis(&items, &cfg, &StandardScorer);
    assert!(!res.coverage.fully_covered());
    assert!(res
        .coverage
        .unmet
        .contains_key(&GoalMask::HIGHEST_CASH_ON_CASH_RETURN.bits()));
}

#[test]
fn fully_covered_when_all_goals_score() {
    let p1 = pr(6000);
    let items = vec![SolveItem {
        id: 1,
        pricer: &p1,
        is_fixed_rate: true,
    }];
    // Only scorable goals.
    let cfg = AnalysisConfig {
        goals: GoalMask::LOWEST_RATE | GoalMask::LOWEST_CASH_TO_CLOSE,
        budget: budget(),
        solver: SolverConfig::default(),
    };
    let res = run_analysis(&items, &cfg, &StandardScorer);
    assert!(res.coverage.fully_covered());
}

// ── T13.5 capstone gate ─────────────────────────────────────────────────────

#[test]
fn epic_13_gate_end_to_end_consumer_analysis() {
    // Two scenarios at different rates, full consumer analysis.
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
    let cfg = AnalysisConfig::consumer(budget());
    let res = run_analysis(&items, &cfg, &StandardScorer);

    // Every outcome: solved scenario + scored goal + provenance.
    for o in &res.outcomes {
        assert!(o.scenario_id == 1 || o.scenario_id == 2);
        assert!(o.score.explain().contains("Source:"));
    }
    // The lower-rate scenario must win on LOWEST_RATE.
    let rate_outcomes: Vec<_> = res
        .outcomes
        .iter()
        .filter(|o| o.goal == GoalMask::LOWEST_RATE)
        .collect();
    let best = rate_outcomes.iter().min_by_key(|o| o.score.value).unwrap();
    assert_eq!(best.scenario_id, 2, "lower-rate scenario wins LOWEST_RATE");
}

#[test]
fn epic_13_gate_convergence_meets_budget() {
    // A CTC goal converges so the solved cash-to-close is within budget.
    let p1 = pr(6000);
    let items = vec![SolveItem {
        id: 1,
        pricer: &p1,
        is_fixed_rate: true,
    }];
    let cfg = AnalysisConfig {
        goals: GoalMask::LOWEST_CASH_TO_CLOSE,
        budget: budget(),
        solver: SolverConfig::default(),
    };
    let res = run_analysis(&items, &cfg, &StandardScorer);
    let o = &res.outcomes[0];
    assert!(o.solved.cash_to_close.0 <= Cents::from_dollars(50_000).0 + 100);
}

#[test]
fn epic_13_gate_no_silent_drops() {
    // Outcomes + skips together account for every (scenario × goal) pair.
    let p1 = pr(6000);
    let items = vec![SolveItem {
        id: 1,
        pricer: &p1,
        is_fixed_rate: true,
    }];
    let cfg = AnalysisConfig::consumer(budget()).with_goal(GoalMask::HIGHEST_MONTHLY_CASH_FLOW);
    let res = run_analysis(&items, &cfg, &StandardScorer);
    let goals = cfg.goals.iter_goals().count();
    // 1 scenario × N goals = outcomes + skips (each pair lands in exactly one).
    assert_eq!(
        res.coverage.total_outcomes + res.coverage.total_skips,
        goals
    );
}
