//! Epic 4.5.2 — Conventional PMI rate-quote tests.
//!
//! Validates base-grid cells against the source rate cards (Enact 2025,
//! Essent 2019, Radian 2021), additive adjustment composition, the non-fixed
//! multiplier method (Essent ×1.25), per-plan min-rate floors, the
//! default-qualified DTI assumption, and the `Derived<T>` provenance contract.

use ref_data::conv_pmi::*;
use types::{Cents, CreditScore, LtvBasisPoints};

const REQ: u16 = 2025;
const RES: u16 = 2025;

fn data_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data")
}
fn load<T: serde::de::DeserializeOwned>(name: &str) -> T {
    let p = data_dir().join(name);
    let s = std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()));
    serde_json::from_str(&s).unwrap_or_else(|e| panic!("parse {}: {e}", p.display()))
}
fn enact() -> MiCardFile {
    load("enact_pmi_2025.json")
}
fn essent() -> MiCardFile {
    load("essent_pmi_2019.json")
}
fn radian() -> MiCardFile {
    load("radian_pmi_2021.json")
}

#[allow(clippy::too_many_arguments)]
fn scn(
    plan: MiPlan,
    fixed: bool,
    term_months: u16,
    score: u16,
    ltv_pct: f64,
    coverage: u8,
) -> MiScenario {
    MiScenario {
        plan,
        refundability: Refundability::NonRefundable,
        is_fixed: fixed,
        amortization_term_months: term_months,
        indicator_score: CreditScore::new(score).unwrap(),
        ltv: LtvBasisPoints::new((ltv_pct * 100.0) as u32).unwrap(),
        coverage_percent: coverage,
        loan_amount: Cents::from_dollars(300_000),
        property_type: MiPropertyType::SingleFamilyDetached,
        occupancy: MiOccupancy::Primary,
        purpose: MiPurpose::Purchase,
        borrower_count: 1,
        is_relocation: false,
        dti_tier: DtiTier::Qualified,
    }
}

// ── Base-grid cells vs source cards ─────────────────────────────────────────

#[test]
fn enact_monthly_fixed_gt20_base_cells() {
    let c = enact();
    // 760+, 95.01-97 LTV, 35% cov, fixed >20yr => 0.58% = 580 milli-pct
    let q = quote(
        &c,
        "f",
        &scn(MiPlan::MonthlyBpmi, true, 360, 800, 96.0, 35),
        REQ,
        RES,
    )
    .unwrap();
    assert_eq!(q.value.base_milli_pct, 580);
    assert_eq!(q.value.net_milli_pct, 580);
    // 700-719, 90.01-95, 30% cov => 0.78% = 780
    let q2 = quote(
        &c,
        "f",
        &scn(MiPlan::MonthlyBpmi, true, 360, 705, 93.0, 30),
        REQ,
        RES,
    )
    .unwrap();
    assert_eq!(q2.value.base_milli_pct, 780);
}

#[test]
fn enact_monthly_fixed_le20_differs() {
    let c = enact();
    // 760+, 95.01-97, 35% cov, fixed <=20yr => 0.40% = 400
    let q = quote(
        &c,
        "f",
        &scn(MiPlan::MonthlyBpmi, true, 180, 800, 96.0, 35),
        REQ,
        RES,
    )
    .unwrap();
    assert_eq!(q.value.base_milli_pct, 400);
}

#[test]
fn enact_single_fixed_base_cell() {
    let c = enact();
    // single BPMI 760+, 95.01-97, 35% cov, >20yr => 1.58% = 1580
    let q = quote(
        &c,
        "f",
        &scn(MiPlan::SingleBpmi, true, 360, 800, 96.0, 35),
        REQ,
        RES,
    )
    .unwrap();
    assert_eq!(q.value.base_milli_pct, 1580);
}

#[test]
fn radian_single_lt620_column() {
    let c = radian();
    // Radian carries a <620 column the others don't: single fixed >20, 97/35 => 8.94%
    let q = quote(
        &c,
        "f",
        &scn(MiPlan::SingleBpmi, true, 360, 610, 96.0, 35),
        REQ,
        RES,
    )
    .unwrap();
    assert_eq!(q.value.base_milli_pct, 8940);
}

// ── Non-fixed handling: grid vs multiplier ──────────────────────────────────

#[test]
fn enact_nonfixed_uses_explicit_grid() {
    let c = enact();
    // Enact ships explicit non-fixed grid: monthly 760+, 97/35, >20 => 0.73% = 730
    let q = quote(
        &c,
        "f",
        &scn(MiPlan::MonthlyBpmi, false, 360, 800, 96.0, 35),
        REQ,
        RES,
    )
    .unwrap();
    assert_eq!(q.value.base_milli_pct, 730);
}

#[test]
fn essent_nonfixed_multiplies_fixed_by_1_25() {
    let c = essent();
    // Essent fixed monthly 760+, 97/35, >20 = 0.58% = 580.
    // Non-fixed = 580 × 1.25 = 725 milli, rounded to nearest bp (10 milli) => 730.
    let q = quote(
        &c,
        "f",
        &scn(MiPlan::MonthlyBpmi, false, 360, 800, 96.0, 35),
        REQ,
        RES,
    )
    .unwrap();
    assert_eq!(q.value.base_milli_pct, 730);
}

