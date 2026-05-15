//! Tasks 4.11 (CBSA/MSA crosswalk) + 4.12 (Texas HOI by ZIP).

use ref_data::{CbsaDesignation, JsonFileStore, RefDataStore, ZipHoiRate};
use types::Cents;

fn store() -> JsonFileStore {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    JsonFileStore::new(manifest.join("data"))
}

// ════════════════════════════════════════════════════════════════════════════
// TASK 4.11 — CBSA / MSA Crosswalk
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn test_cbsa_hays_county_is_metropolitan() {
    let r = store().cbsa_for_county("48209").unwrap().unwrap();
    assert_eq!(r.fips_code, "48209");
    assert_eq!(r.designation, CbsaDesignation::Metropolitan);
    assert!(r.is_metro);
}

#[test]
fn test_cbsa_hays_county_austin_msa() {
    let r = store().cbsa_for_county("48209").unwrap().unwrap();
    assert_eq!(r.cbsa_code.as_deref(), Some("12420"));
    let name = r.cbsa_name.unwrap();
    assert!(name.contains("Austin"), "CBSA name should mention Austin");
}

#[test]
fn test_cbsa_travis_county_same_msa_as_hays() {
    let hays = store().cbsa_for_county("48209").unwrap().unwrap();
    let travis = store().cbsa_for_county("48453").unwrap().unwrap();
    assert_eq!(hays.cbsa_code, travis.cbsa_code, "both in Austin MSA");
}

#[test]
fn test_cbsa_san_francisco_metropolitan() {
    let r = store().cbsa_for_county("06075").unwrap().unwrap();
    assert!(r.is_metro);
    assert_eq!(r.cbsa_code.as_deref(), Some("41860"));
}

#[test]
fn test_cbsa_micropolitan_designation() {
    // Llano County TX (48299) → Marble Falls, TX micro
    let r = store().cbsa_for_county("48299").unwrap().unwrap();
    assert_eq!(r.designation, CbsaDesignation::Micropolitan);
    assert!(!r.is_metro);
    assert!(r.is_micro());
}

#[test]
fn test_cbsa_rural_county_no_cbsa_code() {
    // Crockett County TX (48107) → rural
    let r = store().cbsa_for_county("48107").unwrap().unwrap();
    assert_eq!(r.designation, CbsaDesignation::Rural);
    assert!(r.cbsa_code.is_none());
    assert!(r.cbsa_name.is_none());
    assert!(r.is_rural());
}

#[test]
fn test_cbsa_unknown_fips_returns_none() {
    let r = store().cbsa_for_county("99999").unwrap();
    assert!(r.is_none());
}

#[test]
fn test_cbsa_designation_predicates() {
    assert!(CbsaDesignation::Metropolitan.is_metro());
    assert!(!CbsaDesignation::Micropolitan.is_metro());
    assert!(!CbsaDesignation::Rural.is_metro());

    assert!(CbsaDesignation::Metropolitan.is_urban());
    assert!(CbsaDesignation::Micropolitan.is_urban());
    assert!(!CbsaDesignation::Rural.is_urban());
}

#[test]
fn test_cbsa_all_seeded_counties_found() {
    let counties = ["48209", "48453", "06075", "06037", "12086", "36061"];
    for fips in counties {
        assert!(
            store().cbsa_for_county(fips).unwrap().is_some(),
            "county {fips} must be in CBSA data"
        );
    }
}

// ════════════════════════════════════════════════════════════════════════════
// TASK 4.12 — Texas HOI Premiums by ZIP
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn test_zip_hoi_kyle_tx_found() {
    let r = store().zip_hoi_rate("78640", 2025).unwrap();
    assert!(r.is_some(), "Kyle TX ZIP 78640 must be in ZIP HOI data");
}

#[test]
fn test_zip_hoi_kyle_tx_rate() {
    let r = store().zip_hoi_rate("78640", 2025).unwrap().unwrap();
    assert_eq!(r.zip5, "78640");
    assert_eq!(r.state_abbr, "TX");
    // Kyle TX: 62 bps (0.62% per year)
    assert_eq!(r.annual_rate_bps, 62);
}

