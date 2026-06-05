//! Epic 12 tests — five-gate pruning pipeline.

use scenarios::*;
use types::{Cents, ProgramCode};

fn ctx() -> PruningContext {
    PruningContext {
        monthly_payment_budget: None,
        upfront_cash_budget: None,
        loan_limit: None,
        loan_amount: Cents(30_000_000),
        estimated_monthly_payment: None,
        estimated_cash_to_close: None,
    }
}

fn one_scenario() -> Scenario {
    enumerate_program(ProgramCode::Conventional)
        .into_iter()
        .next()
        .unwrap()
}

// ── Gate 1: eligibility (injected) ──────────────────────────────────────────

struct RejectAll;
impl EligibilityGate for RejectAll {
    fn is_eligible(&self, _: &Scenario) -> bool {
        false
    }
}
impl NetPricingGate for RejectAll {
    fn within_net_pricing_cap(&self, _: &Scenario) -> bool {
        false
    }
}

#[test]
fn eligibility_gate_rejects() {
    let o = evaluate_gates(&one_scenario(), &ctx(), &RejectAll, &PassAll);
    assert_eq!(o, GateOutcome::FailedEligibility);
}

#[test]
fn all_pass_survives() {
    let o = evaluate_gates(&one_scenario(), &ctx(), &PassAll, &PassAll);
    assert_eq!(o, GateOutcome::Survived);
}

// ── Gate 2: payment capacity ────────────────────────────────────────────────

#[test]
fn payment_over_budget_fails() {
    let mut c = ctx();
    c.monthly_payment_budget = Some(Cents(200_000)); // $2,000
    c.estimated_monthly_payment = Some(Cents(250_000)); // $2,500
    assert_eq!(
        evaluate_gates(&one_scenario(), &c, &PassAll, &PassAll),
        GateOutcome::FailedPaymentCapacity
    );
}

#[test]
fn payment_within_budget_passes_gate() {
    let mut c = ctx();
    c.monthly_payment_budget = Some(Cents(300_000));
    c.estimated_monthly_payment = Some(Cents(250_000));
    assert_eq!(
        evaluate_gates(&one_scenario(), &c, &PassAll, &PassAll),
        GateOutcome::Survived
    );
}

#[test]
fn unmeasured_payment_skips_gate() {
    let mut c = ctx();
    c.monthly_payment_budget = Some(Cents(100));
    c.estimated_monthly_payment = None; // can't measure → can't reject
    assert_eq!(
        evaluate_gates(&one_scenario(), &c, &PassAll, &PassAll),
        GateOutcome::Survived
    );
}

// ── Gate 3: cash floor ──────────────────────────────────────────────────────

#[test]
fn cash_over_budget_fails() {
    let mut c = ctx();
    c.upfront_cash_budget = Some(Cents(1_000_000)); // $10k
    c.estimated_cash_to_close = Some(Cents(1_500_000)); // $15k
    assert_eq!(
        evaluate_gates(&one_scenario(), &c, &PassAll, &PassAll),
        GateOutcome::FailedCashFloor
    );
}

// ── Gate 4: loan limit ──────────────────────────────────────────────────────

#[test]
fn loan_over_limit_fails() {
    let mut c = ctx();
    c.loan_amount = Cents(90_000_000);
    c.loan_limit = Some(Cents(76_655_000));
    assert_eq!(
        evaluate_gates(&one_scenario(), &c, &PassAll, &PassAll),
        GateOutcome::FailedLoanLimit
    );
}

// ── Gate 5: net-pricing cap (injected) ──────────────────────────────────────

struct PriceReject;
impl NetPricingGate for PriceReject {
    fn within_net_pricing_cap(&self, _: &Scenario) -> bool {
        false
    }
}

#[test]
fn net_pricing_cap_rejects() {
    assert_eq!(
        evaluate_gates(&one_scenario(), &ctx(), &PassAll, &PriceReject),
        GateOutcome::FailedNetPricingCap
    );
}

// ── Gate ordering: first failure wins ───────────────────────────────────────

#[test]
fn eligibility_checked_before_pricing() {
    // Both injected gates reject → eligibility (gate 1) reported, not pricing.
    let o = evaluate_gates(&one_scenario(), &ctx(), &RejectAll, &RejectAll);
    assert_eq!(o, GateOutcome::FailedEligibility);
}

// ── prune / count_survivors over the universe ───────────────────────────────

#[test]
fn prune_passall_keeps_everything() {
    let universe = enumerate_program(ProgramCode::Conventional);
    let total = universe.len() as u64;
    let n = count_survivors(universe.into_iter(), &ctx(), &PassAll, &PassAll);
    assert_eq!(n, total);
}

#[test]
fn prune_rejectall_keeps_nothing() {
    let universe = enumerate_program(ProgramCode::Conventional);
    let n = count_survivors(universe.into_iter(), &ctx(), &RejectAll, &PassAll);
    assert_eq!(n, 0);
}

#[test]
fn prune_loan_limit_filters_universe() {
    let universe = enumerate_program(ProgramCode::Conventional);
    let mut c = ctx();
    c.loan_amount = Cents(90_000_000);
    c.loan_limit = Some(Cents(76_655_000)); // every scenario over limit
    let n = count_survivors(universe.into_iter(), &c, &PassAll, &PassAll);
    assert_eq!(n, 0);
}

#[test]
fn gate_outcome_survived_predicate() {
    assert!(GateOutcome::Survived.survived());
    assert!(!GateOutcome::FailedLoanLimit.survived());
}
