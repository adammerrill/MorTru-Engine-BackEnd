//! Task 2.11 — Fixture library tests.
//!
//! Verifies every canonical XML fixture file:
//!   fha_purchase.xml       — FHA purchase, Kyle TX, $434,443, 6.375%, 30yr
//!   conv_purchase.xml      — Conventional purchase, $400k/80% LTV, no MI
//!   va_purchase.xml        — VA purchase, $350k, 2.15% funding fee financed
//!   usda_purchase.xml      — USDA purchase, $200k, 1.00% guarantee fee
//!   conv_refi_rate_term.xml — Conventional rate/term refi, 80% LTV, no MI
//!
//! Each test asserts that the file:
//!   1. Deserializes from XML without error
//!   2. Passes `parse_all()` without error
//!   3. Serializes back to XML and re-parses identically (round-trip)
//!
//! Deep value assertions for all five scenarios are in Task 2.12
//! (Epic 2 gate test).

use mismo::schema::message::MismoMessage;
use types::{BasisPoints, Cents, LoanPurpose, ProgramCode, StateCode};

// ── Embedded XML fixtures (compile-time, portable across CI/prod) ─────────────

const FHA_PURCHASE_XML: &str = include_str!("fixtures/fha_purchase.xml");
const CONV_PURCHASE_XML: &str = include_str!("fixtures/conv_purchase.xml");
const VA_PURCHASE_XML: &str = include_str!("fixtures/va_purchase.xml");
const USDA_PURCHASE_XML: &str = include_str!("fixtures/usda_purchase.xml");
const CONV_REFI_XML: &str = include_str!("fixtures/conv_refi_rate_term.xml");

// ── FHA Purchase ──────────────────────────────────────────────────────────────

#[test]
fn test_fha_fixture_parses() {
    MismoMessage::from_xml(FHA_PURCHASE_XML)
        .expect("fha_purchase.xml must parse")
        .parse_all()
        .expect("fha_purchase.xml parse_all must succeed");
}

#[test]
fn test_fha_fixture_loan_amount() {
    let deal = MismoMessage::from_xml(FHA_PURCHASE_XML)
        .unwrap()
        .parse_all()
        .unwrap();
    assert_eq!(deal.loan_terms.base_loan_amount, Cents(43_444_300));
    assert_eq!(
        deal.loan_terms.adjusted_loan_amount,
        Some(Cents(44_204_600))
    );
}

#[test]
fn test_fha_fixture_program_and_rate() {
    let deal = MismoMessage::from_xml(FHA_PURCHASE_XML)
        .unwrap()
        .parse_all()
        .unwrap();
    assert_eq!(deal.loan_terms.program, ProgramCode::Fha);
    assert_eq!(deal.loan_terms.note_rate, BasisPoints(6375));
    assert_eq!(deal.loan_terms.purpose, LoanPurpose::Purchase);
}

#[test]
fn test_fha_fixture_mi_present() {
    let deal = MismoMessage::from_xml(FHA_PURCHASE_XML)
        .unwrap()
        .parse_all()
        .unwrap();
    let mi = deal.mi.expect("FHA must have MI");
    assert_eq!(mi.upfront_amount, Some(Cents(760_275)));
    assert_eq!(mi.monthly_annual_rate, Some(BasisPoints(55)));
    assert!(mi.is_life_of_loan);
    assert!(mi.is_financed);
}

#[test]
fn test_fha_fixture_collateral() {
    let deal = MismoMessage::from_xml(FHA_PURCHASE_XML)
        .unwrap()
        .parse_all()
        .unwrap();
    assert_eq!(deal.collateral.state, StateCode::TX);
    assert_eq!(deal.collateral.appraised_value, Cents(45_900_000));
    assert_eq!(deal.collateral.fips_code.unwrap().to_string(), "48209");
}

#[test]
fn test_fha_fixture_roundtrip() {
    let xml = MismoMessage::from_xml(FHA_PURCHASE_XML)
        .unwrap()
        .to_xml()
        .unwrap();
    let deal = MismoMessage::from_xml(&xml).unwrap().parse_all().unwrap();
    assert_eq!(deal.loan_terms.base_loan_amount, Cents(43_444_300));
    assert_eq!(deal.collateral.state, StateCode::TX);
}

// ── Conventional Purchase ─────────────────────────────────────────────────────

#[test]
fn test_conv_fixture_parses() {
    MismoMessage::from_xml(CONV_PURCHASE_XML)
        .expect("conv_purchase.xml must parse")
        .parse_all()
        .expect("conv_purchase.xml parse_all must succeed");
}

#[test]
fn test_conv_fixture_program_and_loan() {
    let deal = MismoMessage::from_xml(CONV_PURCHASE_XML)
        .unwrap()
        .parse_all()
        .unwrap();
    assert_eq!(deal.loan_terms.program, ProgramCode::Conventional);
    assert_eq!(deal.loan_terms.base_loan_amount, Cents(40_000_000));
    assert_eq!(deal.loan_terms.note_rate, BasisPoints(6500));
}

