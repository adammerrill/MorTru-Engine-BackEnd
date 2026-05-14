//! Task 2.2 gate tests — MISMO enumeration catalog.
//!
//! Verifies every MISMO string → domain type conversion and every new
//! enum type defined in the `enums` module.

use mismo::{
    enums::{
        aus::{try_aus_type, AusRecommendation},
        comp::{CompDisclosure, CompType},
        fee::{FeeCalculationType, FeePaidBy, FeeSection},
        loan_type::{try_amortization_type, try_lien_priority, try_loan_purpose, try_program_code},
        mi::{MiFirstPremiumType, MiRenewalType, MismoMiProgramType},
        party::{AffordableLendingProgram, VaFundingFeeTier},
        property::{try_occupancy, try_property_type},
    },
    MismoError,
};
use types::{
    AmortizationType, AusType, LienPriority, LoanPurpose, LtvBasisPoints, Occupancy, ProgramCode,
    PropertyType,
};

// ── loan_type: LoanPurpose ────────────────────────────────────────────────────

#[test]
fn test_loan_purpose_purchase_from_mismo() {
    assert_eq!(try_loan_purpose("Purchase").unwrap(), LoanPurpose::Purchase);
}

#[test]
fn test_loan_purpose_refinance_from_mismo() {
    assert_eq!(
        try_loan_purpose("Refinance").unwrap(),
        LoanPurpose::RateAndTermRefinance
    );
    // Alias used by some vendors
    assert_eq!(
        try_loan_purpose("LimitedCashOutRefinance").unwrap(),
        LoanPurpose::RateAndTermRefinance
    );
}

#[test]
fn test_loan_purpose_cash_out_refi_from_mismo() {
    assert_eq!(
        try_loan_purpose("CashOutRefinance").unwrap(),
        LoanPurpose::CashOutRefinance
    );
    assert_eq!(
        try_loan_purpose("CashOut").unwrap(),
        LoanPurpose::CashOutRefinance
    );
}

#[test]
fn test_loan_purpose_construction_from_mismo() {
    assert_eq!(
        try_loan_purpose("ConstructionToPermanent").unwrap(),
        LoanPurpose::ConstructionToPermanent
    );
    assert_eq!(
        try_loan_purpose("Construction").unwrap(),
        LoanPurpose::Construction
    );
}

#[test]
fn test_loan_purpose_unknown_returns_invalid_enum_error() {
    let err = try_loan_purpose("NotAPurpose").unwrap_err();
    assert!(
        matches!(
            err,
            MismoError::InvalidEnum {
                element: "LoanPurposeType",
                ..
            }
        ),
        "expected InvalidEnum for LoanPurposeType, got: {err}"
    );
}

// ── loan_type: AmortizationType ───────────────────────────────────────────────

#[test]
fn test_amortization_type_fixed_from_mismo() {
    assert_eq!(
        try_amortization_type("Fixed").unwrap(),
        AmortizationType::Fixed
    );
}

#[test]
fn test_amortization_type_arm_from_mismo() {
    assert_eq!(
        try_amortization_type("AdjustableRate").unwrap(),
        AmortizationType::Arm
    );
    assert_eq!(try_amortization_type("ARM").unwrap(), AmortizationType::Arm);
}

#[test]
fn test_amortization_type_interest_only_from_mismo() {
    assert_eq!(
        try_amortization_type("InterestOnly").unwrap(),
        AmortizationType::InterestOnly
    );
}

#[test]
fn test_amortization_type_unknown_returns_error() {
    let err = try_amortization_type("BalloonPayment").unwrap_err();
    assert!(matches!(
        err,
        MismoError::InvalidEnum {
            element: "AmortizationType",
            ..
        }
    ));
}

// ── loan_type: LienPriority ───────────────────────────────────────────────────

#[test]
fn test_lien_priority_first_lien_from_mismo() {
    assert_eq!(try_lien_priority("FirstLien").unwrap(), LienPriority::First);
}

