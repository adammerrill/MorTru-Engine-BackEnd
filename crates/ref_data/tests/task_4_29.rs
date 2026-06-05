//! Task 4.29 — LLPA / credit-fee pricing tests.
//!
//! Validates GSE grid cells against the source documents (Freddie Exhibit 19
//! Bulletin 2026-03, Fannie LLPA Matrix), the additive special-attribute
//! composition, lender-overlay layering, affordable caps, the max-net floor,
//! eligibility rejection, and the `Derived<T>` provenance contract.

use ref_data::llpa::*;
use types::{Cents, CreditScore, LtvBasisPoints};

const REQ: u16 = 2026;
const RES: u16 = 2026;

fn data_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data")
}
fn load<T: serde::de::DeserializeOwned>(name: &str) -> T {
    let p = data_dir().join(name);
    let s = std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()));
    serde_json::from_str(&s).unwrap_or_else(|e| panic!("parse {}: {e}", p.display()))
}
fn freddie() -> LlpaDatasetFile {
    load("freddie_credit_fees_2026.json")
}
fn fannie() -> LlpaDatasetFile {
    load("fannie_llpa_2026.json")
}
fn uwm() -> LenderOverlayFile {
    load("uwm_overlay_2026.json")
}

fn scenario(score: u16, ltv_pct: f64, purpose: LlpaPurpose) -> LlpaScenario {
    LlpaScenario {
        purpose,
        occupancy: LlpaOccupancy::Primary,
        property_type: LlpaPropertyType::Detached,
        indicator_score: CreditScore::new(score).unwrap(),
        ltv: LtvBasisPoints::new((ltv_pct * 100.0) as u32).unwrap(),
        loan_amount: Cents::from_dollars(300_000),
        is_arm: false,
        is_high_balance: false,
        is_super_conforming: false,
        has_subordinate_financing: false,
        heloc_balance_at_closing: Cents::ZERO,
        has_affordable_second: false,
        state: "FL".to_owned(),
        ami_percent: None,
        is_first_time_homebuyer: false,
        is_high_cost_area: false,
        is_duty_to_serve: false,
        is_home_ready_or_possible: false,
    }
}

// ── GSE base-grid cells, verified against the source documents ──────────────

#[test]
fn freddie_purchase_base_cells_match_exhibit_19() {
    let g = freddie();
    // Exhibit 19 p.E19-4: >=780, >75&<=80 => 0.375% = 38 bps (37.5 → round half-up)
    let p = price(
        &g,
        "freddie_credit_fees_2026.json",
        None,
        &scenario(800, 78.0, LlpaPurpose::Purchase),
        REQ,
        RES,
    )
    .unwrap();
    assert_eq!(p.value.gse_subtotal_bps, 38);
    // <640, >70&<=75 => 2.125% = 213 bps (212.5 → round half-up)
    let p2 = price(
        &g,
        "freddie_credit_fees_2026.json",
        None,
        &scenario(600, 73.0, LlpaPurpose::Purchase),
        REQ,
        RES,
    )
    .unwrap();
    assert_eq!(p2.value.gse_subtotal_bps, 213);
}

#[test]
fn freddie_cashout_not_eligible_above_80() {
    let g = freddie();
    let r = price(
        &g,
        "f.json",
        None,
        &scenario(760, 85.0, LlpaPurpose::CashOutRefi),
        REQ,
        RES,
    );
    assert!(r.is_err());
    let reason = r.unwrap_err().reason;
    assert!(
        reason.contains("cutoff") || reason.contains("Not Eligible"),
        "unexpected ineligibility reason: {reason}"
    );
}

#[test]
fn freddie_cashout_base_cell() {
    let g = freddie();
    // p.E19-8: >=700&<720, >75&<=80 => 3.250% = 325 bps
    let p = price(
        &g,
        "f.json",
        None,
        &scenario(705, 78.0, LlpaPurpose::CashOutRefi),
        REQ,
        RES,
    )
    .unwrap();
    assert_eq!(p.value.gse_subtotal_bps, 325);
}

#[test]
fn fannie_purchase_base_cell() {
    let g = fannie();
    // FNMA >=740&<760, >75&<=80 => 0.875% = 88 bps (87.5 → round half-up)
    let p = price(
        &g,
        "f.json",
        None,
        &scenario(745, 79.0, LlpaPurpose::Purchase),
        REQ,
        RES,
    )
    .unwrap();
    assert_eq!(p.value.gse_subtotal_bps, 88);
}

// ── Special-attribute composition ───────────────────────────────────────────

#[test]
fn investment_attribute_stacks_on_base() {
    let g = freddie();
    let mut s = scenario(760, 79.0, LlpaPurpose::Purchase);
    s.occupancy = LlpaOccupancy::Investment;
    let p = price(&g, "f.json", None, &s, REQ, RES).unwrap();
    // base(760,75_80=0.625%=63) + investment(75_80=3.375%=338) = 401
    assert_eq!(p.value.gse_subtotal_bps, 63 + 338);
    assert!(p
        .value
        .adjustments
        .iter()
        .any(|a| a.description.contains("investment")));
}

