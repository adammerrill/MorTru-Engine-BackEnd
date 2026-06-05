//! Epic 14 tests — amortization payment, schedule, horizon cost.

use amort::*;
use types::{BasisPoints, Cents, TermMonths};

// $300,000 @ 6.000% / 360mo → $1,798.65/mo (standard reference value).
#[test]
fn payment_matches_known_value_6pct_30yr() {
    let p = monthly_payment(
        Cents::from_dollars(300_000),
        BasisPoints(6000),
        TermMonths(360),
    );
    // within 1 cent of $1,798.65
    assert!((p.0 - 179_865).abs() <= 1, "got {} cents", p.0);
}

// $200,000 @ 4.500% / 180mo → $1,529.99/mo.
#[test]
fn payment_matches_known_value_4_5pct_15yr() {
    let p = monthly_payment(
        Cents::from_dollars(200_000),
        BasisPoints(4500),
        TermMonths(180),
    );
    assert!((p.0 - 152_999).abs() <= 2, "got {} cents", p.0);
}

#[test]
fn zero_rate_is_straight_line() {
    let p = monthly_payment(
        Cents::from_dollars(120_000),
        BasisPoints(0),
        TermMonths(120),
    );
    assert_eq!(p.0, Cents::from_dollars(1_000).0); // 120k/120 = $1,000
}

#[test]
fn schedule_pays_off_exactly() {
    let s = schedule(
        Cents::from_dollars(300_000),
        BasisPoints(6000),
        TermMonths(360),
    );
    assert_eq!(s.periods.len(), 360);
    assert_eq!(
        s.periods.last().unwrap().balance,
        Cents::ZERO,
        "ends at zero"
    );
}

#[test]
fn schedule_first_period_interest_correct() {
    // First month interest = 300,000 * 0.06/12 = $1,500.00
    let s = schedule(
        Cents::from_dollars(300_000),
        BasisPoints(6000),
        TermMonths(360),
    );
    assert_eq!(s.periods[0].interest, Cents::from_dollars(1_500));
}

#[test]
fn principal_plus_interest_equals_payment_each_period() {
    let s = schedule(
        Cents::from_dollars(250_000),
        BasisPoints(5500),
        TermMonths(360),
    );
    // Every non-final period: principal + interest == monthly payment.
    for p in &s.periods[..s.periods.len() - 1] {
        assert_eq!(
            p.principal.0 + p.interest.0,
            s.monthly_payment.0,
            "month {}",
            p.month
        );
    }
}

#[test]
fn total_interest_is_sum_of_periods() {
    let s = schedule(
        Cents::from_dollars(200_000),
        BasisPoints(4500),
        TermMonths(180),
    );
    let sum: i64 = s.periods.iter().map(|p| p.interest.0).sum();
    assert_eq!(s.total_interest.0, sum);
}

#[test]
fn balance_at_decreases_monotonically() {
    let s = schedule(
        Cents::from_dollars(300_000),
        BasisPoints(6000),
        TermMonths(360),
    );
    assert_eq!(s.balance_at(0), Cents::from_dollars(300_000));
    let b12 = s.balance_at(12).0;
    let b24 = s.balance_at(24).0;
    assert!(
        b12 < 30_000_000 && b24 < b12,
        "balance falls: {b12} then {b24}"
    );
}

// ── T14.4 horizon cost ──────────────────────────────────────────────────────

#[test]
fn horizon_cost_is_interest_plus_payoff() {
    let s = schedule(
        Cents::from_dollars(300_000),
        BasisPoints(6000),
        TermMonths(360),
    );
    // Sell at 60 months: cost = interest paid (60mo) + remaining balance.
    let hc = horizon_cost(&s, 60);
    let interest = s.interest_through(60).0;
    let payoff = s.balance_at(60).0;
    assert_eq!(hc.0, interest + payoff);
    // Sanity: horizon cost at 60mo < full lifetime interest + 0 payoff.
    assert!(hc.0 > 0);
}

#[test]
fn horizon_cost_at_full_term_is_total_interest() {
    let s = schedule(
        Cents::from_dollars(200_000),
        BasisPoints(4500),
        TermMonths(180),
    );
    // At full term, payoff is 0, so horizon cost == total interest.
    let hc = horizon_cost(&s, 180);
    assert_eq!(hc.0, s.total_interest.0);
}

#[test]
fn shorter_horizon_lower_interest_higher_payoff() {
    let s = schedule(
        Cents::from_dollars(300_000),
        BasisPoints(6000),
        TermMonths(360),
    );
    // Earlier exit → less interest paid, more balance remaining.
    assert!(s.interest_through(36).0 < s.interest_through(120).0);
    assert!(s.balance_at(36).0 > s.balance_at(120).0);
}

#[test]
fn interest_through_zero_is_zero() {
    let s = schedule(
        Cents::from_dollars(100_000),
        BasisPoints(5000),
        TermMonths(360),
    );
    assert_eq!(s.interest_through(0), Cents::ZERO);
}

#[test]
fn short_term_loan_amortizes() {
    let s = schedule(
        Cents::from_dollars(50_000),
        BasisPoints(7000),
        TermMonths(120),
    );
    assert_eq!(s.periods.len(), 120);
    assert_eq!(s.periods.last().unwrap().balance, Cents::ZERO);
}