#[test]
fn test_lien_priority_second_lien_from_mismo() {
    assert_eq!(
        try_lien_priority("SecondLien").unwrap(),
        LienPriority::Second
    );
}

#[test]
fn test_lien_priority_third_lien_from_mismo() {
    assert_eq!(try_lien_priority("ThirdLien").unwrap(), LienPriority::Third);
}

#[test]
fn test_lien_priority_roundtrip() {
    // to_mismo() → try_lien_priority() should produce the original value.
    for &lien in &[
        LienPriority::First,
        LienPriority::Second,
        LienPriority::Third,
    ] {
        let s = lien.to_mismo();
        assert_eq!(
            try_lien_priority(s).unwrap(),
            lien,
            "round-trip failed for {lien:?}"
        );
    }
}

#[test]
fn test_lien_priority_unknown_returns_error() {
    let err = try_lien_priority("FourthLien").unwrap_err();
    assert!(matches!(
        err,
        MismoError::InvalidEnum {
            element: "LienPriorityType",
            ..
        }
    ));
}

// ── loan_type: ProgramCode ────────────────────────────────────────────────────

#[test]
fn test_program_code_fha_from_mismo() {
    assert_eq!(try_program_code("FHA").unwrap(), ProgramCode::Fha);
}

#[test]
fn test_program_code_va_from_mismo() {
    assert_eq!(try_program_code("VA").unwrap(), ProgramCode::Va);
}

#[test]
fn test_program_code_usda_from_mismo() {
    assert_eq!(
        try_program_code("USDARuralDevelopment").unwrap(),
        ProgramCode::Usda
    );
    assert_eq!(try_program_code("USDA").unwrap(), ProgramCode::Usda);
}

#[test]
fn test_program_code_conventional_from_mismo() {
    assert_eq!(
        try_program_code("Conventional").unwrap(),
        ProgramCode::Conventional
    );
}

#[test]
fn test_program_code_unknown_returns_error() {
    let err = try_program_code("NonQM").unwrap_err();
    assert!(matches!(
        err,
        MismoError::InvalidEnum {
            element: "MortgageType",
            ..
        }
    ));
}

// ── property: PropertyType ────────────────────────────────────────────────────

#[test]
fn test_property_type_detached_from_mismo() {
    assert_eq!(
        try_property_type("Detached").unwrap(),
        PropertyType::SingleFamilyDetached
    );
}

#[test]
fn test_property_type_condominium_from_mismo() {
    assert_eq!(
        try_property_type("Condominium").unwrap(),
        PropertyType::Condominium
    );
}

#[test]
fn test_property_type_two_unit_from_mismo() {
    assert_eq!(try_property_type("2-Unit").unwrap(), PropertyType::TwoUnit);
    assert_eq!(
        try_property_type("3-Unit").unwrap(),
        PropertyType::ThreeUnit
    );
    assert_eq!(try_property_type("4-Unit").unwrap(), PropertyType::FourUnit);
}

#[test]
fn test_property_type_manufactured_home_from_mismo() {
    assert_eq!(
        try_property_type("ManufacturedHousing").unwrap(),
        PropertyType::ManufacturedHome
    );
}

#[test]
fn test_property_type_roundtrip() {
    let types = [
        PropertyType::SingleFamilyDetached,
        PropertyType::Condominium,
        PropertyType::PlannedUnitDevelopment,
        PropertyType::TwoUnit,
        PropertyType::ManufacturedHome,
    ];
    for pt in types {
        let s = pt.to_mismo();
        assert_eq!(
            try_property_type(s).unwrap(),
            pt,
            "round-trip failed for {pt:?} (MISMO: {s})"
        );
    }
}

#[test]
fn test_property_type_unknown_returns_error() {
    let err = try_property_type("TimberFrame").unwrap_err();
    assert!(matches!(
        err,
        MismoError::InvalidEnum {
            element: "PropertyStructureType",
            ..
        }
    ));
}

// ── property: Occupancy ───────────────────────────────────────────────────────

