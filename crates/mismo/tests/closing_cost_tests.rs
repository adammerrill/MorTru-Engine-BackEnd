//! Task 2.8 gate tests — closing cost schema.
//!
//! Tests cover: section parsing, fee type registry, tolerance categories,
//! VA/FHA non-allowable rules, lender credit validation, section aggregation,
//! APR fee totals, financed fee handling, and the fee rules JSON round-trip.

use mismo::schema::closing_cost::{
    parse_closing_costs, ClosingCostBlock, FeeEntry, FeeRulesRegistry, FeeSection, FeeTolerance,
    FeeType, MismoClosingCostFee,
};
use types::Cents;

// ── Registry: embedded JSON loads correctly ───────────────────────────────────

#[test]
fn test_fee_rules_registry_loads_from_embedded_json() {
    let registry = FeeRulesRegistry::default();
    // Must contain at least all core fee types
    assert!(registry.get(FeeType::AppraisalFee).is_some());
    assert!(registry.get(FeeType::FhaUfmip).is_some());
    assert!(registry.get(FeeType::LenderCredit).is_some());
}

#[test]
fn test_fee_rules_registry_as_json_roundtrip() {
    let registry = FeeRulesRegistry::default();
    let json = registry.as_json();
    // Must re-parse without error
    let registry2 = FeeRulesRegistry::from_json(json).unwrap();
    assert!(registry2.get(FeeType::TaxServiceFee).is_some());
}

#[test]
fn test_fee_rules_registry_invalid_json_returns_error() {
    let result = FeeRulesRegistry::from_json("{ not valid json }");
    assert!(result.is_err());
}

// ── VA non-allowable fees ─────────────────────────────────────────────────────

#[test]
fn test_tax_service_fee_va_non_allowable() {
    let r = FeeRulesRegistry::default();
    assert!(!r.va_allowable(FeeType::TaxServiceFee));
}

#[test]
fn test_underwriting_fee_va_non_allowable() {
    let r = FeeRulesRegistry::default();
    assert!(!r.va_allowable(FeeType::UnderwritingFee));
}

#[test]
fn test_document_prep_fee_va_non_allowable() {
    let r = FeeRulesRegistry::default();
    assert!(!r.va_allowable(FeeType::DocumentPrepFee));
}

#[test]
fn test_attorney_fee_va_non_allowable() {
    let r = FeeRulesRegistry::default();
    assert!(!r.va_allowable(FeeType::AttorneyFee));
}

#[test]
fn test_survey_fee_va_non_allowable() {
    let r = FeeRulesRegistry::default();
    assert!(!r.va_allowable(FeeType::SurveyFee));
}

#[test]
fn test_appraisal_fee_va_allowable() {
    let r = FeeRulesRegistry::default();
    assert!(r.va_allowable(FeeType::AppraisalFee));
}

// ── APR inclusion ─────────────────────────────────────────────────────────────

#[test]
fn test_origination_fee_is_apr_included() {
    let r = FeeRulesRegistry::default();
    assert!(r.is_apr_included(FeeType::OriginationFee));
}

#[test]
fn test_broker_compensation_is_apr_included() {
    let r = FeeRulesRegistry::default();
    assert!(r.is_apr_included(FeeType::BrokerCompensation));
}

#[test]
fn test_appraisal_fee_not_apr_included() {
    let r = FeeRulesRegistry::default();
    assert!(!r.is_apr_included(FeeType::AppraisalFee));
}

#[test]
fn test_title_policy_not_apr_included() {
    let r = FeeRulesRegistry::default();
    assert!(!r.is_apr_included(FeeType::OwnersTitlePolicy));
    assert!(!r.is_apr_included(FeeType::LendersTitlePolicy));
}

#[test]
fn test_prepaid_interest_is_apr_included() {
    let r = FeeRulesRegistry::default();
    assert!(r.is_apr_included(FeeType::PrepaidInterest));
}

// ── Financed fees ─────────────────────────────────────────────────────────────

#[test]
fn test_fha_ufmip_can_be_financed() {
    let r = FeeRulesRegistry::default();
    assert!(r.can_be_financed(FeeType::FhaUfmip));
}

