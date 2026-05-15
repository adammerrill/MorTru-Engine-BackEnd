//! Tasks 4.7, 4.8, 4.9, 4.10 — USDA rural eligibility, MFH by tract,
//! income limits, and AMI tract data.
//!
//! # Fixture reference values
//!
//! All tests use the JSON seed data in `crates/ref_data/data/`.
//!
//! ## Census tracts seeded
//! | GEOID         | County       | SFH eligible | Notes                    |
//! |---------------|--------------|:------------:|--------------------------|
//! | 48209010905   | Hays TX      | ✓            | Kyle TX primary fixture  |
//! | 48209010906   | Hays TX      | ✓            | Kyle TX condo area       |
//! | 48209950100   | Hays TX      | ✓            | Wimberley TX (100% rural)|
//! | 48453001801   | Travis TX    | ✗            | Austin TX urban          |
//!
//! ## USDA SFGH income limits (2025, Hays County)
//! 1–4 person: $88,550 = Cents(8_855_000)
//! 5–8 person: $101,982.50 = Cents(10_198_250)
//!
//! ## AMI (2025, Kyle TX primary tract 48209010905)
//! 100%: $95,833 = Cents(9_583_300)
//!  80%: $76,667 = Cents(7_666_700) ← HomeReady/HP income gate
//! 115%: $110,208 = Cents(11_020_800)

use chrono::NaiveDate;
use ref_data::{JsonFileStore, RefDataStore};
use types::Cents;

fn store() -> JsonFileStore {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    JsonFileStore::new(manifest.join("data"))
}

// ════════════════════════════════════════════════════════════════════════════
// TASK 4.7 — USDA Rural Eligibility
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn test_usda_rural_kyle_primary_tract_sfh_eligible() {
    let r = store().usda_rural_eligibility("48209010905").unwrap();
    assert!(r.is_some(), "Kyle TX primary tract must be in the store");
    assert!(r.unwrap().is_sfh_eligible);
}

#[test]
fn test_usda_rural_kyle_condo_tract_sfh_eligible() {
    let r = store()
        .usda_rural_eligibility("48209010906")
        .unwrap()
        .unwrap();
    assert!(r.is_sfh_eligible);
}

#[test]
fn test_usda_rural_wimberley_tract_sfh_eligible() {
    let r = store()
        .usda_rural_eligibility("48209950100")
        .unwrap()
        .unwrap();
    assert!(r.is_sfh_eligible);
}

#[test]
fn test_usda_rural_austin_urban_sfh_ineligible() {
    let r = store()
        .usda_rural_eligibility("48453001801")
        .unwrap()
        .unwrap();
    assert!(
        !r.is_sfh_eligible,
        "Austin urban tract must be SFH ineligible"
    );
}

#[test]
fn test_usda_rural_kyle_primary_mfh_eligible() {
    let r = store()
        .usda_rural_eligibility("48209010905")
        .unwrap()
        .unwrap();
    assert!(r.is_mfh_eligible);
}

#[test]
fn test_usda_rural_austin_urban_mfh_ineligible() {
    let r = store()
        .usda_rural_eligibility("48453001801")
        .unwrap()
        .unwrap();
    assert!(
        !r.is_mfh_eligible,
        "Austin urban tract must be MFH ineligible"
    );
}

#[test]
fn test_usda_rural_unknown_tract_returns_none() {
    let r = store().usda_rural_eligibility("99999999999").unwrap();
    assert!(
        r.is_none(),
        "unknown tract should return None, not an error"
    );
}

#[test]
fn test_usda_rural_wimberley_pct_eligible_one_hundred() {
    let r = store()
        .usda_rural_eligibility("48209950100")
        .unwrap()
        .unwrap();
    assert_eq!(r.pct_eligible, Some(100.0));
}

#[test]
fn test_usda_rural_kyle_primary_pct_eligible() {
    let r = store()
        .usda_rural_eligibility("48209010905")
        .unwrap()
        .unwrap();
    let pct = r.pct_eligible.expect("pct_eligible must be populated");
    assert!(
        pct > 0.0 && pct <= 100.0,
        "pct_eligible must be in (0, 100]"
    );
}

#[test]
fn test_usda_rural_austin_pct_eligible_zero() {
    let r = store()
        .usda_rural_eligibility("48453001801")
        .unwrap()
        .unwrap();
    assert_eq!(r.pct_eligible, Some(0.0));
}

#[test]
fn test_usda_rural_fips_code_populated() {
    let r = store()
        .usda_rural_eligibility("48209010905")
        .unwrap()
        .unwrap();
    assert_eq!(r.fips_code, "48209");
    assert_eq!(r.state_abbr, "TX");
}

