//! Task 4.1 — ref_data scaffold gate tests.

use chrono::NaiveDate;
use ref_data::{
    DataVersionManifest, FhaLimitType, FhaLoanLimits, GeoEligibility, GseLoanLimits, RefDataError,
    UsdaIncomeLimit, UsdaMfhByTract, UsdaruralEligibility, VersionId, Versioned,
};
use types::Cents;

// ── VersionId ─────────────────────────────────────────────────────────────────

#[test]
fn test_version_id_format() {
    let date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let v = VersionId::new("fha_loan_limits", date);
    assert_eq!(v.as_str(), "fha_loan_limits:2025-01-01");
}

#[test]
fn test_version_id_display() {
    let date = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
    let v = VersionId::new("usda_income_limits", date);
    assert!(v.to_string().contains("usda_income_limits"));
}

#[test]
fn test_version_id_equality() {
    let date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let a = VersionId::new("gse_loan_limits", date);
    let b = VersionId::new("gse_loan_limits", date);
    assert_eq!(a, b);
}

#[test]
fn test_versioned_wraps_data() {
    let date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let v = Versioned::new("fha_loan_limits", date, 42u32);
    assert_eq!(v.data, 42u32);
    assert_eq!(v.effective_date, date);
    assert!(v.version_id.as_str().starts_with("fha_loan_limits"));
}

// ── DataVersionManifest ───────────────────────────────────────────────────────

#[test]
fn test_manifest_new_has_timestamp() {
    let m = DataVersionManifest::new();
    assert!(m.created_at.is_some());
}

#[test]
fn test_manifest_incomplete_without_fha() {
    let m = DataVersionManifest::new();
    assert!(!m.is_complete_for_program(false, false));
}

#[test]
fn test_manifest_complete_base_programs() {
    let d = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let mut m = DataVersionManifest::new();
    m.fha_loan_limits = Some(VersionId::new("fha_loan_limits", d));
    m.gse_loan_limits = Some(VersionId::new("gse_loan_limits", d));
    m.ami_tract_data = Some(VersionId::new("ami_tract_data", d));
    m.fha_mip_rates = Some(VersionId::new("fha_mip_rates", d));
    m.mi_coverage_reqs = Some(VersionId::new("mi_coverage_reqs", d));
    assert!(m.is_complete_for_program(false, false));
}

#[test]
fn test_manifest_usda_requires_eligibility_and_income() {
    let d = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let mut m = DataVersionManifest::new();
    m.fha_loan_limits = Some(VersionId::new("fha_loan_limits", d));
    m.gse_loan_limits = Some(VersionId::new("gse_loan_limits", d));
    m.ami_tract_data = Some(VersionId::new("ami_tract_data", d));
    m.fha_mip_rates = Some(VersionId::new("fha_mip_rates", d));
    m.mi_coverage_reqs = Some(VersionId::new("mi_coverage_reqs", d));
    // Missing USDA fields
    assert!(!m.is_complete_for_program(true, false));
    // Add USDA fields
    m.usda_rural_eligibility = Some(VersionId::new("usda_rural_eligibility", d));
    m.usda_income_limits = Some(VersionId::new("usda_income_limits", d));
    assert!(m.is_complete_for_program(true, false));
}

// ── FhaLoanLimits ─────────────────────────────────────────────────────────────

fn sample_fha_limits() -> FhaLoanLimits {
    FhaLoanLimits {
        fips_code: "48209".into(),
        state_abbr: "TX".into(),
        county_name: "Hays".into(),
        limit_type: FhaLimitType::Standard,
        limit_1_unit: Cents(52_422_500), // $524,225 floor
        limit_2_unit: Cents(67_100_000),
        limit_3_unit: Cents(81_100_000),
        limit_4_unit: Cents(1_007_575_00),
        effective_year: 2025,
    }
}

#[test]
fn test_fha_limits_limit_for_unit_count() {
    let l = sample_fha_limits();
    assert_eq!(l.limit_for(1), Cents(52_422_500));
    assert_eq!(l.limit_for(2), Cents(67_100_000));
    assert_eq!(l.limit_for(4), Cents(1_007_575_00));
}

#[test]
#[should_panic]
fn test_fha_limits_panics_on_invalid_unit_count() {
    let l = sample_fha_limits();
    let _ = l.limit_for(5);
}

// ── GseLoanLimits ─────────────────────────────────────────────────────────────

fn sample_gse_limits() -> GseLoanLimits {
    GseLoanLimits {
        fips_code: "48209".into(),
        state_abbr: "TX".into(),
        county_name: "Hays".into(),
        cbsa_name: Some("Austin-Round Rock-Georgetown, TX".into()),
        limit_1_unit: Cents(80_650_000), // $806,500 standard
        limit_2_unit: Cents(103_265_000),
        limit_3_unit: Cents(124_815_000),
        limit_4_unit: Cents(155_125_000),
        is_high_cost: false,
        effective_year: 2025,
    }
}

