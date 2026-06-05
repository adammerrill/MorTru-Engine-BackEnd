//! Epic 11 tests — enumeration + month-granular term expansion + gate.

use scenarios::*;
use types::{BalanceType, LoanProduct, ProgramCode, TermBand, TermMonths, Tier};

// ── T11.1/T11.3 product mapping ─────────────────────────────────────────────

#[test]
fn conv_term_maps_to_correct_product() {
    assert_eq!(
        product_for(ProgramCode::Conventional, TermMonths(360)),
        Some(LoanProduct::FixedConv21To30)
    );
    assert_eq!(
        product_for(ProgramCode::Conventional, TermMonths(180)),
        Some(LoanProduct::FixedConv11To15)
    );
}

#[test]
fn fha_and_va_route_to_govt_products() {
    assert_eq!(
        product_for(ProgramCode::Fha, TermMonths(360)),
        Some(LoanProduct::FixedFha16To30)
    );
    assert_eq!(
        product_for(ProgramCode::Va, TermMonths(120)),
        Some(LoanProduct::FixedVa8To15)
    );
}

#[test]
fn usda_only_360() {
    assert_eq!(
        product_for(ProgramCode::Usda, TermMonths(360)),
        Some(LoanProduct::FixedUsda30)
    );
    assert_eq!(product_for(ProgramCode::Usda, TermMonths(240)), None);
}

#[test]
fn out_of_range_term_has_no_product() {
    assert_eq!(product_for(ProgramCode::Conventional, TermMonths(60)), None);
}

// ── T11.4 month-granular expansion (the critical task) ──────────────────────

#[test]
fn conv_expands_every_month_96_to_360() {
    let scenarios = enumerate_program(ProgramCode::Conventional);
    // 96..=360 inclusive = 265 distinct terms (conforming/standard/no-MI baseline).
    let terms: std::collections::BTreeSet<u16> =
        scenarios.iter().map(|s| s.term.0).collect();
    assert_eq!(terms.len(), 265, "expected every month 96..=360");
    assert!(terms.contains(&96) && terms.contains(&360));
    // A non-boundary term must be present (proves granularity, not just bands).
    assert!(terms.contains(&217), "non-boundary month 217 must enumerate");
}

#[test]
fn usda_expands_to_single_term() {
    let scenarios = enumerate_program(ProgramCode::Usda);
    assert_eq!(scenarios.len(), 1);
    assert_eq!(scenarios[0].term, TermMonths(360));
}

#[test]
fn fha_expands_every_month_in_govt_range() {
    let scenarios = enumerate_program(ProgramCode::Fha);
    let terms: std::collections::BTreeSet<u16> = scenarios.iter().map(|s| s.term.0).collect();
    // Govt bands cover 96..=360 too (8To15 + 16To30 contiguous) = 265.
    assert_eq!(terms.len(), 265);
}

// ── T11.2/T11.3 cartesian product ───────────────────────────────────────────

#[test]
fn axes_multiply_across_dimensions() {
    let mut axes = EnumerationAxes::for_programs(vec![ProgramCode::Conventional]);
    axes.balance_types = vec![BalanceType::Conforming, BalanceType::HighBalance];
    axes.tiers = vec![Tier::Elite, Tier::Standard];
    axes.mi_options = vec![0, 1];
    // 265 terms × 2 balance × 2 tier × 2 mi = 2120.
    assert_eq!(axes.count(), 265 * 2 * 2 * 2);
}

#[test]
fn enumerate_is_lazy_and_matches_count() {
    let axes = EnumerationAxes::for_programs(vec![ProgramCode::Conventional]);
    let collected = axes.enumerate().count() as u64;
    assert_eq!(collected, axes.count());
}

#[test]
fn no_duplicate_scenarios() {
    let scenarios = enumerate_program(ProgramCode::Conventional);
    let set: std::collections::HashSet<_> = scenarios.iter().collect();
    assert_eq!(set.len(), scenarios.len(), "no duplicates");
}

#[test]
fn multi_program_axes_union_bands() {
    let axes = EnumerationAxes::for_programs(vec![ProgramCode::Conventional, ProgramCode::Usda]);
    // Conv 265 + USDA 1 = 266 (USDA band is 360-only; conv already covers 360,
    // but USDA maps to a different product, so it is a distinct scenario).
    assert_eq!(axes.count(), 266);
}

// ── Epic 11 gate ────────────────────────────────────────────────────────────

#[test]
fn epic_11_gate_every_enumerated_term_is_valid_for_its_program() {
    for program in [
        ProgramCode::Conventional,
        ProgramCode::Fha,
        ProgramCode::Va,
        ProgramCode::Usda,
    ] {
        for s in enumerate_program(program) {
            // Every scenario's term must map back to its product.
            assert_eq!(
                product_for(s.program, s.term),
                Some(s.product),
                "term {} invalid for {:?}",
                s.term.0,
                program
            );
            assert!((96..=360).contains(&s.term.0));
        }
    }
}

#[test]
fn epic_11_gate_band_month_count_is_inclusive() {
    assert_eq!(TermBand::Band8To10.month_count(), 25); // 96..=120
    assert_eq!(TermBand::Band21To30.month_count(), 120); // 241..=360
    assert_eq!(TermBand::Usda30Only.month_count(), 1);
}
