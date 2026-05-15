//! Tasks 4.2, 4.5, 4.6, 4.21, 4.22 — JsonFileStore + loan limits +
//! program eligibility rules + HOI estimation.
//!
//! All tests use the JsonFileStore with seed data in `crates/ref_data/data/`.

use ref_data::{
    AllProgramRules, JsonFileStore, ProgramEligibilityRules, RefDataStore, StateHoiRate,
    NATIONAL_FALLBACK_RATE_BPS,
};
use types::{Cents, CreditScore, DtiBasisPoints, LtvBasisPoints, ProgramCode};

/// Path to the test data directory shipped with the crate.
fn data_dir() -> std::path::PathBuf {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.join("data")
}

fn store() -> JsonFileStore {
    JsonFileStore::new(data_dir())
}

// ════════════════════════════════════════════════════════════════════════════
// TASK 4.2 — JsonFileStore core behaviour
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn test_store_creates_from_path() {
    let s = store();
    assert!(s.data_dir.exists(), "data_dir must exist");
}

#[test]
fn test_store_missing_fips_returns_not_found() {
    let err = store().fha_loan_limits("99999", 2025).unwrap_err();
    assert!(
        matches!(err, ref_data::RefDataError::NotFound { .. }),
        "unknown FIPS should return NotFound, got {err:?}"
    );
}

#[test]
fn test_store_missing_dataset_returns_storage_error() {
    let err = store().fha_loan_limits("48209", 1900).unwrap_err();
    // No file for year ≤ 1900 → Storage error
    assert!(
        matches!(err, ref_data::RefDataError::Storage(_)),
        "no file for year 1900 should return Storage, got {err:?}"
    );
}

#[test]
fn test_store_current_version_fha() {
    let v = store().current_version("fha_limits").unwrap();
    assert!(
        v.as_str().contains("fha_limits"),
        "version should contain dataset name"
    );
    assert!(v.as_str().contains("2025"), "should find 2025 file");
}

#[test]
fn test_store_current_version_gse() {
    let v = store().current_version("gse_limits").unwrap();
    assert!(v.as_str().contains("2025"));
}

// ════════════════════════════════════════════════════════════════════════════
// TASK 4.5 — FHA loan limits
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn test_fha_hays_county_tx_floor_2025() {
    let v = store().fha_loan_limits("48209", 2025).unwrap();
    let l = &v.data;
    assert_eq!(l.fips_code, "48209");
    assert_eq!(l.state_abbr, "TX");
    assert_eq!(l.county_name, "Hays");
    assert_eq!(l.limit_type, ref_data::FhaLimitType::Floor);
    // 2025 FHA floor: $524,225 = Cents(52_422_500)
    assert_eq!(l.limit_1_unit, Cents(52_422_500));
}

#[test]
fn test_fha_four_unit_hays_county() {
    let l = store().fha_loan_limits("48209", 2025).unwrap().data;
    assert_eq!(l.limit_for(1), Cents(52_422_500));
    assert_eq!(l.limit_for(4), Cents(100_857_500));
}

#[test]
fn test_fha_san_francisco_high_cost() {
    let l = store().fha_loan_limits("06075", 2025).unwrap().data;
    assert_eq!(l.limit_type, ref_data::FhaLimitType::HighCost);
    // SF ceiling: $1,209,750 = Cents(120_975_000)
    assert_eq!(l.limit_1_unit, Cents(120_975_000));
}

#[test]
fn test_fha_travis_county_standard() {
    let l = store().fha_loan_limits("48453", 2025).unwrap().data;
    assert_eq!(l.limit_type, ref_data::FhaLimitType::Standard);
    assert!(
        l.limit_1_unit > Cents(52_422_500),
        "Travis should be above floor"
    );
    assert!(
        l.limit_1_unit < Cents(120_975_000),
        "Travis should be below ceiling"
    );
}

#[test]
fn test_fha_versioned_wrapper_has_version_id() {
    let v = store().fha_loan_limits("48209", 2025).unwrap();
    assert!(v.version_id.as_str().contains("fha_loan_limits"));
    assert!(v.effective_date.format("%Y").to_string() == "2025");
}

#[test]
fn test_fha_year_fallback_uses_most_recent() {
    // Requesting 2030 should return 2025 data (most recent ≤ 2030)
    let l2030 = store().fha_loan_limits("48209", 2030).unwrap().data;
    let l2025 = store().fha_loan_limits("48209", 2025).unwrap().data;
    assert_eq!(l2030.limit_1_unit, l2025.limit_1_unit);
}

#[test]
fn test_fha_loan_within_limit_pass() {
    let store = store();
    let geo = store
        .geo_eligibility("48209", Some("48209010905"), 2025)
        .unwrap();
    // Kyle TX SFR: $442,935 loan ≪ $524,225 FHA floor → within limit
    assert!(geo.fha_loan_within_limit(Cents(44_293_500), 1));
}

