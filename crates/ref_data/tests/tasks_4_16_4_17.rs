//! Tasks 4.16 (LenderProfile + Overlays) and 4.17 (MI Provider + single premium).

use ref_data::{JsonFileStore, MiRateInput, RefDataStore};
use types::ProgramCode;

fn store() -> JsonFileStore {
    JsonFileStore::new(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data"))
}

// ════════════════════════════════════════════════════════════════════════════
// TASK 4.16 — Lender Profiles + Overlays
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn test_lender_profile_found_by_id() {
    let p = store()
        .lender_profile("lndr_abc_mortgage")
        .unwrap()
        .unwrap();
    assert_eq!(p.lender_id, "lndr_abc_mortgage");
    assert_eq!(p.name, "ABC Mortgage Company");
    assert!(p.active);
}

#[test]
fn test_lender_profile_has_nmls_id() {
    let p = store()
        .lender_profile("lndr_abc_mortgage")
        .unwrap()
        .unwrap();
    assert_eq!(p.nmls_id.as_deref(), Some("1234567"));
}

#[test]
fn test_lender_profile_unknown_returns_none() {
    let p = store().lender_profile("lndr_does_not_exist").unwrap();
    assert!(p.is_none());
}

#[test]
fn test_lender_profile_inactive_still_returned() {
    let p = store()
        .lender_profile("lndr_inactive_test")
        .unwrap()
        .unwrap();
    assert!(!p.active);
}

#[test]
fn test_lender_overlays_fha_for_abc() {
    let o = store()
        .lender_overlays("lndr_abc_mortgage", ProgramCode::Fha)
        .unwrap()
        .unwrap();
    assert_eq!(o.lender_id, "lndr_abc_mortgage");
    assert_eq!(o.program, ProgramCode::Fha);
    assert_eq!(o.min_credit_score_override, Some(620));
    assert_eq!(o.dti_max_bps_override, Some(2800));
    assert!(o.max_ltv_bps_override.is_none());
}

#[test]
fn test_lender_overlays_conventional_for_abc() {
    let o = store()
        .lender_overlays("lndr_abc_mortgage", ProgramCode::Conventional)
        .unwrap()
        .unwrap();
    assert_eq!(o.min_credit_score_override, Some(660));
    assert_eq!(o.max_ltv_bps_override, Some(9500));
}

#[test]
fn test_lender_overlays_no_overlay_returns_none() {
    // ABC has no USDA overlay
    let o = store()
        .lender_overlays("lndr_abc_mortgage", ProgramCode::Usda)
        .unwrap();
    assert!(o.is_none());
}

#[test]
fn test_lender_overlays_va_first_national() {
    let o = store()
        .lender_overlays("lndr_first_national", ProgramCode::Va)
        .unwrap()
        .unwrap();
    assert_eq!(o.min_credit_score_override, Some(640));
}

#[test]
fn test_lender_overlay_apply_tightens_min_fico() {
    let agency = store().program_rules(ProgramCode::Fha).unwrap();
    assert_eq!(agency.min_credit_score, 580, "FHA agency min is 580");

    let overlay = store()
        .lender_overlays("lndr_abc_mortgage", ProgramCode::Fha)
        .unwrap()
        .unwrap();
    let tightened = overlay.apply(&agency);
    assert_eq!(
        tightened.min_credit_score, 620,
        "overlay raises min FICO to 620"
    );
    assert_eq!(
        tightened.max_ltv_bps, agency.max_ltv_bps,
        "other fields unchanged"
    );
}

#[test]
fn test_lender_overlay_apply_tightens_max_ltv() {
    let agency = store().program_rules(ProgramCode::Conventional).unwrap();
    let overlay = store()
        .lender_overlays("lndr_abc_mortgage", ProgramCode::Conventional)
        .unwrap()
        .unwrap();
    let tightened = overlay.apply(&agency);
    assert_eq!(tightened.max_ltv_bps, 9500, "overlay caps LTV at 95%");
}

#[test]
fn test_lender_overlay_never_loosens_rules() {
    use chrono::NaiveDate;
    use ref_data::lender::LenderOverlays;
    let agency = store().program_rules(ProgramCode::Fha).unwrap();
    // Try to apply a "loosening" overlay (lower FICO, higher LTV — should be ignored)
    let bad_overlay = LenderOverlays {
        lender_id: "test".to_owned(),
        program: ProgramCode::Fha,
        min_credit_score_override: Some(500), // below agency 580
        max_ltv_bps_override: Some(10000),    // above agency max
        dti_max_bps_override: Some(9999),
        max_va_loan_amount_cents: None,
        effective_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
    };
    let result = bad_overlay.apply(&agency);
    // Agency values must win since they are stricter
    assert_eq!(
        result.min_credit_score, agency.min_credit_score,
        "cannot loosen FICO"
    );
    assert_eq!(result.max_ltv_bps, agency.max_ltv_bps, "cannot loosen LTV");
}

#[test]
fn test_lender_overlay_null_fields_preserve_agency() {
    let agency = store().program_rules(ProgramCode::Va).unwrap();
    let overlay = store()
        .lender_overlays("lndr_first_national", ProgramCode::Va)
        .unwrap()
        .unwrap();
    let tightened = overlay.apply(&agency);
    // Only min_credit_score was overridden (640); max_ltv and dti are None → preserve agency
    assert_eq!(tightened.max_ltv_bps, agency.max_ltv_bps);
    assert_eq!(
        tightened.front_end_dti_max_bps,
        agency.front_end_dti_max_bps
    );
}

#[test]
fn test_multiple_lenders_return_correct_overlays() {
    // First National: VA 640 FICO
    let fn_va = store()
        .lender_overlays("lndr_first_national", ProgramCode::Va)
        .unwrap()
        .unwrap();
    // ABC: FHA 620 FICO
    let abc_fha = store()
        .lender_overlays("lndr_abc_mortgage", ProgramCode::Fha)
        .unwrap()
        .unwrap();
    assert_ne!(
        fn_va.min_credit_score_override,
        abc_fha.min_credit_score_override
    );
}

// ════════════════════════════════════════════════════════════════════════════
// TASK 4.17 — MI Provider metadata + Single Premium BPMI NR rates
// ════════════════════════════════════════════════════════════════════════════

fn sp_input(ltv_bps: u32, cov_pct: u8, fico: u16) -> MiRateInput {
    MiRateInput {
        ltv_bps,
        coverage_pct: cov_pct,
        fico,
        term_months: 360,
        is_non_fixed: false,
    }
}

#[test]
fn test_nmi_sp_bp_nr_30yr_ltv96_cov35_fico760_is_158bps() {
    let r = store()
        .mi_single_premium_bps("nmi", &sp_input(9600, 35, 760), 2025)
        .unwrap();
    assert_eq!(
        r, 158,
        "NMI SP BPMI NR: 95-97% LTV, 35% cov, 760+ FICO = 1.58%"
    );
}

#[test]
fn test_nmi_sp_bp_nr_30yr_ltv92_cov30_fico700() {
    let r = store()
        .mi_single_premium_bps("nmi", &sp_input(9200, 30, 700), 2025)
        .unwrap();
    assert_eq!(r, 252);
}

#[test]
fn test_nmi_sp_bp_nr_30yr_ltv87_cov12_fico760() {
    let r = store()
        .mi_single_premium_bps("nmi", &sp_input(8700, 12, 760), 2025)
        .unwrap();
    assert_eq!(r, 59);
}

#[test]
fn test_nmi_sp_is_higher_than_monthly_for_same_scenario() {
    // Single premium bundles the entire coverage period; should exceed monthly rate
    let monthly = store()
        .mi_monthly_rate("nmi", &sp_input(9200, 25, 760), 2025)
        .unwrap();
    let single_prem = store()
        .mi_single_premium_bps("nmi", &sp_input(9200, 25, 760), 2025)
        .unwrap();
    // Monthly is annual bps; single premium is a one-time charge
    // A 30yr loan at 34 bps/yr × 30 = 1020 expected present-value floor
    // Single premium (120 bps) < 30×monthly: single premium has different pricing economics
    assert!(
        single_prem > monthly,
        "single premium ({single_prem}) > monthly rate ({monthly})"
    );
}

#[test]
fn test_nmi_sp_year_fallback() {
    let r2022 = store()
        .mi_single_premium_bps("nmi", &sp_input(9600, 35, 760), 2022)
        .unwrap();
    let r2030 = store()
        .mi_single_premium_bps("nmi", &sp_input(9600, 35, 760), 2030)
        .unwrap();
    assert_eq!(r2022, r2030);
}
