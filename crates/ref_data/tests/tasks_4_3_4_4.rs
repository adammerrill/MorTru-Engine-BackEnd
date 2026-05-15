//! Tasks 4.3 (SQL migrations) + 4.4 (SqliteStore).
//!
//! Task 4.3 verifies the migration files exist and contain valid SQL.
//! Task 4.4 verifies SqliteStore produces identical results to JsonFileStore
//! for all RefDataStore methods, plus SQLite-specific behaviours.

#[cfg(feature = "sqlite")]
use chrono::NaiveDate;
#[cfg(feature = "sqlite")]
use ref_data::{JsonFileStore, RefDataStore};
#[cfg(feature = "sqlite")]
use types::{Cents, ProgramCode};

#[cfg(feature = "sqlite")]
fn data_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data")
}

#[cfg(feature = "sqlite")]
fn json_store() -> JsonFileStore {
    JsonFileStore::new(data_dir())
}

// ── Re-use sqlite store behind feature gate ───────────────────────────────────
#[cfg(feature = "sqlite")]
fn sqlite_store() -> ref_data::SqliteStore {
    ref_data::SqliteStore::new_test_store().expect("SqliteStore::new_test_store failed")
}

// ════════════════════════════════════════════════════════════════════════════
// TASK 4.3 — SQL migration files
// ════════════════════════════════════════════════════════════════════════════

fn migrations_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("migrations")
}

#[test]
fn test_migrations_directory_exists() {
    assert!(
        migrations_dir().is_dir(),
        "crates/ref_data/migrations/ must exist"
    );
}

#[test]
fn test_all_12_migration_files_exist() {
    let required = [
        "0001_initial_types.sql",
        "0002_fha_loan_limits.sql",
        "0003_gse_loan_limits.sql",
        "0004_usda_rural_eligibility.sql",
        "0005_usda_income_limits.sql",
        "0006_usda_mfh_by_tract.sql",
        "0007_ami_tract_data.sql",
        "0008_program_rules.sql",
        "0009_state_hoi_rates.sql",
        "0010_fha_mip_rates.sql",
        "0011_va_funding_fees.sql",
        "0012_lender_rate_data.sql",
    ];
    for filename in required {
        let path = migrations_dir().join(filename);
        assert!(path.exists(), "migration file missing: {filename}");
    }
}

#[test]
fn test_migration_files_ordered_numerically() {
    let dir = migrations_dir();
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().ends_with(".sql"))
        .collect();
    files.sort_by_key(|e| e.file_name());

    let names: Vec<_> = files
        .iter()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();

    // First and last migration files are anchors
    assert!(names.first().unwrap().starts_with("0001_"));
    assert!(names.last().unwrap().starts_with("0012_"));
}

#[test]
fn test_migration_files_non_empty() {
    let dir = migrations_dir();
    for entry in std::fs::read_dir(&dir).unwrap().flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("sql") {
            let content = std::fs::read_to_string(&path).unwrap();
            assert!(
                content.len() > 50,
                "{} appears empty",
                path.file_name().unwrap().to_string_lossy()
            );
        }
    }
}

#[test]
fn test_migration_0002_contains_fha_table() {
    let sql = std::fs::read_to_string(migrations_dir().join("0002_fha_loan_limits.sql")).unwrap();
    assert!(sql.contains("fha_loan_limits"));
    assert!(sql.contains("fips_code"));
    assert!(sql.contains("limit_1_unit"));
}

#[test]
fn test_migration_0008_contains_program_rules_table() {
    let sql = std::fs::read_to_string(migrations_dir().join("0008_program_rules.sql")).unwrap();
    assert!(sql.contains("program_rules"));
    assert!(sql.contains("min_credit_score"));
    assert!(sql.contains("front_end_dti_max_bps"));
}

