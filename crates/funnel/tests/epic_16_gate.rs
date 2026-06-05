//! Epic 16 / Task T7 — capstone gate.
//!
//! Asserts the funnel is feature-complete and internally consistent: the full
//! 12-step `WizardStep` sequence drives the funnel through all three stages,
//! counts narrow monotonically with provenance at every unlocked stage, and
//! the public contract surface is wired. Mirrors `epic_4_gate` / `epic_1_gate`.

use funnel::*;
use types::{
    Cents, CreditScore, FipsCode, GoalMask, LoanPurpose, Occupancy, ProgramCode, PropertyType,
    StateCode, TermMonths,
};

fn borrower(score: u16, income: i64) -> BorrowerInput {
    BorrowerInput {
        occupancy: Occupancy::PrimaryResidence,
        credit_scores: vec![CreditScore::new(score).unwrap()],
        va: None,
        annual_income: Cents::from_dollars(income),
    }
}

/// A fully-completed submission covering every one of the 12 steps + gap fields.
fn fully_completed() -> PartialAnalysisInput {
    let mut p = PartialAnalysisInput {
        borrower_count: Some(2),
        property_use: Some(Occupancy::PrimaryResidence),
        preferred_term: Some(TermMonths(360)),
        monthly_payment_budget: Some(Cents::from_dollars(3_000)),
        upfront_cash_budget: Some(Cents::from_dollars(50_000)),
        hold_horizon_months: Some(84),
        buyer_agent_commission: Some(Cents::from_dollars(9_000)),
        property_state: StateCode::from_fips(48),
        property_county_fips: FipsCode::new(48, 453).ok(),
        purchase_price: Some(Cents::from_dollars(450_000)),
        property_type: Some(PropertyType::SingleFamilyDetached),
        loan_purpose: Some(LoanPurpose::Purchase),
        program: Some(ProgramCode::Conventional),
        goals: Some(GoalMask::DEFAULT_CONSUMER),
        ..Default::default()
    };
    p.borrowers.push(borrower(760, 110_000));
    p.borrowers.push(borrower(720, 80_000));
    p.seller_credits.concessions_requested = Some(Cents::from_dollars(6_000));
    p.seller_credits.pays_title = true;
    p
}

// ── Gate 1: the 12-step sequence is canonical and stable ────────────────────

#[test]
fn t16_1_wizard_step_sequence_is_twelve_in_order() {
    assert_eq!(WizardStep::ALL.len(), 12);
    // Discriminants must be the stable wire order 1..=12.
    let discriminants: Vec<u8> = WizardStep::ALL.iter().map(|s| *s as u8).collect();
    assert_eq!(discriminants, (1..=12).collect::<Vec<u8>>());
}

// ── Gate 2: a fully-completed input is valid and reaches the final step ─────

#[test]
fn t16_2_full_input_is_valid_and_complete() {
    let p = fully_completed();
    assert!(is_valid(&p), "{:?}", validate(&p));
    assert_eq!(p.furthest_step(), Some(WizardStep::SellerPaysSurvey));
    // All 12 steps register complete.
    assert_eq!(valid_completed_steps(&p).len(), 12);
}

// ── Gate 3: the funnel drives through all three stages ──────────────────────

#[test]
fn t16_3_full_flow_unlocks_all_three_counts() {
    let f = StubFunnel::default();
    let r = step(&fully_completed(), &f);
    assert_eq!(r.stage, FunnelStage::InBudget);
    assert!(r.eligible.is_some() && r.qualified.is_some() && r.in_budget.is_some());
}

// ── Gate 4: counts narrow monotonically with provenance ─────────────────────

#[test]
fn t16_4_counts_monotonic_with_provenance() {
    let f = StubFunnel::default();
    let r = step(&fully_completed(), &f);
    assert!(r.is_monotonic());

    let e = r.eligible.expect("eligible");
    let q = r.qualified.expect("qualified");
    let b = r.in_budget.expect("in_budget");
    assert!(e.value >= q.value && q.value >= b.value);
    assert!(
        e.value > 0,
        "a complete conventional purchase yields >0 eligible"
    );

    // Every count carries a citation + derivation trail.
    for d in [&e, &q, &b] {
        let text = d.explain();
        assert!(text.contains("Source:") && text.contains("Derivation:"));
    }
}

// ── Gate 5: stepwise progression — counts appear in order, never early ──────

#[test]
fn t16_5_stepwise_progression_holds() {
    let f = StubFunnel::default();
    let mut p = PartialAnalysisInput {
        borrower_count: Some(1),
        property_use: Some(Occupancy::PrimaryResidence),
        ..Default::default()
    };

    // Step 1–2: no counts yet.
    assert!(step(&p, &f).eligible.is_none());

    // Step 3: eligible unlocks.
    p.borrowers.push(borrower(740, 90_000));
    let r3 = step(&p, &f);
    assert!(r3.eligible.is_some() && r3.qualified.is_none());

    // Step 5: qualified unlocks (term optional, monthly budget required).
    p.monthly_payment_budget = Some(Cents::from_dollars(2_800));
    let r5 = step(&p, &f);
    assert!(r5.qualified.is_some() && r5.in_budget.is_none());

    // Step 6: in_budget unlocks.
    p.upfront_cash_budget = Some(Cents::from_dollars(35_000));
    let r6 = step(&p, &f);
    assert!(r6.in_budget.is_some());
    assert!(r6.is_monotonic());
}

// ── Gate 6: the contract round-trips through JSON (web boundary) ────────────

#[test]
fn t16_6_contract_json_roundtrips() {
    let p = fully_completed();
    let json = serde_json::to_string(&p).expect("serialize");
    let back: PartialAnalysisInput = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(p, back);
    // And the funnel produces identical counts from the round-tripped input.
    let f = StubFunnel::default();
    let a = step(&p, &f);
    let b = step(&back, &f);
    assert_eq!(a.eligible.unwrap().value, b.eligible.unwrap().value);
}