#[test]
fn test_va_funding_fee_can_be_financed() {
    let r = FeeRulesRegistry::default();
    assert!(r.can_be_financed(FeeType::VaFundingFee));
}

#[test]
fn test_usda_guarantee_fee_can_be_financed() {
    let r = FeeRulesRegistry::default();
    assert!(r.can_be_financed(FeeType::UsdaGuaranteeFee));
}

#[test]
fn test_appraisal_fee_cannot_be_financed() {
    let r = FeeRulesRegistry::default();
    assert!(!r.can_be_financed(FeeType::AppraisalFee));
}

// ── Tolerance categories ──────────────────────────────────────────────────────

#[test]
fn test_origination_fee_zero_tolerance() {
    let r = FeeRulesRegistry::default();
    assert_eq!(
        r.default_tolerance(FeeType::OriginationFee),
        FeeTolerance::Zero
    );
}

#[test]
fn test_appraisal_fee_zero_tolerance() {
    let r = FeeRulesRegistry::default();
    assert_eq!(
        r.default_tolerance(FeeType::AppraisalFee),
        FeeTolerance::Zero
    );
}

#[test]
fn test_deed_recording_ten_pct_tolerance() {
    let r = FeeRulesRegistry::default();
    assert_eq!(
        r.default_tolerance(FeeType::DeedRecordingFee),
        FeeTolerance::TenPct
    );
}

#[test]
fn test_transfer_tax_zero_tolerance() {
    let r = FeeRulesRegistry::default();
    assert_eq!(
        r.default_tolerance(FeeType::DeedTransferTax),
        FeeTolerance::Zero
    );
}

#[test]
fn test_prepaid_interest_unlimited_tolerance() {
    let r = FeeRulesRegistry::default();
    assert_eq!(
        r.default_tolerance(FeeType::PrepaidInterest),
        FeeTolerance::Unlimited
    );
}

// ── FeeSection MISMO string mapping ──────────────────────────────────────────

#[test]
fn test_section_a_from_mismo_string() {
    assert_eq!(
        FeeSection::from_mismo_str("LoanCosts_OriginationCharges").unwrap(),
        FeeSection::A
    );
}

#[test]
fn test_section_b_from_mismo_string() {
    assert_eq!(
        FeeSection::from_mismo_str("LoanCosts_ServicesYouCannotShopFor").unwrap(),
        FeeSection::B
    );
}

#[test]
fn test_section_c_from_mismo_string() {
    assert_eq!(
        FeeSection::from_mismo_str("LoanCosts_ServicesYouCanShopFor").unwrap(),
        FeeSection::C
    );
}

#[test]
fn test_section_e_from_mismo_string() {
    assert_eq!(
        FeeSection::from_mismo_str("OtherCosts_TaxesAndOtherGovernmentFees").unwrap(),
        FeeSection::E
    );
}

#[test]
fn test_section_g_from_mismo_string() {
    assert_eq!(
        FeeSection::from_mismo_str("OtherCosts_InitialEscrowPaymentAtClosing").unwrap(),
        FeeSection::G
    );
}

#[test]
fn test_unknown_section_returns_error() {
    assert!(FeeSection::from_mismo_str("MadeUpSection").is_err());
}

#[test]
fn test_section_label_chars() {
    assert_eq!(FeeSection::A.label(), 'A');
    assert_eq!(FeeSection::E.label(), 'E');
    assert_eq!(FeeSection::G.label(), 'G');
}

#[test]
fn test_section_loan_cost_classification() {
    assert!(FeeSection::A.is_loan_cost());
    assert!(FeeSection::B.is_loan_cost());
    assert!(FeeSection::C.is_loan_cost());
    assert!(!FeeSection::E.is_loan_cost());
    assert!(!FeeSection::G.is_loan_cost());
}

#[test]
fn test_section_other_cost_classification() {
    assert!(FeeSection::E.is_other_cost());
    assert!(FeeSection::F.is_other_cost());
    assert!(FeeSection::G.is_other_cost());
    assert!(FeeSection::H.is_other_cost());
    assert!(!FeeSection::A.is_other_cost());
}