#[test]
fn test_migration_stub_tables_present() {
    let fha_mip = std::fs::read_to_string(migrations_dir().join("0010_fha_mip_rates.sql")).unwrap();
    assert!(
        fha_mip.contains("fha_mip_rates"),
        "Phase 4 stub must define fha_mip_rates table"
    );

    let va = std::fs::read_to_string(migrations_dir().join("0011_va_funding_fees.sql")).unwrap();
    assert!(
        va.contains("va_funding_fees"),
        "Phase 4 stub must define va_funding_fees table"
    );

    let lender =
        std::fs::read_to_string(migrations_dir().join("0012_lender_rate_data.sql")).unwrap();
    assert!(
        lender.contains("lender_profiles"),
        "Phase 5 stub must define lender_profiles table"
    );
    assert!(
        lender.contains("rate_sheets"),
        "Phase 5 stub must define rate_sheets table"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// TASK 4.4 — SqliteStore (behind the `sqlite` feature gate)
// ════════════════════════════════════════════════════════════════════════════

#[cfg(feature = "sqlite")]
#[test]
fn test_sqlite_store_creates_successfully() {
    let _store = sqlite_store();
    // Constructor succeeds — schema created and data seeded
}

#[cfg(feature = "sqlite")]
#[test]
fn test_sqlite_fha_limits_match_json_store() {
    let json = json_store();
    let sqlite = sqlite_store();
    let j = json.fha_loan_limits("48209", 2025).unwrap().data;
    let s = sqlite.fha_loan_limits("48209", 2025).unwrap().data;
    assert_eq!(j.limit_1_unit, s.limit_1_unit, "FHA 1-unit must match");
    assert_eq!(j.limit_4_unit, s.limit_4_unit, "FHA 4-unit must match");
    assert_eq!(j.limit_type, s.limit_type, "limit_type must match");
    assert_eq!(j.county_name, s.county_name);
}

#[cfg(feature = "sqlite")]
#[test]
fn test_sqlite_fha_san_francisco_high_cost() {
    let sqlite = sqlite_store();
    let l = sqlite.fha_loan_limits("06075", 2025).unwrap().data;
    assert_eq!(l.limit_1_unit, Cents(120_975_000));
    assert_eq!(l.limit_type, ref_data::FhaLimitType::HighCost);
}

#[cfg(feature = "sqlite")]
#[test]
fn test_sqlite_fha_year_fallback() {
    let sqlite = sqlite_store();
    let l2030 = sqlite.fha_loan_limits("48209", 2030).unwrap().data;
    let l2025 = sqlite.fha_loan_limits("48209", 2025).unwrap().data;
    assert_eq!(l2030.limit_1_unit, l2025.limit_1_unit);
}

#[cfg(feature = "sqlite")]
#[test]
fn test_sqlite_fha_missing_fips_returns_error() {
    let sqlite = sqlite_store();
    assert!(sqlite.fha_loan_limits("99999", 2025).is_err());
}

#[cfg(feature = "sqlite")]
#[test]
fn test_sqlite_gse_limits_match_json_store() {
    let json = json_store();
    let sqlite = sqlite_store();
    let j = json.gse_loan_limits("48209", 2025).unwrap().data;
    let s = sqlite.gse_loan_limits("48209", 2025).unwrap().data;
    assert_eq!(j.limit_1_unit, s.limit_1_unit);
    assert_eq!(j.is_high_cost, s.is_high_cost);
}

#[cfg(feature = "sqlite")]
#[test]
fn test_sqlite_gse_high_cost_county() {
    let sqlite = sqlite_store();
    let l = sqlite.gse_loan_limits("06075", 2025).unwrap().data;
    assert!(l.is_high_cost);
    assert_eq!(l.limit_1_unit, Cents(120_975_000));
}

#[cfg(feature = "sqlite")]
#[test]
fn test_sqlite_usda_rural_matches_json_store() {
    let json = json_store();
    let sqlite = sqlite_store();
    let j = json.usda_rural_eligibility("48209010905").unwrap().unwrap();
    let s = sqlite
        .usda_rural_eligibility("48209010905")
        .unwrap()
        .unwrap();
    assert_eq!(j.is_sfh_eligible, s.is_sfh_eligible);
    assert_eq!(j.pct_eligible, s.pct_eligible);
}

#[cfg(feature = "sqlite")]
#[test]
fn test_sqlite_usda_rural_austin_ineligible() {
    let sqlite = sqlite_store();
    let r = sqlite
        .usda_rural_eligibility("48453001801")
        .unwrap()
        .unwrap();
    assert!(!r.is_sfh_eligible);
}

#[cfg(feature = "sqlite")]
#[test]
fn test_sqlite_usda_rural_unknown_returns_none() {
    let sqlite = sqlite_store();
    assert!(sqlite
        .usda_rural_eligibility("99999999999")
        .unwrap()
        .is_none());
}

#[cfg(feature = "sqlite")]
#[test]
fn test_sqlite_usda_income_limits_match_json_store() {
    let json = json_store();
    let sqlite = sqlite_store();
    let eff = NaiveDate::from_ymd_opt(2025, 10, 1).unwrap();
    let j = json.usda_income_limits("48209", eff).unwrap().data;
    let s = sqlite.usda_income_limits("48209", eff).unwrap().data;
    assert_eq!(j.limit_size_1, s.limit_size_1);
    assert_eq!(j.limit_size_5, s.limit_size_5);
}

#[cfg(feature = "sqlite")]
#[test]
fn test_sqlite_usda_mfh_matches_json_store() {
    let json = json_store();
    let sqlite = sqlite_store();
    let j = json.usda_mfh_by_tract("48209010905").unwrap().unwrap();
    let s = sqlite.usda_mfh_by_tract("48209010905").unwrap().unwrap();
    assert_eq!(j.fa_projects, s.fa_projects);
    assert_eq!(j.total_units, s.total_units);
}

#[cfg(feature = "sqlite")]
#[test]
fn test_sqlite_ami_tract_matches_json_store() {
    let json = json_store();
    let sqlite = sqlite_store();
    let j = json
        .ami_tract_data("48209010905", 2025)
        .unwrap()
        .unwrap()
        .data;
    let s = sqlite
        .ami_tract_data("48209010905", 2025)
        .unwrap()
        .unwrap()
        .data;
    assert_eq!(j.ami_80pct, s.ami_80pct);
    assert_eq!(j.ami_100pct, s.ami_100pct);
    assert_eq!(j.is_low_income_tract, s.is_low_income_tract);
}

#[cfg(feature = "sqlite")]
#[test]
fn test_sqlite_ami_year_fallback() {
    let sqlite = sqlite_store();
    let d2025 = sqlite
        .ami_tract_data("48209010905", 2025)
        .unwrap()
        .unwrap()
        .data;
    let d2030 = sqlite
        .ami_tract_data("48209010905", 2030)
        .unwrap()
        .unwrap()
        .data;
    assert_eq!(d2025.ami_80pct, d2030.ami_80pct);
}

#[cfg(feature = "sqlite")]
#[test]
fn test_sqlite_program_rules_match_json_store() {
    let json = json_store();
    let sqlite = sqlite_store();
    let j = json.program_rules(ProgramCode::Fha).unwrap();
    let s = sqlite.program_rules(ProgramCode::Fha).unwrap();
    assert_eq!(j.min_credit_score, s.min_credit_score);
    assert_eq!(j.front_end_dti_max_bps, s.front_end_dti_max_bps);
    assert_eq!(j.max_ltv_bps, s.max_ltv_bps);
}

#[cfg(feature = "sqlite")]
#[test]
fn test_sqlite_program_rules_va_dti() {
    let sqlite = sqlite_store();
    let r = sqlite.program_rules(ProgramCode::Va).unwrap();
    assert_eq!(r.front_end_dti_max_bps, 9999);
}

#[cfg(feature = "sqlite")]
#[test]
fn test_sqlite_state_hoi_rate_matches_json_store() {
    let json = json_store();
    let sqlite = sqlite_store();
    let j = json.state_hoi_rate("TX", 2025).unwrap();
    let s = sqlite.state_hoi_rate("TX", 2025).unwrap();
    assert_eq!(j.annual_rate_bps, s.annual_rate_bps);
}

#[cfg(feature = "sqlite")]
#[test]
fn test_sqlite_state_hoi_all_programs_present() {
    let sqlite = sqlite_store();
    for program in [
        ProgramCode::Fha,
        ProgramCode::Va,
        ProgramCode::Usda,
        ProgramCode::Conventional,
        ProgramCode::HomeReady,
    ] {
        assert!(
            sqlite.program_rules(program).is_ok(),
            "{program:?} must be in SQLite program_rules"
        );
    }
}

#[cfg(feature = "sqlite")]
#[test]
fn test_sqlite_geo_eligibility_matches_json_store() {
    let json = json_store();
    let sqlite = sqlite_store();
    let j = json
        .geo_eligibility("48209", Some("48209010905"), 2025)
        .unwrap();
    let s = sqlite
        .geo_eligibility("48209", Some("48209010905"), 2025)
        .unwrap();
    assert_eq!(j.fha_limit_1_unit, s.fha_limit_1_unit);
    assert_eq!(j.gse_limit_1_unit, s.gse_limit_1_unit);
    assert_eq!(j.usda_sfh_eligible, s.usda_sfh_eligible);
    assert_eq!(j.ami_80pct, s.ami_80pct);
}

#[cfg(feature = "sqlite")]
#[test]
fn test_sqlite_geo_eligibility_conservative_without_tract() {
    let sqlite = sqlite_store();
    let geo = sqlite.geo_eligibility("48209", None, 2025).unwrap();
    assert!(!geo.usda_sfh_eligible);
    assert!(geo.ami_80pct.is_none());
}