#[test]
fn test_gse_limits_standard_conforming() {
    let l = sample_gse_limits();
    // $400k well below $806,500 standard limit → not high-balance
    assert!(!l.is_high_balance_amount(Cents(40_000_000), 2025));
}

#[test]
fn test_gse_limits_limit_for_unit_count() {
    let l = sample_gse_limits();
    assert_eq!(l.limit_for(1), Cents(80_650_000));
    assert_eq!(l.limit_for(2), Cents(103_265_000));
}

// ── USDA Rural Eligibility ────────────────────────────────────────────────────

#[test]
fn test_usda_rural_eligible_tract() {
    let e = UsdaruralEligibility {
        geoid: "482090109".into(),
        fips_code: "48209".into(),
        state_abbr: "TX".into(),
        is_sfh_eligible: true,
        is_mfh_eligible: true,
        pct_eligible: Some(98.5),
        source_version: "2018-08-23".into(),
    };
    assert!(e.is_sfh_eligible);
    assert_eq!(e.pct_eligible, Some(98.5));
}

#[test]
fn test_usda_rural_ineligible_tract() {
    // Austin city center tract — falls within ineligible polygon
    let e = UsdaruralEligibility {
        geoid: "484530026".into(),
        fips_code: "48453".into(),
        state_abbr: "TX".into(),
        is_sfh_eligible: false,
        is_mfh_eligible: false,
        pct_eligible: Some(0.0),
        source_version: "2018-08-23".into(),
    };
    assert!(!e.is_sfh_eligible);
}

// ── USDA Income Limits ────────────────────────────────────────────────────────

fn sample_usda_income_limit() -> UsdaIncomeLimit {
    UsdaIncomeLimit {
        fips_code: "48209".into(),
        state_abbr: "TX".into(),
        county_name: "Hays".into(),
        msa_name: Some("Austin-Round Rock-Georgetown, TX".into()),
        program: "SFGH".into(),
        limit_size_1: Cents(8_600_000),  // $86,000
        limit_size_2: Cents(9_825_000),  // $98,250
        limit_size_3: Cents(11_050_000), // $110,500
        limit_size_4: Cents(12_275_000), // $122,750
        limit_size_5: Cents(13_250_000), // $132,500
        limit_size_6: Cents(14_225_000), // $142,250
        limit_size_7: Cents(15_200_000), // $152,000
        limit_size_8: Cents(16_175_000), // $161,750
        effective_date: NaiveDate::from_ymd_opt(2025, 10, 1).unwrap(),
    }
}

#[test]
fn test_usda_income_limit_all_8_sizes() {
    let lim = sample_usda_income_limit();
    for size in 1u8..=8 {
        assert!(lim.limit_for_size(size).is_ok(), "size {size} failed");
    }
}

#[test]
fn test_usda_income_limit_size_3_reference_value() {
    let lim = sample_usda_income_limit();
    assert_eq!(lim.limit_for_size(3).unwrap(), Cents(11_050_000));
}

#[test]
fn test_usda_income_limit_invalid_size_0() {
    let lim = sample_usda_income_limit();
    assert!(matches!(
        lim.limit_for_size(0),
        Err(RefDataError::InvalidHouseholdSize(0))
    ));
}

#[test]
fn test_usda_income_limit_invalid_size_9() {
    let lim = sample_usda_income_limit();
    assert!(matches!(
        lim.limit_for_size(9),
        Err(RefDataError::InvalidHouseholdSize(9))
    ));
}

// ── USDA MFH By Tract ─────────────────────────────────────────────────────────

#[test]
fn test_usda_mfh_hays_county_tract() {
    // From uploaded CSV: GEOID 48209010905 has 1 FA project, 0 units (under construction)
    let t = UsdaMfhByTract {
        geoid: "48209010905".into(),
        fips_code: "48209".into(),
        state_fips: "48".into(),
        county_fips: "209".into(),
        tract_number: "010905".into(),
        tract_name: Some("Census Tract 109.05".into()),
        el_projects: 0,
        el_units: 0,
        fa_projects: 1,
        fa_units: 0,
        cg_projects: 0,
        cg_units: 0,
        gh_projects: 0,
        gh_units: 0,
        mx_projects: 0,
        mx_units: 0,
        total_projects: 1,
        total_units: 0,
    };
    assert!(t.has_usda_projects());
    assert!(t.has_family_housing());
    assert!(!t.has_elderly_housing());
}

#[test]
fn test_usda_mfh_no_projects() {
    let t = UsdaMfhByTract {
        geoid: "12345678901".into(),
        fips_code: "12345".into(),
        state_fips: "12".into(),
        county_fips: "345".into(),
        tract_number: "678901".into(),
        tract_name: None,
        el_projects: 0,
        el_units: 0,
        fa_projects: 0,
        fa_units: 0,
        cg_projects: 0,
        cg_units: 0,
        gh_projects: 0,
        gh_units: 0,
        mx_projects: 0,
        mx_units: 0,
        total_projects: 0,
        total_units: 0,
    };
    assert!(!t.has_usda_projects());
}

