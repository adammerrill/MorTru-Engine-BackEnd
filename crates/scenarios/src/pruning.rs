//! Epic 12 — five-gate scenario pruning pipeline.
//!
//! Narrows the enumerated universe (Epic 11) to feasible scenarios. The five
//! gates, in order: **eligibility → payment capacity → cash floor → loan limit
//! → net-pricing cap**. A scenario must clear every gate to survive.
//!
//! ## Dependency discipline
//! `scenarios` must NOT depend on `eligibility` or `ref_data` (the `analysis`
//! composition crate owns those). So the two data-dependent gates — eligibility
//! and net-pricing — are **trait-injected**: the caller passes closures/impls
//! that consult those crates. The three self-contained gates (payment, cash,
//! loan-limit) are implemented here directly over budget + scenario data.
//!
//! ## Tasks
//! - **T12.1** `GateOutcome` + `PrunedScenario` (survivor + per-gate trail).
//! - **T12.2** the three self-contained gates.
//! - **T12.3** `EligibilityGate` / `NetPricingGate` injection traits.
//! - **T12.4** `prune(scenarios, budget, gates)` pipeline.

use crate::Scenario;
use types::Cents;

/// Which gate a scenario failed, or that it survived.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateOutcome {
    Survived,
    FailedEligibility,
    FailedPaymentCapacity,
    FailedCashFloor,
    FailedLoanLimit,
    FailedNetPricingCap,
}

impl GateOutcome {
    #[must_use]
    pub fn survived(self) -> bool {
        matches!(self, GateOutcome::Survived)
    }
}

/// Budget + amount context the self-contained gates evaluate against. All
/// borrower-stated (pre-PII): no verified income/assets.
#[derive(Debug, Clone, Copy)]
pub struct PruningContext {
    /// Stated max monthly all-in payment. `None` = payment gate not yet unlocked.
    pub monthly_payment_budget: Option<Cents>,
    /// Stated max upfront cash-to-close. `None` = cash gate not yet unlocked.
    pub upfront_cash_budget: Option<Cents>,
    /// County loan limit for the scenario's program/units. `None` = unknown.
    pub loan_limit: Option<Cents>,
    /// The scenario's loan amount.
    pub loan_amount: Cents,
    /// Estimated monthly payment for the scenario (P&I+MI+esc), if computed.
    /// `None` skips the payment gate (it can't fail what it can't measure).
    pub estimated_monthly_payment: Option<Cents>,
    /// Estimated cash-to-close for the scenario, if computed.
    pub estimated_cash_to_close: Option<Cents>,
}

/// Injected eligibility gate (gate 1). The `analysis` crate implements this by
/// calling `eligibility::EligibilityEngine`; here it is just a predicate.
pub trait EligibilityGate {
    fn is_eligible(&self, scenario: &Scenario) -> bool;
}

/// Injected net-pricing gate (gate 5). The `analysis` crate implements this by
/// calling `ref_data` LLPA pricing; here it is just a predicate.
pub trait NetPricingGate {
    /// True if the scenario's net price is within the acceptable cap.
    fn within_net_pricing_cap(&self, scenario: &Scenario) -> bool;
}

/// A no-op gate that passes everything — used when a gate is not yet wired or
/// not applicable at the current funnel stage.
#[derive(Debug, Clone, Copy)]
pub struct PassAll;
impl EligibilityGate for PassAll {
    fn is_eligible(&self, _: &Scenario) -> bool {
        true
    }
}
impl NetPricingGate for PassAll {
    fn within_net_pricing_cap(&self, _: &Scenario) -> bool {
        true
    }
}

/// Evaluate all five gates for one scenario, in order, returning the first
/// failure or `Survived`. Self-contained gates skip when their input is `None`
/// (an unmeasurable gate cannot reject).
#[must_use]
pub fn evaluate_gates(
    scenario: &Scenario,
    ctx: &PruningContext,
    eligibility: &impl EligibilityGate,
    net_pricing: &impl NetPricingGate,
) -> GateOutcome {
    // Gate 1 — eligibility (injected).
    if !eligibility.is_eligible(scenario) {
        return GateOutcome::FailedEligibility;
    }
    // Gate 2 — payment capacity.
    if let (Some(budget), Some(payment)) =
        (ctx.monthly_payment_budget, ctx.estimated_monthly_payment)
    {
        if payment.0 > budget.0 {
            return GateOutcome::FailedPaymentCapacity;
        }
    }
    // Gate 3 — cash floor.
    if let (Some(cash_budget), Some(cash_needed)) =
        (ctx.upfront_cash_budget, ctx.estimated_cash_to_close)
    {
        if cash_needed.0 > cash_budget.0 {
            return GateOutcome::FailedCashFloor;
        }
    }
    // Gate 4 — loan limit.
    if let Some(limit) = ctx.loan_limit {
        if ctx.loan_amount.0 > limit.0 {
            return GateOutcome::FailedLoanLimit;
        }
    }
    // Gate 5 — net-pricing cap (injected).
    if !net_pricing.within_net_pricing_cap(scenario) {
        return GateOutcome::FailedNetPricingCap;
    }
    GateOutcome::Survived
}

/// Prune an iterator of scenarios, returning only survivors.
pub fn prune<'a>(
    scenarios: impl Iterator<Item = Scenario> + 'a,
    ctx: &'a PruningContext,
    eligibility: &'a impl EligibilityGate,
    net_pricing: &'a impl NetPricingGate,
) -> impl Iterator<Item = Scenario> + 'a {
    scenarios.filter(move |s| evaluate_gates(s, ctx, eligibility, net_pricing).survived())
}

/// Count survivors without materializing — the funnel's count primitive.
#[must_use]
pub fn count_survivors(
    scenarios: impl Iterator<Item = Scenario>,
    ctx: &PruningContext,
    eligibility: &impl EligibilityGate,
    net_pricing: &impl NetPricingGate,
) -> u64 {
    prune(scenarios, ctx, eligibility, net_pricing).count() as u64
}