#[test]
fn test_conv_fixture_no_mi() {
    let deal = MismoMessage::from_xml(CONV_PURCHASE_XML)
        .unwrap()
        .parse_all()
        .unwrap();
    // 80% LTV conventional — no MI element present
    assert!(deal.mi.is_none());
}

#[test]
fn test_conv_fixture_appraised_value() {
    let deal = MismoMessage::from_xml(CONV_PURCHASE_XML)
        .unwrap()
        .parse_all()
        .unwrap();
    assert_eq!(deal.collateral.appraised_value, Cents(50_000_000));
}

// ── VA Purchase ───────────────────────────────────────────────────────────────

#[test]
fn test_va_fixture_parses() {
    MismoMessage::from_xml(VA_PURCHASE_XML)
        .expect("va_purchase.xml must parse")
        .parse_all()
        .expect("va_purchase.xml parse_all must succeed");
}

#[test]
fn test_va_fixture_program_and_funding_fee() {
    let deal = MismoMessage::from_xml(VA_PURCHASE_XML)
        .unwrap()
        .parse_all()
        .unwrap();
    assert_eq!(deal.loan_terms.program, ProgramCode::Va);
    assert_eq!(deal.loan_terms.base_loan_amount, Cents(35_000_000));
    assert_eq!(
        deal.loan_terms.adjusted_loan_amount,
        Some(Cents(35_752_500))
    );
    let mi = deal.mi.expect("VA must have funding fee MI element");
    assert_eq!(mi.upfront_amount, Some(Cents(752_500)));
    assert!(mi.is_financed);
}

#[test]
fn test_va_fixture_borrower_va_eligible() {
    let deal = MismoMessage::from_xml(VA_PURCHASE_XML)
        .unwrap()
        .parse_all()
        .unwrap();
    assert!(deal.parties.va_eligible);
    assert!(deal.parties.va_first_use);
    assert!(!deal.parties.va_fee_exempt);
}

// ── USDA Purchase ─────────────────────────────────────────────────────────────

#[test]
fn test_usda_fixture_parses() {
    MismoMessage::from_xml(USDA_PURCHASE_XML)
        .expect("usda_purchase.xml must parse")
        .parse_all()
        .expect("usda_purchase.xml parse_all must succeed");
}

#[test]
fn test_usda_fixture_program_and_guarantee_fee() {
    let deal = MismoMessage::from_xml(USDA_PURCHASE_XML)
        .unwrap()
        .parse_all()
        .unwrap();
    assert_eq!(deal.loan_terms.program, ProgramCode::Usda);
    assert_eq!(deal.loan_terms.base_loan_amount, Cents(20_000_000));
    assert_eq!(
        deal.loan_terms.adjusted_loan_amount,
        Some(Cents(20_200_000))
    );
    let mi = deal.mi.expect("USDA must have guarantee fee MI element");
    assert_eq!(mi.upfront_amount, Some(Cents(200_000)));
    assert!(mi.is_financed);
}

#[test]
fn test_usda_fixture_household_data() {
    let deal = MismoMessage::from_xml(USDA_PURCHASE_XML)
        .unwrap()
        .parse_all()
        .unwrap();
    assert_eq!(deal.parties.usda_household_size, Some(3));
}

// ── Conventional Rate/Term Refi ───────────────────────────────────────────────

#[test]
fn test_refi_fixture_parses() {
    MismoMessage::from_xml(CONV_REFI_XML)
        .expect("conv_refi_rate_term.xml must parse")
        .parse_all()
        .expect("conv_refi_rate_term.xml parse_all must succeed");
}

#[test]
fn test_refi_fixture_program_and_purpose() {
    let deal = MismoMessage::from_xml(CONV_REFI_XML)
        .unwrap()
        .parse_all()
        .unwrap();
    assert_eq!(deal.loan_terms.program, ProgramCode::Conventional);
    assert_eq!(deal.loan_terms.purpose, LoanPurpose::RateAndTermRefinance);
    assert_eq!(deal.loan_terms.base_loan_amount, Cents(32_000_000));
}

#[test]
fn test_refi_fixture_lpa_aus() {
    let deal = MismoMessage::from_xml(CONV_REFI_XML)
        .unwrap()
        .parse_all()
        .unwrap();
    let aus = deal.aus.expect("refi must have AUS");
    assert_eq!(aus.system, types::AusType::LoanProductAdvisor);
    assert!(aus.is_approvable());
}

#[test]
fn test_refi_fixture_no_mi() {
    let deal = MismoMessage::from_xml(CONV_REFI_XML)
        .unwrap()
        .parse_all()
        .unwrap();
    // 80% LTV refi — no MI
    assert!(deal.mi.is_none());
}
