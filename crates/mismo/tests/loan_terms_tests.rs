//! Task 2.3 gate tests — loan terms schema.
//!
//! Verifies that `MortgageTerms` and `Amortization` parse correctly from
//! XML strings, that the `parse()` method produces exact domain-typed values,
//! and that the round-trip through serialization is lossless.
//!
//! All reference values are from the spreadsheet scenario:
//!   FHA purchase, $459,000 price, $434,443 base loan, 6.375% note rate,
//!   360-month term, $10,000 seller credit.

use mismo::{
    schema::loan_terms::{Amortization, LoanTermsParsed, MortgageTerms},
    MismoError,
};
use types::{AmortizationType, BasisPoints, Cents, LienPriority, LoanPurpose, ProgramCode, TermMonths};

// ── Test helpers ──────────────────────────────────────────────────────────────

/// Build the spreadsheet FHA scenario MortgageTerms directly (no XML round-trip).
fn spreadsheet_terms() -> MortgageTerms {
    MortgageTerms {
        base_loan_amount: "434443.00".into(),
        loan_amount_with_financed_mi: Some("442046.00".into()),
        note_rate_percent: "6.375".into(),
        loan_term_months_count: "360".into(),
        mortgage_type: "FHA".into(),
        lien_priority_type: "FirstLien".into(),
        loan_purpose_type: "Purchase".into(),
        holding_period_months: None,
        days_until_closing: None,
        seller_concession_amount: Some("10000.00".into()),
        seller_pays_owners_title: Some("true".into()),
        waive_escrow: None,
        temp_buydown: None,
        subordinate_financing: None,
        high_balance: None,
    }
}

fn fixed_amort() -> Amortization {
    Amortization { amortization_type: "Fixed".into() }
}

// ── Core parse: spreadsheet values ───────────────────────────────────────────

#[test]
fn test_fha_loan_terms_parse_to_typed() {
    let parsed = spreadsheet_terms().parse(&fixed_amort()).unwrap();

    assert_eq!(parsed.program,     ProgramCode::Fha);
    assert_eq!(parsed.purpose,     LoanPurpose::Purchase);
    assert_eq!(parsed.lien,        LienPriority::First);
    assert_eq!(parsed.amortization, AmortizationType::Fixed);
}

#[test]
fn test_base_loan_amount_cents_precision() {
    // $434,443.00 = 43,444,300 cents
    let parsed = spreadsheet_terms().parse(&fixed_amort()).unwrap();
    assert_eq!(parsed.base_loan_amount, Cents(43_444_300));
}

#[test]
fn test_adjusted_loan_amount_with_ufmip() {
    // $442,046.00 = 44,204,600 cents (base + UFMIP financed)
    let parsed = spreadsheet_terms().parse(&fixed_amort()).unwrap();
    assert_eq!(parsed.adjusted_loan_amount, Some(Cents(44_204_600)));
}

#[test]
fn test_note_rate_to_basis_points_6_375() {
    // 6.375% → BasisPoints(6375)
    let parsed = spreadsheet_terms().parse(&fixed_amort()).unwrap();
    assert_eq!(parsed.note_rate, BasisPoints(6375));
}

#[test]
fn test_note_rate_percent_sign_accepted() {
    // MISMO sometimes includes the % symbol
    let mut t = spreadsheet_terms();
    t.note_rate_percent = "6.375%".into();
    let parsed = t.parse(&fixed_amort()).unwrap();
    assert_eq!(parsed.note_rate, BasisPoints(6375));
}

#[test]
fn test_term_months_360_valid() {
    let parsed = spreadsheet_terms().parse(&fixed_amort()).unwrap();
    assert_eq!(parsed.term, TermMonths::new(360).unwrap());
}

#[test]
fn test_term_months_180_valid() {
    let mut t = spreadsheet_terms();
    t.loan_term_months_count = "180".into();
    let parsed = t.parse(&fixed_amort()).unwrap();
    assert_eq!(parsed.term, TermMonths::new(180).unwrap());
}

#[test]
fn test_seller_concession_to_cents() {
    // $10,000.00 = 1,000,000 cents
    let parsed = spreadsheet_terms().parse(&fixed_amort()).unwrap();
    assert_eq!(parsed.seller_concession, Some(Cents(1_000_000)));
}

