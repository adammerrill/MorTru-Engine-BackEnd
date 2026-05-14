//! Task 2.5 gate tests — party/borrower schema.
//!
//! Verifies `PartiesParsed::parse()` for the FHA scenario (single borrower,
//! credit 720) and for VA/USDA/affordable-lending variants.

use mismo::{
    schema::party::{BorrowerDetail, ClosingContext, PartiesParsed},
    MismoError,
};
use types::{Cents, CreditScore, DtiBasisPoints, LtvBasisPoints};

use mismo::enums::party::{AffordableLendingProgram, VaFundingFeeTier};

// ── Test helpers ──────────────────────────────────────────────────────────────

fn fha_borrower() -> BorrowerDetail {
    BorrowerDetail {
        credit_score: Some("720".into()),
        monthly_income: Some("8500.00".into()),
        first_time_homebuyer: None,
        experienced_homebuyer: None,
        self_employed: None,
        va_eligible: None,
        va_first_use: None,
        va_full_entitlement: None,
        va_outstanding_balance: None,
        va_fee_exempt: None,
        usda_household_size: None,
        usda_adult_household_income: None,
        affordable_lending_eligible: None,
        affordable_lending_program: None,
        max_cash_to_close: None,
        max_monthly_pitia: None,
    }
}

fn no_closing() -> Option<&'static ClosingContext> {
    None
}

// ── Single borrower: FHA scenario ────────────────────────────────────────────

#[test]
fn test_single_borrower_count() {
    let p = PartiesParsed::parse(&fha_borrower(), None, no_closing()).unwrap();
    assert_eq!(p.borrower_count, 1);
}

#[test]
fn test_credit_score_720_parses() {
    let p = PartiesParsed::parse(&fha_borrower(), None, no_closing()).unwrap();
    assert_eq!(
        p.qualifying_credit_score,
        Some(CreditScore::new(720).unwrap())
    );
}

#[test]
fn test_monthly_income_to_cents() {
    let p = PartiesParsed::parse(&fha_borrower(), None, no_closing()).unwrap();
    // $8,500.00 = 850,000 cents
    assert_eq!(p.monthly_gross_income, Some(Cents(850_000)));
}

#[test]
fn test_no_credit_score_returns_none() {
    let mut b = fha_borrower();
    b.credit_score = None;
    let p = PartiesParsed::parse(&b, None, no_closing()).unwrap();
    assert!(p.qualifying_credit_score.is_none());
}

#[test]
fn test_credit_score_out_of_range_returns_error() {
    let mut b = fha_borrower();
    b.credit_score = Some("200".into()); // below 300 minimum
    let err = PartiesParsed::parse(&b, None, no_closing()).unwrap_err();
    assert!(matches!(
        err,
        MismoError::OutOfRange {
            element: "CreditScoreValue",
            ..
        }
    ));
}

#[test]
fn test_credit_score_850_is_max_valid() {
    let mut b = fha_borrower();
    b.credit_score = Some("850".into());
    let p = PartiesParsed::parse(&b, None, no_closing()).unwrap();
    assert_eq!(
        p.qualifying_credit_score,
        Some(CreditScore::new(850).unwrap())
    );
}

#[test]
fn test_first_time_homebuyer_flag_true() {
    let mut b = fha_borrower();
    b.first_time_homebuyer = Some("true".into());
    let p = PartiesParsed::parse(&b, None, no_closing()).unwrap();
    assert!(p.first_time_homebuyer);
}

#[test]
fn test_first_time_homebuyer_defaults_false() {
    let p = PartiesParsed::parse(&fha_borrower(), None, no_closing()).unwrap();
    assert!(!p.first_time_homebuyer);
}

// ── Co-borrower: lower score wins ────────────────────────────────────────────

#[test]
fn test_two_borrower_count() {
    let co = fha_borrower();
    let p = PartiesParsed::parse(&fha_borrower(), Some(&co), no_closing()).unwrap();
    assert_eq!(p.borrower_count, 2);
}

#[test]
fn test_coborrower_uses_lower_credit_score() {
    let mut primary = fha_borrower();
    primary.credit_score = Some("750".into());
    let mut secondary = fha_borrower();
    secondary.credit_score = Some("680".into());
    let p = PartiesParsed::parse(&primary, Some(&secondary), no_closing()).unwrap();
    // Industry convention: lower of two scores
    assert_eq!(
        p.qualifying_credit_score,
        Some(CreditScore::new(680).unwrap())
    );
}

#[test]
fn test_coborrower_income_summed() {
    let mut co = fha_borrower();
    co.monthly_income = Some("4000.00".into());
    let p = PartiesParsed::parse(&fha_borrower(), Some(&co), no_closing()).unwrap();
    // $8,500 + $4,000 = $12,500 = 1,250,000 cents
    assert_eq!(p.monthly_gross_income, Some(Cents(1_250_000)));
}

// ── VA borrower ───────────────────────────────────────────────────────────────

