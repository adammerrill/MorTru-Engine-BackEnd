//! Task 4.26 — Seller concession / IPC caps (full agency coverage).

use ref_data::seller_concessions::{
    gnma_inherits_program, CapBasis, ConcessionCapInput, Occupancy,
};
use ref_data::{JsonFileStore, RefDataStore};
use types::Cents;

fn store() -> JsonFileStore {
    JsonFileStore::new(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data"))
}
fn input(program: &str, occ: Occupancy, ltv_bps: u32) -> ConcessionCapInput {
    ConcessionCapInput {
        program: program.to_owned(),
        occupancy: occ,
        ltv_bps,
    }
}

// ── Flat-cap agencies ─────────────────────────────────────────────────────────

#[test]
fn test_fha_cap_is_6pct() {
    let c = store()
        .seller_concession_cap(&input("fha", Occupancy::Primary, 9650), 2025)
        .unwrap()
        .unwrap();
    assert_eq!(c.value.cap_bps, 600);
    assert_eq!(c.value.basis, CapBasis::SalesPrice);
}

#[test]
fn test_usda_cap_is_6pct() {
    let c = store()
        .seller_concession_cap(&input("usda", Occupancy::Primary, 10000), 2025)
        .unwrap()
        .unwrap();
    assert_eq!(c.value.cap_bps, 600);
}

#[test]
fn test_va_cap_is_4pct_reasonable_value() {
    let c = store()
        .seller_concession_cap(&input("va", Occupancy::Primary, 10000), 2025)
        .unwrap()
        .unwrap();
    assert_eq!(c.value.cap_bps, 400);
    assert_eq!(c.value.basis, CapBasis::ReasonableValue);
    assert!(c.value.note.to_lowercase().contains("concession"));
}

#[test]
fn test_fha_flat_across_ltv() {
    let low = store()
        .seller_concession_cap(&input("fha", Occupancy::Primary, 5000), 2025)
        .unwrap()
        .unwrap();
    let high = store()
        .seller_concession_cap(&input("fha", Occupancy::Primary, 9650), 2025)
        .unwrap()
        .unwrap();
    assert_eq!(low.value.cap_bps, high.value.cap_bps);
}

// ── FNMA / FHLMC tiered ───────────────────────────────────────────────────────

#[test]
fn test_fnma_primary_low_ltv_9pct() {
    let c = store()
        .seller_concession_cap(&input("fnma", Occupancy::Primary, 7000), 2025)
        .unwrap()
        .unwrap();
    assert_eq!(c.value.cap_bps, 900, "<=75% LTV → 9%");
}

#[test]
fn test_fnma_primary_mid_ltv_6pct() {
    let c = store()
        .seller_concession_cap(&input("fnma", Occupancy::Primary, 8500), 2025)
        .unwrap()
        .unwrap();
    assert_eq!(c.value.cap_bps, 600, "75-90% LTV → 6%");
}

#[test]
fn test_fnma_primary_high_ltv_3pct() {
    let c = store()
        .seller_concession_cap(&input("fnma", Occupancy::Primary, 9650), 2025)
        .unwrap()
        .unwrap();
    assert_eq!(c.value.cap_bps, 300, ">90% LTV → 3%");
}

#[test]
fn test_fnma_investment_2pct_any_ltv() {
    let lo = store()
        .seller_concession_cap(&input("fnma", Occupancy::Investment, 7000), 2025)
        .unwrap()
        .unwrap();
    let hi = store()
        .seller_concession_cap(&input("fnma", Occupancy::Investment, 9500), 2025)
        .unwrap()
        .unwrap();
    assert_eq!(lo.value.cap_bps, 200);
    assert_eq!(hi.value.cap_bps, 200);
}

#[test]
fn test_fnma_second_home_tiered_like_primary() {
    let c = store()
        .seller_concession_cap(&input("fnma", Occupancy::SecondHome, 7000), 2025)
        .unwrap()
        .unwrap();
    assert_eq!(c.value.cap_bps, 900);
}

#[test]
fn test_fhlmc_matches_fnma_tiers() {
    for ltv in [7000u32, 8500, 9650] {
        let fnma = store()
            .seller_concession_cap(&input("fnma", Occupancy::Primary, ltv), 2025)
            .unwrap()
            .unwrap();
        let fhlmc = store()
            .seller_concession_cap(&input("fhlmc", Occupancy::Primary, ltv), 2025)
            .unwrap()
            .unwrap();
        assert_eq!(fnma.value.cap_bps, fhlmc.value.cap_bps, "ltv={ltv}");
    }
}

#[test]
fn test_fhlmc_cites_5501() {
    let c = store()
        .seller_concession_cap(&input("fhlmc", Occupancy::Primary, 7000), 2025)
        .unwrap()
        .unwrap();
    assert!(c.value.note.contains("5501.5"));
}

#[test]
fn test_ltv_tier_boundary_75pct_inclusive() {
    // exactly 75% (7500 bps) → still 9% tier (upper bound inclusive)
    let c = store()
        .seller_concession_cap(&input("fnma", Occupancy::Primary, 7500), 2025)
        .unwrap()
        .unwrap();
    assert_eq!(c.value.cap_bps, 900);
    // 75.01% → 6% tier
    let c2 = store()
        .seller_concession_cap(&input("fnma", Occupancy::Primary, 7501), 2025)
        .unwrap()
        .unwrap();
    assert_eq!(c2.value.cap_bps, 600);
}

// ── GNMA inheritance ──────────────────────────────────────────────────────────

#[test]
fn test_gnma_inherits_fha() {
    assert_eq!(gnma_inherits_program("fha"), Some("fha"));
}

#[test]
fn test_gnma_inherits_usda_aliases() {
    assert_eq!(gnma_inherits_program("usda"), Some("usda"));
    assert_eq!(gnma_inherits_program("rd"), Some("usda"));
}

#[test]
fn test_gnma_rejects_conventional() {
    assert_eq!(
        gnma_inherits_program("conventional"),
        None,
        "GNMA does not pool conventional"
    );
}

#[test]
fn test_gnma_inherited_cap_lookup() {
    // A GNMA-pooled VA loan inherits VA's 4% cap
    let prog = gnma_inherits_program("va").unwrap();
    let c = store()
        .seller_concession_cap(&input(prog, Occupancy::Primary, 10000), 2025)
        .unwrap()
        .unwrap();
    assert_eq!(c.value.cap_bps, 400);
}

// ── Evaluation: within / exceeds ──────────────────────────────────────────────

#[test]
fn test_evaluate_within_limit() {
    // FHA 6% of $400k = $24k max; propose $18k → within
    let d = store()
        .evaluate_seller_concession(
            &input("fha", Occupancy::Primary, 9650),
            Cents(40_000_000),
            Cents(1_800_000),
            2025,
        )
        .unwrap()
        .unwrap();
    assert!(d.value.within_limit);
    assert_eq!(d.value.max_allowed, Cents(2_400_000));
    assert_eq!(d.value.excess, Cents(0));
}

#[test]
fn test_evaluate_exceeds_limit_computes_excess() {
    // FNMA 95% LTV → 3% of $400k = $12k max; propose $20k → $8k excess
    let d = store()
        .evaluate_seller_concession(
            &input("fnma", Occupancy::Primary, 9500),
            Cents(40_000_000),
            Cents(2_000_000),
            2025,
        )
        .unwrap()
        .unwrap();
    assert!(!d.value.within_limit);
    assert_eq!(d.value.max_allowed, Cents(1_200_000));
    assert_eq!(
        d.value.excess,
        Cents(800_000),
        "excess becomes a sales concession"
    );
}

#[test]
fn test_evaluate_exactly_at_cap_is_within() {
    // VA 4% of $300k = $12k; propose exactly $12k
    let d = store()
        .evaluate_seller_concession(
            &input("va", Occupancy::Primary, 10000),
            Cents(30_000_000),
            Cents(1_200_000),
            2025,
        )
        .unwrap()
        .unwrap();
    assert!(d.value.within_limit);
    assert_eq!(d.value.excess, Cents(0));
}

#[test]
fn test_evaluate_records_excess_in_trail() {
    let d = store()
        .evaluate_seller_concession(
            &input("fnma", Occupancy::Investment, 8000),
            Cents(50_000_000),
            Cents(2_000_000),
            2025,
        )
        .unwrap()
        .unwrap();
    // investment 2% of $500k = $10k; propose $20k → $10k excess
    assert_eq!(d.value.excess, Cents(1_000_000));
    assert!(d
        .steps
        .iter()
        .any(|s| s.outcome.contains("sales concession")));
}

// ── Provenance / misc ─────────────────────────────────────────────────────────

#[test]
fn test_cap_carries_agency_citation() {
    let c = store()
        .seller_concession_cap(&input("fha", Occupancy::Primary, 9650), 2025)
        .unwrap()
        .unwrap();
    assert_eq!(c.provenance.dataset, "seller_concession_caps");
    assert!(c.provenance.source_citation.contains("4000.1"));
}

#[test]
fn test_unknown_program_returns_none() {
    assert!(store()
        .seller_concession_cap(&input("jumbo", Occupancy::Primary, 8000), 2025)
        .unwrap()
        .is_none());
}

#[test]
fn test_year_fallback() {
    let c = store()
        .seller_concession_cap(&input("fha", Occupancy::Primary, 9650), 2030)
        .unwrap()
        .unwrap();
    assert!(c.provenance.is_fallback());
    assert_eq!(c.provenance.resolved_version, 2025);
}

#[test]
fn test_explain_renders_trail() {
    let d = store()
        .evaluate_seller_concession(
            &input("fnma", Occupancy::Primary, 9500),
            Cents(40_000_000),
            Cents(2_000_000),
            2025,
        )
        .unwrap()
        .unwrap();
    let t = d.explain();
    assert!(t.contains("compute_max_allowed"));
    assert!(t.contains("compare_proposed"));
}
