//! Epic 13 / Task T13.4 — analysis configuration & objective selection.
//!
//! Bundles everything an analysis run needs into one `AnalysisConfig`: the
//! enabled goals (`GoalMask`), the borrower budgets (`SolveBudget`), and the
//! solver tuning (`SolverConfig`). Provides persona-default constructors and a
//! `run_analysis` entry that returns both the outcomes and a `CoverageReport`
//! so the API layer can tell the user which objectives were optimized and which
//! were skipped (and why) — never a silent drop.

use crate::{solve_all, GoalScorer, OutcomeSkip, ScenarioOutcome, SolveBudget, SolverConfig};
use std::collections::BTreeMap;
use types::GoalMask;

/// Re-export alias: a scenario to solve (id + pricer + fixed-rate flag).
pub type ScenarioItem<'a, P> = crate::SolveItem<'a, P>;

/// Full configuration for one analysis run.
#[derive(Debug, Clone, Copy)]
pub struct AnalysisConfig {
    /// Which optimization objectives to run.
    pub goals: GoalMask,
    /// Borrower-stated budget targets the goals drive toward.
    pub budget: SolveBudget,
    /// Solver tuning (iterations, tolerance).
    pub solver: SolverConfig,
}

impl AnalysisConfig {
    /// Owner-occupant consumer defaults (`GoalMask::DEFAULT_CONSUMER`).
    #[must_use]
    pub fn consumer(budget: SolveBudget) -> Self {
        AnalysisConfig {
            goals: GoalMask::DEFAULT_CONSUMER,
            budget,
            solver: SolverConfig::default(),
        }
    }

    /// Real-estate investor defaults (`GoalMask::DEFAULT_INVESTOR`).
    #[must_use]
    pub fn investor(budget: SolveBudget) -> Self {
        AnalysisConfig {
            goals: GoalMask::DEFAULT_INVESTOR,
            budget,
            solver: SolverConfig::default(),
        }
    }

    /// Enable an additional objective.
    #[must_use]
    pub fn with_goal(mut self, goal: GoalMask) -> Self {
        self.goals |= goal;
        self
    }

    /// Disable an objective.
    #[must_use]
    pub fn without_goal(mut self, goal: GoalMask) -> Self {
        self.goals &= !goal;
        self
    }

    /// How many objectives are enabled.
    #[must_use]
    pub fn goal_count(&self) -> u32 {
        self.goals.iter_goals().count() as u32
    }
}

/// Per-goal coverage: did the enabled goal produce any outcome, and if some
/// (scenario, goal) pairs were skipped, the reasons.
#[derive(Debug, Clone)]
pub struct CoverageReport {
    /// Goals that produced at least one outcome.
    pub optimized: Vec<GoalMask>,
    /// Goals enabled but that produced no outcome at all, with the reason(s).
    pub unmet: BTreeMap<u64, Vec<OutcomeSkip>>,
    pub total_outcomes: usize,
    pub total_skips: usize,
}

impl CoverageReport {
    /// Did every enabled goal get optimized for at least one scenario?
    #[must_use]
    pub fn fully_covered(&self) -> bool {
        self.unmet.is_empty()
    }
}

/// The full result of an analysis run: solved+scored outcomes + coverage.
#[derive(Debug, Clone)]
pub struct AnalysisResult {
    pub outcomes: Vec<ScenarioOutcome>,
    pub coverage: CoverageReport,
}

/// T13.4 — run an analysis: solve+score every scenario for every enabled goal,
/// then summarize coverage so the caller can report objective-level results.
#[must_use]
pub fn run_analysis<P: crate::ScenarioPricer>(
    items: &[ScenarioItem<'_, P>],
    config: &AnalysisConfig,
    scorer: &impl GoalScorer,
) -> AnalysisResult {
    let (outcomes, skips) = solve_all(items, config.goals, &config.budget, scorer, config.solver);

    // Goals that produced ≥1 outcome.
    let mut optimized: Vec<GoalMask> = outcomes.iter().map(|o| o.goal).collect();
    optimized.sort_by_key(|g| g.bits());
    optimized.dedup();

    // Goals enabled but with zero outcomes → unmet, collect their skip reasons.
    let mut unmet: BTreeMap<u64, Vec<OutcomeSkip>> = BTreeMap::new();
    for goal in config.goals.iter_goals() {
        let produced = outcomes.iter().any(|o| o.goal == goal);
        if !produced {
            let reasons: Vec<OutcomeSkip> = skips
                .iter()
                .filter(|(_, g, _)| *g == goal)
                .map(|(_, _, s)| *s)
                .collect();
            unmet.insert(goal.bits(), reasons);
        }
    }

    let total_outcomes = outcomes.len();
    let total_skips = skips.len();
    AnalysisResult {
        outcomes,
        coverage: CoverageReport {
            optimized,
            unmet,
            total_outcomes,
            total_skips,
        },
    }
}