#[test]
fn test_zip_hoi_kyle_tx_higher_than_state_average() {
    let zip_rate = store().zip_hoi_rate("78640", 2025).unwrap().unwrap();
    let state_rate = store().state_hoi_rate("TX", 2025).unwrap();
    // Kyle 62 bps > TX state avg 56 bps (higher risk area within TX)
    assert!(
        zip_rate.annual_rate_bps > state_rate.annual_rate_bps,
        "Kyle ZIP rate {} should exceed TX state avg {}",
        zip_rate.annual_rate_bps,
        state_rate.annual_rate_bps
    );
}

#[test]
fn test_zip_hoi_monthly_estimate_kyle_tx() {
    let r = store().zip_hoi_rate("78640", 2025).unwrap().unwrap();
    // $459,000 × 0.0062 / 12 = $237/mo (ceiling)
    let monthly = r.monthly_estimate(Cents(45_900_000)).unwrap();
    assert!(monthly.0 > 0, "monthly estimate must be positive");
    // 459_000_00 × 62 / 10_000 = 2_845_800 cents/yr → ceil/12 = 237_150 cents/mo
    assert!(
        monthly.0 >= 23_000 && monthly.0 <= 26_000,
        "Kyle TX HOI $459k should be $230-$260/mo, got Cents({})",
        monthly.0
    );
}

#[test]
fn test_zip_hoi_austin_downtown() {
    let r = store().zip_hoi_rate("78701", 2025).unwrap().unwrap();
    assert_eq!(r.state_abbr, "TX");
    assert_eq!(r.annual_rate_bps, 48);
}

#[test]
fn test_zip_hoi_unknown_zip_returns_none() {
    // Engine should fall through to state_hoi_rate for unknown ZIPs
    let r = store().zip_hoi_rate("00000", 2025).unwrap();
    assert!(r.is_none());
}

#[test]
fn test_zip_hoi_median_premium_populated() {
    let r = store().zip_hoi_rate("78640", 2025).unwrap().unwrap();
    assert!(
        r.median_annual_premium_cents.is_some(),
        "median premium should be populated from TDOI data"
    );
    let median = r.median_annual_premium_cents.unwrap();
    // $2,844/yr median is reasonable for Kyle TX
    assert!(median.0 > 100_000, "median premium should be > $1,000/yr");
}

#[test]
fn test_zip_hoi_sample_size_populated() {
    let r = store().zip_hoi_rate("78640", 2025).unwrap().unwrap();
    assert!(
        r.sample_size.unwrap_or(0) > 1000,
        "sample_size should be substantial"
    );
}

#[test]
fn test_zip_hoi_fallthrough_pattern() {
    // When ZIP not found, caller uses state rate. Simulate the pattern:
    let zip = store().zip_hoi_rate("99999", 2025).unwrap();
    assert!(zip.is_none());
    // Caller falls through to:
    let state = store().state_hoi_rate("TX", 2025).unwrap();
    assert_eq!(state.annual_rate_bps, 56);
}

#[cfg(feature = "sqlite")]
#[test]
fn test_sqlite_cbsa_matches_json_store() {
    let json = store();
    let sqlite = ref_data::SqliteStore::new_test_store().unwrap();
    let j = json.cbsa_for_county("48209").unwrap().unwrap();
    let s = sqlite.cbsa_for_county("48209").unwrap().unwrap();
    assert_eq!(j.cbsa_code, s.cbsa_code);
    assert_eq!(j.designation, s.designation);
}

#[cfg(feature = "sqlite")]
#[test]
fn test_sqlite_zip_hoi_matches_json_store() {
    let json = store();
    let sqlite = ref_data::SqliteStore::new_test_store().unwrap();
    let j = json.zip_hoi_rate("78640", 2025).unwrap().unwrap();
    let s = sqlite.zip_hoi_rate("78640", 2025).unwrap().unwrap();
    assert_eq!(j.annual_rate_bps, s.annual_rate_bps);
}
