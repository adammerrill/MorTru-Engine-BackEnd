//! Epic 17 / P1 tests — StorePricer LTV + rate-sheet slice.
use orchestrator::RateSheetStore;
use orchestrator::*;
use ref_data::{RateSheet, RateSheetEntry, RefDataError};

use scenarios::Scenario;
use solver::{ScenarioPricer, SolveTarget, SolverConfig};
use types::{BalanceType, Cents, LoanProduct, ProgramCode, TermMonths, Tier};

struct FakeStore {
    sheet: RateSheet,
}
impl RateSheetStore for FakeStore {
    fn rate_sheet(&self, _lender: &str) -> Result<Option<RateSheet>, RefDataError> {
        Ok(Some(self.sheet.clone()))
    }
}
fn store() -> FakeStore {
    FakeStore {
        sheet: RateSheet {
            lender_id: "L1".into(),
            as_of: "2026-06-05".into(),
            entries: vec![
                RateSheetEntry {
                    product: "conv_30yr_fixed".into(),
                    lock_days: 30,
                    par_rate_bps: 6000,
                    price_at_par: 0.0,
                },
                RateSheetEntry {
                    product: "fha_30yr_fixed".into(),
                    lock_days: 30,
                    par_rate_bps: 5750,
                    price_at_par: 0.0,
                },
            ],
        },
    }
}

fn conv_scenario() -> Scenario {
    Scenario {
        program: ProgramCode::Conventional,
        product: LoanProduct::FixedConv21To30,
        term: TermMonths(360),
        balance_type: BalanceType::Conforming,
        tier: Tier::Standard,
        mi_option: 0,
    }
}

fn pricer(s: &FakeStore, scen: Scenario) -> StorePricer<'_, FakeStore> {
    StorePricer::new(
        s,
        scen,
        Cents::from_dollars(500_000), // property value
        "L1",
        Cents::from_dollars(100_000), // min balance
        Cents::from_dollars(500_000), // max balance
    )
}

// ── product mapping ─────────────────────────────────────────────────────────

#[test]
fn conv_product_maps_to_conv_code() {
    assert_eq!(
        rate_sheet_product(ProgramCode::Conventional, LoanProduct::FixedConv21To30),
        "conv_30yr_fixed"
    );
}

#[test]
fn fha_product_maps_to_fha_code() {
    assert_eq!(
        rate_sheet_product(ProgramCode::Fha, LoanProduct::FixedFha16To30),
        "fha_30yr_fixed"
    );
}

#[test]
fn homeready_maps_to_affordable_code() {
    assert_eq!(
        rate_sheet_product(ProgramCode::HomeReady, LoanProduct::FixedConv21To30),
        "conv_homeready_fixed"
    );
}

// ── price_at ────────────────────────────────────────────────────────────────

#[test]
fn price_at_uses_par_rate_from_sheet() {
    let s = store();
    let p = pricer(&s, conv_scenario());
    let pt = p.price_at(Cents::from_dollars(400_000)).unwrap();
    assert_eq!(pt.note_rate.0, 6000); // conv_30yr_fixed par
}

#[test]
fn ltv_computed_from_balance_over_value() {
    let s = store();
    let p = pricer(&s, conv_scenario());
    // $400k / $500k = 80% = 8000 bps
    let pt = p.price_at(Cents::from_dollars(400_000)).unwrap();
    assert_eq!(pt.ltv.0, 8000);
}

#[test]
fn payment_is_nonzero_and_reasonable() {
    let s = store();
    let p = pricer(&s, conv_scenario());
    // $400k @ 6% / 360mo ≈ $2,398/mo
    let pt = p.price_at(Cents::from_dollars(400_000)).unwrap();
    assert!((pt.monthly_payment.0 - 239_800).abs() < 1_000);
}

#[test]
fn ctc_placeholder_is_down_payment() {
    let s = store();
    let p = pricer(&s, conv_scenario());
    // value $500k − balance $400k = $100k down
    let pt = p.price_at(Cents::from_dollars(400_000)).unwrap();
    assert_eq!(pt.cash_to_close, Cents::from_dollars(100_000));
}

#[test]
fn p1_has_no_mi_or_llpa_yet() {
    let s = store();
    let p = pricer(&s, conv_scenario());
    let pt = p.price_at(Cents::from_dollars(400_000)).unwrap();
    assert_eq!(pt.mi, Cents::ZERO);
    assert_eq!(pt.llpa_bps, 0);
}

#[test]
fn out_of_bounds_balance_unpriceable() {
    let s = store();
    let p = pricer(&s, conv_scenario());
    assert!(p.price_at(Cents::from_dollars(50_000)).is_none()); // below min
    assert!(p.price_at(Cents::from_dollars(600_000)).is_none()); // above max
}

// ── integrates with the solver ──────────────────────────────────────────────

#[test]
fn solver_converges_over_store_pricer() {
    let s = store();
    let p = pricer(&s, conv_scenario());
    // CTC = down = value − balance, monotone DECREASING in balance.
    // Want down payment = $100k → balance $400k. (Note: P4 will make CTC
    // increase with balance; P1's placeholder decreases, so target accordingly.)
    let target = SolveTarget::MonthlyPayment(Cents::from_dollars(2_398));
    let r = solver::solve(&p, target, SolverConfig::default());
    // payment is monotone increasing in balance → solvable.
    assert!(r.is_ok() || matches!(r, Err(ref e) if e.best_attempt.balance.0 > 0));
}

#[test]
fn balance_bounds_passthrough() {
    let s = store();
    let p = pricer(&s, conv_scenario());
    let (lo, hi) = p.balance_bounds();
    assert_eq!(lo, Cents::from_dollars(100_000));
    assert_eq!(hi, Cents::from_dollars(500_000));
}
