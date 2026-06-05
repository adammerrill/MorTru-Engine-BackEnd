//! Task 14.9 — Pareto frontier over scored scenarios.
//!
//! Given a set of scenarios, each with `GoalScores` (goal-bit → score, where
//! **lower is better** by the T13.1 convention) and the `enabled: GoalMask`,
//! returns the non-dominated set: a scenario is on the frontier iff no other
//! scenario is ≤ it on every enabled goal AND < it on at least one.
//!
//! Equivalently (the contract from `goal_mask.rs`): a scenario survives unless
//! some other scenario **dominates** it — is no worse on every enabled goal and
//! strictly better on at least one.
//!
//! ## Comparison semantics
//! Only goals present in BOTH scenarios' `GoalScores` are compared. A goal a
//! scenario didn't score (inapplicable / skipped) does not participate — it
//! neither helps nor hurts dominance. If two scenarios share no comparable
//! goal, neither dominates the other (both survive).
//!
//! ## Complexity
//! O(n²·g) pairwise — n scenarios, g enabled goals. Adequate for the pruned
//! feasible set; if n grows large the ML feasibility gate (Epic 12) shrinks it
//! upstream. Documented as a known bound.

use crate::GoalScores;
use types::GoalMask;

/// A scenario entering the frontier computation: an id + its per-goal scores.
#[derive(Debug, Clone)]
pub struct FrontierCandidate {
    pub id: u64,
    pub scores: GoalScores,
}

/// The dominance relation between two candidates over the enabled goals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dominance {
    /// `a` dominates `b` (a ≤ b on all compared goals, < on ≥1).
    ADominatesB,
    /// `b` dominates `a`.
    BDominatesA,
    /// Neither dominates (incomparable or equal).
    Neither,
}

/// Compare two candidates over the enabled goals. Only goals present in both
/// (and enabled) are considered.
#[must_use]
pub fn dominance(a: &FrontierCandidate, b: &FrontierCandidate, enabled: GoalMask) -> Dominance {
    let mut a_strictly_better_somewhere = false;
    let mut b_strictly_better_somewhere = false;
    let mut compared_any = false;

    for goal in enabled.iter_goals() {
        let bit = goal.bits();
        let (Some(sa), Some(sb)) = (a.scores.get(&bit), b.scores.get(&bit)) else {
            continue; // goal not scored by both → not comparable on this axis
        };
        compared_any = true;
        // lower is better
        match sa.value.cmp(&sb.value) {
            std::cmp::Ordering::Less => a_strictly_better_somewhere = true,
            std::cmp::Ordering::Greater => b_strictly_better_somewhere = true,
            std::cmp::Ordering::Equal => {}
        }
    }

    if !compared_any {
        return Dominance::Neither;
    }
    match (a_strictly_better_somewhere, b_strictly_better_somewhere) {
        // a never worse, strictly better somewhere → a dominates b.
        (true, false) => Dominance::ADominatesB,
        (false, true) => Dominance::BDominatesA,
        // both better somewhere (trade-off) or fully equal → neither.
        _ => Dominance::Neither,
    }
}

/// Task 14.9 — compute the Pareto frontier: the candidates not dominated by any
/// other. Returns frontier candidate ids in ascending id order (deterministic).
#[must_use]
pub fn pareto_frontier(candidates: &[FrontierCandidate], enabled: GoalMask) -> Vec<u64> {
    let mut frontier: Vec<u64> = Vec::new();
    for (i, c) in candidates.iter().enumerate() {
        let dominated = candidates.iter().enumerate().any(|(j, other)| {
            i != j && matches!(dominance(c, other, enabled), Dominance::BDominatesA)
        });
        if !dominated {
            frontier.push(c.id);
        }
    }
    frontier.sort_unstable();
    frontier.dedup();
    frontier
}

/// Convenience: partition candidates into (frontier, dominated) id lists.
#[must_use]
pub fn partition_frontier(
    candidates: &[FrontierCandidate],
    enabled: GoalMask,
) -> (Vec<u64>, Vec<u64>) {
    let frontier = pareto_frontier(candidates, enabled);
    let dominated: Vec<u64> = candidates
        .iter()
        .map(|c| c.id)
        .filter(|id| !frontier.contains(id))
        .collect();
    (frontier, dominated)
}