#[test]
fn test_seller_pays_title_true() {
    let parsed = spreadsheet_terms().parse(&fixed_amort()).unwrap();
    assert!(parsed.seller_pays_title);
}

// ── Optional fields ───────────────────────────────────────────────────────────

#[test]
fn test_adjusted_loan_amount_absent_is_none() {
    let mut t = spreadsheet_terms();
    t.loan_amount_with_financed_mi = None;
    let parsed = t.parse(&fixed_amort()).unwrap();
    assert!(parsed.adjusted_loan_amount.is_none());
}

#[test]
fn test_seller_concession_absent_is_none() {
    let mut t = spreadsheet_terms();
    t.seller_concession_amount = None;
    let parsed = t.parse(&fixed_amort()).unwrap();
    assert!(parsed.seller_concession.is_none());
}

#[test]
fn test_waive_escrow_defaults_false_when_absent() {
    let parsed = spreadsheet_terms().parse(&fixed_amort()).unwrap();
    assert!(!parsed.waive_escrow);
}

#[test]
fn test_waive_escrow_true_when_set() {
    let mut t = spreadsheet_terms();
    t.waive_escrow = Some("true".into());
    let parsed = t.parse(&fixed_amort()).unwrap();
    assert!(parsed.waive_escrow);
}

#[test]
fn test_high_balance_flag_parses() {
    let mut t = spreadsheet_terms();
    t.high_balance = Some("true".into());
    let parsed = t.parse(&fixed_amort()).unwrap();
    assert!(parsed.is_high_balance);
}

#[test]
fn test_days_until_closing_optional_parses() {
    let mut t = spreadsheet_terms();
    t.days_until_closing = Some("6".into()); // 6 days to first payment
    let parsed = t.parse(&fixed_amort()).unwrap();
    assert_eq!(parsed.days_until_closing, Some(6));
}

#[test]
fn test_holding_period_months_optional_parses() {
    let mut t = spreadsheet_terms();
    t.holding_period_months = Some("60".into()); // 5-year hold
    let parsed = t.parse(&fixed_amort()).unwrap();
    assert_eq!(parsed.holding_period_months, Some(60));
}

// ── Validation errors ─────────────────────────────────────────────────────────

#[test]
fn test_invalid_mortgage_type_returns_invalid_enum_error() {
    let mut t = spreadsheet_terms();
    t.mortgage_type = "SubprimeMortgage".into();
    let err = t.parse(&fixed_amort()).unwrap_err();
    assert!(
        matches!(err, MismoError::InvalidEnum { element: "MortgageType", .. }),
        "expected InvalidEnum for MortgageType, got: {err}"
    );
}

#[test]
fn test_invalid_lien_priority_returns_error() {
    let mut t = spreadsheet_terms();
    t.lien_priority_type = "FifthLien".into();
    let err = t.parse(&fixed_amort()).unwrap_err();
    assert!(matches!(err, MismoError::InvalidEnum { element: "LienPriorityType", .. }));
}

#[test]
fn test_invalid_loan_purpose_returns_error() {
    let mut t = spreadsheet_terms();
    t.loan_purpose_type = "Speculation".into();
    let err = t.parse(&fixed_amort()).unwrap_err();
    assert!(matches!(err, MismoError::InvalidEnum { element: "LoanPurposeType", .. }));
}

#[test]
fn test_term_months_out_of_range_returns_error() {
    let mut t = spreadsheet_terms();
    t.loan_term_months_count = "60".into(); // below 120 minimum
    let err = t.parse(&fixed_amort()).unwrap_err();
    assert!(matches!(err, MismoError::OutOfRange { element: "LoanTermMonthsCount", .. }));
}

#[test]
fn test_non_numeric_amount_returns_error() {
    let mut t = spreadsheet_terms();
    t.base_loan_amount = "four hundred thousand".into();
    let err = t.parse(&fixed_amort()).unwrap_err();
    assert!(matches!(err, MismoError::OutOfRange { element: "BaseLoanAmount", .. }));
}

#[test]
fn test_invalid_amortization_type_returns_error() {
    let t = spreadsheet_terms();
    let bad_amort = Amortization { amortization_type: "Balloon".into() };
    let err = t.parse(&bad_amort).unwrap_err();
    assert!(matches!(err, MismoError::InvalidEnum { element: "AmortizationType", .. }));
}

// ── Program types ─────────────────────────────────────────────────────────────