#[test]
fn test_fha_loan_within_limit_fail() {
    let store = store();
    let geo = store
        .geo_eligibility("48209", Some("48209010905"), 2025)
        .unwrap();
    // $600,000 > $524,225 floor → exceeds limit
    assert!(!geo.fha_loan_within_limit(Cents(60_000_000), 1));
}

// ════════════════════════════════════════════════════════════════════════════
// TASK 4.6 — GSE conforming limits
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn test_gse_hays_county_standard_2025() {
    let l = store().gse_loan_limits("48209", 2025).unwrap().data;
    assert_eq!(l.fips_code, "48209");
    assert!(!l.is_high_cost);
    // 2025 standard: $806,500 = Cents(80_650_000)
    assert_eq!(l.limit_1_unit, Cents(80_650_000));
}

#[test]
fn test_gse_san_francisco_high_cost() {
    let l = store().gse_loan_limits("06075", 2025).unwrap().data;
    assert!(l.is_high_cost);
    assert_eq!(l.limit_1_unit, Cents(120_975_000));
}

#[test]
fn test_gse_high_balance_amount_true() {
    let l = store().gse_loan_limits("06075", 2025).unwrap().data;
    // $900,000: above standard $806,500 AND ≤ SF county limit
    assert!(l.is_high_balance_amount(Cents(90_000_000), 2025));
}

#[test]
fn test_gse_high_balance_amount_false_standard_county() {
    let l = store().gse_loan_limits("48209", 2025).unwrap().data;
    // Hays is not high-cost — no high-balance possible
    assert!(!l.is_high_balance_amount(Cents(82_000_000), 2025));
}

#[test]
fn test_gse_high_balance_amount_false_above_county_limit() {
    let l = store().gse_loan_limits("06075", 2025).unwrap().data;
    // $1,300,000 > county limit → not high-balance (would be jumbo)
    assert!(!l.is_high_balance_amount(Cents(130_000_000), 2025));
}

#[test]
fn test_gse_loan_status_conforming_pass() {
    let store = store();
    let geo = store.geo_eligibility("48209", None, 2025).unwrap();
    let (eligible, is_hb) = geo.gse_loan_status(Cents(44_293_500), 1);
    assert!(eligible);
    assert!(!is_hb); // Hays is not high-cost
}

#[test]
fn test_gse_loan_status_high_balance() {
    let store = store();
    let geo = store.geo_eligibility("06075", None, 2025).unwrap();
    // $900k in SF: eligible + high-balance
    let (eligible, is_hb) = geo.gse_loan_status(Cents(90_000_000), 1);
    assert!(eligible);
    assert!(is_hb);
}

#[test]
fn test_gse_loan_status_jumbo_ineligible() {
    let store = store();
    let geo = store.geo_eligibility("06075", None, 2025).unwrap();
    // $1.4M > SF county limit → not conforming-eligible (jumbo)
    let (eligible, _) = geo.gse_loan_status(Cents(140_000_000), 1);
    assert!(!eligible);
}

#[test]
fn test_gse_four_unit_limits() {
    let l = store().gse_loan_limits("48209", 2025).unwrap().data;
    assert_eq!(l.limit_for(1), Cents(80_650_000));
    assert_eq!(l.limit_for(4), Cents(155_125_000));
}

// ════════════════════════════════════════════════════════════════════════════
// TASK 4.21 — Program eligibility rules
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn test_program_rules_fha_credit_minimum() {
    let rules = store().program_rules(ProgramCode::Fha).unwrap();
    assert_eq!(rules.min_credit_score, 580);
    assert_eq!(rules.min_credit_score_alt, Some(500));
}

#[test]
fn test_program_rules_fha_alt_credit_at_10pct_down() {
    let rules = store().program_rules(ProgramCode::Fha).unwrap();
    // Standard (< 10% down): 580 minimum
    let min_3pt5 = rules.min_credit_score_for_down_payment(350); // 3.5%
    assert_eq!(min_3pt5, 580);
    // Alt tier (≥ 10% down): 500 minimum
    let min_10 = rules.min_credit_score_for_down_payment(1000); // 10%
    assert_eq!(min_10, 500);
}

#[test]
fn test_program_rules_fha_max_ltv() {
    let rules = store().program_rules(ProgramCode::Fha).unwrap();
    let score_720 = CreditScore::new(720).unwrap();
    let score_540 = CreditScore::new(540).unwrap();
    // Standard credit → 96.5%
    assert_eq!(rules.max_ltv_for(score_720, false), LtvBasisPoints(9650));
    // Low credit (500-579) → 90%
    assert_eq!(rules.max_ltv_for(score_540, false), LtvBasisPoints(9000));
}