#[test]
fn test_occupancy_primary_residence_from_mismo() {
    assert_eq!(
        try_occupancy("PrimaryResidence").unwrap(),
        Occupancy::PrimaryResidence
    );
}

#[test]
fn test_occupancy_second_home_from_mismo() {
    assert_eq!(try_occupancy("SecondHome").unwrap(), Occupancy::SecondHome);
}

#[test]
fn test_occupancy_investment_from_mismo() {
    assert_eq!(try_occupancy("Investor").unwrap(), Occupancy::Investment);
}

#[test]
fn test_occupancy_unknown_returns_error() {
    let err = try_occupancy("Vacation").unwrap_err();
    assert!(matches!(
        err,
        MismoError::InvalidEnum {
            element: "PropertyUsageType",
            ..
        }
    ));
}

// ── party: VaFundingFeeTier ───────────────────────────────────────────────────

#[test]
fn test_va_funding_fee_first_use_below_5pct_is_215_bps() {
    let ltv = LtvBasisPoints::new(9650).unwrap(); // 96.5% LTV = < 5% down
    let tier = VaFundingFeeTier::from_inputs(true, ltv, false, false, false);
    assert_eq!(tier, VaFundingFeeTier::FirstUseBelow5Pct);
    assert_eq!(tier.rate_bps().0, 215);
}

#[test]
fn test_va_funding_fee_first_use_5_to_10pct_is_150_bps() {
    let ltv = LtvBasisPoints::new(9500).unwrap(); // 95% LTV = exactly 5% down boundary
    let tier = VaFundingFeeTier::from_inputs(true, ltv, false, false, false);
    assert_eq!(tier, VaFundingFeeTier::FirstUse5To10Pct);
    assert_eq!(tier.rate_bps().0, 150);
}

#[test]
fn test_va_funding_fee_first_use_above_10pct_is_125_bps() {
    let ltv = LtvBasisPoints::new(9000).unwrap(); // 90% LTV = exactly 10% down boundary
    let tier = VaFundingFeeTier::from_inputs(true, ltv, false, false, false);
    assert_eq!(tier, VaFundingFeeTier::FirstUseAbove10Pct);
    assert_eq!(tier.rate_bps().0, 125);
}

#[test]
fn test_va_funding_fee_subsequent_below_5pct_is_330_bps() {
    let ltv = LtvBasisPoints::new(9700).unwrap();
    let tier = VaFundingFeeTier::from_inputs(false, ltv, false, false, false);
    assert_eq!(tier, VaFundingFeeTier::SubsequentBelow5Pct);
    assert_eq!(tier.rate_bps().0, 330);
}

#[test]
fn test_va_funding_fee_exempt_returns_zero_bps() {
    let ltv = LtvBasisPoints::new(9700).unwrap();
    let tier = VaFundingFeeTier::from_inputs(true, ltv, false, false, true);
    assert_eq!(tier, VaFundingFeeTier::Exempt);
    assert_eq!(tier.rate_bps().0, 0);
}

#[test]
fn test_va_funding_fee_irrrl_is_50_bps() {
    let ltv = LtvBasisPoints::new(8000).unwrap();
    let tier = VaFundingFeeTier::from_inputs(false, ltv, false, true, false);
    assert_eq!(tier, VaFundingFeeTier::Irrrl);
    assert_eq!(tier.rate_bps().0, 50);
}

#[test]
fn test_va_funding_fee_cash_out_refi_first_use_is_215_bps() {
    let ltv = LtvBasisPoints::new(9000).unwrap();
    let tier = VaFundingFeeTier::from_inputs(true, ltv, true, false, false);
    assert_eq!(tier, VaFundingFeeTier::CashOutRefiFirstUse);
    assert_eq!(tier.rate_bps().0, 215);
}

