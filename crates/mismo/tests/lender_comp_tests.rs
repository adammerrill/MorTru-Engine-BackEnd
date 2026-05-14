//! Task 2.7 gate tests — lender compensation schema.
//!
//! Reference scenario: FHA purchase, Kyle TX.
//!   Loan $434,443; broker comp $4,899.24; borrower-paid; Section A.

use mismo::{
    enums::comp::{CompDisclosure, CompType},
    schema::lender_comp::LenderComp,
    MismoError,
};
use rust_decimal::Decimal;
use std::str::FromStr;
use types::{BasisPoints, Cents};

use mismo::schema::lender_comp::LenderCompParsed;

// ── Test helpers ──────────────────────────────────────────────────────────────

/// FHA purchase reference — borrower-paid broker comp.
fn fha_borrower_paid_comp() -> LenderComp {
    LenderComp {
        amount: "4899.24".into(),
        comp_bps: Some("112.76".into()),
        comp_type: "BorrowerPaid".into(),
        disclose_in_section_a: Some("true".into()),
        cap_amount: None,
    }
}

fn lender_paid_comp() -> LenderComp {
    LenderComp {
        amount: "3500.00".into(),
        comp_bps: Some("80.0".into()),
        comp_type: "LenderPaid".into(),
        disclose_in_section_a: Some("false".into()),
        cap_amount: None,
    }
}

// ── Amount parsing ────────────────────────────────────────────────────────────

#[test]
fn test_comp_amount_to_cents() {
    let p = fha_borrower_paid_comp().parse().unwrap();
    assert_eq!(p.amount, Cents(489_924));
}

#[test]
fn test_comp_amount_invalid_returns_error() {
    let mut c = fha_borrower_paid_comp();
    c.amount = "not_a_number".into();
    assert!(matches!(
        c.parse().unwrap_err(),
        MismoError::OutOfRange { .. }
    ));
}

// ── Comp BPS parsing ──────────────────────────────────────────────────────────

#[test]
fn test_comp_bps_112_rounds_to_integer_113() {
    let p = fha_borrower_paid_comp().parse().unwrap();
    // "112.76" rounds to BasisPoints(113)
    assert_eq!(p.comp_bps, Some(BasisPoints(113)));
}

#[test]
fn test_comp_bps_decimal_preserved() {
    let p = fha_borrower_paid_comp().parse().unwrap();
    assert_eq!(
        p.comp_bps_decimal,
        Some(Decimal::from_str("112.76").unwrap())
    );
}

#[test]
fn test_comp_bps_absent_is_none() {
    let mut c = fha_borrower_paid_comp();
    c.comp_bps = None;
    let p = c.parse().unwrap();
    assert!(p.comp_bps.is_none());
    assert!(p.comp_bps_decimal.is_none());
}

// ── Comp type and disclosure ──────────────────────────────────────────────────

#[test]
fn test_borrower_paid_type_parses() {
    let p = fha_borrower_paid_comp().parse().unwrap();
    assert_eq!(p.comp_type, CompType::BorrowerPaid);
}

#[test]
fn test_borrower_paid_disclosed_in_section_a() {
    let p = fha_borrower_paid_comp().parse().unwrap();
    assert_eq!(p.disclosure, CompDisclosure::InSectionA);
}

#[test]
fn test_lender_paid_type_parses() {
    let p = lender_paid_comp().parse().unwrap();
    assert_eq!(p.comp_type, CompType::LenderPaid);
}

#[test]
fn test_lender_paid_disclosed_on_page3() {
    let p = lender_paid_comp().parse().unwrap();
    assert_eq!(p.disclosure, CompDisclosure::OnPage3);
}

#[test]
fn test_section_a_indicator_inferred_from_comp_type_when_absent() {
    // When DisclosedInSectionAIndicator is absent, infer from comp type
    let mut c = fha_borrower_paid_comp();
    c.disclose_in_section_a = None;
    let p = c.parse().unwrap();
    assert_eq!(p.disclosure, CompDisclosure::InSectionA);
}

