//! Epic 2 gate test — end-to-end MISMO 3.4 schema validation.
//!
//! Exercises all five canonical fixtures through the full parse chain:
//!   XML string → MismoMessage → parse_all() → ParsedDeal
//!
//! Every reference value established during Tasks 2.3–2.11 is verified
//! here. A failure in this file means a regression in the schema layer.
//!
//! When this file goes green:
//!   - `mismo` is added to the CI coverage gate (≥97% threshold)
//!   - Epic 2 is declared complete
//!   - Epic 3 (RESO) and Epic 4 (Reference Data) can begin in parallel

use mismo::{enums::aus::AusRecommendation, schema::message::MismoMessage};
use types::{AusType, BasisPoints, Cents, DtiBasisPoints, LoanPurpose, ProgramCode, StateCode};

const FHA_XML: &str = include_str!("fixtures/fha_purchase.xml");
const CONV_XML: &str = include_str!("fixtures/conv_purchase.xml");
const VA_XML: &str = include_str!("fixtures/va_purchase.xml");
const USDA_XML: &str = include_str!("fixtures/usda_purchase.xml");
const REFI_XML: &str = include_str!("fixtures/conv_refi_rate_term.xml");

// ── Gate 1: all five fixtures parse without error ─────────────────────────────

#[test]
fn gate_all_five_fixtures_parse() {
    for (name, xml) in [
        ("fha_purchase", FHA_XML),
        ("conv_purchase", CONV_XML),
        ("va_purchase", VA_XML),
        ("usda_purchase", USDA_XML),
        ("conv_refi_rate_term", REFI_XML),
    ] {
        MismoMessage::from_xml(xml)
            .unwrap_or_else(|e| panic!("{name}: from_xml failed: {e}"))
            .parse_all()
            .unwrap_or_else(|e| panic!("{name}: parse_all failed: {e}"));
    }
}

// ── Gate 2: all five round-trip losslessly ────────────────────────────────────

#[test]
fn gate_all_five_fixtures_roundtrip() {
    for (name, xml) in [
        ("fha_purchase", FHA_XML),
        ("conv_purchase", CONV_XML),
        ("va_purchase", VA_XML),
        ("usda_purchase", USDA_XML),
        ("conv_refi_rate_term", REFI_XML),
    ] {
        let original = MismoMessage::from_xml(xml)
            .unwrap_or_else(|e| panic!("{name}: from_xml failed: {e}"))
            .parse_all()
            .unwrap_or_else(|e| panic!("{name}: parse_all failed: {e}"));

        let xml2 = MismoMessage::from_xml(xml)
            .unwrap()
            .to_xml()
            .unwrap_or_else(|e| panic!("{name}: to_xml failed: {e}"));

        let roundtrip = MismoMessage::from_xml(&xml2)
            .unwrap_or_else(|e| panic!("{name}: re-parse after to_xml failed: {e}"))
            .parse_all()
            .unwrap_or_else(|e| panic!("{name}: parse_all after roundtrip failed: {e}"));

        assert_eq!(
            original.loan_terms.base_loan_amount, roundtrip.loan_terms.base_loan_amount,
            "{name}: base_loan_amount changed after roundtrip"
        );
        assert_eq!(
            original.collateral.state, roundtrip.collateral.state,
            "{name}: state changed after roundtrip"
        );
    }
}

// ── Gate 3: FHA reference scenario — complete field verification ──────────────
//
// These are the values established as ground truth throughout the build.
// Any change here represents a regression.

#[test]
fn gate_fha_loan_terms_complete() {
    let d = MismoMessage::from_xml(FHA_XML)
        .unwrap()
        .parse_all()
        .unwrap();
    assert_eq!(d.loan_terms.base_loan_amount, Cents(43_444_300));
    assert_eq!(d.loan_terms.adjusted_loan_amount, Some(Cents(44_204_600)));
    assert_eq!(d.loan_terms.note_rate, BasisPoints(6375));
    assert_eq!(d.loan_terms.term.0, 360);
    assert_eq!(d.loan_terms.program, ProgramCode::Fha);
    assert_eq!(d.loan_terms.purpose, LoanPurpose::Purchase);
}

