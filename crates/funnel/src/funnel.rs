//! Epic 16 / Tasks T3+T4 — the funnel computation seam and a deterministic stub.
//!
//! T3 defines `ScenarioFunnel` — the trait Epics 8–12 (`eligibility`,
//! `scenarios`) implement once real enumeration exists. The funnel returns
//! three monotonically-narrowing counts, each a `Derived<u64>`:
//!
//!   eligible  ≥  qualified  ≥  in_budget
//!
//! - **eligible**  — scenarios whose program/location/borrower inputs pass
//!   loan-product eligibility (unlocks after `WizardStep::BorrowerDetails`).
//! - **qualified** — eligible scenarios that also pass the term/payment gates
//!   (unlocks after `MonthlyPaymentBudget`).
//! - **in_budget** — qualified scenarios within the stated monthly + cash
//!   budgets (unlocks after `UpfrontCashBudget`, refined by later steps).
//!
//! T4 ships `StubFunnel`: a deterministic implementation over a fixed notional
//! scenario universe so the funnel is end-to-end testable BEFORE the real
//! enumeration crates land. It applies coarse, explainable filters driven by
//! the partial input. **Replaced by real enumeration when Epics 8–12 land** —
//! callers depend only on the `ScenarioFunnel` trait, never on `StubFunnel`.

use crate::contract::{PartialAnalysisInput, WizardStep};
use types::{Derived, Provenance};

/// Which funnel counts the current input has unlocked. Stepwise: a later count
/// cannot unlock before its prerequisite step is complete.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunnelStage {
    /// No count yet — borrower details incomplete.
    None,
    /// `eligible` available (through `BorrowerDetails`).
    Eligible,
    /// `eligible` + `qualified` (through `MonthlyPaymentBudget`).
    Qualified,
    /// All three (through `UpfrontCashBudget` and beyond).
    InBudget,
}

impl FunnelStage {
    /// The stage unlocked by the furthest completed step.
    #[must_use]
    pub fn from_input(input: &PartialAnalysisInput) -> FunnelStage {
        // Prerequisite chain: BorrowerDetails → MonthlyPaymentBudget → UpfrontCashBudget.
        if !input.step_complete(WizardStep::BorrowerDetails) {
            return FunnelStage::None;
        }
        if !input.step_complete(WizardStep::MonthlyPaymentBudget) {
            return FunnelStage::Eligible;
        }
        if !input.step_complete(WizardStep::UpfrontCashBudget) {
            return FunnelStage::Qualified;
        }
        FunnelStage::InBudget
    }

    #[must_use]
    pub fn has_eligible(self) -> bool {
        !matches!(self, FunnelStage::None)
    }
    #[must_use]
    pub fn has_qualified(self) -> bool {
        matches!(self, FunnelStage::Qualified | FunnelStage::InBudget)
    }
    #[must_use]
    pub fn has_in_budget(self) -> bool {
        matches!(self, FunnelStage::InBudget)
    }
}

/// The three funnel counts, each carrying provenance. A count is `None` until
/// its stage unlocks — the web layer renders only present counts.
#[derive(Debug, Clone)]
pub struct FunnelResponse {
    pub stage: FunnelStage,
    pub eligible: Option<Derived<u64>>,
    pub qualified: Option<Derived<u64>>,
    pub in_budget: Option<Derived<u64>>,
}

impl FunnelResponse {
    /// Invariant the UI relies on: counts narrow monotonically where present.
    #[must_use]
    pub fn is_monotonic(&self) -> bool {
        let e = self.eligible.as_ref().map(|d| d.value);
        let q = self.qualified.as_ref().map(|d| d.value);
        let b = self.in_budget.as_ref().map(|d| d.value);
        let eq = match (e, q) {
            (Some(e), Some(q)) => e >= q,
            _ => true,
        };
        let qb = match (q, b) {
            (Some(q), Some(b)) => q >= b,
            _ => true,
        };
        eq && qb
    }
}

/// The funnel computation seam. Epics 8–12 implement this over real scenario
/// enumeration; `StubFunnel` implements it deterministically for now.
///
/// Implementors compute counts for whatever stages the input has unlocked.
/// Each count MUST be a `Derived<u64>` so provenance flows to the client, and
/// the results MUST satisfy `eligible ≥ qualified ≥ in_budget`.
pub trait ScenarioFunnel {
    fn eligible(&self, input: &PartialAnalysisInput) -> Derived<u64>;
    fn qualified(&self, input: &PartialAnalysisInput) -> Derived<u64>;
    fn in_budget(&self, input: &PartialAnalysisInput) -> Derived<u64>;
}

// ── T4 — deterministic stub ─────────────────────────────────────────────────

/// A deterministic `ScenarioFunnel` over a notional fixed scenario universe.
/// Counts are derived by coarse, explainable filters so the funnel computes
/// end-to-end before the real enumeration crates exist.
///
/// **Not production pricing.** Replaced by `eligibility`/`scenarios` (Epics
/// 8–12) implementing `ScenarioFunnel` over the real scenario set.
#[derive(Debug, Clone, Copy)]
pub struct StubFunnel {
    /// Size of the notional scenario universe before any filtering.
    universe: u64,
}

impl Default for StubFunnel {
    fn default() -> Self {
        // A plausible notional universe: program × term × coverage × MI-plan
        // combinations. Fixed so counts are deterministic and testable.
        StubFunnel { universe: 480 }
    }
}

