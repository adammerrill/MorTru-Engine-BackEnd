//! Epic 8 / T8.1 tests — Conventional + FHA loan-product eligibility.

use eligibility::*;
fn nd() -> chrono::NaiveDate {
    chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()
}
use ref_data::{FhaLimitType, VersionId, *};
use types::{Cents, CreditScore, LtvBasisPoints, Occupancy, ProgramCode, PropertyType};

// ── Fake store with controllable rules + limits ─────────────────────────────

struct FakeStore {
    conv: ProgramEligibilityRules,
    fha: ProgramEligibilityRules,
    gse: GseLoanLimits,
    fha_lim: FhaLoanLimits,
}

fn conv_rules() -> ProgramEligibilityRules {
    ProgramEligibilityRules {
        program: ProgramCode::Conventional,
        min_credit_score: 620,
        min_credit_score_alt: None,
        alt_credit_min_down_payment_bps: None,
        max_ltv_bps: 9700,
        max_ltv_bps_alt_credit: None,
        max_ltv_bps_high_balance: Some(9500),
        front_end_dti_max_bps: 4500,
        requires_primary_residence: false,
        requires_first_time_buyer: false,
        requires_va_entitlement: false,
        requires_usda_eligibility: false,
        requires_ami_income_check: false,
        effective_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
    }
}
fn fha_rules() -> ProgramEligibilityRules {
    ProgramEligibilityRules {
        program: ProgramCode::Fha,
        min_credit_score: 580,
        min_credit_score_alt: Some(500),
        alt_credit_min_down_payment_bps: Some(1000), // 10% down → 500 min
        max_ltv_bps: 9650,
        max_ltv_bps_alt_credit: Some(9000),
        max_ltv_bps_high_balance: None,
        front_end_dti_max_bps: 3100,
        requires_primary_residence: true,
        requires_first_time_buyer: false,
        requires_va_entitlement: false,
        requires_usda_eligibility: false,
        requires_ami_income_check: false,
        effective_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
    }
}

impl EligibilityData for FakeStore {
    fn program_rules(&self, p: ProgramCode) -> RefDataResult<ProgramEligibilityRules> {
        Ok(match p {
            ProgramCode::Conventional => self.conv.clone(),
            ProgramCode::Fha => self.fha.clone(),
            other => {
                return Err(RefDataError::NotFound {
                    data_type: "program_rules",
                    fips: format!("{other:?}"),
                    year: 0,
                })
            }
        })
    }
    fn gse_loan_limits(&self, _f: &str, _year: u16) -> RefDataResult<Versioned<GseLoanLimits>> {
        Ok(Versioned {
            data: self.gse.clone(),
            version_id: VersionId::new("gse", nd()),
            effective_date: nd(),
        })
    }
    fn fha_loan_limits(&self, _f: &str, _year: u16) -> RefDataResult<Versioned<FhaLoanLimits>> {
        Ok(Versioned {
            data: self.fha_lim.clone(),
            version_id: VersionId::new("fha", nd()),
            effective_date: nd(),
        })
    }
}

fn store() -> FakeStore {
    FakeStore {
        conv: conv_rules(),
        fha: fha_rules(),
        // baseline 1-unit $766,550; 1u limit $766,550 (non-high-cost county)
        gse: GseLoanLimits {
            fips_code: "48453".into(),
            state_abbr: "TX".into(),
            county_name: "Travis".into(),
            cbsa_name: None,
            is_high_cost: true,
            effective_year: 2026,
            limit_1_unit: Cents(114_982_500),
            limit_2_unit: Cents(147_180_000),
            limit_3_unit: Cents(177_905_000),
            limit_4_unit: Cents(221_087_500),
        },
        // FHA "floor" county 1u $498,257
        fha_lim: FhaLoanLimits {
            fips_code: "48453".into(),
            state_abbr: "TX".into(),
            county_name: "Travis".into(),
            limit_type: FhaLimitType::Floor,
            effective_year: 2026,
            limit_1_unit: Cents(49_825_700),
            limit_2_unit: Cents(63_800_000),
            limit_3_unit: Cents(77_115_000),
            limit_4_unit: Cents(95_830_000),
        },
    }
}

fn scn(program: ProgramCode, score: u16, ltv_bps: u32, loan: i64) -> EligibilityScenario {
    EligibilityScenario {
        program,
        representative_score: CreditScore::new(score).unwrap(),
        loan_amount: Cents(loan),
        property_value: Cents((loan as f64 / (ltv_bps as f64 / 10_000.0)) as i64),
        ltv: LtvBasisPoints(ltv_bps),
        occupancy: Occupancy::PrimaryResidence,
        property_type: PropertyType::SingleFamilyDetached,
        county_fips: "48453".into(),
        year: 2026,
    }
}

fn eng(s: &FakeStore) -> StoreEligibilityEngine<'_, FakeStore> {
    StoreEligibilityEngine::new(s)
}

// ── Conventional ────────────────────────────────────────────────────────────

#[test]
fn conv_clean_scenario_is_eligible() {
    let s = store();
    let v = eng(&s)
        .evaluate_program(
            &scn(ProgramCode::Conventional, 740, 8000, 30_000_000),
            ProgramCode::Conventional,
        )
        .unwrap();
    assert!(v.value.is_eligible(), "{:?}", v.value.reasons());
}

#[test]
fn conv_low_credit_rejected() {
    let s = store();
    let v = eng(&s)
        .evaluate_program(
            &scn(ProgramCode::Conventional, 600, 8000, 30_000_000),
            ProgramCode::Conventional,
        )
        .unwrap();
    assert!(v.value.reasons().iter().any(|e| matches!(
        e,
        types::EligibilityError::CreditScoreBelowMinimum { minimum: 620, .. }
    )));
}