#[test]
fn gate_fha_collateral_complete() {
    let d = MismoMessage::from_xml(FHA_XML)
        .unwrap()
        .parse_all()
        .unwrap();
    assert_eq!(d.collateral.state, StateCode::TX);
    assert_eq!(d.collateral.appraised_value, Cents(45_900_000));
    assert_eq!(d.collateral.sales_price, Some(Cents(45_900_000)));
    assert_eq!(d.collateral.fips_code.unwrap().to_string(), "48209");
    assert_eq!(d.collateral.city.to_lowercase(), "kyle");
}

#[test]
fn gate_fha_parties_complete() {
    let d = MismoMessage::from_xml(FHA_XML)
        .unwrap()
        .parse_all()
        .unwrap();
    assert_eq!(d.parties.qualifying_credit_score.unwrap().0, 720);
    assert_eq!(d.parties.borrower_count, 1);
    assert!(d.parties.first_time_homebuyer);
    assert_eq!(d.parties.monthly_gross_income, Some(Cents(850_000)));
}

#[test]
fn gate_fha_mi_complete() {
    let d = MismoMessage::from_xml(FHA_XML)
        .unwrap()
        .parse_all()
        .unwrap();
    let mi = d.mi.expect("FHA must have MI");
    assert_eq!(mi.upfront_rate, Some(BasisPoints(175)));
    assert_eq!(mi.upfront_amount, Some(Cents(760_275)));
    assert!(mi.is_financed);
    assert_eq!(mi.monthly_annual_rate, Some(BasisPoints(55)));
    assert!(mi.is_life_of_loan);
    assert!(mi.is_declining);
    assert_eq!(mi.required_months, Some(24));
}

#[test]
fn gate_fha_lender_comp_complete() {
    let d = MismoMessage::from_xml(FHA_XML)
        .unwrap()
        .parse_all()
        .unwrap();
    let comp = d.lender_comp.expect("FHA fixture must have lender comp");
    assert_eq!(comp.amount, Cents(489_924));
}

#[test]
fn gate_fha_aus_complete() {
    let d = MismoMessage::from_xml(FHA_XML)
        .unwrap()
        .parse_all()
        .unwrap();
    let aus = d.aus.expect("FHA fixture must have AUS");
    assert_eq!(aus.system, AusType::DesktopUnderwriter);
    assert_eq!(aus.recommendation, Some(AusRecommendation::ApproveEligible));
    assert!(aus.is_approvable());
    assert_eq!(aus.case_id.as_deref(), Some("DU-2025-FHA-001"));
}

#[test]
fn gate_fha_qualification_complete() {
    let d = MismoMessage::from_xml(FHA_XML)
        .unwrap()
        .parse_all()
        .unwrap();
    let q = d
        .qualification
        .expect("FHA fixture must have qualification");
    assert_eq!(q.qualifying_rate, Some(BasisPoints(6375)));
    assert_eq!(q.housing_ratio, Some(DtiBasisPoints::new(2850)));
    assert_eq!(q.total_dti, Some(DtiBasisPoints::new(4300)));
}

// ── Gate 4: program diversity across all five fixtures ────────────────────────

#[test]
fn gate_five_distinct_programs_and_purposes() {
    let cases = [
        (FHA_XML, ProgramCode::Fha, LoanPurpose::Purchase),
        (CONV_XML, ProgramCode::Conventional, LoanPurpose::Purchase),
        (VA_XML, ProgramCode::Va, LoanPurpose::Purchase),
        (USDA_XML, ProgramCode::Usda, LoanPurpose::Purchase),
        (
            REFI_XML,
            ProgramCode::Conventional,
            LoanPurpose::RateAndTermRefinance,
        ),
    ];
    for (xml, expected_program, expected_purpose) in cases {
        let d = MismoMessage::from_xml(xml).unwrap().parse_all().unwrap();
        assert_eq!(d.loan_terms.program, expected_program);
        assert_eq!(d.loan_terms.purpose, expected_purpose);
    }
}