#[test]
fn test_usda_rural_source_version_populated() {
    let r = store()
        .usda_rural_eligibility("48209010905")
        .unwrap()
        .unwrap();
    assert!(!r.source_version.is_empty(), "source_version must be set");
    // Source dataset is 2018 Census-based
    assert!(
        r.source_version.starts_with("2018"),
        "source is 2018 Census data"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// TASK 4.8 — USDA Multi-Family Housing by Tract
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn test_mfh_kyle_primary_tract_found() {
    let r = store().usda_mfh_by_tract("48209010905").unwrap();
    assert!(r.is_some(), "Kyle TX primary tract must be in MFH data");
}

#[test]
fn test_mfh_kyle_primary_has_family_projects() {
    let r = store().usda_mfh_by_tract("48209010905").unwrap().unwrap();
    assert_eq!(r.fa_projects, 1, "one family housing project seeded");
    assert_eq!(r.fa_units, 24, "24 family units seeded");
}

#[test]
fn test_mfh_kyle_primary_total_projects_and_units() {
    let r = store().usda_mfh_by_tract("48209010905").unwrap().unwrap();
    assert_eq!(r.total_projects, 1);
    assert_eq!(r.total_units, 24);
}

#[test]
fn test_mfh_kyle_primary_has_usda_projects() {
    let r = store().usda_mfh_by_tract("48209010905").unwrap().unwrap();
    assert!(r.has_usda_projects());
}

#[test]
fn test_mfh_kyle_primary_has_family_housing() {
    let r = store().usda_mfh_by_tract("48209010905").unwrap().unwrap();
    assert!(r.has_family_housing());
}

#[test]
fn test_mfh_kyle_primary_no_elderly_housing() {
    let r = store().usda_mfh_by_tract("48209010905").unwrap().unwrap();
    assert!(
        !r.has_elderly_housing(),
        "no elderly (EL) projects in fixture"
    );
    assert_eq!(r.el_projects, 0);
    assert_eq!(r.el_units, 0);
}

#[test]
fn test_mfh_unknown_tract_returns_none() {
    let r = store().usda_mfh_by_tract("99999999999").unwrap();
    assert!(r.is_none());
}

#[test]
fn test_mfh_austin_urban_tract_not_in_mfh_data() {
    // Austin urban tract exists in rural eligibility (ineligible) but
    // has no MFH projects — not present in the MFH dataset.
    let r = store().usda_mfh_by_tract("48453001801").unwrap();
    assert!(r.is_none());
}

#[test]
fn test_mfh_fips_code_populated() {
    let r = store().usda_mfh_by_tract("48209010905").unwrap().unwrap();
    assert_eq!(r.fips_code, "48209");
    assert_eq!(r.state_fips, "48");
    assert_eq!(r.county_fips, "209");
}

#[test]
fn test_mfh_non_family_project_types_zero() {
    let r = store().usda_mfh_by_tract("48209010905").unwrap().unwrap();
    // Only family (FA) housing exists in this fixture
    assert_eq!(r.cg_projects, 0); // congregate
    assert_eq!(r.gh_projects, 0); // group home
    assert_eq!(r.mx_projects, 0); // mixed-use
}

// ════════════════════════════════════════════════════════════════════════════
// TASK 4.9 — USDA Income Limits
// ════════════════════════════════════════════════════════════════════════════

fn oct_2025() -> NaiveDate {
    NaiveDate::from_ymd_opt(2025, 10, 1).unwrap()
}

#[test]
fn test_usda_income_hays_county_found() {
    let v = store().usda_income_limits("48209", oct_2025()).unwrap();
    assert_eq!(v.data.fips_code, "48209");
    assert_eq!(v.data.county_name, "Hays");
    assert_eq!(v.data.state_abbr, "TX");
}

#[test]
fn test_usda_income_hays_1person_limit() {
    let l = store()
        .usda_income_limits("48209", oct_2025())
        .unwrap()
        .data;
    // 2025 Hays County SFGH 1-person: $88,550
    assert_eq!(l.limit_size_1, Cents(8_855_000));
}

#[test]
fn test_usda_income_hays_3person_same_as_1person() {
    let l = store()
        .usda_income_limits("48209", oct_2025())
        .unwrap()
        .data;
    // Sizes 1–4 share the same limit under USDA SFGH
    assert_eq!(l.limit_size_3, Cents(8_855_000));
}

#[test]
fn test_usda_income_hays_4person_limit() {
    let l = store()
        .usda_income_limits("48209", oct_2025())
        .unwrap()
        .data;
    assert_eq!(l.limit_size_4, Cents(8_855_000));
}

#[test]
fn test_usda_income_hays_5person_higher_than_4() {
    let l = store()
        .usda_income_limits("48209", oct_2025())
        .unwrap()
        .data;
    // 5-person limit jumps at the 115% AMI threshold
    assert!(
        l.limit_size_5 > l.limit_size_4,
        "5-person limit must exceed 4-person limit"
    );
    assert_eq!(l.limit_size_5, Cents(10_198_250));
}

#[test]
fn test_usda_income_hays_8person_limit() {
    let l = store()
        .usda_income_limits("48209", oct_2025())
        .unwrap()
        .data;
    assert_eq!(l.limit_size_8, Cents(10_198_250));
}

#[test]
fn test_usda_income_limit_for_size_method_sizes_1_through_8() {
    let l = store()
        .usda_income_limits("48209", oct_2025())
        .unwrap()
        .data;
    for size in 1..=8_u8 {
        let r = l.limit_for_size(size);
        assert!(r.is_ok(), "limit_for_size({size}) must succeed");
        assert!(r.unwrap().0 > 0, "limit_for_size({size}) must be positive");
    }
}

#[test]
fn test_usda_income_limit_for_size_0_returns_error() {
    let l = store()
        .usda_income_limits("48209", oct_2025())
        .unwrap()
        .data;
    assert!(l.limit_for_size(0).is_err());
}

#[test]
fn test_usda_income_limit_for_size_9_returns_error() {
    let l = store()
        .usda_income_limits("48209", oct_2025())
        .unwrap()
        .data;
    assert!(l.limit_for_size(9).is_err());
}

#[test]
fn test_usda_income_eligible_below_3person_limit() {
    let geo = store()
        .geo_eligibility("48209", Some("48209010905"), 2025)
        .unwrap();
    // $85,000/yr household, 3-person → below $88,550 limit → eligible
    assert!(geo.usda_income_eligible(Cents(8_500_000), 3));
}

#[test]
fn test_usda_income_eligible_at_limit_boundary() {
    let geo = store()
        .geo_eligibility("48209", Some("48209010905"), 2025)
        .unwrap();
    // Exactly at limit → eligible (≤, not <)
    assert!(geo.usda_income_eligible(Cents(8_855_000), 3));
}

#[test]
fn test_usda_income_ineligible_above_3person_limit() {
    let geo = store()
        .geo_eligibility("48209", Some("48209010905"), 2025)
        .unwrap();
    // $92,000/yr, 3-person → above $88,550 limit → ineligible
    assert!(!geo.usda_income_eligible(Cents(9_200_000), 3));
}

#[test]
fn test_usda_income_ineligible_size_0_invalid() {
    let geo = store()
        .geo_eligibility("48209", Some("48209010905"), 2025)
        .unwrap();
    // size 0 is invalid — conservative false
    assert!(!geo.usda_income_eligible(Cents(5_000_000), 0));
}

#[test]
fn test_usda_income_travis_county_higher_than_hays() {
    let hays = store()
        .usda_income_limits("48209", oct_2025())
        .unwrap()
        .data;
    let travis = store()
        .usda_income_limits("48453", oct_2025())
        .unwrap()
        .data;
    // Travis (Austin MSA) has higher area incomes → higher limits
    assert!(
        travis.limit_size_1 > hays.limit_size_1,
        "Travis limit ${} should exceed Hays limit ${}",
        travis.limit_size_1.0 / 100,
        hays.limit_size_1.0 / 100,
    );
}

#[test]
fn test_usda_income_program_is_sfgh() {
    let l = store()
        .usda_income_limits("48209", oct_2025())
        .unwrap()
        .data;
    assert_eq!(l.program, "SFGH");
}

#[test]
fn test_usda_income_unknown_fips_returns_not_found() {
    let err = store().usda_income_limits("99999", oct_2025()).unwrap_err();
    assert!(matches!(err, ref_data::RefDataError::NotFound { .. }));
}

#[test]
fn test_usda_income_version_metadata_populated() {
    let v = store().usda_income_limits("48209", oct_2025()).unwrap();
    assert!(v.version_id.as_str().contains("usda_income_limits"));
}

// ════════════════════════════════════════════════════════════════════════════
// TASK 4.10 — AMI Tract Data
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn test_ami_kyle_primary_tract_found() {
    let r = store().ami_tract_data("48209010905", 2025).unwrap();
    assert!(r.is_some(), "Kyle TX primary tract must be in AMI data");
}

#[test]
fn test_ami_kyle_primary_100pct() {
    let d = store()
        .ami_tract_data("48209010905", 2025)
        .unwrap()
        .unwrap()
        .data;
    // $95,833/yr = Cents(9_583_300)
    assert_eq!(d.ami_100pct, Some(Cents(9_583_300)));
}

#[test]
fn test_ami_kyle_primary_80pct() {
    let d = store()
        .ami_tract_data("48209010905", 2025)
        .unwrap()
        .unwrap()
        .data;
    // HomeReady/HP gate: $76,667/yr = Cents(7_666_700)
    assert_eq!(d.ami_80pct, Some(Cents(7_666_700)));
}

#[test]
fn test_ami_kyle_primary_115pct() {
    let d = store()
        .ami_tract_data("48209010905", 2025)
        .unwrap()
        .unwrap()
        .data;
    assert_eq!(d.ami_115pct, Some(Cents(11_020_800)));
}

#[test]
fn test_ami_kyle_primary_not_low_income_tract() {
    let d = store()
        .ami_tract_data("48209010905", 2025)
        .unwrap()
        .unwrap()
        .data;
    assert!(!d.is_low_income_tract);
    assert!(!d.hp_income_limit_waived);
}

#[test]
fn test_ami_all_thresholds_ascending() {
    let d = store()
        .ami_tract_data("48209010905", 2025)
        .unwrap()
        .unwrap()
        .data;
    let pct50 = d.ami_50pct.unwrap();
    let pct80 = d.ami_80pct.unwrap();
    let pct100 = d.ami_100pct.unwrap();
    let pct115 = d.ami_115pct.unwrap();
    assert!(pct50 < pct80, "50% < 80%");
    assert!(pct80 < pct100, "80% < 100%");
    assert!(pct100 < pct115, "100% < 115%");
}

#[test]
fn test_ami_travis_county_higher_than_hays() {
    let hays = store()
        .ami_tract_data("48209010905", 2025)
        .unwrap()
        .unwrap()
        .data;
    let travis = store()
        .ami_tract_data("48453001801", 2025)
        .unwrap()
        .unwrap()
        .data;
    assert!(
        travis.ami_100pct.unwrap() > hays.ami_100pct.unwrap(),
        "Travis (Austin) AMI should exceed Hays County AMI"
    );
}

#[test]
fn test_ami_unknown_tract_returns_none() {
    let r = store().ami_tract_data("99999999999", 2025).unwrap();
    assert!(r.is_none());
}

#[test]
fn test_ami_year_fallback_2030_returns_2025_data() {
    let d2025 = store()
        .ami_tract_data("48209010905", 2025)
        .unwrap()
        .unwrap()
        .data;
    let d2030 = store()
        .ami_tract_data("48209010905", 2030)
        .unwrap()
        .unwrap()
        .data;
    assert_eq!(d2025.ami_100pct, d2030.ami_100pct);
}

#[test]
fn test_ami_version_wrapper_populated() {
    let v = store()
        .ami_tract_data("48209010905", 2025)
        .unwrap()
        .unwrap();
    assert!(v.version_id.as_str().contains("ami_tract_data"));
}

#[test]
fn test_ami_fips_and_state_populated() {
    let d = store()
        .ami_tract_data("48209010905", 2025)
        .unwrap()
        .unwrap()
        .data;
    assert_eq!(d.fips_code, "48209");
    assert_eq!(d.state_abbr, "TX");
    assert_eq!(d.county_name, "Hays");
}

// ── HomeReady / HomePossible income eligibility ───────────────────────────────

#[test]
fn test_hp_income_eligible_below_80pct_ami() {
    let geo = store()
        .geo_eligibility("48209", Some("48209010905"), 2025)
        .unwrap();
    // $70,000/yr < $76,667 (80% AMI) → HomeReady eligible
    assert!(geo.hp_income_eligible(Cents(7_000_000)));
}

#[test]
fn test_hp_income_eligible_at_80pct_ami_boundary() {
    let geo = store()
        .geo_eligibility("48209", Some("48209010905"), 2025)
        .unwrap();
    // Exactly at 80% AMI → eligible (≤ limit)
    assert!(geo.hp_income_eligible(Cents(7_666_700)));
}

#[test]
fn test_hp_income_ineligible_above_80pct_ami() {
    let geo = store()
        .geo_eligibility("48209", Some("48209010905"), 2025)
        .unwrap();
    // $85,000/yr > $76,667 (80% AMI) → not eligible
    assert!(!geo.hp_income_eligible(Cents(8_500_000)));
}

#[test]
fn test_hp_income_ineligible_no_tract_no_ami_data() {
    // Without a tract, geo_eligibility has no AMI data → conservative false
    let geo = store().geo_eligibility("48209", None, 2025).unwrap();
    assert!(!geo.hp_income_eligible(Cents(7_000_000)));
}

#[test]
fn test_geo_eligibility_ami_80pct_populated_when_tract_known() {
    let geo = store()
        .geo_eligibility("48209", Some("48209010905"), 2025)
        .unwrap();
    assert_eq!(geo.ami_80pct, Some(Cents(7_666_700)));
}

#[test]
fn test_geo_eligibility_ami_absent_when_no_tract() {
    let geo = store().geo_eligibility("48209", None, 2025).unwrap();
    assert!(geo.ami_80pct.is_none());
    assert!(geo.ami_100pct.is_none());
}