#[test]
fn test_va_funding_fee_cash_out_refi_subsequent_is_330_bps() {
    let ltv = LtvBasisPoints::new(9000).unwrap();
    let tier = VaFundingFeeTier::from_inputs(false, ltv, true, false, false);
    assert_eq!(tier, VaFundingFeeTier::CashOutRefiSubsequent);
    assert_eq!(tier.rate_bps().0, 330);
}

// ── party: AffordableLendingProgram ───────────────────────────────────────────

#[test]
fn test_affordable_lending_homeready_from_str() {
    assert_eq!(
        AffordableLendingProgram::try_from_str("HomeReady").unwrap(),
        AffordableLendingProgram::HomeReady
    );
}

#[test]
fn test_affordable_lending_homepossible_from_str() {
    assert_eq!(
        AffordableLendingProgram::try_from_str("HomePossible").unwrap(),
        AffordableLendingProgram::HomePossible
    );
}

#[test]
fn test_affordable_lending_none_from_empty_string() {
    assert_eq!(
        AffordableLendingProgram::try_from_str("").unwrap(),
        AffordableLendingProgram::None
    );
    assert_eq!(
        AffordableLendingProgram::try_from_str("None").unwrap(),
        AffordableLendingProgram::None
    );
}

// ── mi: MismoMiProgramType ────────────────────────────────────────────────────

#[test]
fn test_mi_program_fha_from_mismo() {
    let p = MismoMiProgramType::try_from_str("FHAUpfrontMIP").unwrap();
    assert_eq!(p, MismoMiProgramType::FhaMip);
    assert!(p.has_upfront());
    assert!(p.has_monthly());
}

#[test]
fn test_mi_program_va_from_mismo() {
    let p = MismoMiProgramType::try_from_str("VAFundingFee").unwrap();
    assert_eq!(p, MismoMiProgramType::VaFundingFee);
    assert!(p.has_upfront());
    assert!(!p.has_monthly()); // VA has no monthly MI
}

#[test]
fn test_mi_program_usda_from_mismo() {
    let p = MismoMiProgramType::try_from_str("USDAGuaranteeFee").unwrap();
    assert_eq!(p, MismoMiProgramType::UsdaGuaranteeFee);
    assert!(p.has_upfront());
    assert!(p.has_monthly());
}

#[test]
fn test_mi_program_conventional_pmi_from_mismo() {
    let p = MismoMiProgramType::try_from_str("PrivateMI").unwrap();
    assert_eq!(p, MismoMiProgramType::ConventionalPmi);
    assert!(!p.has_upfront()); // PMI has no upfront
    assert!(p.has_monthly());
}

#[test]
fn test_mi_program_none_from_empty() {
    assert_eq!(
        MismoMiProgramType::try_from_str("None").unwrap(),
        MismoMiProgramType::None
    );
}

// ── mi: MiRenewalType ─────────────────────────────────────────────────────────

#[test]
fn test_mi_renewal_type_declining_from_str() {
    assert_eq!(
        MiRenewalType::try_from_str("Declining").unwrap(),
        MiRenewalType::Declining
    );
}

#[test]
fn test_mi_renewal_type_eleven_year_from_str() {
    assert_eq!(
        MiRenewalType::try_from_str("ElevenYear").unwrap(),
        MiRenewalType::ElevenYear
    );
}

#[test]
fn test_mi_renewal_type_unknown_returns_error() {
    let err = MiRenewalType::try_from_str("Quarterly").unwrap_err();
    assert!(matches!(
        err,
        MismoError::InvalidEnum {
            element: "MIPremiumRenewalType",
            ..
        }
    ));
}

// ── mi: MiFirstPremiumType ────────────────────────────────────────────────────

#[test]
fn test_mi_first_premium_at_closing_from_str() {
    assert_eq!(
        MiFirstPremiumType::try_from_str("AtClosing").unwrap(),
        MiFirstPremiumType::AtClosing
    );
}

// ── aus: AusType ──────────────────────────────────────────────────────────────

#[test]
fn test_aus_type_du_from_mismo() {
    assert_eq!(
        try_aus_type("DesktopUnderwriter").unwrap(),
        AusType::DesktopUnderwriter
    );
}