#[test]
fn test_program_rules_fha_dti_limit() {
    let rules = store().program_rules(ProgramCode::Fha).unwrap();
    assert_eq!(rules.front_end_dti_limit(), DtiBasisPoints::new(3100));
}

#[test]
fn test_program_rules_fha_dti_eligible_pass() {
    let rules = store().program_rules(ProgramCode::Fha).unwrap();
    assert!(rules.dti_eligible(DtiBasisPoints::new(2800)));
    assert!(rules.dti_eligible(DtiBasisPoints::new(3100)));
}

#[test]
fn test_program_rules_fha_dti_eligible_fail() {
    let rules = store().program_rules(ProgramCode::Fha).unwrap();
    assert!(!rules.dti_eligible(DtiBasisPoints::new(3101)));
    assert!(!rules.dti_eligible(DtiBasisPoints::new(5000)));
}

#[test]
fn test_program_rules_va_no_dti_limit() {
    let rules = store().program_rules(ProgramCode::Va).unwrap();
    // VA uses residual income — DTI always passes (9999 = 99.99%)
    assert!(rules.dti_eligible(DtiBasisPoints::new(6000)));
    assert!(rules.dti_eligible(DtiBasisPoints::new(8000)));
}

#[test]
fn test_program_rules_va_no_down_payment_required() {
    let rules = store().program_rules(ProgramCode::Va).unwrap();
    let score = CreditScore::new(660).unwrap();
    assert_eq!(rules.max_ltv_for(score, false), LtvBasisPoints(10000));
}

#[test]
fn test_program_rules_usda_strict_credit_minimum() {
    let rules = store().program_rules(ProgramCode::Usda).unwrap();
    assert_eq!(rules.min_credit_score, 640);
    assert!(rules.min_credit_score_alt.is_none());
}

#[test]
fn test_program_rules_usda_dti_tighter_than_fha() {
    let fha_rules = store().program_rules(ProgramCode::Fha).unwrap();
    let usda_rules = store().program_rules(ProgramCode::Usda).unwrap();
    assert!(usda_rules.front_end_dti_max_bps < fha_rules.front_end_dti_max_bps);
}

#[test]
fn test_program_rules_conventional_standard_max_ltv() {
    let rules = store().program_rules(ProgramCode::Conventional).unwrap();
    let score = CreditScore::new(720).unwrap();
    // Standard conventional: 95% max LTV
    assert_eq!(rules.max_ltv_for(score, false), LtvBasisPoints(9500));
    // High-balance: 90% max LTV
    assert_eq!(rules.max_ltv_for(score, true), LtvBasisPoints(9000));
}

#[test]
fn test_program_rules_home_ready_requires_ami_check() {
    let rules = store().program_rules(ProgramCode::HomeReady).unwrap();
    assert!(rules.requires_ami_income_check);
    assert!(!rules.requires_first_time_buyer);
}

#[test]
fn test_program_rules_home_one_requires_ftib() {
    let rules = store().program_rules(ProgramCode::HomeOne).unwrap();
    assert!(rules.requires_first_time_buyer);
    assert!(!rules.requires_ami_income_check);
}

#[test]
fn test_program_rules_usda_requires_both_eligibility_checks() {
    let rules = store().program_rules(ProgramCode::Usda).unwrap();
    assert!(rules.requires_usda_eligibility);
    assert!(rules.requires_primary_residence);
}

#[test]
fn test_program_rules_unknown_program_returns_not_found() {
    // Jumbo is in ProgramCode but not in our rules file
    let err = store().program_rules(ProgramCode::Jumbo).unwrap_err();
    assert!(matches!(err, ref_data::RefDataError::NotFound { .. }));
}

#[test]
fn test_program_rules_credit_score_eligible_fha_720() {
    let rules = store().program_rules(ProgramCode::Fha).unwrap();
    assert!(rules.credit_score_eligible(CreditScore::new(720).unwrap(), 350));
}

#[test]
fn test_program_rules_credit_score_ineligible_below_500() {
    let rules = store().program_rules(ProgramCode::Fha).unwrap();
    // 499 is below FHA alt minimum of 500
    assert!(!rules.credit_score_eligible(CreditScore::new(499).unwrap(), 1000));
}

// ════════════════════════════════════════════════════════════════════════════
// TASK 4.22 — State HOI estimation rates
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn test_hoi_rate_texas_2025() {
    let rate = store().state_hoi_rate("TX", 2025).unwrap();
    assert_eq!(rate.state_abbr, "TX");
    assert_eq!(rate.annual_rate_bps, 56); // 0.56% per year
}

