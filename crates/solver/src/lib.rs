//! Epic 13 / Task T13.1 — goal scoring.
//!
//! Defines `SolvedScenario` (a priced/amortized scenario the iterative solver
//! produces in later tasks), `GoalScore` (a single goal's comparable value),
//! and `GoalScorer` (one scoring function per `GoalMask` bit).
//!
//! ## Canonical direction: LOWER IS BETTER
//! Every score is normalized so a smaller value ranks higher. Cost/rate/payment
//! goals score their natural value; "maximize" goals (equity, cash-on-cash)
//! score the negation. This single convention lets the Pareto frontier (Task
//! 14.9) treat all goals uniformly: a scenario dominates if it is ≤ on every
//! enabled score and < on at least one.
//!
//! ## Applicability
//! A scorer returns `None` when its goal does not apply to the scenario (e.g.
//! ARM-shock on a fixed loan, investor goals on a consumer scenario lacking
//! rental data). The frontier skips `None` scores rather than penalizing.
//!
//! T13.1 defines the scoring contract over solved inputs. The iterative solver
//! that *produces* `SolvedScenario` (rate↔MI↔LLPA↔balance convergence) is
//! T13.2+; the Pareto frontier that *consumes* `GoalScores` is Task 14.9.

use std::collections::BTreeMap;
use types::{BasisPoints, Cents, Derived, GoalMask, Provenance};

/// A fully solved scenario: the enumerated/priced/amortized unit the scorers
/// read. Produced by the iterative solver (T13.2+). All monetary fields are
/// the *solved* outputs for this scenario's converged starting balance.
#[derive(Debug, Clone)]
pub struct SolvedScenario {
    /// Stable identity for frontier bookkeeping (e.g. a `ScenarioKey` as u64).
    pub id: u64,
    pub note_rate: BasisPoints,
    pub apr: BasisPoints,
    pub monthly_payment: Cents,
    pub cash_to_close: Cents,
    /// Total interest + remaining balance over the hold horizon (`amort`).
    pub horizon_cost: Cents,
    /// Total interest over the full amortization schedule.
    pub lifetime_cost: Cents,
    /// Origination + underwriting fees, excluding discount points.
    pub lender_fees: Cents,
    /// Upfront MI only (FHA UFMIP / VA funding fee / USDA guarantee).
    pub upfront_mi: Cents,
    /// Total MI over the hold horizon (upfront + monthly).
    pub total_mi: Cents,
    /// Principal paid down by the hold horizon (for equity goals).
    pub equity_at_horizon: Cents,
    /// True for fixed-rate products; ARM-only goals return `None` otherwise.
    pub is_fixed_rate: bool,
}

/// A single goal's score for one scenario. Lower is better (see module docs).
/// Carries `Derived` provenance so the client sees *why* a scenario ranked.
pub type GoalScore = Derived<i64>;

/// The scores for one scenario across all enabled goals, keyed by the goal's
/// raw bit value. Consumed by the Pareto frontier.
pub type GoalScores = BTreeMap<u64, GoalScore>;

fn prov(goal_name: &str) -> Provenance {
    Provenance {
        dataset: "goal_scorer".to_owned(),
        source_file: "solver::scoring".to_owned(),
        source_citation: format!("Epic 13 T13.1 GoalScorer — {goal_name}"),
        effective_date: "2026-06-05".to_owned(),
        record_id: goal_name.to_owned(),
        requested_version: 0,
        resolved_version: 0,
    }
}

fn scored(goal_name: &str, value: i64, basis: String) -> GoalScore {
    let mut d = Derived::new(value, prov(goal_name));
    d.push_step("goal_score", basis, format!("score={value} (lower=better)"));
    d
}

/// The goal scoring seam. The default `StandardScorer` implements every
/// consumer goal that depends only on solved cost/rate/payment data; investor
/// goals requiring rental/DSCR inputs are scored when those inputs are present
/// in later tasks.
pub trait GoalScorer {
    /// Score one goal for one scenario. `None` = goal not applicable here.
    fn score(&self, goal: GoalMask, solved: &SolvedScenario) -> Option<GoalScore>;