// ── ClosingCostBlock aggregation ──────────────────────────────────────────────

fn make_fee(section: FeeSection, borrower: i64, financed: bool) -> FeeEntry {
    FeeEntry {
        section,
        fee_type: FeeType::Other,
        description: "Test fee".into(),
        borrower_amount: Cents(borrower),
        seller_amount: Cents(0),
        lender_amount: Cents(0),
        other_amount: Cents(0),
        tolerance: FeeTolerance::Unlimited,
        is_financed: financed,
        is_apr_included: false,
        source: mismo::schema::closing_cost::FeeSource::Static,
        sequence: 1,
    }
}

fn make_apr_fee(section: FeeSection, borrower: i64) -> FeeEntry {
    let mut f = make_fee(section, borrower, false);
    f.is_apr_included = true;
    f
}

#[test]
fn test_section_d_total_loan_costs() {
    let mut block = ClosingCostBlock::default();
    block.add(make_fee(FeeSection::A, 109_500, false)); // $1,095
    block.add(make_fee(FeeSection::B, 80_000, false)); // $800
    block.add(make_fee(FeeSection::C, 263_500, false)); // $2,635
                                                        // Section D = A + B + C = $4,530
    assert_eq!(block.total_loan_costs_borrower(), Cents(453_000));
}

#[test]
fn test_financed_fee_excluded_from_section_d() {
    let mut block = ClosingCostBlock::default();
    block.add(make_fee(FeeSection::B, 80_000, false)); // $800 — counts
    block.add(make_fee(FeeSection::B, 760_275, true)); // $7,602.75 — financed, excluded
    assert_eq!(block.section_b_borrower(), Cents(80_000));
    assert_eq!(block.total_financed(), Cents(760_275));
}

#[test]
fn test_section_i_total_other_costs() {
    let mut block = ClosingCostBlock::default();
    block.add(make_fee(FeeSection::E, 17_400, false)); // recording
    block.add(make_fee(FeeSection::F, 45_000, false)); // prepaids
    block.add(make_fee(FeeSection::G, 220_000, false)); // escrow
    block.add(make_fee(FeeSection::H, 17_500, false)); // HOA
                                                       // Section I = $299.00
    assert_eq!(block.total_other_costs_borrower(), Cents(299_900));
}

#[test]
fn test_section_j_total_closing_costs_after_lender_credit() {
    let mut block = ClosingCostBlock::default();
    block.add(make_fee(FeeSection::A, 109_500, false));
    block.add(make_fee(FeeSection::B, 80_000, false));
    // Lender credit of $500
    let mut credit = make_fee(FeeSection::J, 50_000, false);
    credit.fee_type = FeeType::LenderCredit;
    block.add(credit);
    // J = (1095 + 800) - 500 = $1,395
    assert_eq!(block.total_closing_costs_borrower(), Cents(139_500));
}

#[test]
fn test_lender_credit_validation_passes_when_within_costs() {
    let mut block = ClosingCostBlock::default();
    block.add(make_fee(FeeSection::A, 1_000_000, false)); // $10,000
    let mut credit = make_fee(FeeSection::J, 500_000, false);
    credit.fee_type = FeeType::LenderCredit;
    block.add(credit);
    assert!(block.validate_lender_credits().is_ok());
}

#[test]
fn test_lender_credit_validation_fails_when_exceeds_costs() {
    let mut block = ClosingCostBlock::default();
    block.add(make_fee(FeeSection::A, 500_000, false)); // $5,000
    let mut credit = make_fee(FeeSection::J, 1_000_000, false);
    credit.fee_type = FeeType::LenderCredit;
    block.add(credit);
    assert!(block.validate_lender_credits().is_err());
}

#[test]
fn test_apr_fee_total() {
    let mut block = ClosingCostBlock::default();
    block.add(make_apr_fee(FeeSection::A, 109_500)); // $1,095 APR
    block.add(make_fee(FeeSection::B, 80_000, false)); // $800 NOT APR
    block.add(make_apr_fee(FeeSection::B, 7_794)); // $77.94 APR (credit report)
    assert_eq!(block.total_apr_fees(), Cents(117_294));
}

