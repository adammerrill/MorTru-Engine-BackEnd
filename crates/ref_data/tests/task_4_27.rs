//! Task 4.27 — VA loan limits, guaranty, and entitlement analysis.

use ref_data::va_entitlement::{
    CoeEntitlementCode, EntitlementStatus, VaGuarantyInput, VaLoanPurpose,
};
use ref_data::{JsonFileStore, RefDataStore};
use types::Cents;

fn store() -> JsonFileStore {
    JsonFileStore::new(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data"))
}

fn base() -> VaGuarantyInput {
    VaGuarantyInput {
        county_conforming_limit: Cents(80_650_000), // $806,500 (2025 standard)
        entitlement_status: EntitlementStatus::Full,
        entitlement_used_cents: 0,
        proposed_loan_amount: Cents(40_000_000),
        down_payment: Cents(0),
        loan_purpose: VaLoanPurpose::Purchase,
        total_borrowers: 1,
        veteran_borrowers: 1,
        nonveteran_coborrowers_all_spouses: false,
        disability_exempt: false,
        lender_max_va_loan_cents: None,
    }
}

// ── County limit delegation ───────────────────────────────────────────────────

#[test]
fn test_va_county_limit_delegates_to_gse() {
    let d = store()
        .va_county_loan_limit(Cents(80_650_000), 2025)
        .unwrap();
    assert_eq!(d.value, Cents(80_650_000));
    assert!(d
        .steps
        .iter()
        .any(|s| s.rule == "delegate_to_gse_conforming_limit"));
}

#[test]
fn test_va_county_limit_high_cost() {
    let d = store()
        .va_county_loan_limit(Cents(120_975_000), 2025)
        .unwrap();
    assert_eq!(d.value, Cents(120_975_000));
}

// ── Full entitlement ──────────────────────────────────────────────────────────

#[test]
fn test_full_entitlement_no_va_cap() {
    let d = store().va_guaranty(&base(), 2025).unwrap();
    assert_eq!(
        d.value.zero_down_max_cents,
        i64::MAX,
        "full entitlement: no VA cap"
    );
    assert_eq!(d.value.required_down_payment_cents, 0);
}

#[test]
fn test_full_entitlement_guaranty_is_25pct() {
    // $400k loan → 25% guaranty = $100k
    let d = store().va_guaranty(&base(), 2025).unwrap();
    assert_eq!(d.value.guaranty_cents, 10_000_000);
    assert_eq!(d.value.guaranty_pct_bps, 2500);
    assert!(d.value.meets_25pct_guaranty);
}

#[test]
fn test_full_entitlement_large_loan_still_zero_down() {
    let mut i = base();
    i.proposed_loan_amount = Cents(90_000_000); // $900k > county limit, but full entitlement
    let d = store().va_guaranty(&i, 2025).unwrap();
    assert_eq!(
        d.value.required_down_payment_cents, 0,
        "full entitlement ignores county limit"
    );
}

// ── Lender overlay (max VA loan) ──────────────────────────────────────────────

#[test]
fn test_lender_overlay_caps_full_entitlement() {
    let mut i = base();
    i.proposed_loan_amount = Cents(90_000_000); // $900k
    i.lender_max_va_loan_cents = Some(80_650_000); // lender caps at $806,500
    let d = store().va_guaranty(&i, 2025).unwrap();
    assert!(d.value.lender_capped);
    assert_eq!(d.value.zero_down_max_cents, 80_650_000);
    // gap above cap must be funded: $900k - $806.5k = $93.5k
    assert_eq!(d.value.required_down_payment_cents, 9_350_000);
}

#[test]
fn test_lender_overlay_not_triggered_when_loan_under_cap() {
    let mut i = base();
    i.lender_max_va_loan_cents = Some(80_650_000);
    // $400k loan well under cap
    let d = store().va_guaranty(&i, 2025).unwrap();
    assert!(!d.value.lender_capped);
    assert_eq!(d.value.required_down_payment_cents, 0);
}

// ── Partial entitlement ───────────────────────────────────────────────────────

#[test]
fn test_partial_entitlement_remaining_and_zero_down_max() {
    // 25% of $806,500 = $201,625 max; used $50,000 → remaining $151,625; ×4 = $606,500
    let mut i = base();
    i.entitlement_status = EntitlementStatus::Partial;
    i.entitlement_used_cents = 5_000_000;
    i.proposed_loan_amount = Cents(50_000_000); // $500k, under zero-down max
    let d = store().va_guaranty(&i, 2025).unwrap();
    assert_eq!(
        d.value.zero_down_max_cents, 60_650_000,
        "remaining×4 = $606,500"
    );
    assert_eq!(
        d.value.required_down_payment_cents, 0,
        "$500k under zero-down max"
    );
}

#[test]
fn test_partial_entitlement_shortfall_down_payment() {
    // remaining $151,625; zero-down max $606,500; loan $700k → gap $93,500; DP = 25% = $23,375
    let mut i = base();
    i.entitlement_status = EntitlementStatus::Partial;
    i.entitlement_used_cents = 5_000_000;
    i.proposed_loan_amount = Cents(70_000_000);
    let d = store().va_guaranty(&i, 2025).unwrap();
    assert_eq!(d.value.zero_down_max_cents, 60_650_000);
    assert_eq!(
        d.value.required_down_payment_cents, 2_337_500,
        "25% of $93,500 gap"
    );
}

#[test]
fn test_partial_entitlement_uses_county_limit() {
    // higher county limit → more remaining entitlement → higher zero-down max
    let mut lo = base();
    lo.entitlement_status = EntitlementStatus::Partial;
    lo.entitlement_used_cents = 5_000_000;
    lo.county_conforming_limit = Cents(80_650_000);
    let mut hi = lo.clone();
    hi.county_conforming_limit = Cents(120_975_000); // high-cost
    let dlo = store().va_guaranty(&lo, 2025).unwrap();
    let dhi = store().va_guaranty(&hi, 2025).unwrap();
    assert!(dhi.value.zero_down_max_cents > dlo.value.zero_down_max_cents);
}

// ── Joint loans ───────────────────────────────────────────────────────────────

#[test]
fn test_joint_loan_nonveteran_triggers_down_payment() {
    // veteran + non-veteran non-spouse, 2 borrowers, $400k
    let mut i = base();
    i.total_borrowers = 2;
    i.veteran_borrowers = 1;
    i.nonveteran_coborrowers_all_spouses = false;
    let d = store().va_guaranty(&i, 2025).unwrap();
    assert!(d.value.is_joint_nonveteran_loan);
    // non-veteran share = $200k; 12.5% = $25,000
    assert_eq!(d.value.required_down_payment_cents, 2_500_000);
}

#[test]
fn test_joint_loan_guaranty_only_veteran_share() {
    let mut i = base();
    i.total_borrowers = 2;
    i.veteran_borrowers = 1;
    let d = store().va_guaranty(&i, 2025).unwrap();
    // guaranty = 25% of veteran's $200k share = $50k
    assert_eq!(d.value.guaranty_cents, 5_000_000);
}

#[test]
fn test_nonveteran_spouse_exception_no_joint_dp() {
    let mut i = base();
    i.total_borrowers = 2;
    i.veteran_borrowers = 1;
    i.nonveteran_coborrowers_all_spouses = true; // spouse → full guaranty
    let d = store().va_guaranty(&i, 2025).unwrap();
    assert!(!d.value.is_joint_nonveteran_loan);
    assert_eq!(
        d.value.guaranty_cents, 10_000_000,
        "full 25% of $400k, spouse not a joint borrower"
    );
    assert_eq!(d.value.required_down_payment_cents, 0);
}

#[test]
fn test_two_veterans_combine_no_joint_dp() {
    let mut i = base();
    i.total_borrowers = 2;
    i.veteran_borrowers = 2; // both veterans
    let d = store().va_guaranty(&i, 2025).unwrap();
    assert!(!d.value.is_joint_nonveteran_loan);
}

// ── Disability exemption ──────────────────────────────────────────────────────

#[test]
fn test_disability_funding_fee_exempt() {
    let mut i = base();
    i.disability_exempt = true;
    let d = store().va_guaranty(&i, 2025).unwrap();
    assert!(d.value.funding_fee_exempt);
    assert!(d.steps.iter().any(|s| s.rule == "funding_fee_exemption"));
}

#[test]
fn test_no_disability_not_exempt() {
    let d = store().va_guaranty(&base(), 2025).unwrap();
    assert!(!d.value.funding_fee_exempt);
}

// ── Provenance ────────────────────────────────────────────────────────────────

#[test]
fn test_guaranty_carries_va_citation() {
    let d = store().va_guaranty(&base(), 2025).unwrap();
    assert_eq!(d.provenance.dataset, "va_entitlement");
    assert!(d.provenance.source_citation.contains("26-7"));
}

#[test]
fn test_year_fallback() {
    let d = store().va_guaranty(&base(), 2030).unwrap();
    assert!(d.provenance.is_fallback());
    assert_eq!(d.provenance.resolved_version, 2025);
}

#[test]
fn test_explain_renders_trail() {
    let mut i = base();
    i.entitlement_status = EntitlementStatus::Partial;
    i.entitlement_used_cents = 5_000_000;
    i.proposed_loan_amount = Cents(70_000_000);
    let t = store().va_guaranty(&i, 2025).unwrap().explain();
    assert!(t.contains("partial_entitlement_remaining"));
    assert!(t.contains("shortfall_down_payment"));
}

#[test]
fn test_combined_partial_disability_joint() {
    // exercises multiple branches together
    let mut i = base();
    i.entitlement_status = EntitlementStatus::Partial;
    i.entitlement_used_cents = 5_000_000;
    i.disability_exempt = true;
    i.total_borrowers = 2;
    i.veteran_borrowers = 1;
    let d = store().va_guaranty(&i, 2025).unwrap();
    assert!(d.value.funding_fee_exempt);
    assert!(d.value.is_joint_nonveteran_loan);
}

// ── Statutory guaranty bands (38 CFR 36.4302) — exhaustive ────────────────────

#[test]
fn test_band_under_45k_is_50pct() {
    let mut i = base();
    i.proposed_loan_amount = Cents(4_000_000); // $40,000
    let d = store().va_guaranty(&i, 2025).unwrap();
    assert_eq!(
        d.value.guaranty_cents, 2_000_000,
        "50% of $40k = $20k (NOT 25%)"
    );
}

#[test]
fn test_band_45k_to_56250_is_flat_22500() {
    let mut i = base();
    i.proposed_loan_amount = Cents(5_000_000); // $50,000
    let d = store().va_guaranty(&i, 2025).unwrap();
    assert_eq!(d.value.guaranty_cents, 2_250_000, "flat $22,500");
}

#[test]
fn test_band_56250_to_144k_is_40pct_capped_36k() {
    let mut i = base();
    i.proposed_loan_amount = Cents(8_000_000); // $80,000 → 40% = $32,000
    let d = store().va_guaranty(&i, 2025).unwrap();
    assert_eq!(d.value.guaranty_cents, 3_200_000);
    // at $144,000 → 40% = $57,600 but capped at $36,000
    i.proposed_loan_amount = Cents(14_400_000);
    let d2 = store().va_guaranty(&i, 2025).unwrap();
    assert_eq!(d2.value.guaranty_cents, 3_600_000, "capped at $36,000");
}

#[test]
fn test_band_over_144k_is_25pct_uncapped() {
    let mut i = base();
    i.proposed_loan_amount = Cents(20_000_000); // $200,000 → 25% = $50,000
    let d = store().va_guaranty(&i, 2025).unwrap();
    assert_eq!(d.value.guaranty_cents, 5_000_000);
    // $300,000 → 25% = $75,000, UNCAPPED post-2020 (Blue Water Navy removed the cap)
    i.proposed_loan_amount = Cents(30_000_000);
    let d2 = store().va_guaranty(&i, 2025).unwrap();
    assert_eq!(
        d2.value.guaranty_cents, 7_500_000,
        "25% uncapped, not the stale $60k cap"
    );
}

#[test]
fn test_band_note_recorded() {
    let mut i = base();
    i.proposed_loan_amount = Cents(4_000_000);
    let d = store().va_guaranty(&i, 2025).unwrap();
    assert!(d.value.guaranty_band_note.contains("50%"));
}

// ── IRRRL ─────────────────────────────────────────────────────────────────────

#[test]
fn test_irrrl_flat_25pct_even_small_loan() {
    // IRRRL ignores the band schedule — 25% even on a $40k loan
    let mut i = base();
    i.loan_purpose = VaLoanPurpose::Irrrl;
    i.proposed_loan_amount = Cents(4_000_000);
    let d = store().va_guaranty(&i, 2025).unwrap();
    assert!(d.value.is_irrrl);
    assert_eq!(
        d.value.guaranty_cents, 1_000_000,
        "IRRRL: flat 25% of $40k = $10k"
    );
}

#[test]
fn test_purchase_not_flagged_irrrl() {
    let d = store().va_guaranty(&base(), 2025).unwrap();
    assert!(!d.value.is_irrrl);
}

// ── COE entitlement codes (identifiers) ───────────────────────────────────────

#[test]
fn test_coe_code_never_used_is_full() {
    assert_eq!(
        CoeEntitlementCode::NeverUsed.implies_status(),
        EntitlementStatus::Full
    );
}

#[test]
fn test_coe_code_in_use_is_partial() {
    assert_eq!(
        CoeEntitlementCode::InUseNotRestored.implies_status(),
        EntitlementStatus::Partial
    );
}

#[test]
fn test_coe_code_one_time_restoration_is_full() {
    assert_eq!(
        CoeEntitlementCode::OneTimeRestoration.implies_status(),
        EntitlementStatus::Full
    );
}

#[test]
fn test_coe_code_substitution_is_full() {
    assert_eq!(
        CoeEntitlementCode::SubstitutionOfEntitlement.implies_status(),
        EntitlementStatus::Full
    );
}
