//! Task 2.6 gate tests — MI schema.
//!
//! Reference scenario: FHA purchase, Kyle TX, $434,443 base loan, 6.375% 30yr.
//!   UFMIP 1.75% = $7,602.75 financed; monthly MIP 0.55%/yr life-of-loan;
//!   24-month minimum collection; declining balance recalculation.

use mismo::{
    enums::mi::{MiFirstPremiumType, MiRenewalType, MismoMiProgramType},
    schema::mi::MiDataDetail,
    MismoError,
};
use types::{BasisPoints, Cents, LtvBasisPoints};

// ── Test helpers ──────────────────────────────────────────────────────────────

/// FHA purchase reference MI detail.
/// UFMIP 1.75%, $7,602.75, financed; monthly 0.55%/yr; 24-month min;
/// cancellation LTV 0.0 (life-of-loan); declining.
fn fha_purchase_mi() -> MiDataDetail {
    MiDataDetail {
        mi_type: "BorrowerPaid".into(),
        mi_program_type: "FHAUpfrontMIP".into(),
        upfront_rate_percent: Some("1.75".into()),
        upfront_amount: Some("7602.7525".into()),
        financed: Some("true".into()),
        monthly_rate_percent: Some("0.55".into()),
        cancellation_ltv_percent: Some("0.0".into()),
        required_months: Some("24".into()),
        calculation_method: Some("Declining".into()),
        remittance_type: Some("AtClosing".into()),
    }
}

fn va_purchase_mi() -> MiDataDetail {
    MiDataDetail {
        mi_type: "BorrowerPaid".into(),
        mi_program_type: "VAFundingFee".into(),
        upfront_rate_percent: Some("2.15".into()),
        upfront_amount: Some("9352.50".into()),
        financed: Some("true".into()),
        monthly_rate_percent: None,
        cancellation_ltv_percent: None,
        required_months: None,
        calculation_method: None,
        remittance_type: Some("AtClosing".into()),
    }
}

fn usda_purchase_mi() -> MiDataDetail {
    MiDataDetail {
        mi_type: "BorrowerPaid".into(),
        mi_program_type: "USDAGuaranteeFee".into(),
        upfront_rate_percent: Some("1.00".into()),
        upfront_amount: Some("2000.00".into()),
        financed: Some("true".into()),
        monthly_rate_percent: Some("0.35".into()),
        cancellation_ltv_percent: None,
        required_months: None,
        calculation_method: Some("Level".into()),
        remittance_type: Some("AtClosing".into()),
    }
}

fn conv_pmi() -> MiDataDetail {
    MiDataDetail {
        mi_type: "BorrowerPaid".into(),
        mi_program_type: "PrivateMI".into(),
        upfront_rate_percent: None,
        upfront_amount: None,
        financed: Some("false".into()),
        monthly_rate_percent: Some("0.68".into()),
        cancellation_ltv_percent: Some("80.0".into()),
        required_months: Some("24".into()),
        calculation_method: Some("Declining".into()),
        remittance_type: None,
    }
}

// ── FHA: program ──────────────────────────────────────────────────────────────

#[test]
fn test_fha_program_type_parses() {
    let p = fha_purchase_mi().parse().unwrap();
    assert_eq!(p.program, MismoMiProgramType::FhaMip);
}

// ── FHA: upfront premium ──────────────────────────────────────────────────────

#[test]
fn test_fha_upfront_rate_175_bps() {
    let p = fha_purchase_mi().parse().unwrap();
    // 1.75% UFMIP = 175 standard finance basis points
    assert_eq!(p.upfront_rate, Some(BasisPoints(175)));
}

#[test]
fn test_fha_upfront_amount_to_cents() {
    // $7,602.7525 → rounds to $7,602.75 = 760,275 cents
    let p = fha_purchase_mi().parse().unwrap();
    assert_eq!(p.upfront_amount, Some(Cents(760_275)));
}

#[test]
fn test_fha_upfront_is_financed() {
    let p = fha_purchase_mi().parse().unwrap();
    assert!(p.is_financed);
}

// ── FHA: monthly MIP ─────────────────────────────────────────────────────────

#[test]
fn test_fha_monthly_rate_55_bps() {
    let p = fha_purchase_mi().parse().unwrap();
    // 0.55% annual = 55 standard finance basis points
    assert_eq!(p.monthly_annual_rate, Some(BasisPoints(55)));
}

