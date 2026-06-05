//! Epic 14 — TRID-compliant amortization (core schedule + horizon cost).
//!
//! Fixed-rate fully-amortizing math, computed in `rust_decimal` for exactness
//! then rounded to whole `Cents`. The standard payment formula:
//!
//!   P = L · r / (1 − (1+r)^−n)
//!
//! where L = principal (decimal dollars), r = monthly rate (annual/12), n = term
//! months. r = 0 degrades to L/n.
//!
//! ## Tasks
//! - **T14.1** `AmortizationSchedule` — period rows + totals.
//! - **T14.2** `monthly_payment(...)` — the payment formula, cents-exact.
//! - **T14.3** `schedule(...)` / `balance_at(month)` — period table + payoff.
//! - **T14.4** `horizon_cost(...)` — interest paid + remaining balance over the
//!   borrower's hold period (the horizon-cost `GoalMask` input).
//!
//! MI-cancellation rules (HPA 78% conv / FHA 132-mo / VA none / USDA annual)
//! are a flagged follow-up — they ride on top of this schedule, not within it.

use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use types::{BasisPoints, Cents, TermMonths};

/// One period of the amortization schedule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AmortPeriod {
    /// 1-based month index.
    pub month: u16,
    pub interest: Cents,
    pub principal: Cents,
    /// Remaining balance after this period.
    pub balance: Cents,
}

/// A full amortization schedule with precomputed totals.
#[derive(Debug, Clone)]
pub struct AmortizationSchedule {
    pub principal: Cents,
    pub annual_rate: BasisPoints,
    pub term: TermMonths,
    pub monthly_payment: Cents,
    pub periods: Vec<AmortPeriod>,
    pub total_interest: Cents,
}

impl AmortizationSchedule {
    /// Remaining balance after `month` payments (0 = original principal).
    /// Saturates at the schedule end.
    #[must_use]
    pub fn balance_at(&self, month: u16) -> Cents {
        if month == 0 {
            return self.principal;
        }
        let idx = (month as usize).min(self.periods.len());
        if idx == 0 {
            self.principal
        } else {
            self.periods[idx - 1].balance
        }
    }

    /// Cumulative interest paid through `month` (inclusive).
    #[must_use]
    pub fn interest_through(&self, month: u16) -> Cents {
        let idx = (month as usize).min(self.periods.len());
        let sum: i64 = self.periods[..idx].iter().map(|p| p.interest.0).sum();
        Cents(sum)
    }
}

fn dec_cents(c: Cents) -> Decimal {
    Decimal::from(c.0) / Decimal::from(100)
}
fn to_cents(d: Decimal) -> Cents {
    let cents = (d * Decimal::from(100)).round();
    Cents(cents.to_i64().unwrap_or(0))
}

/// T14.2 — the fixed monthly payment (P&I), cents-exact.
/// `r = 0` → straight-line principal/term.
#[must_use]
pub fn monthly_payment(principal: Cents, annual_rate: BasisPoints, term: TermMonths) -> Cents {
    let l = dec_cents(principal);
    let n = i64::from(term.0);
    if n <= 0 {
        return Cents::ZERO;
    }
    let r = annual_rate.to_decimal_rate() / Decimal::from(12);
    if r.is_zero() {
        return to_cents(l / Decimal::from(n));
    }
    // (1+r)^n via repeated multiply (n ≤ 360, exact in Decimal).
    let one_plus_r = Decimal::ONE + r;
    let mut pow = Decimal::ONE;
    for _ in 0..n {
        pow *= one_plus_r;
    }
    // P = L·r / (1 − (1+r)^−n) = L·r·pow / (pow − 1)
    let payment = l * r * pow / (pow - Decimal::ONE);
    to_cents(payment)
}

/// T14.1 + T14.3 — build the full schedule. The final period absorbs rounding
/// drift so the balance lands exactly at zero.
#[must_use]
pub fn schedule(
    principal: Cents,
    annual_rate: BasisPoints,
    term: TermMonths,
) -> AmortizationSchedule {
    let payment = monthly_payment(principal, annual_rate, term);
    let n = term.0;
    let monthly_r = annual_rate.to_decimal_rate() / Decimal::from(12);

    let mut periods = Vec::with_capacity(n as usize);
    let mut balance = principal;
    let mut total_interest = Cents::ZERO;

    for month in 1..=n {
        let interest = to_cents(dec_cents(balance) * monthly_r);
        let mut principal_paid = Cents(payment.0 - interest.0);

        // Final period (or overshoot): pay off exactly.
        if month == n || principal_paid.0 >= balance.0 {
            principal_paid = balance;
        }
        balance = Cents(balance.0 - principal_paid.0);
        total_interest = Cents(total_interest.0 + interest.0);
        periods.push(AmortPeriod {
            month,
            interest,
            principal: principal_paid,
            balance,
        });
        if balance.0 <= 0 {
            break;
        }
    }

    AmortizationSchedule {
        principal,
        annual_rate,
        term,
        monthly_payment: payment,
        periods,
        total_interest,
    }
}

/// T14.4 — horizon cost: total interest paid through the hold period PLUS the
/// remaining balance owed at payoff. This is the figure the "Lowest Horizon
/// Cost" `GoalMask` goal minimizes — what the loan actually costs if sold/
/// refinanced at `hold_months`, not over the full term.
#[must_use]
pub fn horizon_cost(schedule: &AmortizationSchedule, hold_months: u16) -> Cents {
    let interest = schedule.interest_through(hold_months);
    let payoff = schedule.balance_at(hold_months);
    Cents(interest.0 + payoff.0)
}
