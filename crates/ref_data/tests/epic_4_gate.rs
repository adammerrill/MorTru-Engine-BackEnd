//! Task 4.28 — Epic 4 capstone gate.
//!
//! Proves the full ref_data surface from the Epic 4 revision tasks composes
//! through the `RefDataStore` trait. Unlike the per-task unit tests, this gate
//! drives COHERENT end-to-end borrower scenarios across all four families
//! (MCC 4.24, DPA 4.25, seller concessions 4.26, VA entitlement 4.27) and
//! asserts each returns a fully-populated `Derived` with provenance — the same
//! contract Epic 7's PITIA assembly will consume.

use ref_data::dpa_catalog::DpaEligibilityInput;
use ref_data::mcc_catalog::MccEligibilityInput;
use ref_data::seller_concessions::{ConcessionCapInput, Occupancy};
use ref_data::va_entitlement::{EntitlementStatus, VaGuarantyInput, VaLoanPurpose};
use ref_data::{JsonFileStore, RefDataStore};
use types::Cents;

fn store() -> JsonFileStore {
    JsonFileStore::new(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data"))
}

const YEAR: u16 = 2025;

fn fha_cap_input() -> ConcessionCapInput {
    ConcessionCapInput {
        program: "fha".into(),
        occupancy: Occupancy::Primary,
        ltv_bps: 9650,
    }
}

// ── Coverage: every family resolves through the trait ─────────────────────────

#[test]
fn gate_all_four_families_available() {
    let s = store();
    assert!(s.mcc_program("TX", YEAR).unwrap().is_some(), "MCC (4.24)");
    assert!(
        !s.dpa_programs_for_state("TX", YEAR).unwrap().is_empty(),
        "DPA (4.25)"
    );
    assert!(
        s.seller_concession_cap(&fha_cap_input(), YEAR)
            .unwrap()
            .is_some(),
        "seller concessions (4.26)"
    );
    let _ = s.va_county_loan_limit(Cents(80_650_000), YEAR).unwrap();
}

#[test]
fn gate_every_family_carries_provenance() {
    let s = store();
    let mcc = s.mcc_program("TX", YEAR).unwrap().unwrap();
    let dpa_list = s.dpa_programs_for_state("TX", YEAR).unwrap();
    let dpa = &dpa_list[0];
    let conc = s
        .seller_concession_cap(&fha_cap_input(), YEAR)
        .unwrap()
        .unwrap();
    let va = s.va_county_loan_limit(Cents(80_650_000), YEAR).unwrap();
    for (name, dataset, steps) in [
        ("mcc", mcc.provenance.dataset.as_str(), mcc.steps.len()),
        ("dpa", dpa.provenance.dataset.as_str(), dpa.steps.len()),
        (
            "concession",
            conc.provenance.dataset.as_str(),
            conc.steps.len(),
        ),
        ("va", va.provenance.dataset.as_str(), va.steps.len()),
    ] {
        assert!(!dataset.is_empty(), "{name} has dataset");
        assert!(steps >= 1, "{name} has a derivation trail");
    }
}

#[test]
fn gate_every_family_supports_year_fallback() {
    let s = store();
    assert!(s
        .mcc_program("TX", 2099)
        .unwrap()
        .unwrap()
        .provenance
        .is_fallback());
    assert!(s.dpa_programs_for_state("TX", 2099).unwrap()[0]
        .provenance
        .is_fallback());
    assert!(s
        .va_county_loan_limit(Cents(80_650_000), 2099)
        .unwrap()
        .provenance
        .is_fallback());
}

// ── Scenario A: Texas FHA first-time buyer (MCC + DPA + concession together) ──

#[test]
fn gate_scenario_tx_fha_fthb_composes() {
    let s = store();
    let income = Cents(9_000_000);
    let price = Cents(35_000_000);

    let mcc = s
        .mcc_evaluate(
            &MccEligibilityInput {
                state: "TX".into(),
                is_first_time_homebuyer: true,
                is_veteran: false,
                in_targeted_area: false,
                household_size: 3,
                annual_household_income: income,
                purchase_price: price,
            },
            YEAR,
        )
        .unwrap()
        .unwrap();
    assert!(mcc.value.eligible);

    let dpa = s
        .dpa_evaluate(
            "dpa_tx_tsahc_hstx",
            &DpaEligibilityInput {
                state: "TX".into(),
                county_fips: "48209".into(),
                is_first_time_homebuyer: true,
                is_veteran: false,
                in_targeted_area: false,
                household_size: 3,
                annual_household_income: income,
                purchase_price: price,
                loan_type: "fha".into(),
                credit_score: 680,
                dti_bps: 4000,
                borrower_contribution_bps: 0,
                using_agency_first_mortgage: true,
                completed_homebuyer_education: true,
                hero_category: None,
            },
            YEAR,
        )
        .unwrap()
        .unwrap();
    assert!(dpa.value.eligible, "{:?}", dpa.value.disqualifiers);

    let conc = s
        .evaluate_seller_concession(&fha_cap_input(), price, Cents(1_500_000), YEAR)
        .unwrap()
        .unwrap();
    assert!(conc.value.within_limit, "6% of $350k = $21k, $15k within");
}

// ── Scenario B: VA purchase, full entitlement, with lender overlay ────────────

#[test]
fn gate_scenario_va_purchase_full_entitlement() {
    let s = store();
    let limit = s.va_county_loan_limit(Cents(80_650_000), YEAR).unwrap();
    assert_eq!(limit.value, Cents(80_650_000));

    let va = s
        .va_guaranty(
            &VaGuarantyInput {
                county_conforming_limit: limit.value,
                entitlement_status: EntitlementStatus::Full,
                entitlement_used_cents: 0,
                proposed_loan_amount: Cents(40_000_000),
                down_payment: Cents(0),
                loan_purpose: VaLoanPurpose::Purchase,
                total_borrowers: 1,
                veteran_borrowers: 1,
                nonveteran_coborrowers_all_spouses: false,
                disability_exempt: true,
                lender_max_va_loan_cents: Some(80_650_000),
            },
            YEAR,
        )
        .unwrap();
    assert_eq!(va.value.guaranty_cents, 10_000_000, "25% of $400k");
    assert!(
        va.value.funding_fee_exempt,
        "disability exemption flows through"
    );
    assert!(!va.value.lender_capped, "loan under the lender cap");
}

// ── Scenario C: high-LTV conventional concession (4.26 tier math) ─────────────

#[test]
fn gate_scenario_conventional_high_ltv_concession_excess() {
    let s = store();
    let conc = s
        .evaluate_seller_concession(
            &ConcessionCapInput {
                program: "fnma".into(),
                occupancy: Occupancy::Primary,
                ltv_bps: 9500,
            },
            Cents(40_000_000),
            Cents(2_000_000),
            YEAR,
        )
        .unwrap()
        .unwrap();
    assert!(!conc.value.within_limit);
    assert_eq!(conc.value.excess, Cents(800_000));
}

// ── Cross-cutting: a single Derived can be fully explained ────────────────────

#[test]
fn gate_full_scenario_explains_end_to_end() {
    let s = store();
    let va = s
        .va_guaranty(
            &VaGuarantyInput {
                county_conforming_limit: Cents(80_650_000),
                entitlement_status: EntitlementStatus::Partial,
                entitlement_used_cents: 5_000_000,
                proposed_loan_amount: Cents(50_000_000),
                down_payment: Cents(0),
                loan_purpose: VaLoanPurpose::Purchase,
                total_borrowers: 1,
                veteran_borrowers: 1,
                nonveteran_coborrowers_all_spouses: false,
                disability_exempt: false,
                lender_max_va_loan_cents: None,
            },
            YEAR,
        )
        .unwrap();
    let trail = va.explain();
    assert!(trail.contains("Source:"));
}