impl StubFunnel {
    #[must_use]
    pub fn new(universe: u64) -> Self {
        StubFunnel { universe }
    }

    fn provenance(&self, record: &str) -> Provenance {
        Provenance {
            dataset: "funnel_stub".to_owned(),
            source_file: "stub_funnel".to_owned(),
            source_citation: "Epic 16 T4 deterministic stub — NOT real enumeration".to_owned(),
            effective_date: "2026-06-05".to_owned(),
            record_id: record.to_owned(),
            requested_version: 0,
            resolved_version: 0,
        }
    }

    /// Coarse eligibility retention: narrows the universe by occupancy and
    /// program presence. Deterministic, monotone in "more constraints = fewer".
    fn eligible_count(&self, input: &PartialAnalysisInput) -> u64 {
        let mut n = self.universe;
        // A specific program constrains to that family's share.
        if input.program.is_some() {
            n = n / 4 + n % 4; // ~one program family
        }
        // Investment occupancy excludes government-only products.
        if matches!(input.property_use, Some(types::Occupancy::Investment)) {
            n = n * 3 / 5;
        }
        // More borrowers → marginally more product fits (≥2 borrower credits).
        if input.borrower_count.unwrap_or(1) >= 2 {
            n = n + n / 20;
        }
        n.min(self.universe)
    }

    /// Qualified ⊆ eligible: term + monthly-payment gate retention.
    fn qualified_count(&self, input: &PartialAnalysisInput, eligible: u64) -> u64 {
        let mut n = eligible;
        // A fixed preferred term collapses the term sweep to one band.
        if input.preferred_term.is_some() {
            n /= 3;
        }
        // A monthly budget prunes higher-payment scenarios.
        if let Some(budget) = input.monthly_payment_budget {
            // Lower budgets retain fewer; scale by a coarse, bounded factor.
            let dollars = (budget.0 / 100).max(0) as u64; // cents→dollars
            let retain_pct = (dollars / 50).clamp(40, 90); // 40–90%
            n = n * retain_pct / 100;
        }
        n.min(eligible)
    }

    /// In-budget ⊆ qualified: cash-to-close + seller-credit retention.
    fn in_budget_count(&self, input: &PartialAnalysisInput, qualified: u64) -> u64 {
        let mut n = qualified;
        if let Some(cash) = input.upfront_cash_budget {
            let dollars = (cash.0 / 100).max(0) as u64;
            let retain_pct = (dollars / 1_000).clamp(30, 95); // 30–95%
            n = n * retain_pct / 100;
        }
        // Seller-paid credits relax the cash constraint slightly.
        if input.seller_credits.concessions_requested.is_some()
            || input
                .seller_credits
                .agent_commission_paid_by_seller
                .is_some()
        {
            n = (n + n / 10).min(qualified);
        }
        n.min(qualified)
    }
}

// ── T5 — orchestration ──────────────────────────────────────────────────────

/// Compute the funnel response for the current partial input against any
/// `ScenarioFunnel` implementation. Returns only the counts the input has
/// unlocked (stepwise), each a `Derived<u64>`.
pub fn step(input: &PartialAnalysisInput, funnel: &impl ScenarioFunnel) -> FunnelResponse {
    let stage = FunnelStage::from_input(input);
    FunnelResponse {
        stage,
        eligible: stage.has_eligible().then(|| funnel.eligible(input)),
        qualified: stage.has_qualified().then(|| funnel.qualified(input)),
        in_budget: stage.has_in_budget().then(|| funnel.in_budget(input)),
    }
}

impl ScenarioFunnel for StubFunnel {
    fn eligible(&self, input: &PartialAnalysisInput) -> Derived<u64> {
        let n = self.eligible_count(input);
        let mut d = Derived::new(n, self.provenance("eligible"));
        d.push_step(
            "stub_eligibility_filter",
            format!(
                "universe={} program={:?} use={:?} borrowers={:?}",
                self.universe, input.program, input.property_use, input.borrower_count
            ),
            format!("{n} eligible scenarios (stub)"),
        );
        d
    }

    fn qualified(&self, input: &PartialAnalysisInput) -> Derived<u64> {
        let eligible = self.eligible_count(input);
        let n = self.qualified_count(input, eligible);
        let mut d = Derived::new(n, self.provenance("qualified"));
        d.push_step(
            "stub_qualification_filter",
            format!(
                "eligible={eligible} term={:?} monthly_budget={:?}",
                input.preferred_term, input.monthly_payment_budget
            ),
            format!("{n} qualified scenarios (stub; ⊆ {eligible} eligible)"),
        );
        d
    }

    fn in_budget(&self, input: &PartialAnalysisInput) -> Derived<u64> {
        let eligible = self.eligible_count(input);
        let qualified = self.qualified_count(input, eligible);
        let n = self.in_budget_count(input, qualified);
        let mut d = Derived::new(n, self.provenance("in_budget"));
        d.push_step(
            "stub_budget_filter",
            format!(
                "qualified={qualified} cash_budget={:?} seller_credits={}",
                input.upfront_cash_budget,
                input.seller_credits.concessions_requested.is_some()
                    || input
                        .seller_credits
                        .agent_commission_paid_by_seller
                        .is_some()
            ),
            format!("{n} qualified-in-budget scenarios (stub; ⊆ {qualified} qualified)"),
        );
        d
    }
}
