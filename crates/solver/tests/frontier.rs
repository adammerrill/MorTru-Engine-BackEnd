//! Task 14.9 tests — Pareto frontier.

use solver::*;
use types::{GoalMask, Provenance};

fn score(value: i64) -> GoalScore {
    types::Derived::new(
        value,
        Provenance {
            dataset: "test".into(),
            source_file: "test".into(),
            source_citation: "test".into(),
            effective_date: "2026".into(),
            record_id: "t".into(),
            requested_version: 0,
            resolved_version: 0,
        },
    )
}

fn cand(id: u64, pairs: &[(GoalMask, i64)]) -> FrontierCandidate {
    let mut scores = GoalScores::new();
    for (g, v) in pairs {
        scores.insert(g.bits(), score(*v));
    }
    FrontierCandidate { id, scores }
}

const RATE: GoalMask = GoalMask::LOWEST_RATE;
const CTC: GoalMask = GoalMask::LOWEST_CASH_TO_CLOSE;

// ── dominance ───────────────────────────────────────────────────────────────

#[test]
fn strictly_better_on_all_dominates() {
    let a = cand(1, &[(RATE, 100), (CTC, 100)]);
    let b = cand(2, &[(RATE, 200), (CTC, 200)]);
    assert_eq!(dominance(&a, &b, RATE | CTC), Dominance::ADominatesB);
}

#[test]
fn better_on_one_equal_on_other_dominates() {
    let a = cand(1, &[(RATE, 100), (CTC, 100)]);
    let b = cand(2, &[(RATE, 100), (CTC, 200)]); // equal rate, worse ctc
    assert_eq!(dominance(&a, &b, RATE | CTC), Dominance::ADominatesB);
}

#[test]
fn tradeoff_is_neither() {
    let a = cand(1, &[(RATE, 100), (CTC, 200)]); // better rate, worse ctc
    let b = cand(2, &[(RATE, 200), (CTC, 100)]); // worse rate, better ctc
    assert_eq!(dominance(&a, &b, RATE | CTC), Dominance::Neither);
}

#[test]
fn equal_is_neither() {
    let a = cand(1, &[(RATE, 100), (CTC, 100)]);
    let b = cand(2, &[(RATE, 100), (CTC, 100)]);
    assert_eq!(dominance(&a, &b, RATE | CTC), Dominance::Neither);
}

#[test]
fn no_shared_goals_is_neither() {
    let a = cand(1, &[(RATE, 100)]);
    let b = cand(2, &[(CTC, 100)]);
    assert_eq!(dominance(&a, &b, RATE | CTC), Dominance::Neither);
}

// ── frontier ────────────────────────────────────────────────────────────────

#[test]
fn single_goal_frontier_is_best_only() {
    let cands = vec![
        cand(1, &[(RATE, 100)]),
        cand(2, &[(RATE, 200)]),
        cand(3, &[(RATE, 150)]),
    ];
    assert_eq!(pareto_frontier(&cands, RATE), vec![1]);
}

#[test]
fn single_goal_ties_all_survive() {
    let cands = vec![cand(1, &[(RATE, 100)]), cand(2, &[(RATE, 100)])];
    // Neither dominates (equal) → both on frontier.
    assert_eq!(pareto_frontier(&cands, RATE), vec![1, 2]);
}

#[test]
fn dominated_scenario_excluded() {
    let cands = vec![
        cand(1, &[(RATE, 100), (CTC, 100)]), // dominates 3
        cand(2, &[(RATE, 200), (CTC, 50)]),  // trade-off vs 1
        cand(3, &[(RATE, 300), (CTC, 300)]), // dominated by 1
    ];
    let f = pareto_frontier(&cands, RATE | CTC);
    assert!(f.contains(&1) && f.contains(&2));
    assert!(!f.contains(&3), "3 is dominated");
}

#[test]
fn frontier_members_are_mutually_nondominated() {
    // Property: no frontier member dominates another.
    let cands = vec![
        cand(1, &[(RATE, 100), (CTC, 400)]),
        cand(2, &[(RATE, 200), (CTC, 300)]),
        cand(3, &[(RATE, 300), (CTC, 200)]),
        cand(4, &[(RATE, 400), (CTC, 100)]),
        cand(5, &[(RATE, 250), (CTC, 350)]), // dominated by 2
    ];
    let f = pareto_frontier(&cands, RATE | CTC);
    // 1-4 are the trade-off curve; 5 is dominated by 2.
    assert_eq!(f, vec![1, 2, 3, 4]);
    // verify mutual non-domination among frontier members
    let members: Vec<_> = cands.iter().filter(|c| f.contains(&c.id)).collect();
    for a in &members {
        for b in &members {
            if a.id != b.id {
                assert_eq!(
                    dominance(a, b, RATE | CTC),
                    Dominance::Neither,
                    "{} vs {} should be non-dominated",
                    a.id,
                    b.id
                );
            }
        }
    }
}

#[test]
fn empty_input_empty_frontier() {
    assert!(pareto_frontier(&[], RATE).is_empty());
}

#[test]
fn frontier_is_deterministic_sorted() {
    let cands = vec![
        cand(3, &[(RATE, 100)]),
        cand(1, &[(RATE, 100)]),
        cand(2, &[(RATE, 100)]),
    ];
    assert_eq!(pareto_frontier(&cands, RATE), vec![1, 2, 3]); // sorted ids
}

#[test]
fn partition_splits_frontier_and_dominated() {
    let cands = vec![
        cand(1, &[(RATE, 100), (CTC, 100)]),
        cand(2, &[(RATE, 300), (CTC, 300)]), // dominated
    ];
    let (frontier, dominated) = partition_frontier(&cands, RATE | CTC);
    assert_eq!(frontier, vec![1]);
    assert_eq!(dominated, vec![2]);
}