// ── MISMO XML parse round-trip ────────────────────────────────────────────────

fn fha_origination_fee_xml() -> MismoClosingCostFee {
    MismoClosingCostFee {
        section_type: "LoanCosts_OriginationCharges".into(),
        description: "Application Fee".into(),
        total_amount: Some("1095.00".into()),
        borrower_amount: Some("1095.00".into()),
        seller_amount: None,
        lender_amount: None,
        paid_by: Some("Borrower".into()),
        financed: Some("false".into()),
        apr_affected: Some("true".into()),
        sequence_number: Some("1".into()),
        fee_type_code: Some("ApplicationFee".into()),
    }
}

fn fha_ufmip_xml() -> MismoClosingCostFee {
    MismoClosingCostFee {
        section_type: "LoanCosts_ServicesYouCannotShopFor".into(),
        description: "FHA Upfront MIP".into(),
        total_amount: Some("7602.75".into()),
        borrower_amount: Some("7602.75".into()),
        seller_amount: None,
        lender_amount: None,
        paid_by: Some("Borrower".into()),
        financed: Some("true".into()),
        apr_affected: Some("true".into()),
        sequence_number: Some("5".into()),
        fee_type_code: Some("FhaUfmip".into()),
    }
}

#[test]
fn test_parse_application_fee_from_xml() {
    let registry = FeeRulesRegistry::default();
    let entry = fha_origination_fee_xml()
        .parse_with_registry(&registry)
        .unwrap();
    assert_eq!(entry.section, FeeSection::A);
    assert_eq!(entry.fee_type, FeeType::ApplicationFee);
    assert_eq!(entry.borrower_amount, Cents(109_500));
    assert!(entry.is_apr_included);
    assert!(!entry.is_financed);
    assert_eq!(entry.sequence, 1);
}

#[test]
fn test_parse_ufmip_as_financed_section_b() {
    let registry = FeeRulesRegistry::default();
    let entry = fha_ufmip_xml().parse_with_registry(&registry).unwrap();
    assert_eq!(entry.section, FeeSection::B);
    assert_eq!(entry.fee_type, FeeType::FhaUfmip);
    assert_eq!(entry.borrower_amount, Cents(760_275));
    assert!(entry.is_financed);
}

#[test]
fn test_parse_closing_costs_list() {
    let fees = vec![fha_origination_fee_xml(), fha_ufmip_xml()];
    let registry = FeeRulesRegistry::default();
    let block = parse_closing_costs(&fees, &registry).unwrap();
    assert_eq!(block.section_a.len(), 1);
    assert_eq!(block.section_b.len(), 1);
    assert_eq!(block.section_a_borrower(), Cents(109_500));
    assert_eq!(block.total_financed(), Cents(760_275));
}

#[test]
fn test_fee_entry_total_amount_sums_all_parties() {
    let registry = FeeRulesRegistry::default();
    let mut fee_xml = fha_origination_fee_xml();
    fee_xml.seller_amount = Some("460.00".into()); // seller pays portion
    let entry = fee_xml.parse_with_registry(&registry).unwrap();
    // borrower $1,095 + seller $460 = $1,555
    assert_eq!(entry.total_amount(), Cents(155_500));
}

#[test]
fn test_all_fee_types_have_registered_rules() {
    let registry = FeeRulesRegistry::default();
    // Spot check all critical fee types are registered
    let required = [
        FeeType::AppraisalFee,
        FeeType::FhaUfmip,
        FeeType::VaFundingFee,
        FeeType::UsdaGuaranteeFee,
        FeeType::OwnersTitlePolicy,
        FeeType::LendersTitlePolicy,
        FeeType::DeedRecordingFee,
        FeeType::DeedTransferTax,
        FeeType::PrepaidInterest,
        FeeType::TaxEscrow,
        FeeType::HoaTransferFee,
        FeeType::LenderCredit,
    ];
    for fee_type in required {
        assert!(
            registry.get(fee_type).is_some(),
            "{fee_type:?} not found in registry"
        );
    }
}