#[test]
fn test_fha_required_months_24() {
    let p = fha_purchase_mi().parse().unwrap();
    assert_eq!(p.required_months, Some(24));
}

// ── FHA: life-of-loan detection ───────────────────────────────────────────────

#[test]
fn test_fha_cancellation_ltv_zero_triggers_life_of_loan() {
    let p = fha_purchase_mi().parse().unwrap();
    // cancellation_ltv_percent "0.0" → LtvBasisPoints(0) → is_life_of_loan
    assert!(p.is_life_of_loan);
    assert_eq!(p.cancellation_ltv, Some(LtvBasisPoints::new(0).unwrap()));
}

#[test]
fn test_fha_is_declining() {
    let p = fha_purchase_mi().parse().unwrap();
    assert!(p.is_declining);
}

#[test]
fn test_fha_first_premium_at_closing() {
    let p = fha_purchase_mi().parse().unwrap();
    assert_eq!(p.first_premium_timing, Some(MiFirstPremiumType::AtClosing));
}

// ── Conventional cancellation at 80% LTV ────────────────────────────────────

#[test]
fn test_conv_pmi_cancellation_ltv_80pct() {
    let p = conv_pmi().parse().unwrap();
    assert_eq!(p.cancellation_ltv, Some(LtvBasisPoints::new(8000).unwrap()));
    assert!(!p.is_life_of_loan);
}

#[test]
fn test_conv_pmi_program_type() {
    let p = conv_pmi().parse().unwrap();
    assert_eq!(p.program, MismoMiProgramType::ConventionalPmi);
}

#[test]
fn test_conv_pmi_monthly_rate_68_bps() {
    let p = conv_pmi().parse().unwrap();
    assert_eq!(p.monthly_annual_rate, Some(BasisPoints(68)));
}

#[test]
fn test_conv_pmi_no_upfront() {
    let p = conv_pmi().parse().unwrap();
    assert!(p.upfront_rate.is_none());
    assert!(p.upfront_amount.is_none());
    assert!(!p.is_financed);
}

// ── VA: funding fee, no monthly ──────────────────────────────────────────────

#[test]
fn test_va_program_type() {
    let p = va_purchase_mi().parse().unwrap();
    assert_eq!(p.program, MismoMiProgramType::VaFundingFee);
}

#[test]
fn test_va_upfront_rate_215_bps() {
    let p = va_purchase_mi().parse().unwrap();
    // 2.15% VA funding fee = 215 standard finance basis points
    assert_eq!(p.upfront_rate, Some(BasisPoints(215)));
}

#[test]
fn test_va_no_monthly_premium() {
    let p = va_purchase_mi().parse().unwrap();
    assert!(p.monthly_annual_rate.is_none());
}

#[test]
fn test_va_no_cancellation_ltv() {
    let p = va_purchase_mi().parse().unwrap();
    assert!(p.cancellation_ltv.is_none());
    assert!(!p.is_life_of_loan);
}

#[test]
fn test_va_is_financed() {
    let p = va_purchase_mi().parse().unwrap();
    assert!(p.is_financed);
}

// ── USDA: upfront + annual ────────────────────────────────────────────────────

#[test]
fn test_usda_program_type() {
    let p = usda_purchase_mi().parse().unwrap();
    assert_eq!(p.program, MismoMiProgramType::UsdaGuaranteeFee);
}

#[test]
fn test_usda_upfront_rate_100_bps() {
    let p = usda_purchase_mi().parse().unwrap();
    // 1.00% upfront guarantee fee = 100 standard finance basis points
    assert_eq!(p.upfront_rate, Some(BasisPoints(100)));
}

#[test]
fn test_usda_annual_rate_35_bps() {
    let p = usda_purchase_mi().parse().unwrap();
    // 0.35% annual = 35 standard finance basis points
    assert_eq!(p.monthly_annual_rate, Some(BasisPoints(35)));
}

#[test]
fn test_usda_is_level_not_declining() {
    let p = usda_purchase_mi().parse().unwrap();
    assert!(!p.is_declining);
    assert_eq!(p.renewal_type, Some(MiRenewalType::Level));
}

// ── Error handling ────────────────────────────────────────────────────────────

#[test]
fn test_unknown_mi_program_returns_error() {
    let mut mi = fha_purchase_mi();
    mi.mi_program_type = "AlienInsurance".into();
    let err = mi.parse().unwrap_err();
    assert!(matches!(
        err,
        MismoError::InvalidEnum {
            element: "MIPremiumSourceType",
            ..
        }
    ));
}