// ── GeoEligibility ────────────────────────────────────────────────────────────

fn sample_geo_eligibility() -> GeoEligibility {
    GeoEligibility {
        fips_code: "48209".into(),
        tract_geoid: Some("482090109".into()),
        effective_year: 2025,
        fha_limit_1_unit: Cents(52_422_500),
        fha_limit_2_unit: Cents(67_100_000),
        fha_limit_3_unit: Cents(81_100_000),
        fha_limit_4_unit: Cents(1_007_575_00),
        fha_limit_type: FhaLimitType::Standard,
        gse_limit_1_unit: Cents(80_650_000),
        gse_limit_2_unit: Cents(103_265_000),
        gse_limit_3_unit: Cents(124_815_000),
        gse_limit_4_unit: Cents(155_125_000),
        gse_is_high_cost: false,
        usda_sfh_eligible: true,
        usda_mfh_eligible: true,
        usda_pct_eligible: Some(98.5),
        // Hays County TX SFGH limits for 2025 (approx)
        usda_income_limits: [
            Cents(8_600_000),
            Cents(9_825_000),
            Cents(11_050_000),
            Cents(12_275_000),
            Cents(13_250_000),
            Cents(14_225_000),
            Cents(15_200_000),
            Cents(16_175_000),
        ],
        ami_100pct: Some(Cents(11_700_000)), // $117,000 Austin metro
        ami_50pct: Some(Cents(5_850_000)),
        ami_80pct: Some(Cents(9_360_000)),
        ami_115pct: Some(Cents(13_455_000)),
        is_low_income_tract: false,
        hp_income_limit_waived: false,
    }
}

#[test]
fn test_geo_fha_loan_within_limit_pass() {
    let g = sample_geo_eligibility();
    // FHA loan: $434,443 base + $7,603 UFMIP = $442,046 adjusted
    assert!(g.fha_loan_within_limit(Cents(44_204_600), 1));
}

#[test]
fn test_geo_fha_loan_within_limit_fail() {
    let g = sample_geo_eligibility();
    // Over the $524,225 limit
    assert!(!g.fha_loan_within_limit(Cents(53_000_000), 1));
}

#[test]
fn test_geo_gse_conforming_standard() {
    let g = sample_geo_eligibility();
    // $400k on standard county → conforming, not high-balance
    let (eligible, high_bal) = g.gse_loan_status(Cents(40_000_000), 1);
    assert!(eligible);
    assert!(!high_bal);
}

#[test]
fn test_geo_gse_jumbo_ineligible() {
    let g = sample_geo_eligibility();
    // $850k > $806,500 limit in standard county → jumbo
    let (eligible, _) = g.gse_loan_status(Cents(85_000_000), 1);
    assert!(!eligible);
}

#[test]
fn test_geo_usda_income_eligible() {
    let g = sample_geo_eligibility();
    // 3-person household, $100k income, limit $110,500 → eligible
    assert!(g.usda_income_eligible(Cents(10_000_000), 3));
    // $115k income → ineligible
    assert!(!g.usda_income_eligible(Cents(11_500_000), 3));
}

#[test]
fn test_geo_usda_income_invalid_size() {
    let g = sample_geo_eligibility();
    assert!(!g.usda_income_eligible(Cents(5_000_000), 0));
    assert!(!g.usda_income_eligible(Cents(5_000_000), 9));
}

#[test]
fn test_geo_hp_income_eligible_with_data() {
    let g = sample_geo_eligibility();
    // $90k income, 80% AMI is $93,600 → eligible
    assert!(g.hp_income_eligible(Cents(9_000_000)));
    // $100k > $93,600 → not eligible
    assert!(!g.hp_income_eligible(Cents(10_000_000)));
}

#[test]
fn test_geo_hp_income_waived_in_low_income_tract() {
    let mut g = sample_geo_eligibility();
    g.hp_income_limit_waived = true;
    // Any income is eligible in a low-income census tract
    assert!(g.hp_income_eligible(Cents(99_999_999)));
}

// ── RefDataError ──────────────────────────────────────────────────────────────

#[test]
fn test_ref_data_error_not_found_display() {
    let e = RefDataError::NotFound {
        data_type: "fha_loan_limits",
        fips: "48209".into(),
        year: 2025,
    };
    assert!(e.to_string().contains("48209"));
    assert!(e.to_string().contains("2025"));
}

#[test]
fn test_ref_data_error_invalid_household_size() {
    let e = RefDataError::InvalidHouseholdSize(9);
    assert!(e.to_string().contains("9"));
}

#[test]
fn test_ref_data_error_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<RefDataError>();
}
