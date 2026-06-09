//! Epic 17 / P1+P2 tests — StorePricer LTV + rate-sheet + LLPA.

use orchestrator::*;
use ref_data::{
    ConvMiCoverage, ConvMiInput, FhaMipInput, FhaMipResult, GseAgency, Ineligible, LlpaOccupancy,
    LlpaPricing, LlpaPropertyType, LlpaPurpose, LlpaScenario, MiCompany, MiPlan, MiRateQuote,
    MiScenario, MiUnavailable, MipDuration, PriceAdjustment, RateSheet, RateSheetEntry,
    RefDataError, RefDataResult, UsdaGuaranteeFees, VaFeeInput,
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
    mi_net_milli_pct: i32,
    fha: (u16, u16),
    va_fee_bps: u32,
    usda: (u32, u32),
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

impl MiStore for FakeStore {
    fn conv_mi_coverage(&self, _i: &ConvMiInput, _y: u16) -> RefDataResult<ConvMiCoverage> {
        Ok(ConvMiCoverage {
            standard_pct: 25,
            minimum_pct: 25,
            llpa_with_minimum: false,
        })
    }
    fn mi_rate_quote(
        &self,
        company: MiCompany,
        _s: &MiScenario,
        _y: u16,
    ) -> RefDataResult<Result<types::Derived<MiRateQuote>, MiUnavailable>> {
        let q = MiRateQuote {
            company,
            plan: MiPlan::MonthlyBpmi,
            base_milli_pct: self.mi_net_milli_pct,
            adjustments: vec![],
            net_milli_pct: self.mi_net_milli_pct,
            floored: false,
        };
        Ok(Ok(types::Derived::new(q, test_prov())))
    }
    fn fha_mip(&self, _i: &FhaMipInput, _y: u16) -> RefDataResult<FhaMipResult> {
        Ok(FhaMipResult {
            ufmip_bps: self.fha.0,
            annual_mip_bps: self.fha.1,
            duration: MipDuration::LoanTerm,
        })
    }
    fn va_funding_fee(&self, _i: &VaFeeInput, _y: u16) -> RefDataResult<u32> {
        Ok(self.va_fee_bps)
    }
    fn usda_guarantee_fees(&self, _y: u16) -> RefDataResult<UsdaGuaranteeFees> {
        Ok(UsdaGuaranteeFees {
            upfront_fee_bps: self.usda.0,
            annual_fee_bps: self.usda.1,
            effective_date: "2026".into(),
            fiscal_year: 2026,
        })
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
                RateSheetEntry {
                    product: "va_30yr_fixed".into(),
                    lock_days: 30,
                    par_rate_bps: 5625,
                    price_at_par: 0.0,
                },
                RateSheetEntry {
                    product: "usda_30yr_fixed".into(),
                    lock_days: 30,
                    par_rate_bps: 5875,
                    price_at_par: 0.0,
                },
            ],
        },
        llpa_total_bps: llpa_bps,
        llpa_ineligible: ineligible,
        mi_net_milli_pct: 0,
        fha: (0, 0),
        va_fee_bps: 0,
        usda: (0, 0),
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

// ── P3 MI tests ──────────────────────────────────────────────────────────────

fn store_full() -> FakeStore {
    let mut s = store(0, false);
    s.mi_net_milli_pct = 580; // 0.58% annual conv PMI
    s.fha = (175, 55); // 1.75% UFMIP, 0.55% annual MIP
    s.va_fee_bps = 215; // 2.15% funding fee
    s.usda = (100, 35); // 1.00% upfront, 0.35% annual
    s
}

fn scen(program: ProgramCode, product: LoanProduct) -> Scenario {
    Scenario {
        program,
        product,
        term: TermMonths(360),
        balance_type: BalanceType::Conforming,
        tier: Tier::Standard,
        mi_option: 0,
    }
}

#[test]
fn conv_no_pmi_at_or_below_80_ltv() {
    let s = store_full();
    let p = pricer(
        &s,
        scen(ProgramCode::Conventional, LoanProduct::FixedConv21To30),
    );
    let pt = p.price_at(Cents::from_dollars(400_000)).unwrap(); // 80%
    assert_eq!(pt.mi, Cents::ZERO);
}

#[test]
fn conv_pmi_above_80_ltv_adds_monthly() {
    let s = store_full();
    let p = pricer(
        &s,
        scen(ProgramCode::Conventional, LoanProduct::FixedConv21To30),
    );
    let pt = p.price_at(Cents::from_dollars(450_000)).unwrap(); // 90%
    assert!(pt.mi.0 > 0);
    // ~ 450_000_00 * 580 / 100_000 / 12 = $217.50/mo
    assert!((pt.mi.0 - 21_750).abs() < 50);
}

#[test]
fn fha_has_upfront_and_monthly_mip() {
    let s = store_full();
    let p = pricer(&s, scen(ProgramCode::Fha, LoanProduct::FixedFha16To30));
    let pt = p.price_at(Cents::from_dollars(450_000)).unwrap();
    assert!(pt.mi.0 > 0);
    // CTC includes $50k down + $7,875 UFMIP
    assert!(pt.cash_to_close.0 >= Cents::from_dollars(57_875).0 - 100);
}

#[test]
fn va_has_upfront_fee_no_monthly() {
    let s = store_full();
    let p = pricer(&s, scen(ProgramCode::Va, LoanProduct::FixedVa16To30));
    let pt = p.price_at(Cents::from_dollars(450_000)).unwrap();
    // 2.15% of $450k = $9,675 upfront, no monthly
    assert!((pt.mi.0 - Cents::from_dollars(9_675).0).abs() < 100);
}

#[test]
fn usda_has_upfront_and_annual() {
    let s = store_full();
    let p = pricer(&s, scen(ProgramCode::Usda, LoanProduct::FixedUsda30));
    let pt = p.price_at(Cents::from_dollars(450_000)).unwrap();
    assert!(pt.mi.0 > 0);
    assert!(pt.cash_to_close.0 >= Cents::from_dollars(54_500).0 - 100); // down + 1% upfront
}

#[test]
fn fha_mi_raises_payment() {
    let s = store_full();
    let with_mi = pricer(&s, scen(ProgramCode::Fha, LoanProduct::FixedFha16To30))
        .price_at(Cents::from_dollars(450_000))
        .unwrap();
    let s0 = store(0, false);
    let no_mi = pricer(&s0, scen(ProgramCode::Fha, LoanProduct::FixedFha16To30))
        .price_at(Cents::from_dollars(450_000))
        .unwrap();
    assert!(with_mi.monthly_payment.0 > no_mi.monthly_payment.0);
}
