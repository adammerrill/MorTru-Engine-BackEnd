//! Task 4.24 — MCC program catalog with full derivation provenance.

use ref_data::mcc_catalog::{estimate_annual_credit, MccEligibilityInput};
use ref_data::{JsonFileStore, RefDataStore};
use types::Cents;

fn store() -> JsonFileStore {
    JsonFileStore::new(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data"))
}

fn input(
    state: &str,
    fthb: bool,
    veteran: bool,
    targeted: bool,
    size: u8,
    income: i64,
    price: i64,
) -> MccEligibilityInput {
    MccEligibilityInput {
        state: state.to_owned(),
        is_first_time_homebuyer: fthb,
        is_veteran: veteran,
        in_targeted_area: targeted,
        household_size: size,
        annual_household_income: Cents(income),
        purchase_price: Cents(price),
    }
}

// ── Catalog lookup + provenance ───────────────────────────────────────────────

#[test]
fn test_mcc_lookup_texas_found() {
    let d = store().mcc_program("TX", 2025).unwrap().unwrap();
    assert_eq!(d.value.program_id, "mcc_tx_tdhca");
    assert_eq!(d.value.credit_rate_bps, 4000);
}

#[test]
fn test_mcc_lookup_case_insensitive_state() {
    let lower = store().mcc_program("tx", 2025).unwrap().unwrap();
    assert_eq!(lower.value.program_id, "mcc_tx_tdhca");
}

#[test]
fn test_mcc_lookup_unknown_state_returns_none() {
    assert!(store().mcc_program("WY", 2025).unwrap().is_none());
}

#[test]
fn test_mcc_lookup_carries_provenance() {
    let d = store().mcc_program("TX", 2025).unwrap().unwrap();
    assert_eq!(d.provenance.dataset, "mcc_catalog");
    assert_eq!(d.provenance.source_file, "mcc_catalog_2025.json");
    assert_eq!(d.provenance.record_id, "mcc_tx_tdhca");
    assert_eq!(d.provenance.effective_date, "2025-01-01");
    assert!(d.provenance.source_citation.contains("HFA"));
}

#[test]
fn test_mcc_lookup_has_derivation_step() {
    let d = store().mcc_program("TX", 2025).unwrap().unwrap();
    assert_eq!(d.steps.len(), 1);
    assert_eq!(d.steps[0].rule, "lookup_program_by_state");
}

#[test]
fn test_mcc_year_fallback_records_resolved_version() {
    let d = store().mcc_program("TX", 2030).unwrap().unwrap();
    assert_eq!(d.provenance.requested_version, 2030);
    assert_eq!(d.provenance.resolved_version, 2025);
    assert!(
        d.provenance.is_fallback(),
        "2030 request should fall back to 2025 data"
    );
}

#[test]
fn test_mcc_exact_year_is_not_fallback() {
    let d = store().mcc_program("TX", 2025).unwrap().unwrap();
    assert!(!d.provenance.is_fallback());
}

#[test]
fn test_mcc_three_states_seeded() {
    for st in ["TX", "CA", "FL"] {
        assert!(
            store().mcc_program(st, 2025).unwrap().is_some(),
            "{st} missing"
        );
    }
}

// ── Eligibility: passing path ─────────────────────────────────────────────────

#[test]
fn test_mcc_eligible_clean_fthb() {
    let d = store()
        .mcc_evaluate(
            &input("TX", true, false, false, 3, 9_000_000, 35_000_000),
            2025,
        )
        .unwrap()
        .unwrap();
    assert!(d.value.eligible);
    assert!(d.value.disqualifiers.is_empty());
}

#[test]
fn test_mcc_eligible_records_all_gate_steps() {
    let d = store()
        .mcc_evaluate(
            &input("TX", true, false, false, 3, 9_000_000, 35_000_000),
            2025,
        )
        .unwrap()
        .unwrap();
    let rules: Vec<&str> = d.steps.iter().map(|s| s.rule.as_str()).collect();
    assert!(rules.contains(&"first_time_homebuyer_requirement"));
    assert!(rules.contains(&"income_limit"));
    assert!(rules.contains(&"purchase_price_limit"));
    assert!(rules.contains(&"final_determination"));
}

// ── Eligibility: FTHB exemptions ──────────────────────────────────────────────

#[test]
fn test_mcc_veteran_exempt_from_fthb() {
    // Not a FTHB, but a veteran → exempt
    let d = store()
        .mcc_evaluate(
            &input("TX", false, true, false, 2, 8_000_000, 30_000_000),
            2025,
        )
        .unwrap()
        .unwrap();
    assert!(d.value.eligible, "veteran should be exempt from FTHB");
    let vet_step = d
        .steps
        .iter()
        .find(|s| s.rule == "first_time_homebuyer_requirement")
        .unwrap();
    assert!(vet_step.outcome.contains("veteran exemption"));
}

#[test]
fn test_mcc_targeted_area_exempt_from_fthb() {
    let d = store()
        .mcc_evaluate(
            &input("TX", false, false, true, 2, 8_000_000, 30_000_000),
            2025,
        )
        .unwrap()
        .unwrap();
    assert!(d.value.eligible, "targeted-area buyer exempt from FTHB");
    let step = d
        .steps
        .iter()
        .find(|s| s.rule == "first_time_homebuyer_requirement")
        .unwrap();
    assert!(step.outcome.contains("targeted-area exemption"));
}

#[test]
fn test_mcc_non_fthb_no_exemption_fails() {
    let d = store()
        .mcc_evaluate(
            &input("TX", false, false, false, 2, 8_000_000, 30_000_000),
            2025,
        )
        .unwrap()
        .unwrap();
    assert!(!d.value.eligible);
    assert!(d
        .value
        .disqualifiers
        .iter()
        .any(|x| x.contains("first-time")));
}

// ── Eligibility: income + price gates ─────────────────────────────────────────

#[test]
fn test_mcc_income_over_limit_fails() {
    // TX 3+ limit is $126,500; use $130,000
    let d = store()
        .mcc_evaluate(
            &input("TX", true, false, false, 4, 13_000_000, 30_000_000),
            2025,
        )
        .unwrap()
        .unwrap();
    assert!(!d.value.eligible);
    assert!(d.value.disqualifiers.iter().any(|x| x.contains("income")));
}

#[test]
fn test_mcc_targeted_area_raises_income_limit() {
    // $130,000 fails non-targeted (limit $126,500) but passes targeted (limit $154,000)
    let fail = store()
        .mcc_evaluate(
            &input("TX", true, false, false, 4, 13_000_000, 30_000_000),
            2025,
        )
        .unwrap()
        .unwrap();
    let pass = store()
        .mcc_evaluate(
            &input("TX", true, false, true, 4, 13_000_000, 30_000_000),
            2025,
        )
        .unwrap()
        .unwrap();
    assert!(!fail.value.eligible);
    assert!(pass.value.eligible, "targeted area raises income ceiling");
}

#[test]
fn test_mcc_price_over_limit_fails() {
    // TX non-targeted price limit $417,000; use $450,000
    let d = store()
        .mcc_evaluate(
            &input("TX", true, false, false, 2, 8_000_000, 45_000_000),
            2025,
        )
        .unwrap()
        .unwrap();
    assert!(!d.value.eligible);
    assert!(d.value.disqualifiers.iter().any(|x| x.contains("price")));
}

#[test]
fn test_mcc_multiple_disqualifiers_all_recorded() {
    // Over income AND over price AND not FTHB
    let d = store()
        .mcc_evaluate(
            &input("TX", false, false, false, 4, 20_000_000, 60_000_000),
            2025,
        )
        .unwrap()
        .unwrap();
    assert!(!d.value.eligible);
    assert_eq!(
        d.value.disqualifiers.len(),
        3,
        "all three failures recorded"
    );
}

// ── Credit estimate + cap ─────────────────────────────────────────────────────

#[test]
fn test_mcc_credit_estimate_under_cap() {
    // CA 20% rate, no cap. $8,000 interest × 20% = $1,600
    let prog = store().mcc_program("CA", 2025).unwrap().unwrap();
    let credit = estimate_annual_credit(prog, Cents(800_000));
    assert_eq!(credit.value, Cents(160_000));
}

#[test]
fn test_mcc_credit_estimate_hits_2000_cap() {
    // TX 40% rate, $2,000 cap. $12,000 interest × 40% = $4,800 → capped to $2,000
    let prog = store().mcc_program("TX", 2025).unwrap().unwrap();
    let credit = estimate_annual_credit(prog, Cents(1_200_000));
    assert_eq!(credit.value, Cents(200_000), "credit capped at $2,000");
    let cap_step = credit
        .steps
        .iter()
        .find(|s| s.rule == "apply_annual_cap")
        .unwrap();
    assert!(cap_step.outcome.contains("capped"));
}

#[test]
fn test_mcc_credit_estimate_preserves_catalog_provenance() {
    let prog = store().mcc_program("TX", 2025).unwrap().unwrap();
    let credit = estimate_annual_credit(prog, Cents(500_000));
    // Provenance from the catalog lookup survives the map() into a credit estimate
    assert_eq!(credit.provenance.record_id, "mcc_tx_tdhca");
    assert_eq!(credit.provenance.dataset, "mcc_catalog");
}

#[test]
fn test_mcc_explain_renders_full_trail() {
    let d = store()
        .mcc_evaluate(
            &input("TX", true, false, false, 3, 9_000_000, 35_000_000),
            2025,
        )
        .unwrap()
        .unwrap();
    let text = d.explain();
    assert!(text.contains("Source:"));
    assert!(text.contains("mcc_tx_tdhca"));
    assert!(text.contains("Derivation:"));
    assert!(text.contains("final_determination"));
}
