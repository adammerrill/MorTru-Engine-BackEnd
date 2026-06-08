//! Epic 17 / P1+P2 tests — StorePricer LTV + rate-sheet + LLPA.

use orchestrator::*;
use ref_data::{
    GseAgency, Ineligible, LlpaOccupancy, LlpaPricing, LlpaPropertyType, LlpaPurpose, LlpaScenario,
    PriceAdjustment, RateSheet, RateSheetEntry, RefDataError, RefDataResult,
};
use scenarios::Scenario;
use solver::ScenarioPricer;
use types::{BalanceType, Cents, CreditScore, LoanProduct, ProgramCode, TermMonths, Tier};

fn test_prov() -> types::Provenance {
    types::Provenance {
        dataset: "test".into(),
        source_file: "test".into(),
        source_citation: "test".into(),
        effective_date: "2026-06-05".into(),
        record_id: "t".into(),
        requested_version: 0,
        resolved_version: 0,
    }
}

struct FakeStore {
    sheet: RateSheet,
    llpa_total_bps: i32,
    llpa_ineligible: bool,
}
impl RateSheetStore for FakeStore {
    fn rate_sheet(&self, _lender: &str) -> Result<Option<RateSheet>, RefDataError> {
        Ok(Some(self.sheet.clone()))
    }
}
impl LlpaStore for FakeStore {
    fn llpa_price(
        &self,
        agency: GseAgency,
        _scenario: &LlpaScenario,
        _lender_id: Option<&str>,
        _year: u16,
    ) -> RefDataResult<Result<types::Derived<LlpaPricing>, Ineligible>> {
        if self.llpa_ineligible {
            return Ok(Err(Ineligible {
                reason: "test".into(),
            }));
        }
        let pricing = LlpaPricing {
            agency,
            adjustments: vec![PriceAdjustment {
                adjustment_type: "test".into(),
                description: "test".into(),
                bps: self.llpa_total_bps,
            }],
            gse_subtotal_bps: self.llpa_total_bps,
            lender_subtotal_bps: 0,
            capped: false,
            total_bps: self.llpa_total_bps,
        };
        Ok(Ok(types::Derived::new(pricing, test_prov())))
    }
}

fn store(llpa_bps: i32, ineligible: bool) -> FakeStore {
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
        llpa_total_bps: llpa_bps,
        llpa_ineligible: ineligible,
    }
}

fn ctx() -> PricingContext {
    PricingContext {
        indicator_score: CreditScore::new(760).unwrap(),
        purpose: LlpaPurpose::Purchase,
        occupancy: LlpaOccupancy::Primary,
        property_type: LlpaPropertyType::Detached,
        state: "TX".into(),
        is_first_time_homebuyer: false,
        is_high_cost_area: false,
        ami_percent: None,
        agency: GseAgency::Fannie,
        year: 2026,
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
        Cents::from_dollars(500_000),
        "L1",
        ctx(),
        Cents::from_dollars(100_000),
        Cents::from_dollars(500_000),
    )
}

#[test]
fn conv_product_maps_to_conv_code() {
    assert_eq!(
        rate_sheet_product(ProgramCode::Conventional, LoanProduct::FixedConv21To30),
        "conv_30yr_fixed"
    );
}
#[test]
fn price_at_uses_par_rate_and_ltv() {
    let s = store(0, false);
    let p = pricer(&s, conv_scenario());
    let pt = p.price_at(Cents::from_dollars(400_000)).unwrap();
    assert_eq!(pt.note_rate.0, 6000);
    assert_eq!(pt.ltv.0, 8000);
}
#[test]
fn product_is_fixed_classifies() {
    assert!(product_is_fixed(LoanProduct::FixedConv21To30));
    assert!(!product_is_fixed(LoanProduct::Arm5_6Sofr));
}
#[test]
fn out_of_bounds_unpriceable() {
    let s = store(0, false);
    let p = pricer(&s, conv_scenario());
    assert!(p.price_at(Cents::from_dollars(50_000)).is_none());
    assert!(p.price_at(Cents::from_dollars(600_000)).is_none());
}
#[test]
fn no_llpa_ctc_is_down_payment_only() {
    let s = store(0, false);
    let p = pricer(&s, conv_scenario());
    let pt = p.price_at(Cents::from_dollars(400_000)).unwrap();
    assert_eq!(pt.llpa_bps, 0);
    assert_eq!(pt.cash_to_close, Cents::from_dollars(100_000));
}
#[test]
fn positive_llpa_adds_points_to_ctc() {
    let s = store(125, false);
    let p = pricer(&s, conv_scenario());
    let pt = p.price_at(Cents::from_dollars(400_000)).unwrap();
    assert_eq!(pt.llpa_bps, 125);
    assert_eq!(pt.cash_to_close, Cents::from_dollars(105_000));
}
#[test]
fn negative_llpa_rebate_reduces_ctc() {
    let s = store(-50, false);
    let p = pricer(&s, conv_scenario());
    let pt = p.price_at(Cents::from_dollars(400_000)).unwrap();
    assert_eq!(pt.cash_to_close, Cents::from_dollars(98_000));
}
#[test]
fn ineligible_llpa_contributes_zero() {
    let s = store(125, true);
    let p = pricer(&s, conv_scenario());
    let pt = p.price_at(Cents::from_dollars(400_000)).unwrap();
    assert_eq!(pt.llpa_bps, 0);
    assert_eq!(pt.cash_to_close, Cents::from_dollars(100_000));
}
#[test]
fn government_program_skips_gse_llpa() {
    let fha = Scenario {
        program: ProgramCode::Fha,
        product: LoanProduct::FixedFha16To30,
        term: TermMonths(360),
        balance_type: BalanceType::Conforming,
        tier: Tier::Standard,
        mi_option: 0,
    };

    let s = store(125, false);
    let p = pricer(&s, fha);
    let pt = p.price_at(Cents::from_dollars(400_000)).unwrap();
    assert_eq!(pt.llpa_bps, 0);
}
#[test]
fn llpa_scales_with_balance() {
    let s = store(100, false);
    let p = pricer(&s, conv_scenario());
    let small = p.price_at(Cents::from_dollars(200_000)).unwrap();
    let large = p.price_at(Cents::from_dollars(400_000)).unwrap();
    let small_pts = small.cash_to_close.0 - Cents::from_dollars(300_000).0;
    let large_pts = large.cash_to_close.0 - Cents::from_dollars(100_000).0;
    assert_eq!(small_pts, Cents::from_dollars(2_000).0);
    assert_eq!(large_pts, Cents::from_dollars(4_000).0);
}