#[test]
fn detached_condo_exempt_from_condo_fee() {
    let g = freddie();
    let mut base = scenario(760, 78.0, LlpaPurpose::Purchase);
    base.property_type = LlpaPropertyType::DetachedCondo;
    let mut attached = base.clone();
    attached.property_type = LlpaPropertyType::AttachedCondo;
    let rb = price(&g, "f.json", None, &base, REQ, RES).unwrap();
    let ra = price(&g, "f.json", None, &attached, REQ, RES).unwrap();
    // attached condo adds 0.750% = 75 bps at 75_80; detached adds nothing
    assert_eq!(ra.value.gse_subtotal_bps - rb.value.gse_subtotal_bps, 75);
}

#[test]
fn secondary_financing_skipped_when_heloc_zero() {
    let g = freddie();
    let mut s = scenario(760, 78.0, LlpaPurpose::Purchase);
    s.has_subordinate_financing = true;
    s.heloc_balance_at_closing = Cents::ZERO;
    let p = price(&g, "f.json", None, &s, REQ, RES).unwrap();
    assert!(!p
        .value
        .adjustments
        .iter()
        .any(|a| a.description.contains("secondary")));
}

// ── Lender overlay composition ──────────────────────────────────────────────

#[test]
fn refi_incentive_and_trac_layer_on_freddie() {
    let g = freddie();
    let ov = uwm();
    let s = scenario(780, 78.0, LlpaPurpose::NoCashOutRefi); // FL, $300k
    let p = price(
        &g,
        "f.json",
        Some(("uwm_overlay_2026.json", &ov)),
        &s,
        REQ,
        RES,
    )
    .unwrap();
    // gse no-cashout >=780,75_80 => 0.500% = 50 bps
    assert_eq!(p.value.gse_subtotal_bps, 50);
    // refi -75 + TRAC band ($300k → -40) = -115
    assert_eq!(p.value.lender_subtotal_bps, -75 - 40);
    assert_eq!(p.value.total_bps, 50 - 75 - 40);
}

#[test]
fn texas_state_adjuster_applies_on_fixed_30yr() {
    let g = fannie();
    let ov = uwm();
    let mut s = scenario(760, 78.0, LlpaPurpose::Purchase);
    s.state = "TX".to_owned();
    let p = price(&g, "f.json", Some(("o.json", &ov)), &s, REQ, RES).unwrap();
    assert!(p
        .value
        .adjustments
        .iter()
        .any(|a| a.description.contains("TX") && a.bps == -14));
}

#[test]
fn max_net_floor_clamps_deep_credit() {
    let g = fannie();
    let ov = uwm();
    // small loan ($120k → TRAC -60) + refi -75 + NY -44 on a 0-fee base
    let mut s = scenario(800, 50.0, LlpaPurpose::NoCashOutRefi);
    s.loan_amount = Cents::from_dollars(120_000);
    s.state = "NY".to_owned();
    let p = price(&g, "f.json", Some(("o.json", &ov)), &s, REQ, RES).unwrap();
    assert!(
        p.value.total_bps >= -400,
        "floor at -400, got {}",
        p.value.total_bps
    );
}

#[test]
fn affordable_cap_collapses_positive_fees_keeps_credits() {
    let g = fannie();
    let ov = uwm();
    let mut s = scenario(660, 95.0, LlpaPurpose::Purchase);
    s.is_home_ready_or_possible = true;
    s.ami_percent = Some(70);
    s.loan_amount = Cents::from_dollars(1_500_000); // >$1M TRAC = 0, FL = 0
    let p = price(&g, "f.json", Some(("o.json", &ov)), &s, REQ, RES).unwrap();
    assert!(p.value.capped);
    assert_eq!(p.value.total_bps, 0);
}

// ── Eligibility (tighten-only overlay) ──────────────────────────────────────

#[test]
fn uwm_investment_max_ltv_80_rejects_90() {
    let g = freddie();
    let ov = uwm();
    let mut s = scenario(760, 90.0, LlpaPurpose::Purchase);
    s.occupancy = LlpaOccupancy::Investment;
    let r = price(&g, "f.json", Some(("o.json", &ov)), &s, REQ, RES);
    assert!(r.is_err());
    assert!(r.unwrap_err().reason.contains("investment_property"));
}

// ── Provenance contract (the family invariant) ──────────────────────────────

#[test]
fn pricing_is_fully_explainable() {
    let g = freddie();
    let ov = uwm();
    let s = scenario(740, 75.0, LlpaPurpose::Purchase);
    let p = price(
        &g,
        "freddie_credit_fees_2026.json",
        Some(("uwm_overlay_2026.json", &ov)),
        &s,
        REQ,
        RES,
    )
    .unwrap();
    let trail = p.explain();
    assert!(trail.contains("Source:"));
    assert!(trail.contains("Exhibit 19"));
    assert!(trail.contains("Derivation:"));
}

#[test]
fn year_fallback_recorded_in_provenance() {
    let g = freddie();
    let s = scenario(780, 78.0, LlpaPurpose::Purchase);
    // requested 2030, resolved 2026 → fallback flagged
    let p = price(&g, "freddie_credit_fees_2026.json", None, &s, 2030, 2026).unwrap();
    assert!(p.provenance.is_fallback());
    assert!(p.explain().contains("FALLBACK"));
}

#[test]
fn cost_for_converts_bps_to_dollars() {
    let g = freddie();
    let s = scenario(800, 78.0, LlpaPurpose::Purchase); // 38 bps on $300k
    let p = price(&g, "f.json", None, &s, REQ, RES).unwrap();
    // 38 bps × $300,000 / 10_000 = $1,140
    assert_eq!(p.value.cost_for(s.loan_amount), Cents::from_dollars(1_140));
}
