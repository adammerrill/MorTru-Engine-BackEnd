//! Tasks 4.19 (GeoEligibility integration), 4.20 (Epic 4 gate), 4.23 (FHA condo).

use ref_data::{CondoApprovalStatus, JsonFileStore, RefDataStore};
use types::Cents;

fn store() -> JsonFileStore {
    JsonFileStore::new(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data"))
}

// ════════════════════════════════════════════════════════════════════════════
// TASK 4.23 — FHA Condo Project Approval
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn test_fha_condo_approved_project_found() {
    let p = store().fha_condo_project("A000001TX").unwrap().unwrap();
    assert_eq!(p.fha_project_id, "A000001TX");
    assert_eq!(p.status, CondoApprovalStatus::Approved);
    assert!(p.status.is_currently_approved());
}

#[test]
fn test_fha_condo_approved_has_correct_county() {
    let p = store().fha_condo_project("A000001TX").unwrap().unwrap();
    assert_eq!(
        p.county_fips, "48209",
        "Kyle TX condo should be in Hays County"
    );
}

#[test]
fn test_fha_condo_expired_project_not_approved() {
    let p = store().fha_condo_project("A000003TX").unwrap().unwrap();
    assert_eq!(p.status, CondoApprovalStatus::Expired);
    assert!(!p.status.is_currently_approved());
}

#[test]
fn test_fha_condo_withdrawn_project_not_approved() {
    let p = store().fha_condo_project("A000004TX").unwrap().unwrap();
    assert_eq!(p.status, CondoApprovalStatus::Withdrawn);
    assert!(!p.status.is_currently_approved());
}

#[test]
fn test_fha_condo_unknown_id_returns_none() {
    let p = store().fha_condo_project("XXXXXXXXXX").unwrap();
    assert!(p.is_none());
}

#[test]
fn test_fha_condo_approved_has_units_count() {
    let p = store().fha_condo_project("A000001TX").unwrap().unwrap();
    assert!(p.units_in_project.unwrap_or(0) > 0);
}

#[test]
fn test_fha_condo_approval_expiry_populated() {
    let p = store().fha_condo_project("A000001TX").unwrap().unwrap();
    assert!(
        p.approval_expiry.is_some(),
        "approved project must have expiry date"
    );
}

#[test]
fn test_fha_condo_kyle_tx_project_is_in_zip_78640() {
    let p = store().fha_condo_project("A000001TX").unwrap().unwrap();
    assert_eq!(p.zip5, "78640");
}

// ════════════════════════════════════════════════════════════════════════════
// TASK 4.19 — GeoEligibility Unified Query
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn test_geo_eligibility_kyle_tx_sfr_fha_limits_correct() {
    let geo = store()
        .geo_eligibility("48209", Some("48209010905"), 2025)
        .unwrap();
    assert_eq!(geo.fha_limit_1_unit, Cents(52_422_500), "Kyle TX FHA floor");
    assert_eq!(
        geo.gse_limit_1_unit,
        Cents(80_650_000),
        "Kyle TX GSE standard"
    );
}

#[test]
fn test_geo_eligibility_kyle_tx_sfr_usda_eligible() {
    let geo = store()
        .geo_eligibility("48209", Some("48209010905"), 2025)
        .unwrap();
    assert!(geo.usda_sfh_eligible);
    assert!(geo.usda_income_eligible(Cents(8_800_000), 3));
}

#[test]
fn test_geo_eligibility_kyle_tx_sfr_ami_check() {
    let geo = store()
        .geo_eligibility("48209", Some("48209010905"), 2025)
        .unwrap();
    assert!(geo.ami_80pct.is_some());
    assert!(
        geo.hp_income_eligible(Cents(7_500_000)),
        "income ≤80% AMI qualifies for HomeReady"
    );
    assert!(
        !geo.hp_income_eligible(Cents(9_000_000)),
        "income >80% AMI disqualifies"
    );
}

#[test]
fn test_geo_eligibility_kyle_tx_condo_same_as_sfr() {
    // Same county and FIPS — loan limits should match
    let sfr = store()
        .geo_eligibility("48209", Some("48209010905"), 2025)
        .unwrap();
    let condo = store()
        .geo_eligibility("48209", Some("48209010906"), 2025)
        .unwrap();
    assert_eq!(sfr.fha_limit_1_unit, condo.fha_limit_1_unit);
    assert_eq!(sfr.gse_limit_1_unit, condo.gse_limit_1_unit);
}

#[test]
fn test_geo_eligibility_austin_urban_usda_ineligible() {
    let geo = store()
        .geo_eligibility("48453", Some("48453001801"), 2025)
        .unwrap();
    assert!(
        !geo.usda_sfh_eligible,
        "Travis County urban tract: USDA ineligible"
    );
}

#[test]
fn test_geo_eligibility_wimberley_rural_fully_eligible() {
    let geo = store()
        .geo_eligibility("48209", Some("48209950100"), 2025)
        .unwrap();
    assert!(geo.usda_sfh_eligible);
    assert_eq!(geo.usda_pct_eligible, Some(100.0));
}

#[test]
fn test_geo_eligibility_no_tract_conservative_usda() {
    let geo = store().geo_eligibility("48209", None, 2025).unwrap();
    assert!(
        !geo.usda_sfh_eligible,
        "no tract → conservative: USDA ineligible"
    );
    assert!(geo.ami_80pct.is_none(), "no tract → no AMI data");
}

#[test]
fn test_geo_eligibility_sf_high_cost_county() {
    let geo = store().geo_eligibility("06075", None, 2025).unwrap();
    assert!(geo.gse_is_high_cost);
    assert!(
        geo.gse_limit_1_unit > Cents(80_650_000),
        "SF GSE limit exceeds national standard"
    );
}

#[test]
fn test_geo_eligibility_year_fallback() {
    let geo_2025 = store()
        .geo_eligibility("48209", Some("48209010905"), 2025)
        .unwrap();
    let geo_2030 = store()
        .geo_eligibility("48209", Some("48209010905"), 2030)
        .unwrap();
    assert_eq!(
        geo_2025.fha_limit_1_unit, geo_2030.fha_limit_1_unit,
        "year fallback"
    );
}

// SqliteStore: geo_eligibility end-to-end via SQL
#[cfg(feature = "sqlite")]
#[test]
fn test_sqlite_geo_eligibility_kyle_tx_sfr_matches_json() {
    let json = store();
    let sqlite = ref_data::SqliteStore::new_test_store().unwrap();
    let j = json
        .geo_eligibility("48209", Some("48209010905"), 2025)
        .unwrap();
    let s = sqlite
        .geo_eligibility("48209", Some("48209010905"), 2025)
        .unwrap();
    assert_eq!(j.fha_limit_1_unit, s.fha_limit_1_unit);
    assert_eq!(j.usda_sfh_eligible, s.usda_sfh_eligible);
    assert_eq!(j.ami_80pct, s.ami_80pct);
}

#[cfg(feature = "sqlite")]
#[test]
fn test_sqlite_geo_eligibility_austin_usda_ineligible() {
    let sqlite = ref_data::SqliteStore::new_test_store().unwrap();
    let geo = sqlite
        .geo_eligibility("48453", Some("48453001801"), 2025)
        .unwrap();
    assert!(!geo.usda_sfh_eligible);
}

// ════════════════════════════════════════════════════════════════════════════
// TASK 4.20 — Epic 4 Gate: 5 fixture scenarios
// ════════════════════════════════════════════════════════════════════════════

// ── Fixture 1: Kyle TX SFR ────────────────────────────────────────────────────

#[test]
fn test_gate_kyle_sfr_fha_qualifiable() {
    use ref_data::{FhaMipInput, VaFeeInput, VaLoanPurpose, VaUse, VeteranCategory};
    let s = store();
    // Geo
    let geo = s
        .geo_eligibility("48209", Some("48209010905"), 2025)
        .unwrap();
    assert!(
        geo.fha_loan_within_limit(Cents(38_000_000), 1),
        "FHA $380k < $524k limit"
    );
    // Program rules
    let fha = s.program_rules(types::ProgramCode::Fha).unwrap();
    assert!(
        fha.credit_score_eligible(types::CreditScore(650), 350),
        "650 FICO qualifies FHA"
    );
    // MIP
    let mip = s
        .fha_mip(
            &FhaMipInput {
                term_months: 360,
                ltv_bps: 9650,
                base_loan_cents: 38_000_000,
                is_streamline_pre_2009: false,
            },
            2025,
        )
        .unwrap();
    assert_eq!(mip.ufmip_bps, 175);
    assert!(mip.annual_mip_bps > 0);
    // VA fee
    let va_fee = s
        .va_funding_fee(
            &VaFeeInput {
                category: VeteranCategory::RegularMilitary,
                purpose: VaLoanPurpose::PurchaseOrConstruction,
                use_: VaUse::FirstTime,
                down_payment_bps: 0,
            },
            2025,
        )
        .unwrap();
    assert_eq!(va_fee, 215);
}

#[test]
fn test_gate_kyle_sfr_conventional_qualifiable() {
    use ref_data::{ConvMiInput, ConvMiProgram, LlpaInput};
    let s = store();
    let geo = s
        .geo_eligibility("48209", Some("48209010905"), 2025)
        .unwrap();
    // GSE loan within limit
    let (within, _) = geo.gse_loan_status(Cents(38_000_000), 1);
    assert!(within, "conventional $380k < GSE standard limit");
    // Conv MI coverage
    let cov = s
        .conv_mi_coverage(
            &ConvMiInput {
                program: ConvMiProgram::Standard,
                term_months: 360,
                ltv_bps: 9500,
                is_arm: false,
                is_standard_manufactured: false,
            },
            2025,
        )
        .unwrap();
    assert_eq!(cov.standard_pct, 30);
    // LLPA
    let llpa = s
        .llpa_total(
            "fnma",
            &LlpaInput {
                fico: 720,
                ltv_bps: 9500,
                loan_purpose: "purchase".to_owned(),
                occupancy: "primary".to_owned(),
                is_standard_manufactured: false,
                is_high_balance: false,
            },
            2025,
        )
        .unwrap();
    assert!(llpa >= 0);
}

#[test]
fn test_gate_kyle_sfr_usda_qualifiable() {
    let s = store();
    let geo = s
        .geo_eligibility("48209", Some("48209010905"), 2025)
        .unwrap();
    let usda = s.program_rules(types::ProgramCode::Usda).unwrap();
    assert!(geo.usda_sfh_eligible);
    assert!(
        geo.usda_income_eligible(Cents(7_500_000), 3),
        "USDA income check passes"
    );
    assert!(
        usda.credit_score_eligible(types::CreditScore(680), 0),
        "680 FICO qualifies USDA"
    );
    // Fees
    let fees = s.usda_guarantee_fees(2025).unwrap();
    assert!(fees.upfront_fee_bps > 0);
    assert!(fees.annual_fee_bps > 0);
}

#[test]
fn test_gate_kyle_sfr_homready_ami_gate() {
    let s = store();
    let geo = s
        .geo_eligibility("48209", Some("48209010905"), 2025)
        .unwrap();
    // HomeReady requires income ≤ 80% AMI (~$76,667 for this tract)
    assert!(
        geo.hp_income_eligible(Cents(7_000_000)),
        "$70k qualifies HomeReady"
    );
    assert!(
        !geo.hp_income_eligible(Cents(9_000_000)),
        "$90k exceeds 80% AMI"
    );
}

// ── Fixture 2: Kyle TX Condo ──────────────────────────────────────────────────

#[test]
fn test_gate_kyle_condo_fha_requires_approval() {
    let s = store();
    // The condo project is approved
    let project = s.fha_condo_project("A000001TX").unwrap().unwrap();
    assert!(
        project.status.is_currently_approved(),
        "Kyle condo project FHA-approved"
    );
    // Geo same as SFR for loan limits
    let geo = s
        .geo_eligibility("48209", Some("48209010906"), 2025)
        .unwrap();
    assert!(geo.fha_loan_within_limit(Cents(38_000_000), 1));
}

#[test]
fn test_gate_expired_condo_fha_ineligible() {
    let s = store();
    let project = s.fha_condo_project("A000003TX").unwrap().unwrap();
    assert!(
        !project.status.is_currently_approved(),
        "expired project: FHA ineligible"
    );
}

// ── Fixture 3: Manufactured home (Wimberley TX rural) ────────────────────────

#[test]
fn test_gate_manufactured_home_usda_eligible() {
    let s = store();
    let geo = s
        .geo_eligibility("48209", Some("48209950100"), 2025)
        .unwrap();
    assert!(geo.usda_sfh_eligible, "Wimberley rural: USDA eligible");
    assert_eq!(geo.usda_pct_eligible, Some(100.0));
}

#[test]
fn test_gate_manufactured_home_conv_mi_coverage() {
    use ref_data::{ConvMiInput, ConvMiProgram};
    let s = store();
    // Standard manufactured home coverage (limited to ≤95% LTV)
    let cov = s
        .conv_mi_coverage(
            &ConvMiInput {
                program: ConvMiProgram::Standard,
                term_months: 360,
                ltv_bps: 9200,
                is_arm: false,
                is_standard_manufactured: true,
            },
            2025,
        )
        .unwrap();
    assert_eq!(
        cov.standard_pct, 30,
        "standard MH: 30% coverage at 90-95% LTV"
    );
}

#[test]
fn test_gate_manufactured_home_llpa_premium() {
    use ref_data::LlpaInput;
    let s = store();
    let base = s
        .llpa_total(
            "fnma",
            &LlpaInput {
                fico: 720,
                ltv_bps: 9200,
                loan_purpose: "purchase".to_owned(),
                occupancy: "primary".to_owned(),
                is_standard_manufactured: false,
                is_high_balance: false,
            },
            2025,
        )
        .unwrap();
    let with_mfg = s
        .llpa_total(
            "fnma",
            &LlpaInput {
                fico: 720,
                ltv_bps: 9200,
                loan_purpose: "purchase".to_owned(),
                occupancy: "primary".to_owned(),
                is_standard_manufactured: true,
                is_high_balance: false,
            },
            2025,
        )
        .unwrap();
    assert_eq!(with_mfg - base, 50, "manufactured home adds 50 bps LLPA");
}

// ── Fixture 4: Travis County 4-unit ─────────────────────────────────────────

#[test]
fn test_gate_4unit_austin_fha_4unit_limit() {
    let s = store();
    let geo = s
        .geo_eligibility("48453", Some("48453001801"), 2025)
        .unwrap();
    // 4-unit FHA limit is higher than 1-unit
    assert!(
        geo.fha_limit_4_unit > geo.fha_limit_1_unit,
        "FHA 4-unit limit must exceed 1-unit limit"
    );
}

#[test]
fn test_gate_4unit_austin_usda_ineligible() {
    let s = store();
    let geo = s
        .geo_eligibility("48453", Some("48453001801"), 2025)
        .unwrap();
    assert!(
        !geo.usda_sfh_eligible,
        "Travis urban tract: USDA SFH ineligible"
    );
}

#[test]
fn test_gate_4unit_fha_program_allows_4_units() {
    let s = store();
    let fha = s.program_rules(types::ProgramCode::Fha).unwrap();
    // FHA allows 4-unit; max_ltv for primary residence applies
    assert!(
        fha.max_ltv_bps >= 9650,
        "FHA allows high LTV on 4-unit primary"
    );
}

// ── All programs: complete PITIA building blocks available ───────────────────

#[test]
fn test_gate_all_pitia_components_available_for_kyle_sfr() {
    let s = store();
    // Property taxes and HOI come from reso/property data (not ref_data)
    // ref_data provides: HOI rate estimate, MIP/funding fee, MI premium

    // HOI rate
    let hoi = s.state_hoi_rate("TX", 2025).unwrap();
    assert!(hoi.annual_rate_bps > 0);

    // FHA MIP
    let mip = s
        .fha_mip(
            &ref_data::FhaMipInput {
                term_months: 360,
                ltv_bps: 9650,
                base_loan_cents: 40_000_000,
                is_streamline_pre_2009: false,
            },
            2025,
        )
        .unwrap();
    assert!(mip.annual_mip_bps > 0);

    // VA funding fee
    let va_fee = s
        .va_funding_fee(
            &ref_data::VaFeeInput {
                category: ref_data::VeteranCategory::RegularMilitary,
                purpose: ref_data::VaLoanPurpose::PurchaseOrConstruction,
                use_: ref_data::VaUse::FirstTime,
                down_payment_bps: 0,
            },
            2025,
        )
        .unwrap();
    assert!(va_fee > 0);

    // Conv MI monthly rate
    let mi_rate = s
        .mi_monthly_rate(
            "nmi",
            &ref_data::MiRateInput {
                ltv_bps: 9500,
                coverage_pct: 30,
                fico: 720,
                term_months: 360,
                is_non_fixed: false,
            },
            2025,
        )
        .unwrap();
    assert!(mi_rate > 0);

    // USDA annual fee
    let usda = s.usda_guarantee_fees(2025).unwrap();
    assert!(usda.annual_fee_bps > 0);

    // Rate sheet (par rate for PITI P+I calculation)
    let sheet = s.rate_sheet("lndr_abc_mortgage").unwrap().unwrap();
    assert!(sheet.find("fha_30yr_fixed", 30).unwrap().par_rate_bps > 0);
}