#[test]
fn test_aus_type_lpa_from_mismo() {
    assert_eq!(
        try_aus_type("LoanProductAdvisor").unwrap(),
        AusType::LoanProductAdvisor
    );
}

#[test]
fn test_aus_type_fha_total_scorecard_from_mismo() {
    assert_eq!(try_aus_type("FHATotalScorecard").unwrap(), AusType::Got);
}

#[test]
fn test_aus_type_roundtrip() {
    let types = [
        AusType::DesktopUnderwriter,
        AusType::LoanProductAdvisor,
        AusType::Got,
        AusType::Gus,
        AusType::Manual,
    ];
    for t in types {
        let s = t.to_mismo();
        assert_eq!(try_aus_type(s).unwrap(), t, "round-trip failed for {t:?}");
    }
}

#[test]
fn test_aus_type_unknown_returns_error() {
    let err = try_aus_type("Proprietary").unwrap_err();
    assert!(matches!(
        err,
        MismoError::InvalidEnum {
            element: "AutomatedUnderwritingSystemType",
            ..
        }
    ));
}

// ── aus: AusRecommendation ────────────────────────────────────────────────────

#[test]
fn test_aus_recommendation_approve_eligible_from_str() {
    assert_eq!(
        AusRecommendation::try_from_str("ApproveEligible").unwrap(),
        AusRecommendation::ApproveEligible
    );
    // DU uses both forms
    assert_eq!(
        AusRecommendation::try_from_str("Approve").unwrap(),
        AusRecommendation::ApproveEligible
    );
    // LPA uses Accept
    assert_eq!(
        AusRecommendation::try_from_str("Accept").unwrap(),
        AusRecommendation::ApproveEligible
    );
}

#[test]
fn test_aus_recommendation_refer_from_str() {
    assert_eq!(
        AusRecommendation::try_from_str("Refer").unwrap(),
        AusRecommendation::Refer
    );
    assert_eq!(
        AusRecommendation::try_from_str("Caution").unwrap(),
        AusRecommendation::Refer
    );
}

#[test]
fn test_aus_recommendation_ineligible_from_str() {
    assert_eq!(
        AusRecommendation::try_from_str("Ineligible").unwrap(),
        AusRecommendation::Ineligible
    );
}

#[test]
fn test_aus_recommendation_is_approvable() {
    assert!(AusRecommendation::ApproveEligible.is_approvable());
    assert!(AusRecommendation::ApproveIneligible.is_approvable());
    assert!(!AusRecommendation::Refer.is_approvable());
    assert!(!AusRecommendation::ReferWithCaution.is_approvable());
    assert!(!AusRecommendation::Ineligible.is_approvable());
}

// ── fee: FeeSection ───────────────────────────────────────────────────────────

#[test]
fn test_fee_section_a_from_mismo_string() {
    assert_eq!(
        FeeSection::try_from_str("LoanCosts_OriginationCharges").unwrap(),
        FeeSection::A
    );
    assert_eq!(FeeSection::try_from_str("A").unwrap(), FeeSection::A);
}

#[test]
fn test_fee_section_b_from_mismo_string() {
    assert_eq!(
        FeeSection::try_from_str("LoanCosts_ServicesNotShoppedFor").unwrap(),
        FeeSection::B
    );
}

#[test]
fn test_fee_section_is_loan_cost_classification() {
    assert!(FeeSection::A.is_loan_cost());
    assert!(FeeSection::B.is_loan_cost());
    assert!(FeeSection::C.is_loan_cost());
    assert!(!FeeSection::E.is_loan_cost());
    assert!(!FeeSection::G.is_loan_cost());
}

#[test]
fn test_fee_section_is_other_cost_classification() {
    assert!(FeeSection::E.is_other_cost());
    assert!(FeeSection::F.is_other_cost());
    assert!(FeeSection::G.is_other_cost());
    assert!(FeeSection::H.is_other_cost());
    assert!(!FeeSection::A.is_other_cost());
}