    /// Score every enabled goal, skipping inapplicable ones.
    fn score_all(&self, enabled: GoalMask, solved: &SolvedScenario) -> GoalScores {
        let mut scores = GoalScores::new();
        for goal in enabled.iter_goals() {
            if let Some(s) = self.score(goal, solved) {
                scores.insert(goal.bits(), s);
            }
        }
        scores
    }
}

/// Standard consumer-goal scorer. Lower-is-better normalization throughout.
#[derive(Debug, Clone, Copy, Default)]
pub struct StandardScorer;

impl GoalScorer for StandardScorer {
    fn score(&self, goal: GoalMask, s: &SolvedScenario) -> Option<GoalScore> {
        // Each arm maps a single goal bit to its comparable value.
        match goal {
            GoalMask::LOWEST_HORIZON_COST => Some(scored(
                "LOWEST_HORIZON_COST",
                s.horizon_cost.0,
                format!("horizon_cost={}c", s.horizon_cost.0),
            )),
            GoalMask::LOWEST_LIFETIME_COST => Some(scored(
                "LOWEST_LIFETIME_COST",
                s.lifetime_cost.0,
                format!("lifetime_cost={}c", s.lifetime_cost.0),
            )),
            GoalMask::LOWEST_PAYMENT | GoalMask::LOWEST_PAYMENT_AT_MAX_TERM => Some(scored(
                "LOWEST_PAYMENT",
                s.monthly_payment.0,
                format!("payment={}c", s.monthly_payment.0),
            )),
            GoalMask::LOWEST_CASH_TO_CLOSE => Some(scored(
                "LOWEST_CASH_TO_CLOSE",
                s.cash_to_close.0,
                format!("ctc={}c", s.cash_to_close.0),
            )),
            GoalMask::LOWEST_LENDER_FEES => Some(scored(
                "LOWEST_LENDER_FEES",
                s.lender_fees.0,
                format!("lender_fees={}c", s.lender_fees.0),
            )),
            GoalMask::LOWEST_RATE => Some(scored(
                "LOWEST_RATE",
                i64::from(s.note_rate.0),
                format!("rate_bps={}", s.note_rate.0),
            )),
            GoalMask::LOWEST_APR => Some(scored(
                "LOWEST_APR",
                i64::from(s.apr.0),
                format!("apr_bps={}", s.apr.0),
            )),
            GoalMask::LOWEST_MI_COST => Some(scored(
                "LOWEST_MI_COST",
                s.total_mi.0,
                format!("total_mi={}c", s.total_mi.0),
            )),
            GoalMask::LOWEST_UPFRONT_MI => Some(scored(
                "LOWEST_UPFRONT_MI",
                s.upfront_mi.0,
                format!("upfront_mi={}c", s.upfront_mi.0),
            )),
            // "Maximize" goals: negate so lower-is-better holds.
            GoalMask::MAX_EQUITY_AT_HORIZON | GoalMask::MAX_PRINCIPAL_AT_HORIZON => Some(scored(
                "MAX_EQUITY_AT_HORIZON",
                -s.equity_at_horizon.0,
                format!("equity={}c (negated)", s.equity_at_horizon.0),
            )),
            // ARM-only goal: applicable only to adjustable products.
            GoalMask::MINIMIZE_ARM_PAYMENT_SHOCK => {
                if s.is_fixed_rate {
                    None
                } else {
                    Some(scored(
                        "MINIMIZE_ARM_PAYMENT_SHOCK",
                        s.monthly_payment.0,
                        "arm payment proxy".to_owned(),
                    ))
                }
            }
            // Fixed-rate scores perfectly on stability (score 0); ARMs worse.
            GoalMask::HIGHEST_PAYMENT_STABILITY => Some(scored(
                "HIGHEST_PAYMENT_STABILITY",
                if s.is_fixed_rate { 0 } else { 1 },
                format!("fixed={}", s.is_fixed_rate),
            )),
            // Goals requiring request-context (target payment/ctc, investor
            // rental/DSCR data) are scored in later tasks when those inputs
            // are threaded through. Not applicable to a bare SolvedScenario.
            _ => None,
        }
    }
}

pub mod converge;
pub use converge::*;
pub mod multi;
pub use multi::*;

pub mod config;
pub use config::*;

pub mod frontier;
pub use frontier::*;
