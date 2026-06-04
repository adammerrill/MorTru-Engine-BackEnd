//! Task 4.25 — DPA catalog (TX/CA/FL) with provenance.

use ref_data::dpa_catalog::{
    estimate_dpa_amount, DpaAmountBasis, DpaAssistanceType, DpaEligibilityInput, HeroCategory,
    JurisdictionLevel,
};
use ref_data::{JsonFileStore, RefDataStore};
use types::Cents;

fn store() -> JsonFileStore {
    JsonFileStore::new(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data"))
}

#[allow(clippy::too_many_arguments)]
fn elig(
    state: &str,
    fips: &str,
    fthb: bool,
    vet: bool,
    targeted: bool,
    size: u8,
    income: i64,
    price: i64,
    loan: &str,
    score: u16,
    dti_bps: u32,
    hero: Option<HeroCategory>,
) -> DpaEligibilityInput {
    DpaEligibilityInput {
        state: state.to_owned(),
        county_fips: fips.to_owned(),
        is_first_time_homebuyer: fthb,
        is_veteran: vet,
        in_targeted_area: targeted,
        household_size: size,
        annual_household_income: Cents(income),
        purchase_price: Cents(price),
        loan_type: loan.to_owned(),
        credit_score: score,
        dti_bps,
        borrower_contribution_bps: 0,
        using_agency_first_mortgage: true,
        completed_homebuyer_education: true,
        hero_category: hero,
    }
}

// ── Catalog lookup + coverage ─────────────────────────────────────────────────

#[test]
fn test_dpa_texas_has_five_programs() {
    let progs = store().dpa_programs_for_state("TX", 2025).unwrap();
    assert_eq!(progs.len(), 5, "TX: TDHCA x2, TSAHC x2, Tarrant County");
}

#[test]
fn test_dpa_california_has_myhome() {
    let progs = store().dpa_programs_for_state("CA", 2025).unwrap();
    assert!(progs
        .iter()
        .any(|p| p.value.program_id == "dpa_ca_calhfa_myhome"));
}

#[test]
fn test_dpa_florida_has_four_programs() {
    let progs = store().dpa_programs_for_state("FL", 2025).unwrap();
    assert_eq!(progs.len(), 4, "FL: Assist, HLP, PLUS, Hometown Heroes");
}

#[test]
fn test_dpa_state_lookup_case_insensitive() {
    assert_eq!(store().dpa_programs_for_state("tx", 2025).unwrap().len(), 5);
}

#[test]
fn test_dpa_unknown_state_empty() {
    assert!(store()
        .dpa_programs_for_state("WY", 2025)
        .unwrap()
        .is_empty());
}

#[test]
fn test_dpa_program_by_id() {
    let p = store()
        .dpa_program("dpa_tx_tdhca_mfth", 2025)
        .unwrap()
        .unwrap();
    assert_eq!(p.value.program_name, "My First Texas Home");
    assert_eq!(p.value.assistance_type, DpaAssistanceType::DeferredLoan);
}

#[test]
fn test_dpa_program_unknown_id_none() {
    assert!(store().dpa_program("dpa_zz_nope", 2025).unwrap().is_none());
}

// ── Structure / classification ────────────────────────────────────────────────

#[test]
fn test_dpa_tarrant_is_county_jurisdiction() {
    let p = store()
        .dpa_program("dpa_tx_tarrant_hba", 2025)
        .unwrap()
        .unwrap();
    assert_eq!(p.value.jurisdiction_level, JurisdictionLevel::County);
    assert_eq!(p.value.jurisdiction_fips.as_deref(), Some("48439"));
}

#[test]
fn test_dpa_tdhca_is_state_jurisdiction() {
    let p = store()
        .dpa_program("dpa_tx_tdhca_mfth", 2025)
        .unwrap()
        .unwrap();
    assert_eq!(p.value.jurisdiction_level, JurisdictionLevel::State);
    assert!(p.value.jurisdiction_fips.is_none());
}

#[test]
fn test_dpa_florida_assist_is_fixed_dollar_10k() {
    let p = store()
        .dpa_program("dpa_fl_fhfc_assist", 2025)
        .unwrap()
        .unwrap();
    assert_eq!(p.value.amount_basis, DpaAmountBasis::FixedDollar);
    assert_eq!(p.value.max_amount_cents, 1_000_000);
}

#[test]
fn test_dpa_fl_hlp_is_amortizing_at_3pct() {
    let p = store()
        .dpa_program("dpa_fl_fhfc_hlp", 2025)
        .unwrap()
        .unwrap();
    assert_eq!(p.value.assistance_type, DpaAssistanceType::AmortizingLoan);
    assert_eq!(p.value.interest_rate_bps, 300);
    assert_eq!(p.value.term_months, 180);
}

#[test]
fn test_dpa_fl_plus_is_forgivable_5yr() {
    let p = store()
        .dpa_program("dpa_fl_fhfc_plus", 2025)
        .unwrap()
        .unwrap();
    assert_eq!(p.value.assistance_type, DpaAssistanceType::ForgivableLoan);
    assert_eq!(p.value.forgivable_years, 5);
}

// ── Amount estimation + provenance ────────────────────────────────────────────

#[test]
fn test_dpa_amount_percent_of_loan() {
    // TDHCA MFTH: 5% of loan. $300k loan → $15,000
    let p = store()
        .dpa_program("dpa_tx_tdhca_mfth", 2025)
        .unwrap()
        .unwrap();
    let amt = estimate_dpa_amount(p, Cents(38_000_000), Cents(30_000_000));
    assert_eq!(amt.value, Cents(1_500_000));
}

#[test]
fn test_dpa_amount_percent_of_price() {
    // CalHFA MyHome: 3.5% of price. $400k → $14,000
    let p = store()
        .dpa_program("dpa_ca_calhfa_myhome", 2025)
        .unwrap()
        .unwrap();
    let amt = estimate_dpa_amount(p, Cents(40_000_000), Cents(38_000_000));
    assert_eq!(amt.value, Cents(1_400_000));
}

#[test]
fn test_dpa_amount_fixed_dollar() {
    // FL Assist: fixed $10,000 regardless of price/loan
    let p = store()
        .dpa_program("dpa_fl_fhfc_assist", 2025)
        .unwrap()
        .unwrap();
    let amt = estimate_dpa_amount(p, Cents(30_000_000), Cents(29_000_000));
    assert_eq!(amt.value, Cents(1_000_000));
}

#[test]
fn test_dpa_amount_hits_cap() {
    // Hometown Heroes: 5% of loan capped at $35,000. $800k loan × 5% = $40k → capped
    let p = store()
        .dpa_program("dpa_fl_fhfc_heroes", 2025)
        .unwrap()
        .unwrap();
    let amt = estimate_dpa_amount(p, Cents(82_000_000), Cents(80_000_000));
    assert_eq!(amt.value, Cents(3_500_000), "capped at $35,000");
    assert!(amt.steps.iter().any(|s| s.outcome.contains("capped")));
}

#[test]
fn test_dpa_amount_preserves_provenance() {
    let p = store()
        .dpa_program("dpa_tx_tdhca_mfth", 2025)
        .unwrap()
        .unwrap();
    let amt = estimate_dpa_amount(p, Cents(38_000_000), Cents(30_000_000));
    assert_eq!(amt.provenance.record_id, "dpa_tx_tdhca_mfth");
    assert_eq!(amt.provenance.dataset, "dpa_catalog");
}

// ── Eligibility gates ─────────────────────────────────────────────────────────

#[test]
fn test_dpa_eligible_clean_tdhca_hays() {
    let i = elig(
        "TX", "48209", true, false, false, 3, 12_000_000, 40_000_000, "fha", 680, 4000, None,
    );
    let d = store()
        .dpa_evaluate("dpa_tx_tdhca_mfth", &i, 2025)
        .unwrap()
        .unwrap();
    assert!(d.value.eligible, "{:?}", d.value.disqualifiers);
}

#[test]
fn test_dpa_mcth_allows_repeat_buyer() {
    // My Choice does not require FTHB
    let i = elig(
        "TX",
        "48209",
        false,
        false,
        false,
        2,
        12_000_000,
        40_000_000,
        "conventional",
        680,
        4000,
        None,
    );
    let d = store()
        .dpa_evaluate("dpa_tx_tdhca_mcth", &i, 2025)
        .unwrap()
        .unwrap();
    assert!(d.value.eligible);
}

#[test]
fn test_dpa_mfth_rejects_repeat_buyer_no_exemption() {
    let i = elig(
        "TX", "48209", false, false, false, 2, 12_000_000, 40_000_000, "fha", 680, 4000, None,
    );
    let d = store()
        .dpa_evaluate("dpa_tx_tdhca_mfth", &i, 2025)
        .unwrap()
        .unwrap();
    assert!(!d.value.eligible);
    assert!(d
        .value
        .disqualifiers
        .iter()
        .any(|x| x.contains("first-time")));
}

#[test]
fn test_dpa_veteran_exempt_from_fthb() {
    let i = elig(
        "TX", "48209", false, true, false, 2, 12_000_000, 40_000_000, "va", 680, 4000, None,
    );
    let d = store()
        .dpa_evaluate("dpa_tx_tdhca_mfth", &i, 2025)
        .unwrap()
        .unwrap();
    assert!(d.value.eligible, "veteran exempt");
}

#[test]
fn test_dpa_conventional_rejected_by_government_only_program() {
    // MFTH serves fha/va/usda only — conventional should fail
    let i = elig(
        "TX",
        "48209",
        true,
        false,
        false,
        2,
        12_000_000,
        40_000_000,
        "conventional",
        680,
        4000,
        None,
    );
    let d = store()
        .dpa_evaluate("dpa_tx_tdhca_mfth", &i, 2025)
        .unwrap()
        .unwrap();
    assert!(!d.value.eligible);
    assert!(d
        .value
        .disqualifiers
        .iter()
        .any(|x| x.contains("loan type")));
}

#[test]
fn test_dpa_income_over_limit_fails() {
    // Hays TSAHC limit $167,250; use $200,000
    let i = elig(
        "TX", "48209", true, false, false, 2, 20_000_000, 40_000_000, "fha", 680, 4000, None,
    );
    let d = store()
        .dpa_evaluate("dpa_tx_tsahc_hstx", &i, 2025)
        .unwrap()
        .unwrap();
    assert!(!d.value.eligible);
    assert!(d.value.disqualifiers.iter().any(|x| x.contains("income")));
}

#[test]
fn test_dpa_price_over_limit_fails() {
    // Hays TSAHC price limit $593,363; use $650,000
    let i = elig(
        "TX", "48209", true, false, false, 2, 15_000_000, 65_000_000, "fha", 680, 4000, None,
    );
    let d = store()
        .dpa_evaluate("dpa_tx_tsahc_hstx", &i, 2025)
        .unwrap()
        .unwrap();
    assert!(!d.value.eligible);
    assert!(d.value.disqualifiers.iter().any(|x| x.contains("price")));
}

#[test]
fn test_dpa_targeted_area_raises_price_limit() {
    // $650k fails non-targeted (593k) but passes targeted (725k) in Hays
    let fail = elig(
        "TX", "48209", true, false, false, 2, 15_000_000, 65_000_000, "fha", 680, 4000, None,
    );
    let pass = elig(
        "TX", "48209", true, false, true, 2, 15_000_000, 65_000_000, "fha", 680, 4000, None,
    );
    assert!(
        !store()
            .dpa_evaluate("dpa_tx_tsahc_hstx", &fail, 2025)
            .unwrap()
            .unwrap()
            .value
            .eligible
    );
    assert!(
        store()
            .dpa_evaluate("dpa_tx_tsahc_hstx", &pass, 2025)
            .unwrap()
            .unwrap()
            .value
            .eligible
    );
}

#[test]
fn test_dpa_credit_below_min_fails() {
    // TSAHC min 640; use 620
    let i = elig(
        "TX", "48209", true, false, false, 2, 12_000_000, 40_000_000, "fha", 620, 4000, None,
    );
    let d = store()
        .dpa_evaluate("dpa_tx_tsahc_hstx", &i, 2025)
        .unwrap()
        .unwrap();
    assert!(!d.value.eligible);
    assert!(d.value.disqualifiers.iter().any(|x| x.contains("credit")));
}

#[test]
fn test_dpa_county_default_fallback_for_unlisted_county() {
    // El Paso (48141) not listed for TSAHC → default limit $123,500
    let i = elig(
        "TX", "48141", true, false, false, 2, 15_000_000, 40_000_000, "fha", 680, 4000, None,
    );
    let d = store()
        .dpa_evaluate("dpa_tx_tsahc_hstx", &i, 2025)
        .unwrap()
        .unwrap();
    // $150k > $123.5k default → income fail
    assert!(!d.value.eligible);
    assert!(d.value.disqualifiers.iter().any(|x| x.contains("income")));
}

// ── Hero programs ─────────────────────────────────────────────────────────────

#[test]
fn test_dpa_heroes_requires_hero_category() {
    let non_hero = elig(
        "TX", "48209", true, false, false, 2, 12_000_000, 40_000_000, "fha", 680, 4000, None,
    );
    let d = store()
        .dpa_evaluate("dpa_tx_tsahc_h4tx", &non_hero, 2025)
        .unwrap()
        .unwrap();
    assert!(!d.value.eligible);
    assert!(d.value.disqualifiers.iter().any(|x| x.contains("hero")));
}

#[test]
fn test_dpa_heroes_accepts_teacher() {
    let teacher = elig(
        "TX",
        "48209",
        true,
        false,
        false,
        2,
        12_000_000,
        40_000_000,
        "fha",
        680,
        4000,
        Some(HeroCategory::Teacher),
    );
    let d = store()
        .dpa_evaluate("dpa_tx_tsahc_h4tx", &teacher, 2025)
        .unwrap()
        .unwrap();
    assert!(d.value.eligible);
}

#[test]
fn test_dpa_fl_heroes_accepts_healthcare() {
    let nurse = elig(
        "FL",
        "12086",
        true,
        false,
        false,
        2,
        10_000_000,
        30_000_000,
        "fha",
        680,
        4000,
        Some(HeroCategory::Healthcare),
    );
    let d = store()
        .dpa_evaluate("dpa_fl_fhfc_heroes", &nurse, 2025)
        .unwrap()
        .unwrap();
    assert!(d.value.eligible, "{:?}", d.value.disqualifiers);
}

// ── Provenance / explain ──────────────────────────────────────────────────────

#[test]
fn test_dpa_year_fallback() {
    let d = store()
        .dpa_program("dpa_tx_tdhca_mfth", 2030)
        .unwrap()
        .unwrap();
    assert!(d.provenance.is_fallback());
    assert_eq!(d.provenance.resolved_version, 2025);
}

#[test]
fn test_dpa_explain_renders_trail() {
    let i = elig(
        "TX", "48209", true, false, false, 3, 12_000_000, 40_000_000, "fha", 680, 4000, None,
    );
    let d = store()
        .dpa_evaluate("dpa_tx_tdhca_mfth", &i, 2025)
        .unwrap()
        .unwrap();
    let t = d.explain();
    assert!(t.contains("Source:"));
    assert!(t.contains("final_determination"));
}