#[test]
fn test_hoi_rate_case_insensitive() {
    let rate_upper = store().state_hoi_rate("TX", 2025).unwrap();
    let rate_lower = store().state_hoi_rate("tx", 2025).unwrap();
    assert_eq!(rate_upper.annual_rate_bps, rate_lower.annual_rate_bps);
}

#[test]
fn test_hoi_monthly_estimate_kyle_tx() {
    let rate = store().state_hoi_rate("TX", 2025).unwrap();
    // $459,000 × 0.0056 / 12 = $213.75 → ceiling = $214 = Cents(21_400)
    let estimate = rate.monthly_estimate(Cents(45_900_000)).unwrap();
    // 459_000_00 × 56 / 10_000 = 2570_400 cents/year → ceil / 12 = 214_200 cents/mo
    assert!(estimate.0 > 0, "estimate should be positive");
    // Approximately $214/month — verify reasonable range
    assert!(
        estimate.0 >= 20_000 && estimate.0 <= 25_000,
        "TX HOI estimate for $459k should be $200-$250/mo, got Cents({})",
        estimate.0
    );
}

#[test]
fn test_hoi_monthly_estimate_zero_value_returns_none() {
    let rate = StateHoiRate {
        state_abbr: "TX".into(),
        annual_rate_bps: 56,
        effective_year: 2025,
    };
    assert_eq!(rate.monthly_estimate(Cents(0)), None);
}

#[test]
fn test_hoi_rate_florida_higher_than_texas() {
    let tx = store().state_hoi_rate("TX", 2025).unwrap();
    let fl = store().state_hoi_rate("FL", 2025).unwrap();
    assert!(
        fl.annual_rate_bps > tx.annual_rate_bps,
        "FL should have higher HOI than TX"
    );
}

#[test]
fn test_hoi_national_fallback_defined() {
    assert_eq!(NATIONAL_FALLBACK_RATE_BPS, 85);
}

#[test]
fn test_hoi_rate_unknown_state_returns_not_found() {
    let err = store().state_hoi_rate("ZZ", 2025).unwrap_err();
    assert!(matches!(err, ref_data::RefDataError::NotFound { .. }));
}

#[test]
fn test_hoi_all_50_states_plus_dc_covered() {
    let states = [
        "TX", "CA", "FL", "NY", "IL", "WA", "CO", "GA", "NC", "AZ", "NV", "OR", "VA", "MA", "OH",
        "MI", "TN", "MN", "SC", "OK", "KS", "NE", "IA", "MO", "AL", "MS", "LA", "AR", "KY", "WV",
        "IN", "WI", "MD", "NJ", "PA", "CT", "RI", "NH", "VT", "ME", "MT", "ID", "WY", "ND", "SD",
        "NM", "UT", "AK", "HI", "DC",
    ];
    for s in states {
        store()
            .state_hoi_rate(s, 2025)
            .unwrap_or_else(|_| panic!("Missing HOI rate for state {s}"));
    }
}

// ════════════════════════════════════════════════════════════════════════════
// Unified geo_eligibility integration
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn test_geo_eligibility_kyle_tx_sfr() {
    let geo = store()
        .geo_eligibility("48209", Some("48209010905"), 2025)
        .unwrap();
    assert_eq!(geo.fips_code, "48209");
    assert_eq!(geo.fha_limit_1_unit, Cents(52_422_500));
    assert_eq!(geo.gse_limit_1_unit, Cents(80_650_000));
    assert!(!geo.gse_is_high_cost);
    assert!(geo.usda_sfh_eligible);
    assert!(geo.ami_80pct.is_some());
}

#[test]
fn test_geo_eligibility_austin_tx_quadruplex() {
    let geo = store()
        .geo_eligibility("48453", Some("48453001801"), 2025)
        .unwrap();
    assert!(!geo.usda_sfh_eligible, "Austin should be USDA-ineligible");
}

#[test]
fn test_geo_eligibility_no_tract_conservative_usda() {
    // Without tract_geoid, USDA is conservatively false
    let geo = store().geo_eligibility("48209", None, 2025).unwrap();
    assert!(!geo.usda_sfh_eligible);
}

#[test]
fn test_geo_eligibility_usda_income_check_hays_3person() {
    let geo = store()
        .geo_eligibility("48209", Some("48209010905"), 2025)
        .unwrap();
    // 3-person household: usda_income_limits[2] ($88,550 = Cents(8_855_000))
    let limit_3person = geo.usda_income_limits[2];
    assert_eq!(limit_3person, Cents(8_855_000));

    // $85,000/yr household income → eligible
    assert!(geo.usda_income_eligible(Cents(8_500_000), 3));
    // $95,000/yr household income → ineligible
    assert!(!geo.usda_income_eligible(Cents(9_500_000), 3));
}
