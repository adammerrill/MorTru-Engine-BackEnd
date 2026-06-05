//! Epic 13 / Task T13.3 — multi-goal solve over a scenario set.
//!
//! Orchestrates T13.2 (`solve`, the bisection convergence loop) and T13.1
//! (`GoalScorer`) across many scenarios and the enabled `GoalMask`:
//!
//!   for each scenario:
//!     for each enabled goal:
//!       target = goal → SolveTarget (within the borrower's budget)
//!       solve(pricer, target) → converged PricedPoint
//!       score the resulting SolvedScenario for that goal
//!
//! The output is a flat set of `(scenario_id, goal, SolvedScenario, GoalScore)`
//! the Pareto frontier (Task 14.9) consumes.
//!
//! ## Seams (dependency discipline)
//! - `ScenarioPricer` per scenario — injected (T13.2), backed by ref_data+amort
//!   in the composition crate.
//! - `goal → SolveTarget` mapping needs the borrower's stated budgets, passed
//!   in `SolveBudget`. Goals with no balance-driving target (e.g. LOWEST_RATE)
//!   solve at the max eligible balance, then score.

use crate::{
    solve, GoalScore, GoalScorer, NonConvergeReason, PricedPoint, ScenarioPricer, SolveTarget,
    SolvedScenario, SolverConfig,
};
use types::{Cents, Derived, GoalMask};

/// The borrower's stated budgets — the targets goals drive toward.
#[derive(Debug, Clone, Copy, Default)]
pub struct SolveBudget {
    pub cash_to_close: Option<Cents>,
    pub monthly_payment: Option<Cents>,
    pub horizon_cost: Option<Cents>,
}

/// One solved+scored result for a (scenario, goal) pair.
#[derive(Debug, Clone)]
pub struct ScenarioOutcome {
    pub scenario_id: u64,
    pub goal: GoalMask,
    pub solved: SolvedScenario,
    pub score: GoalScore,
}

/// Why a (scenario, goal) pair produced no outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutcomeSkip {
    /// The goal needs a budget target that wasn't provided.
    NoTargetForGoal,
    /// The solver could not converge for this scenario/goal.
    NonConvergent(NonConvergeReason),
    /// The goal is not scorable for the solved scenario.
    NotScorable,
}

/// Map a goal to the budget quantity it drives the starting balance toward.
/// Cost/payment/cash goals target their stated budget; rate/APR/fee goals do
/// not drive balance (they rank at the max eligible balance) → `None`.
#[must_use]
pub fn target_for(goal: GoalMask, budget: &SolveBudget) -> Option<SolveTarget> {
    match goal {
        GoalMask::LOWEST_CASH_TO_CLOSE | GoalMask::LOWEST_CTC_AT_TARGET_PAYMENT => {
            budget.cash_to_close.map(SolveTarget::CashToClose)
        }
        GoalMask::LOWEST_PAYMENT
        | GoalMask::LOWEST_PAYMENT_AT_MAX_TERM
        | GoalMask::LOWEST_HORIZON_AT_TARGET_PAYMENT => {
            budget.monthly_payment.map(SolveTarget::MonthlyPayment)
        }
        GoalMask::LOWEST_HORIZON_COST | GoalMask::LOWEST_HORIZON_AT_TARGET_CTC => budget
            .horizon_cost
            .map(SolveTarget::HorizonCost)
            .or_else(|| budget.cash_to_close.map(SolveTarget::CashToClose)),
        // Rate/APR/fee/MI goals don't drive the balance; rank at max eligible.
        _ => None,
    }
}

/// Bridge a converged `PricedPoint` into a `SolvedScenario` for scoring.
/// Fields the pricer doesn't compute (lifetime cost, fees, equity) are taken
/// from the pricer's richer output in production; here the point carries the
/// solved essentials and the rest default to the point's known values.
#[must_use]
fn priced_to_solved(id: u64, p: &PricedPoint, is_fixed_rate: bool) -> SolvedScenario {
    SolvedScenario {
        id,
        note_rate: p.note_rate,
        apr: p.note_rate, // APR ≥ rate; refined when fee basket (Epic 10) wires in
        monthly_payment: p.monthly_payment,
        cash_to_close: p.cash_to_close,
        horizon_cost: p.horizon_cost,
        lifetime_cost: p.horizon_cost, // refined by amort full-term in composition
        lender_fees: Cents::ZERO,
        upfront_mi: Cents::ZERO,
        total_mi: p.mi,
        equity_at_horizon: Cents(p.balance.0 - p.cash_to_close.0).max(Cents::ZERO),
        is_fixed_rate,
    }
}

/// A scenario to solve: its id, a pricer, and whether it is fixed-rate.
pub struct SolveItem<'a, P: ScenarioPricer> {
    pub id: u64,
    pub pricer: &'a P,
    pub is_fixed_rate: bool,
}

impl<P: ScenarioPricer> std::fmt::Debug for SolveItem<'_, P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SolveItem")
            .field("id", &self.id)
            .field("is_fixed_rate", &self.is_fixed_rate)
            .finish_non_exhaustive()
    }
}

/// T13.3 — solve+score one scenario for one goal.
///
/// For balance-driving goals, solve to the goal's budget target; for ranking
/// goals (rate/APR/fee), price at the max eligible balance and score.
#[allow(clippy::result_large_err)]
pub fn solve_goal<P: ScenarioPricer>(
    item: &SolveItem<'_, P>,
    goal: GoalMask,
    budget: &SolveBudget,
    scorer: &impl GoalScorer,
    config: SolverConfig,
) -> Result<ScenarioOutcome, OutcomeSkip> {
    let priced: PricedPoint = match target_for(goal, budget) {
        Some(target) => {
            let d: Derived<PricedPoint> = solve(item.pricer, target, config)
                .map_err(|nc| OutcomeSkip::NonConvergent(nc.reason))?;
            d.value
        }
        None => {
            // Ranking goal: price at the max eligible balance.
            let (_, hi) = item.pricer.balance_bounds();
            item.pricer
                .price_at(hi)
                .ok_or(OutcomeSkip::NoTargetForGoal)?
        }
    };

    let solved = priced_to_solved(item.id, &priced, item.is_fixed_rate);
    let score = scorer
        .score(goal, &solved)
        .ok_or(OutcomeSkip::NotScorable)?;
    Ok(ScenarioOutcome {
        scenario_id: item.id,
        goal,
        solved,
        score,
    })
}

/// T13.3 — solve+score every scenario for every enabled goal. Skips are
/// collected separately so the caller can report coverage without failing.
#[must_use]
pub fn solve_all<P: ScenarioPricer>(
    items: &[SolveItem<'_, P>],
    enabled: GoalMask,
    budget: &SolveBudget,
    scorer: &impl GoalScorer,
    config: SolverConfig,
) -> (Vec<ScenarioOutcome>, Vec<(u64, GoalMask, OutcomeSkip)>) {
    let mut outcomes = Vec::new();
    let mut skips = Vec::new();
    for item in items {
        for goal in enabled.iter_goals() {
            match solve_goal(item, goal, budget, scorer, config) {
                Ok(o) => outcomes.push(o),
                Err(skip) => skips.push((item.id, goal, skip)),
            }
        }
    }
    (outcomes, skips)
}