#[test]
fn test_va_program_parses() {
    let mut t = spreadsheet_terms();
    t.mortgage_type = "VA".into();
    let parsed = t.parse(&fixed_amort()).unwrap();
    assert_eq!(parsed.program, ProgramCode::Va);
}

#[test]
fn test_usda_program_parses() {
    let mut t = spreadsheet_terms();
    t.mortgage_type = "USDARuralDevelopment".into();
    let parsed = t.parse(&fixed_amort()).unwrap();
    assert_eq!(parsed.program, ProgramCode::Usda);
}

#[test]
fn test_conventional_program_parses() {
    let mut t = spreadsheet_terms();
    t.mortgage_type = "Conventional".into();
    let parsed = t.parse(&fixed_amort()).unwrap();
    assert_eq!(parsed.program, ProgramCode::Conventional);
}

// ── XML round-trip ────────────────────────────────────────────────────────────

#[test]
fn test_mortgage_terms_xml_roundtrip() {
    let original = spreadsheet_terms();
    let xml = mismo::xml::serialize::to_xml(&original).unwrap();

    assert!(xml.contains("434443.00"), "XML should contain base loan amount");
    assert!(xml.contains("6.375"),     "XML should contain note rate");
    assert!(xml.contains("FHA"),       "XML should contain mortgage type");
    assert!(xml.contains("360"),       "XML should contain term");

    let restored: MortgageTerms = mismo::xml::parse::from_xml(&xml).unwrap();
    let parsed_original = original.parse(&fixed_amort()).unwrap();
    let parsed_restored = restored.parse(&fixed_amort()).unwrap();

    assert_eq!(parsed_original.base_loan_amount, parsed_restored.base_loan_amount);
    assert_eq!(parsed_original.note_rate,        parsed_restored.note_rate);
    assert_eq!(parsed_original.term,             parsed_restored.term);
    assert_eq!(parsed_original.program,          parsed_restored.program);
}

#[test]
fn test_amortization_xml_roundtrip() {
    let original = Amortization { amortization_type: "Fixed".into() };
    let xml = mismo::xml::serialize::to_xml(&original).unwrap();
    assert!(xml.contains("Fixed"));
    let restored: Amortization = mismo::xml::parse::from_xml(&xml).unwrap();
    assert_eq!(restored.amortization_type, "Fixed");
}

// ── From XML strings (full integration) ──────────────────────────────────────

#[test]
fn test_parse_mortgage_terms_from_xml_string() {
    let xml = r#"<MORTGAGE_TERMS>
        <BaseLoanAmount>434443.00</BaseLoanAmount>
        <LoanAmountWithFinancedMI>442046.00</LoanAmountWithFinancedMI>
        <NoteRatePercent>6.375</NoteRatePercent>
        <LoanTermMonthsCount>360</LoanTermMonthsCount>
        <MortgageType>FHA</MortgageType>
        <LienPriorityType>FirstLien</LienPriorityType>
        <LoanPurposeType>Purchase</LoanPurposeType>
        <SellerConcessionAmount>10000.00</SellerConcessionAmount>
        <SellerPaysOwnersTitleIndicator>true</SellerPaysOwnersTitleIndicator>
    </MORTGAGE_TERMS>"#;

    let amort_xml = r#"<AMORTIZATION>
        <AmortizationType>Fixed</AmortizationType>
    </AMORTIZATION>"#;

    let terms: MortgageTerms = mismo::xml::parse::from_xml(xml).unwrap();
    let amort: Amortization = mismo::xml::parse::from_xml(amort_xml).unwrap();
    let parsed: LoanTermsParsed = terms.parse(&amort).unwrap();

    // Verify all spreadsheet reference values
    assert_eq!(parsed.base_loan_amount,   Cents(43_444_300));
    assert_eq!(parsed.adjusted_loan_amount, Some(Cents(44_204_600)));
    assert_eq!(parsed.note_rate,          BasisPoints(6375));
    assert_eq!(parsed.term,               TermMonths::new(360).unwrap());
    assert_eq!(parsed.program,            ProgramCode::Fha);
    assert_eq!(parsed.lien,               LienPriority::First);
    assert_eq!(parsed.purpose,            LoanPurpose::Purchase);
    assert_eq!(parsed.amortization,       AmortizationType::Fixed);
    assert_eq!(parsed.seller_concession,  Some(Cents(1_000_000)));
    assert!(parsed.seller_pays_title);
}
