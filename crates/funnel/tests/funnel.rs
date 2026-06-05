//! Epic 16 / T3+T4+T5 — funnel trait, stub impl, orchestration tests.

use funnel::{step, BorrowerInput, FunnelStage, PartialAnalysisInput, ScenarioFunnel, StubFunnel};
use types::{Cents, CreditScore, Occupancy, ProgramCode, TermMonths};

fn b(score: u16, income: i64) -> BorrowerInput {
    BorrowerInput {
        occupancy: Occupancy::PrimaryResidence,
        credit_scores: vec![CreditScore::new(score).unwrap()],
        va: None,
        annual_income: Cents::from_dollars(income),
    }
}

/// Build an input completed through a given stage.
fn through_eligible() -> PartialAnalysisInput {
    let mut p = PartialAnalysisInput {
        borrower_count: Some(1),
        property_use: Some(Occupancy::PrimaryResidence),
        ..Default::default()
    };
    p.borrowers.push(b(740, 90_000));
    p
}
fn through_qualified() -> PartialAnalysisInput {
    let mut p = through_eligible();
    p.preferred_term = Some(TermMonths(360));
    p.monthly_payment_budget = Some(Cents::from_dollars(2_500));
    p
}
fn through_in_budget() -> PartialAnalysisInput {
    let mut p = through_qualified();
    p.upfront_cash_budget = Some(Cents::from_dollars(40_000));
    p
}

// ── FunnelStage gating ──────────────────────────────────────────────────────

#[test]
fn stage_none_before_borrower_details() {
    let p = PartialAnalysisInput {
        borrower_count: Some(1),
        property_use: Some(Occupancy::PrimaryResidence),
        ..Default::default()
    };
    assert_eq!(FunnelStage::from_input(&p), FunnelStage::None);
}

#[test]
fn stage_eligible_after_borrower_details() {
    assert_eq!(
        FunnelStage::from_input(&through_eligible()),
        FunnelStage::Eligible
    );
}

#[test]
fn stage_qualified_after_monthly_budget() {
    assert_eq!(
        FunnelStage::from_input(&through_qualified()),
        FunnelStage::Qualified
    );
}

#[test]
fn stage_in_budget_after_cash_budget() {
    assert_eq!(
        FunnelStage::from_input(&through_in_budget()),
        FunnelStage::InBudget
    );
}

#[test]
fn stage_predicates() {
    assert!(!FunnelStage::None.has_eligible());
    assert!(FunnelStage::Eligible.has_eligible());
    assert!(!FunnelStage::Eligible.has_qualified());
    assert!(FunnelStage::Qualified.has_qualified());
    assert!(!FunnelStage::Qualified.has_in_budget());
    assert!(FunnelStage::InBudget.has_in_budget());
}

// ── step() orchestration: only unlocked counts present ──────────────────────

#[test]
fn step_none_returns_no_counts() {
    let f = StubFunnel::default();
    let r = step(&PartialAnalysisInput::default(), &f);
    assert_eq!(r.stage, FunnelStage::None);
    assert!(r.eligible.is_none() && r.qualified.is_none() && r.in_budget.is_none());
}

#[test]
fn step_eligible_only() {
    let f = StubFunnel::default();
    let r = step(&through_eligible(), &f);
    assert!(r.eligible.is_some());
    assert!(r.qualified.is_none() && r.in_budget.is_none());
}

#[test]
fn step_qualified_unlocks_two() {
    let f = StubFunnel::default();
    let r = step(&through_qualified(), &f);
    assert!(r.eligible.is_some() && r.qualified.is_some());
    assert!(r.in_budget.is_none());
}

#[test]
fn step_in_budget_unlocks_all_three() {
    let f = StubFunnel::default();
    let r = step(&through_in_budget(), &f);
    assert!(r.eligible.is_some() && r.qualified.is_some() && r.in_budget.is_some());
}

// ── Monotonic narrowing invariant: eligible ≥ qualified ≥ in_budget ─────────

#[test]
fn counts_narrow_monotonically() {
    let f = StubFunnel::default();
    let r = step(&through_in_budget(), &f);
    assert!(r.is_monotonic());
    let e = r.eligible.unwrap().value;
    let q = r.qualified.unwrap().value;
    let bdg = r.in_budget.unwrap().value;
    assert!(e >= q, "eligible {e} >= qualified {q}");
    assert!(q >= bdg, "qualified {q} >= in_budget {bdg}");
}

#[test]
fn is_monotonic_true_with_partial_counts() {
    // Only eligible present → vacuously monotonic.
    let f = StubFunnel::default();
    let r = step(&through_eligible(), &f);
    assert!(r.is_monotonic());
}

// ── Stub filters respond to constraints (more constraints → fewer) ──────────

#[test]
fn program_constraint_reduces_eligible() {
    let f = StubFunnel::default();
    let unconstrained = f.eligible(&through_eligible()).value;
    let mut p = through_eligible();
    p.program = Some(ProgramCode::Fha);
    let constrained = f.eligible(&p).value;
    assert!(
        constrained < unconstrained,
        "{constrained} < {unconstrained}"
    );
}

#[test]
fn investment_occupancy_reduces_eligible() {
    let f = StubFunnel::default();
    let primary = f.eligible(&through_eligible()).value;
    let mut p = through_eligible();
    p.property_use = Some(Occupancy::Investment);
    let investment = f.eligible(&p).value;
    assert!(investment < primary);
}

#[test]
fn two_borrowers_slightly_increase_eligible() {
    let f = StubFunnel::default();
    let one = f.eligible(&through_eligible()).value;
    let mut p = through_eligible();
    p.borrower_count = Some(2);
    p.borrowers.push(b(720, 70_000));
    let two = f.eligible(&p).value;
    assert!(two >= one);
}

#[test]
fn lower_monthly_budget_reduces_qualified() {
    let f = StubFunnel::default();
    let mut hi = through_qualified();
    hi.monthly_payment_budget = Some(Cents::from_dollars(9_000));
    let mut lo = through_qualified();
    lo.monthly_payment_budget = Some(Cents::from_dollars(1_500));
    assert!(f.qualified(&lo).value <= f.qualified(&hi).value);
}

#[test]
fn seller_credits_relax_in_budget() {
    let f = StubFunnel::default();
    let base = f.in_budget(&through_in_budget()).value;
    let mut p = through_in_budget();
    p.seller_credits.concessions_requested = Some(Cents::from_dollars(8_000));
    assert!(f.in_budget(&p).value >= base);
}

// ── Provenance flows through every count ────────────────────────────────────

#[test]
fn every_count_is_explainable() {
    let f = StubFunnel::default();
    let r = step(&through_in_budget(), &f);
    for d in [
        r.eligible.unwrap(),
        r.qualified.unwrap(),
        r.in_budget.unwrap(),
    ] {
        let text = d.explain();
        assert!(text.contains("Source:"));
        assert!(text.contains("stub"), "cites stub basis: {text}");
        assert!(text.contains("Derivation:"));
    }
}

#[test]
fn custom_universe_scales_counts() {
    let small = StubFunnel::new(40);
    let large = StubFunnel::new(4000);
    assert!(large.eligible(&through_eligible()).value > small.eligible(&through_eligible()).value);
}