#[test]
fn test_va_eligible_flag() {
    let mut b = fha_borrower();
    b.va_eligible = Some("true".into());
    let p = PartiesParsed::parse(&b, None, no_closing()).unwrap();
    assert!(p.va_eligible);
}

#[test]
fn test_va_first_use_true() {
    let mut b = fha_borrower();
    b.va_eligible = Some("true".into());
    b.va_first_use = Some("true".into());
    let p = PartiesParsed::parse(&b, None, no_closing()).unwrap();
    assert!(p.va_eligible);
    assert!(p.va_first_use);
}

#[test]
fn test_va_full_entitlement_flag() {
    let mut b = fha_borrower();
    b.va_full_entitlement = Some("true".into());
    let p = PartiesParsed::parse(&b, None, no_closing()).unwrap();
    assert!(p.va_full_entitlement);
}

#[test]
fn test_va_outstanding_balance_to_cents() {
    let mut b = fha_borrower();
    b.va_outstanding_balance = Some("150000.00".into());
    let p = PartiesParsed::parse(&b, None, no_closing()).unwrap();
    assert_eq!(p.va_outstanding_balance, Some(Cents(15_000_000)));
}

#[test]
fn test_va_exempt_flag() {
    let mut b = fha_borrower();
    b.va_eligible = Some("true".into());
    b.va_fee_exempt = Some("true".into());
    let p = PartiesParsed::parse(&b, None, no_closing()).unwrap();
    assert!(p.va_fee_exempt);
}

#[test]
fn test_va_funding_fee_tier_none_before_ltv_known() {
    let mut b = fha_borrower();
    b.va_eligible = Some("true".into());
    b.va_first_use = Some("true".into());
    let p = PartiesParsed::parse(&b, None, no_closing()).unwrap();
    // Tier is None until LTV is attached
    assert!(p.va_funding_fee_tier.is_none());
}

#[test]
fn test_va_funding_fee_tier_derived_from_inputs() {
    let mut b = fha_borrower();
    b.va_eligible = Some("true".into());
    b.va_first_use = Some("true".into());
    let p = PartiesParsed::parse(&b, None, no_closing()).unwrap();
    // LTV 96.5% = first use below 5% down → 215 bps
    let ltv = LtvBasisPoints::new(9650).unwrap();
    let p = p.with_va_tier(ltv, false, false);
    assert_eq!(
        p.va_funding_fee_tier,
        Some(VaFundingFeeTier::FirstUseBelow5Pct)
    );
    assert_eq!(p.va_funding_fee_tier.unwrap().rate_bps().0, 215);
}

#[test]
fn test_va_exempt_tier_is_zero() {
    let mut b = fha_borrower();
    b.va_eligible = Some("true".into());
    b.va_fee_exempt = Some("true".into());
    let p = PartiesParsed::parse(&b, None, no_closing()).unwrap();
    let ltv = LtvBasisPoints::new(9650).unwrap();
    let p = p.with_va_tier(ltv, false, false);
    assert_eq!(p.va_funding_fee_tier, Some(VaFundingFeeTier::Exempt));
    assert_eq!(p.va_funding_fee_tier.unwrap().rate_bps().0, 0);
}

#[test]
fn test_non_va_borrower_tier_stays_none_after_with_va_tier() {
    // va_eligible = false → tier stays None even after with_va_tier
    let p = PartiesParsed::parse(&fha_borrower(), None, no_closing()).unwrap();
    let ltv = LtvBasisPoints::new(9650).unwrap();
    let p = p.with_va_tier(ltv, false, false);
    assert!(p.va_funding_fee_tier.is_none());
}

// ── USDA borrower ─────────────────────────────────────────────────────────────

#[test]
fn test_usda_household_size_parses() {
    let mut b = fha_borrower();
    b.usda_household_size = Some("4".into());
    let p = PartiesParsed::parse(&b, None, no_closing()).unwrap();
    assert_eq!(p.usda_household_size, Some(4));
}

#[test]
fn test_usda_adult_income_to_cents() {
    let mut b = fha_borrower();
    b.usda_adult_household_income = Some("72000.00".into());
    let p = PartiesParsed::parse(&b, None, no_closing()).unwrap();
    // $72,000.00 annual = 7,200,000 cents
    assert_eq!(p.usda_adult_household_income, Some(Cents(7_200_000)));
}

#[test]
fn test_usda_fields_absent_when_not_provided() {
    let p = PartiesParsed::parse(&fha_borrower(), None, no_closing()).unwrap();
    assert!(p.usda_household_size.is_none());
    assert!(p.usda_adult_household_income.is_none());
}

// ── Affordable lending ────────────────────────────────────────────────────────

#[test]
fn test_affordable_lending_eligible_flag() {
    let mut b = fha_borrower();
    b.affordable_lending_eligible = Some("true".into());
    b.affordable_lending_program = Some("HomeReady".into());
    let p = PartiesParsed::parse(&b, None, no_closing()).unwrap();
    assert!(p.affordable_lending_eligible);
    assert_eq!(
        p.affordable_lending_program,
        AffordableLendingProgram::HomeReady
    );
}