#[test]
fn test_fee_section_labels_are_single_letters() {
    for (section, label) in [
        (FeeSection::A, "A"),
        (FeeSection::B, "B"),
        (FeeSection::C, "C"),
        (FeeSection::E, "E"),
        (FeeSection::F, "F"),
        (FeeSection::G, "G"),
        (FeeSection::H, "H"),
    ] {
        assert_eq!(section.label(), label);
    }
}

#[test]
fn test_fee_section_unknown_returns_error() {
    let err = FeeSection::try_from_str("SectionD").unwrap_err();
    assert!(matches!(
        err,
        MismoError::InvalidEnum {
            element: "IntegratedDisclosureSectionType",
            ..
        }
    ));
}

// ── fee: FeePaidBy ────────────────────────────────────────────────────────────

#[test]
fn test_fee_paid_by_borrower_from_str() {
    assert_eq!(
        FeePaidBy::try_from_str("Borrower").unwrap(),
        FeePaidBy::Borrower
    );
    assert_eq!(
        FeePaidBy::try_from_str("BorrowerPaid").unwrap(),
        FeePaidBy::Borrower
    );
}

#[test]
fn test_fee_paid_by_seller_from_str() {
    assert_eq!(
        FeePaidBy::try_from_str("Seller").unwrap(),
        FeePaidBy::Seller
    );
    assert_eq!(
        FeePaidBy::try_from_str("SellerPaid").unwrap(),
        FeePaidBy::Seller
    );
}

#[test]
fn test_fee_paid_by_lender_from_str() {
    assert_eq!(
        FeePaidBy::try_from_str("Lender").unwrap(),
        FeePaidBy::Lender
    );
}

// ── fee: FeeCalculationType ───────────────────────────────────────────────────

#[test]
fn test_fee_calculation_type_formula_from_str() {
    assert_eq!(
        FeeCalculationType::try_from_str("Formula").unwrap(),
        FeeCalculationType::Formula
    );
}

#[test]
fn test_fee_calculation_type_numerical_from_str() {
    assert_eq!(
        FeeCalculationType::try_from_str("Numerical").unwrap(),
        FeeCalculationType::Numerical
    );
}

// ── comp: CompType ────────────────────────────────────────────────────────────

#[test]
fn test_comp_type_borrower_paid_from_str() {
    assert_eq!(
        CompType::try_from_str("BorrowerPaid").unwrap(),
        CompType::BorrowerPaid
    );
    assert_eq!(
        CompType::try_from_str("Borrower").unwrap(),
        CompType::BorrowerPaid
    );
}

#[test]
fn test_comp_type_lender_paid_from_str() {
    assert_eq!(
        CompType::try_from_str("LenderPaid").unwrap(),
        CompType::LenderPaid
    );
}

#[test]
fn test_comp_type_borrower_paid_disclosed_in_section_a() {
    assert!(CompType::BorrowerPaid.disclosed_in_section_a());
    assert!(!CompType::LenderPaid.disclosed_in_section_a());
}

#[test]
fn test_comp_disclosure_from_comp_type() {
    assert_eq!(
        CompDisclosure::from_comp_type(CompType::BorrowerPaid),
        CompDisclosure::InSectionA
    );
    assert_eq!(
        CompDisclosure::from_comp_type(CompType::LenderPaid),
        CompDisclosure::OnPage3
    );
}

#[test]
fn test_comp_type_unknown_returns_error() {
    let err = CompType::try_from_str("AgentPaid").unwrap_err();
    assert!(matches!(
        err,
        MismoError::InvalidEnum {
            element: "CompensationType",
            ..
        }
    ));
}

// ── Coverage gap fill: every enum variant that lacked a direct test ──────────
// These tests exercise all match arms not hit by the main test matrix above,
// preventing the llvm-cov --fail-under-lines 97 gate from firing.