#[test]
fn test_borrower_paid_with_false_section_a_returns_error() {
    // TRID violation: borrower-paid comp MUST appear in Section A
    let mut c = fha_borrower_paid_comp();
    c.disclose_in_section_a = Some("false".into());
    assert!(matches!(
        c.parse().unwrap_err(),
        MismoError::InvalidEnum { .. }
    ));
}

#[test]
fn test_unknown_comp_type_returns_error() {
    let mut c = fha_borrower_paid_comp();
    c.comp_type = "EmployeePaid".into();
    assert!(matches!(
        c.parse().unwrap_err(),
        MismoError::InvalidEnum {
            element: "CompensationType",
            ..
        }
    ));
}

// ── compute_from_bps ─────────────────────────────────────────────────────────

#[test]
fn test_compute_from_bps_clean_round_numbers() {
    // $100,000 × 100 bps (1.00%) = $1,000.00
    let result = LenderCompParsed::compute_from_bps(Cents(10_000_000), BasisPoints(100));
    assert_eq!(result, Cents(100_000));
}

#[test]
fn test_compute_from_bps_150_bps() {
    // $200,000 × 150 bps (1.50%) = $3,000.00
    let result = LenderCompParsed::compute_from_bps(Cents(20_000_000), BasisPoints(150));
    assert_eq!(result, Cents(300_000));
}

#[test]
fn test_compute_from_bps_zero_returns_zero() {
    let result = LenderCompParsed::compute_from_bps(Cents(43_444_300), BasisPoints(0));
    assert_eq!(result, Cents(0));
}

// ── compute_from_bps_decimal ──────────────────────────────────────────────────

#[test]
fn test_compute_from_bps_decimal_exact_reference() {
    // FHA reference: $434,443 × 112.76 bps → near $4,899.24
    // Exact: 43_444_300 × 112.76 / 10_000 = 489_850 cents ≈ $4,898.50
    // The XML amount field ($4,899.24) is the source of truth; the BPS
    // is provided for audit. Decimal precision is used in Epic 11 fee engine.
    let bps = Decimal::from_str("112.76").unwrap();
    let result = LenderCompParsed::compute_from_bps_decimal(Cents(43_444_300), bps).unwrap();
    // Result within $2 of the reference comp amount
    let diff = (result.0 - 489_924_i64).abs();
    assert!(
        diff < 200,
        "expected within $2 of $4,899.24, got {}",
        result.0
    );
}

#[test]
fn test_compute_from_bps_decimal_clean_value() {
    // $100,000 × 112.5 bps (1.125%) = $1,125.00
    let bps = Decimal::from_str("112.5").unwrap();
    let result = LenderCompParsed::compute_from_bps_decimal(Cents(10_000_000), bps).unwrap();
    assert_eq!(result, Cents(112_500));
}

// ── Cap amount ────────────────────────────────────────────────────────────────

#[test]
fn test_comp_cap_amount_parses() {
    let mut c = fha_borrower_paid_comp();
    c.cap_amount = Some("5000.00".into());
    let p = c.parse().unwrap();
    assert_eq!(p.cap_amount, Some(Cents(500_000)));
}

#[test]
fn test_comp_cap_absent_is_none() {
    let p = fha_borrower_paid_comp().parse().unwrap();
    assert!(p.cap_amount.is_none());
}

// ── XML round-trip ────────────────────────────────────────────────────────────

#[test]
fn test_lender_comp_xml_roundtrip() {
    let comp = fha_borrower_paid_comp();
    let xml = mismo::xml::serialize::to_xml(&comp).unwrap();
    assert!(xml.contains("4899.24"));
    assert!(xml.contains("112.76"));
    assert!(xml.contains("BorrowerPaid"));

    let restored: LenderComp = mismo::xml::parse::from_xml(&xml).unwrap();
    let p = restored.parse().unwrap();
    assert_eq!(p.amount, Cents(489_924));
    assert_eq!(p.comp_type, CompType::BorrowerPaid);
    assert_eq!(p.disclosure, CompDisclosure::InSectionA);
}