#[test]
fn test_affordable_lending_defaults_none_when_absent() {
    let p = PartiesParsed::parse(&fha_borrower(), None, no_closing()).unwrap();
    assert!(!p.affordable_lending_eligible);
    assert_eq!(p.affordable_lending_program, AffordableLendingProgram::None);
}

#[test]
fn test_home_possible_program_parses() {
    let mut b = fha_borrower();
    b.affordable_lending_program = Some("HomePossible".into());
    let p = PartiesParsed::parse(&b, None, no_closing()).unwrap();
    assert_eq!(
        p.affordable_lending_program,
        AffordableLendingProgram::HomePossible
    );
}

// ── Budget constraints ────────────────────────────────────────────────────────

#[test]
fn test_max_cash_to_close_budget_to_cents() {
    let mut b = fha_borrower();
    b.max_cash_to_close = Some("30000.00".into());
    let p = PartiesParsed::parse(&b, None, no_closing()).unwrap();
    assert_eq!(p.max_cash_to_close, Some(Cents(3_000_000)));
}

#[test]
fn test_max_monthly_pitia_to_cents() {
    let mut b = fha_borrower();
    b.max_monthly_pitia = Some("2800.00".into());
    let p = PartiesParsed::parse(&b, None, no_closing()).unwrap();
    assert_eq!(p.max_monthly_pitia, Some(Cents(280_000)));
}

#[test]
fn test_budget_constraints_absent_when_not_provided() {
    let p = PartiesParsed::parse(&fha_borrower(), None, no_closing()).unwrap();
    assert!(p.max_cash_to_close.is_none());
    assert!(p.max_monthly_pitia.is_none());
}

// ── Closing context ───────────────────────────────────────────────────────────

fn tx_closing() -> ClosingContext {
    ClosingContext {
        earnest_money: Some("5000.00".into()),
        option_fee: Some("200.00".into()),
        target_dti: Some("43.0".into()),
        requested_term_months: None,
    }
}

#[test]
fn test_earnest_money_to_cents() {
    let ctx = tx_closing();
    let p = PartiesParsed::parse(&fha_borrower(), None, Some(&ctx)).unwrap();
    // $5,000 = 500,000 cents
    assert_eq!(p.earnest_money, Some(Cents(500_000)));
}

#[test]
fn test_option_fee_to_cents() {
    let ctx = tx_closing();
    let p = PartiesParsed::parse(&fha_borrower(), None, Some(&ctx)).unwrap();
    // $200 = 20,000 cents
    assert_eq!(p.option_fee, Some(Cents(20_000)));
}

#[test]
fn test_target_dti_to_dti_basis_points() {
    let ctx = tx_closing();
    let p = PartiesParsed::parse(&fha_borrower(), None, Some(&ctx)).unwrap();
    // 43.0% = DtiBasisPoints(4300)
    assert_eq!(p.target_dti, Some(DtiBasisPoints::new(4300)));
}

#[test]
fn test_closing_context_absent_fields_are_none() {
    let p = PartiesParsed::parse(&fha_borrower(), None, no_closing()).unwrap();
    assert!(p.earnest_money.is_none());
    assert!(p.option_fee.is_none());
    assert!(p.target_dti.is_none());
}

// ── XML round-trip ────────────────────────────────────────────────────────────

#[test]
fn test_borrower_detail_xml_roundtrip() {
    let mut b = fha_borrower();
    b.first_time_homebuyer = Some("true".into());
    b.max_cash_to_close = Some("30000.00".into());

    let xml = mismo::xml::serialize::to_xml(&b).unwrap();
    assert!(xml.contains("720"));
    assert!(xml.contains("8500.00"));
    assert!(xml.contains("true"));

    let restored: BorrowerDetail = mismo::xml::parse::from_xml(&xml).unwrap();
    let p = PartiesParsed::parse(&restored, None, no_closing()).unwrap();
    assert_eq!(
        p.qualifying_credit_score,
        Some(CreditScore::new(720).unwrap())
    );
    assert_eq!(p.monthly_gross_income, Some(Cents(850_000)));
    assert!(p.first_time_homebuyer);
}

#[test]
fn test_closing_context_xml_roundtrip() {
    let ctx = tx_closing();
    let xml = mismo::xml::serialize::to_xml(&ctx).unwrap();
    assert!(xml.contains("5000.00"));
    assert!(xml.contains("200.00"));
    assert!(xml.contains("43.0"));

    let restored: ClosingContext = mismo::xml::parse::from_xml(&xml).unwrap();
    let p = PartiesParsed::parse(&fha_borrower(), None, Some(&restored)).unwrap();
    assert_eq!(p.earnest_money, Some(Cents(500_000)));
    assert_eq!(p.option_fee, Some(Cents(20_000)));
    assert_eq!(p.target_dti, Some(DtiBasisPoints::new(4300)));
}