#[test]
fn test_affordable_lending_homeone_from_str() {
    assert_eq!(
        AffordableLendingProgram::try_from_str("HomeOne").unwrap(),
        AffordableLendingProgram::HomeOne
    );
}

#[test]
fn test_mi_renewal_type_level_from_str() {
    assert_eq!(
        MiRenewalType::try_from_str("Level").unwrap(),
        MiRenewalType::Level
    );
}

#[test]
fn test_mi_renewal_type_annual_from_str() {
    assert_eq!(
        MiRenewalType::try_from_str("Annual").unwrap(),
        MiRenewalType::Annual
    );
}

#[test]
fn test_mi_first_premium_first_payment_from_str() {
    assert_eq!(
        MiFirstPremiumType::try_from_str("FirstPayment").unwrap(),
        MiFirstPremiumType::FirstPayment
    );
}

#[test]
fn test_mi_first_premium_deferred_from_str() {
    assert_eq!(
        MiFirstPremiumType::try_from_str("Deferred").unwrap(),
        MiFirstPremiumType::Deferred
    );
}

#[test]
fn test_aus_recommendation_approve_ineligible_from_str() {
    let r = AusRecommendation::try_from_str("ApproveIneligible").unwrap();
    assert_eq!(r, AusRecommendation::ApproveIneligible);
    assert!(r.is_approvable());
}

#[test]
fn test_aus_recommendation_refer_with_caution_from_str() {
    let r = AusRecommendation::try_from_str("ReferWithCaution").unwrap();
    assert_eq!(r, AusRecommendation::ReferWithCaution);
    assert!(!r.is_approvable());
}

#[test]
fn test_aus_recommendation_out_of_scope_maps_to_ineligible() {
    assert_eq!(
        AusRecommendation::try_from_str("OutOfScope").unwrap(),
        AusRecommendation::Ineligible
    );
}

#[test]
fn test_fee_section_c_e_f_g_h_from_mismo_string() {
    assert_eq!(
        FeeSection::try_from_str("LoanCosts_ServicesShoppedFor").unwrap(),
        FeeSection::C
    );
    assert_eq!(FeeSection::try_from_str("C").unwrap(), FeeSection::C);
    assert_eq!(
        FeeSection::try_from_str("OtherCosts_TaxesAndGovernmentFees").unwrap(),
        FeeSection::E
    );
    assert_eq!(FeeSection::try_from_str("E").unwrap(), FeeSection::E);
    assert_eq!(
        FeeSection::try_from_str("OtherCosts_Prepaids").unwrap(),
        FeeSection::F
    );
    assert_eq!(FeeSection::try_from_str("F").unwrap(), FeeSection::F);
    assert_eq!(
        FeeSection::try_from_str("OtherCosts_InitialEscrowPayment").unwrap(),
        FeeSection::G
    );
    assert_eq!(FeeSection::try_from_str("G").unwrap(), FeeSection::G);
    assert_eq!(
        FeeSection::try_from_str("OtherCosts_Other").unwrap(),
        FeeSection::H
    );
    assert_eq!(FeeSection::try_from_str("H").unwrap(), FeeSection::H);
}

#[test]
fn test_fee_paid_by_other_from_str() {
    assert_eq!(FeePaidBy::try_from_str("Other").unwrap(), FeePaidBy::Other);
    assert_eq!(
        FeePaidBy::try_from_str("PaidByOther").unwrap(),
        FeePaidBy::Other
    );
}

#[test]
fn test_fee_calculation_type_unavailable_from_str() {
    assert_eq!(
        FeeCalculationType::try_from_str("Unavailable").unwrap(),
        FeeCalculationType::Unavailable
    );
}

#[test]
fn test_comp_type_split_from_str() {
    assert_eq!(CompType::try_from_str("Split").unwrap(), CompType::Split);
}

#[test]
fn test_comp_disclosure_split_routes_to_page3() {
    assert_eq!(
        CompDisclosure::from_comp_type(CompType::Split),
        CompDisclosure::OnPage3
    );
}
