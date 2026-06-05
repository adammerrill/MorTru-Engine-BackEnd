//! Epic 13 / Task T13.2 — iterative convergence solver.
//!
//! Solves the circular dependency (starting balance ↔ LTV ↔ MI ↔ LLPA ↔ rate
//! ↔ cash-to-close) by bisection on the starting loan balance until the
//! realized target meets the goal within tolerance.
//!
//! ## Dependency discipline
//! The solver does NOT depend on `ref_data`/`amort`. Pricing a balance is the
//! `ScenarioPricer` seam — the composition crate injects a real implementation
//! backed by `ref_data` (MI/LLPA) + `amort` (payment/horizon). Here it is a
//! trait, tested against a fixture pricer.
//!
//! ## Why bisection
//! The balance→CTC function is monotone but **piecewise** (MI/LLPA tier edges
//! introduce jumps). Bisection needs no derivative, tolerates discontinuities,
//! and converges in ≤ log2(range/tolerance) iterations — ~20 for a $1M range
//! at $1 tolerance. Newton/secant would thrash at tier crosses.

use types::{BasisPoints, Cents, Derived, LtvBasisPoints, Provenance};

/// What the borrower's goal targets — the quantity the solver drives to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolveTarget {
    CashToClose(Cents),
    MonthlyPayment(Cents),
    HorizonCost(Cents),
}

impl SolveTarget {
    #[must_use]
    pub fn value(self) -> Cents {
        match self {
            SolveTarget::CashToClose(c)
            | SolveTarget::MonthlyPayment(c)
            | SolveTarget::HorizonCost(c) => c,
        }
    }
}

/// A scenario priced at one candidate starting balance — the output of one
/// iteration. The `realized` value is compared to the goal target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PricedPoint {
    pub balance: Cents,
    pub ltv: LtvBasisPoints,
    pub mi: Cents,
    pub llpa_bps: i32,
    pub note_rate: BasisPoints,
    pub monthly_payment: Cents,
    pub cash_to_close: Cents,
    pub horizon_cost: Cents,
}

impl PricedPoint {
    /// The quantity this point realizes for a given target kind.
    #[must_use]
    pub fn realized(&self, target: SolveTarget) -> Cents {
        match target {
            SolveTarget::CashToClose(_) => self.cash_to_close,
            SolveTarget::MonthlyPayment(_) => self.monthly_payment,
            SolveTarget::HorizonCost(_) => self.horizon_cost,
        }
    }
}

/// Why a solve did not converge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NonConvergeReason {
    MaxIters,
    GoalInfeasibleInBounds,
    NoEligibleBalance,
}

/// A non-convergent solve carries the closest attempt for UI fallback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NonConvergent {
    pub reason: NonConvergeReason,
    pub best_attempt: PricedPoint,
}

/// Solver configuration. Defaults: 20 iterations, $1 tolerance.
#[derive(Debug, Clone, Copy)]
pub struct SolverConfig {
    pub max_iters: u16,
    pub tolerance: Cents,
}

impl Default for SolverConfig {
    fn default() -> Self {
        SolverConfig {
            max_iters: 20,
            tolerance: Cents::from_dollars(1),
        }
    }
}

/// The pricing seam: price a scenario at a candidate starting balance.
/// Implemented by the composition crate over `ref_data` + `amort`; here it is
/// injected so the solver is dependency-clean and fixture-testable.
pub trait ScenarioPricer {
    /// Price at `balance`. Returns `None` if the balance is not priceable
    /// (e.g. outside eligible bounds for this scenario).
    fn price_at(&self, balance: Cents) -> Option<PricedPoint>;

    /// Eligible starting-balance bounds `[min, max]` for the scenario.
    fn balance_bounds(&self) -> (Cents, Cents);
}

fn prov() -> Provenance {
    Provenance {
        dataset: "iterative_solver".to_owned(),
        source_file: "solver::converge".to_owned(),
        source_citation: "Epic 13 T13.2 bisection convergence".to_owned(),
        effective_date: "2026-06-05".to_owned(),
        record_id: "solve".to_owned(),
        requested_version: 0,
        resolved_version: 0,
    }
}

/// Which of two priced points is closer to the target.
fn closer(target: SolveTarget, a: PricedPoint, b: PricedPoint) -> PricedPoint {
    let da = (a.realized(target).0 - target.value().0).abs();
    let db = (b.realized(target).0 - target.value().0).abs();
    if da <= db {
        a
    } else {
        b
    }
}

/// T13.2 — solve one scenario for one target by bisection on starting balance.
///
/// Returns a `Derived<PricedPoint>` (the converged point + full iteration
/// trail) on success, or `NonConvergent` with the closest attempt.
///
/// # Monotonicity assumption
/// `realized(balance)` is assumed non-decreasing in `balance` (more loan → more
/// CTC/payment/horizon cost). The bisection direction relies on this.
#[allow(clippy::result_large_err)]
pub fn solve(
    pricer: &impl ScenarioPricer,
    target: SolveTarget,
    config: SolverConfig,
) -> Result<Derived<PricedPoint>, NonConvergent> {
    let (mut lo, mut hi) = pricer.balance_bounds();

    // Seed `best` with a priceable endpoint; if neither prices, no eligible balance.
    let seed = pricer.price_at(lo).or_else(|| pricer.price_at(hi));
    let mut best = match seed {
        Some(p) => p,
        None => {
            // Construct a zero point only to carry the reason; bounds unpriceable.
            return Err(NonConvergent {
                reason: NonConvergeReason::NoEligibleBalance,
                best_attempt: PricedPoint {
                    balance: lo,
                    ltv: LtvBasisPoints(0),
                    mi: Cents::ZERO,
                    llpa_bps: 0,
                    note_rate: BasisPoints(0),
                    monthly_payment: Cents::ZERO,
                    cash_to_close: Cents::ZERO,
                    horizon_cost: Cents::ZERO,
                },
            });
        }
    };

    let mut steps: Vec<(String, String, String)> = Vec::new();

    for i in 0..config.max_iters {
        let mid = Cents((lo.0 + hi.0) / 2);
        let Some(priced) = pricer.price_at(mid) else {
            // Unpriceable mid: shrink toward the priceable side.
            hi = mid;
            continue;
        };
        let realized = priced.realized(target).0;
        let diff = realized - target.value().0;
        steps.push((
            "solver_iter".to_owned(),
            format!(
                "i={i} balance={} ltv={} mi={} llpa={} rate={} realized={realized}",
                mid.0, priced.ltv.0, priced.mi.0, priced.llpa_bps, priced.note_rate.0
            ),
            format!("diff={diff}"),
        ));
        best = closer(target, best, priced);

        if diff.abs() <= config.tolerance.0 {
            let mut d = Derived::new(priced, prov());
            for (r, inp, out) in steps {
                d.push_step(r, inp, out);
            }
            return Ok(d);
        }
        // realized too high → need smaller balance; too low → larger.
        if diff > 0 {
            hi = mid;
        } else {
            lo = mid;
        }
        // Bracket collapsed without hitting tolerance → infeasible between
        // adjacent priceable balances (tier discontinuity).
        if hi.0 - lo.0 <= 1 {
            return Err(NonConvergent {
                reason: NonConvergeReason::GoalInfeasibleInBounds,
                best_attempt: best,
            });
        }
    }

    Err(NonConvergent {
        reason: NonConvergeReason::MaxIters,
        best_attempt: best,
    })
}