#[test]
fn test_invalid_upfront_rate_returns_error() {
    let mut mi = fha_purchase_mi();
    mi.upfront_rate_percent = Some("not_a_number".into());
    let err = mi.parse().unwrap_err();
    assert!(matches!(err, MismoError::OutOfRange { .. }));
}

#[test]
fn test_absent_upfront_fields_are_none() {
    let mut mi = fha_purchase_mi();
    mi.upfront_rate_percent = None;
    mi.upfront_amount = None;
    let p = mi.parse().unwrap();
    assert!(p.upfront_rate.is_none());
    assert!(p.upfront_amount.is_none());
}

// ── MismoMiProgramType predicates ────────────────────────────────────────────

#[test]
fn test_program_has_upfront_predicate() {
    assert!(MismoMiProgramType::FhaMip.has_upfront());
    assert!(MismoMiProgramType::VaFundingFee.has_upfront());
    assert!(MismoMiProgramType::UsdaGuaranteeFee.has_upfront());
    assert!(!MismoMiProgramType::ConventionalPmi.has_upfront());
    assert!(!MismoMiProgramType::None.has_upfront());
}

#[test]
fn test_program_has_monthly_predicate() {
    assert!(MismoMiProgramType::FhaMip.has_monthly());
    assert!(!MismoMiProgramType::VaFundingFee.has_monthly());
    assert!(MismoMiProgramType::UsdaGuaranteeFee.has_monthly());
    assert!(MismoMiProgramType::ConventionalPmi.has_monthly());
    assert!(!MismoMiProgramType::None.has_monthly());
}

// ── XML round-trip ────────────────────────────────────────────────────────────

#[test]
fn test_fha_mi_xml_roundtrip() {
    let mi = fha_purchase_mi();
    let xml = mismo::xml::serialize::to_xml(&mi).unwrap();
    assert!(xml.contains("1.75"));
    assert!(xml.contains("7602.7525"));
    assert!(xml.contains("0.55"));
    assert!(xml.contains("Declining"));

    let restored: MiDataDetail = mismo::xml::parse::from_xml(&xml).unwrap();
    let p = restored.parse().unwrap();
    assert_eq!(p.program, MismoMiProgramType::FhaMip);
    assert_eq!(p.upfront_rate, Some(BasisPoints(175)));
    assert_eq!(p.upfront_amount, Some(Cents(760_275)));
    assert!(p.is_financed);
    assert_eq!(p.monthly_annual_rate, Some(BasisPoints(55)));
    assert!(p.is_life_of_loan);
    assert!(p.is_declining);
}

#[test]
fn test_parse_fha_mi_from_xml_string() {
    let xml = r#"<MI_DATA_DETAIL>
        <MIType>BorrowerPaid</MIType>
        <MIPremiumSourceType>FHAUpfrontMIP</MIPremiumSourceType>
        <MIUpfrontRatePercent>1.75</MIUpfrontRatePercent>
        <MIUpfrontPremiumAmount>7602.7525</MIUpfrontPremiumAmount>
        <MIFinancedIndicator>true</MIFinancedIndicator>
        <MIMonthlyPremiumRatePercent>0.55</MIMonthlyPremiumRatePercent>
        <MICancellationLTVPercent>0.0</MICancellationLTVPercent>
        <MIRequiredMonthsCount>24</MIRequiredMonthsCount>
        <MIPremiumCalculationMethodType>Declining</MIPremiumCalculationMethodType>
        <MIPaymentRemittanceType>AtClosing</MIPaymentRemittanceType>
    </MI_DATA_DETAIL>"#;

    let mi: MiDataDetail = mismo::xml::parse::from_xml(xml).unwrap();
    let p = mi.parse().unwrap();

    assert_eq!(p.program, MismoMiProgramType::FhaMip);
    assert_eq!(p.upfront_rate, Some(BasisPoints(175)));
    assert_eq!(p.upfront_amount, Some(Cents(760_275)));
    assert!(p.is_financed);
    assert_eq!(p.monthly_annual_rate, Some(BasisPoints(55)));
    assert_eq!(p.required_months, Some(24));
    assert!(p.is_life_of_loan);
    assert!(p.is_declining);
    assert_eq!(p.first_premium_timing, Some(MiFirstPremiumType::AtClosing));
}