#[test]
fn conv_high_balance_tightens_ltv() {
    let s = store();
    // loan above baseline → high-balance → max LTV 95% not 97%. 96% LTV fails.
    let v = eng(&s)
        .evaluate_program(
            &scn(ProgramCode::Conventional, 760, 9600, 90_000_000),
            ProgramCode::Conventional,
        )
        .unwrap();
    assert!(v.value.reasons().iter().any(|e| matches!(
        e,
        types::EligibilityError::LtvExceedsLimit {
            limit_bps: 9500,
            ..
        }
    )));
}

#[test]
fn conv_over_limit_rejected() {
    let s = store();
    // 1-unit limit is $766,550; ask for $900k.
    let v = eng(&s)
        .evaluate_program(
            &scn(ProgramCode::Conventional, 760, 8000, 120_000_000),
            ProgramCode::Conventional,
        )
        .unwrap();
    assert!(v
        .value
        .reasons()
        .iter()
        .any(|e| matches!(e, types::EligibilityError::LoanAmountOutOfRange { .. })));
}

#[test]
fn conv_two_unit_uses_higher_limit() {
    let s = store();
    let mut sc = scn(ProgramCode::Conventional, 760, 8000, 120_000_000);
    sc.property_type = PropertyType::TwoUnit; // 2-unit limit $981,500 — $900k OK
    let v = eng(&s)
        .evaluate_program(&sc, ProgramCode::Conventional)
        .unwrap();
    assert!(!v
        .value
        .reasons()
        .iter()
        .any(|e| matches!(e, types::EligibilityError::LoanAmountOutOfRange { .. })));
}

// ── FHA ─────────────────────────────────────────────────────────────────────

#[test]
fn fha_clean_scenario_is_eligible() {
    let s = store();
    let v = eng(&s)
        .evaluate_program(
            &scn(ProgramCode::Fha, 680, 9650, 30_000_000),
            ProgramCode::Fha,
        )
        .unwrap();
    assert!(v.value.is_eligible(), "{:?}", v.value.reasons());
}

#[test]
fn fha_alt_credit_tier_with_ten_percent_down() {
    let s = store();
    // score 520 (<580) but 12% down (88% LTV) → alt min 500 applies, alt LTV cap 90%.
    let v = eng(&s)
        .evaluate_program(
            &scn(ProgramCode::Fha, 520, 8800, 30_000_000),
            ProgramCode::Fha,
        )
        .unwrap();
    // credit passes (520 ≥ 500 alt min), LTV 88% ≤ 90% alt cap.
    assert!(v.value.is_eligible(), "{:?}", v.value.reasons());
}

#[test]
fn fha_low_credit_low_down_rejected() {
    let s = store();
    // score 520, only 3.5% down (96.5% LTV) → alt min not unlocked (need 10% down).
    let v = eng(&s)
        .evaluate_program(
            &scn(ProgramCode::Fha, 520, 9650, 30_000_000),
            ProgramCode::Fha,
        )
        .unwrap();
    assert!(v.value.reasons().iter().any(|e| matches!(
        e,
        types::EligibilityError::CreditScoreBelowMinimum { minimum: 580, .. }
    )));
}

#[test]
fn fha_investment_occupancy_rejected() {
    let s = store();
    let mut sc = scn(ProgramCode::Fha, 700, 9000, 30_000_000);
    sc.occupancy = Occupancy::Investment;
    let v = eng(&s).evaluate_program(&sc, ProgramCode::Fha).unwrap();
    assert!(v
        .value
        .reasons()
        .iter()
        .any(|e| matches!(e, types::EligibilityError::IneligibleOccupancy { .. })));
}

#[test]
fn mobile_home_rejected_property_type() {
    let s = store();
    let mut sc = scn(ProgramCode::Conventional, 760, 8000, 30_000_000);
    sc.property_type = PropertyType::MobileHome;
    let v = eng(&s)
        .evaluate_program(&sc, ProgramCode::Conventional)
        .unwrap();
    assert!(v
        .value
        .reasons()
        .iter()
        .any(|e| matches!(e, types::EligibilityError::IneligiblePropertyType { .. })));
}

// ── Multi-failure + provenance ──────────────────────────────────────────────

#[test]
fn all_failures_reported_together() {
    let s = store();
    // low credit + over-limit + investment occupancy on FHA.
    let mut sc = scn(ProgramCode::Fha, 400, 9650, 90_000_000);
    sc.occupancy = Occupancy::Investment;
    let v = eng(&s).evaluate_program(&sc, ProgramCode::Fha).unwrap();
    let reasons = v.value.reasons();
    assert!(reasons.len() >= 3, "expected ≥3 reasons, got {reasons:?}");
}

#[test]
fn verdict_is_explainable() {
    let s = store();
    let v = eng(&s)
        .evaluate_program(
            &scn(ProgramCode::Conventional, 740, 8000, 30_000_000),
            ProgramCode::Conventional,
        )
        .unwrap();
    let text = v.explain();
    assert!(text.contains("Source:"));
    assert!(text.contains("credit_score") && text.contains("ltv") && text.contains("loan_limit"));
}

#[test]
fn unimplemented_program_errors_cleanly() {
    let s = store();
    let r = eng(&s).evaluate_program(
        &scn(ProgramCode::Va, 740, 8000, 30_000_000),
        ProgramCode::Va,
    );
    assert!(r.is_err()); // program_rules returns NotFound for VA in this fake
}
