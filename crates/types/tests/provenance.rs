//! Coverage for `provenance.rs` — the `Derived<T>` / `Provenance` /
//! `DerivationStep` foundation. Previously exercised only transitively via
//! `ref_data`; this gives `types` its own direct coverage of every public item.

use types::{DerivationStep, Derived, Provenance};

fn sample_provenance(req: u16, res: u16) -> Provenance {
    Provenance {
        dataset: "mcc_catalog".to_string(),
        source_file: "mcc_catalog_2025.json".to_string(),
        source_citation: "TDHCA Texas MCC Program Guidelines (2025)".to_string(),
        effective_date: "2025-01-01".to_string(),
        record_id: "mcc_tx_tdhca".to_string(),
        requested_version: req,
        resolved_version: res,
    }
}

#[test]
fn provenance_is_fallback_true_when_versions_differ() {
    let p = sample_provenance(2026, 2025);
    assert!(p.is_fallback());
}

#[test]
fn provenance_is_fallback_false_when_versions_match() {
    let p = sample_provenance(2025, 2025);
    assert!(!p.is_fallback());
}

#[test]
fn derivation_step_new_sets_all_fields() {
    let s = DerivationStep::new("rule_x", "input_a=1", "satisfied");
    assert_eq!(s.rule, "rule_x");
    assert_eq!(s.inputs, "input_a=1");
    assert_eq!(s.outcome, "satisfied");
}

#[test]
fn derived_new_holds_value_and_provenance() {
    let d = Derived::new(42u32, sample_provenance(2025, 2025));
    assert_eq!(d.value, 42);
    assert_eq!(d.provenance.dataset, "mcc_catalog");
    assert!(d.steps.is_empty());
}

#[test]
fn derived_with_step_appends_and_chains() {
    let d = Derived::new(10u32, sample_provenance(2025, 2025))
        .with_step("step1", "in1", "out1")
        .with_step("step2", "in2", "out2");
    assert_eq!(d.steps.len(), 2);
    assert_eq!(d.steps[0].rule, "step1");
    assert_eq!(d.steps[1].outcome, "out2");
}

#[test]
fn derived_push_step_mutates_in_place() {
    let mut d = Derived::new(7u32, sample_provenance(2025, 2025));
    d.push_step("only_step", "x=1", "done");
    assert_eq!(d.steps.len(), 1);
    assert_eq!(d.steps[0].inputs, "x=1");
}

#[test]
fn derived_map_transforms_value_preserving_trail() {
    let d = Derived::new(5u32, sample_provenance(2025, 2025)).with_step("s", "i", "o");
    let mapped = d.map(|v| v * 2);
    assert_eq!(mapped.value, 10);
    assert_eq!(mapped.steps.len(), 1, "steps preserved across map");
    assert_eq!(mapped.provenance.record_id, "mcc_tx_tdhca");
}

#[test]
fn explain_includes_source_and_derivation_when_fallback() {
    let d = Derived::new(99u32, sample_provenance(2030, 2025)).with_step("check", "in", "out");
    let text = d.explain();
    assert!(text.contains("Source:"), "explain shows source: {text}");
    assert!(text.contains("mcc_catalog"));
    assert!(text.contains("FALLBACK"), "fallback flagged: {text}");
    assert!(text.contains("Derivation:"), "shows steps: {text}");
}

#[test]
fn explain_without_steps_still_renders_source() {
    let d = Derived::new(1u32, sample_provenance(2025, 2025));
    let text = d.explain();
    assert!(text.contains("Source:"));
    assert!(!text.contains("FALLBACK"));
}