// ── Gate 5: MI diversity ──────────────────────────────────────────────────────

#[test]
fn gate_mi_present_for_government_absent_for_conventional_no_pmi() {
    // FHA, VA, USDA all have MI/fee elements; Conv 80% LTV and refi do not
    assert!(
        MismoMessage::from_xml(FHA_XML)
            .unwrap()
            .parse_all()
            .unwrap()
            .mi
            .is_some(),
        "FHA must have MI"
    );
    assert!(
        MismoMessage::from_xml(VA_XML)
            .unwrap()
            .parse_all()
            .unwrap()
            .mi
            .is_some(),
        "VA must have funding fee"
    );
    assert!(
        MismoMessage::from_xml(USDA_XML)
            .unwrap()
            .parse_all()
            .unwrap()
            .mi
            .is_some(),
        "USDA must have guarantee fee"
    );
    assert!(
        MismoMessage::from_xml(CONV_XML)
            .unwrap()
            .parse_all()
            .unwrap()
            .mi
            .is_none(),
        "Conv 80% LTV must have no MI"
    );
    assert!(
        MismoMessage::from_xml(REFI_XML)
            .unwrap()
            .parse_all()
            .unwrap()
            .mi
            .is_none(),
        "Conv refi 80% LTV must have no MI"
    );
}

// ── Gate 6: AUS system diversity ─────────────────────────────────────────────

#[test]
fn gate_aus_system_diversity() {
    // DU for FHA/Conv purchase, LPA for refi, GUS for USDA
    let fha_aus = MismoMessage::from_xml(FHA_XML)
        .unwrap()
        .parse_all()
        .unwrap()
        .aus
        .unwrap();
    let conv_aus = MismoMessage::from_xml(CONV_XML)
        .unwrap()
        .parse_all()
        .unwrap()
        .aus
        .unwrap();
    let usda_aus = MismoMessage::from_xml(USDA_XML)
        .unwrap()
        .parse_all()
        .unwrap()
        .aus
        .unwrap();
    let refi_aus = MismoMessage::from_xml(REFI_XML)
        .unwrap()
        .parse_all()
        .unwrap()
        .aus
        .unwrap();

    assert_eq!(fha_aus.system, AusType::DesktopUnderwriter);
    assert_eq!(conv_aus.system, AusType::DesktopUnderwriter);
    assert_eq!(usda_aus.system, AusType::Gus);
    assert_eq!(refi_aus.system, AusType::LoanProductAdvisor);

    // All are approvable
    for aus in [&fha_aus, &conv_aus, &usda_aus, &refi_aus] {
        assert!(aus.is_approvable(), "{:?} should be approvable", aus.system);
    }
}

// ── Gate 7: VA-specific borrower data ────────────────────────────────────────

#[test]
fn gate_va_borrower_entitlement_flags() {
    let d = MismoMessage::from_xml(VA_XML).unwrap().parse_all().unwrap();
    assert!(d.parties.va_eligible, "VA borrower must be eligible");
    assert!(d.parties.va_first_use, "VA fixture is first use");
    assert!(!d.parties.va_fee_exempt, "VA fixture is not fee-exempt");
    // Funding fee: 2.15% of $350,000 = $7,525
    assert_eq!(d.mi.unwrap().upfront_amount, Some(Cents(752_500)));
}

// ── Gate 8: USDA-specific borrower data ──────────────────────────────────────

#[test]
fn gate_usda_household_data() {
    let d = MismoMessage::from_xml(USDA_XML)
        .unwrap()
        .parse_all()
        .unwrap();
    assert_eq!(d.parties.usda_household_size, Some(3));
    // Guarantee fee: 1.00% of $200,000 = $2,000
    let mi = d.mi.unwrap();
    assert_eq!(mi.upfront_amount, Some(Cents(200_000)));
    assert!(mi.is_financed);
    // Annual fee: 0.35%/yr
    assert_eq!(mi.monthly_annual_rate, Some(BasisPoints(35)));
}