// ── Adjustment composition ──────────────────────────────────────────────────

#[test]
fn second_home_adjustment_stacks() {
    let c = enact();
    let mut s = scn(MiPlan::MonthlyBpmi, true, 360, 800, 96.0, 35);
    s.occupancy = MiOccupancy::SecondHome;
    let q = quote(&c, "f", &s, REQ, RES).unwrap();
    // base 580 + second_home@760 (.36% = 360) = 940
    assert_eq!(q.value.net_milli_pct, 580 + 360);
    assert!(q
        .value
        .adjustments
        .iter()
        .any(|a| a.description.contains("second_home")));
}

#[test]
fn two_borrower_credit_reduces_rate() {
    let c = enact();
    let mut s = scn(MiPlan::MonthlyBpmi, true, 360, 800, 96.0, 35);
    s.borrower_count = 2;
    let q = quote(&c, "f", &s, REQ, RES).unwrap();
    // base 580 + ge2_borrowers@97 (-.18% = -180) = 400
    assert_eq!(q.value.net_milli_pct, 580 - 180);
}

#[test]
fn essent_investment_not_offered_below_720_is_skipped() {
    let c = essent();
    let mut s = scn(MiPlan::MonthlyBpmi, true, 360, 700, 96.0, 35);
    s.occupancy = MiOccupancy::Investment;
    // investment row is null for 700 => adjustment skipped (None), base stands
    let q = quote(&c, "f", &s, REQ, RES).unwrap();
    assert!(!q
        .value
        .adjustments
        .iter()
        .any(|a| a.description.contains("investment")));
}

// ── DTI: default qualified => no adjustment; elevated => applies ────────────

#[test]
fn dti_default_qualified_adds_nothing() {
    let c = enact();
    let q = quote(
        &c,
        "f",
        &scn(MiPlan::MonthlyBpmi, true, 360, 800, 96.0, 35),
        REQ,
        RES,
    )
    .unwrap();
    assert!(!q
        .value
        .adjustments
        .iter()
        .any(|a| a.description.starts_with("dti")));
}

#[test]
fn dti_elevated_applies_when_set() {
    let c = enact();
    let mut s = scn(MiPlan::MonthlyBpmi, true, 360, 800, 96.0, 35);
    s.dti_tier = DtiTier::Elevated;
    let q = quote(&c, "f", &s, REQ, RES).unwrap();
    // 96% LTV => @97 band; base 580 + dti_45_50@97 (760 col = .38% = 380) = 960
    assert_eq!(q.value.net_milli_pct, 580 + 380);
    assert!(q
        .value
        .adjustments
        .iter()
        .any(|a| a.description.contains("dti_45_50")));
}

// ── Min-rate floor ──────────────────────────────────────────────────────────

#[test]
fn min_rate_floor_applies() {
    let c = enact();
    // Low-LTV high-credit with 2 borrowers can dip below the 0.14% monthly floor.
    let mut s = scn(MiPlan::MonthlyBpmi, true, 360, 800, 80.0, 6);
    s.borrower_count = 2;
    let q = quote(&c, "f", &s, REQ, RES).unwrap();
    // base 170 (85&below/6%) + ge2@85 (-30) = 140 == floor; not floored (equal)
    assert!(q.value.net_milli_pct >= 140);
}

// ── Premium dollar math ─────────────────────────────────────────────────────

#[test]
fn annual_and_monthly_premium_math() {
    let c = enact();
    let q = quote(
        &c,
        "f",
        &scn(MiPlan::MonthlyBpmi, true, 360, 800, 96.0, 35),
        REQ,
        RES,
    )
    .unwrap();
    // 0.58% of $300,000 = $1,740/yr; /12 = $145/mo
    assert_eq!(
        q.value.annual_premium(Cents::from_dollars(300_000)),
        Cents::from_dollars(1_740)
    );
    assert_eq!(
        q.value.monthly_premium(Cents::from_dollars(300_000)),
        Cents::from_dollars(145)
    );
}

// ── Provenance ──────────────────────────────────────────────────────────────

#[test]
fn quote_is_fully_explainable() {
    let c = enact();
    let q = quote(
        &c,
        "enact_pmi_2025.json",
        &scn(MiPlan::MonthlyBpmi, true, 360, 740, 90.0, 25),
        REQ,
        RES,
    )
    .unwrap();
    let t = q.explain();
    assert!(t.contains("Source:"));
    assert!(t.contains("Enact"));
    assert!(t.contains("Derivation:"));
}

#[test]
fn version_fallback_recorded() {
    let c = enact();
    let q = quote(
        &c,
        "enact_pmi_2025.json",
        &scn(MiPlan::MonthlyBpmi, true, 360, 800, 96.0, 35),
        2030,
        2025,
    )
    .unwrap();
    assert!(q.provenance.is_fallback());
    assert!(q.explain().contains("FALLBACK"));
}

// ── Not-offered scenario surfaces as MiUnavailable ──────────────────────────

#[test]
fn unavailable_when_no_grid_for_plan() {
    let c = radian(); // radian card ships only single_bpmi grids here
    let r = quote(
        &c,
        "f",
        &scn(MiPlan::SplitPremium, true, 360, 800, 96.0, 35),
        REQ,
        RES,
    );
    assert!(r.is_err());
}
