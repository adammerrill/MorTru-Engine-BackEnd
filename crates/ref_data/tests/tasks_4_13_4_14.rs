//! Tasks 4.13 (FHA MIP rates) and 4.14 (VA funding fee).

use ref_data::{
    FhaMipInput, FhaMipResult, JsonFileStore, MipDuration, RefDataStore, VaFeeInput, VaLoanPurpose,
    VaUse, VeteranCategory,
};
use types::Cents;

fn store() -> JsonFileStore {
    JsonFileStore::new(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data"))
}

fn fha(term_months: u16, ltv_bps: u32, base_loan_cents: i64) -> FhaMipInput {
    FhaMipInput {
        term_months,
        ltv_bps,
        base_loan_cents,
        is_streamline_pre_2009: false,
    }
}

// ════════════════════════════════════════════════════════════════════════════
// TASK 4.13 — FHA MIP Rates
// ════════════════════════════════════════════════════════════════════════════

// ── UFMIP ────────────────────────────────────────────────────────────────────

#[test]
fn test_fha_ufmip_standard_is_175_bps() {
    let r = store().fha_mip(&fha(360, 9650, 45_900_000), 2025).unwrap();
    assert_eq!(r.ufmip_bps, 175, "standard UFMIP must be 1.75%");
}

#[test]
fn test_fha_ufmip_streamline_pre2009_is_1_bp() {
    let input = FhaMipInput {
        term_months: 360,
        ltv_bps: 8500,
        base_loan_cents: 40_000_000,
        is_streamline_pre_2009: true,
    };
    let r = store().fha_mip(&input, 2025).unwrap();
    assert_eq!(r.ufmip_bps, 1, "streamline pre-2009 UFMIP must be 0.01%");
}

// ── Annual MIP: term > 15 years, standard balance ────────────────────────────

#[test]
fn test_fha_annual_mip_30yr_standard_ltv_le90_is_50bps_11yr() {
    let r = store().fha_mip(&fha(360, 8800, 45_900_000), 2025).unwrap();
    assert_eq!(r.annual_mip_bps, 50);
    assert_eq!(r.duration, MipDuration::Years(11));
}

#[test]
fn test_fha_annual_mip_30yr_standard_ltv_le95_is_50bps_loan_term() {
    let r = store().fha_mip(&fha(360, 9200, 45_900_000), 2025).unwrap();
    assert_eq!(r.annual_mip_bps, 50);
    assert_eq!(r.duration, MipDuration::LoanTerm);
}

#[test]
fn test_fha_annual_mip_30yr_standard_ltv_over95_is_55bps_loan_term() {
    let r = store().fha_mip(&fha(360, 9650, 45_900_000), 2025).unwrap();
    assert_eq!(r.annual_mip_bps, 55);
    assert_eq!(r.duration, MipDuration::LoanTerm);
}

// ── Annual MIP: term > 15 years, high-balance ────────────────────────────────

#[test]
fn test_fha_annual_mip_30yr_high_balance_ltv_le90_is_70bps_11yr() {
    let r = store().fha_mip(&fha(360, 8800, 80_000_000), 2025).unwrap();
    assert_eq!(r.annual_mip_bps, 70);
    assert_eq!(r.duration, MipDuration::Years(11));
}

#[test]
fn test_fha_annual_mip_30yr_high_balance_ltv_le95_is_70bps_loan_term() {
    let r = store().fha_mip(&fha(360, 9200, 80_000_000), 2025).unwrap();
    assert_eq!(r.annual_mip_bps, 70);
    assert_eq!(r.duration, MipDuration::LoanTerm);
}

#[test]
fn test_fha_annual_mip_30yr_high_balance_ltv_over95_is_75bps_loan_term() {
    let r = store().fha_mip(&fha(360, 9650, 80_000_000), 2025).unwrap();
    assert_eq!(r.annual_mip_bps, 75);
    assert_eq!(r.duration, MipDuration::LoanTerm);
}

// ── Annual MIP: term ≤ 15 years ───────────────────────────────────────────────

#[test]
fn test_fha_annual_mip_15yr_standard_ltv_le90_is_15bps_11yr() {
    let r = store().fha_mip(&fha(180, 8500, 40_000_000), 2025).unwrap();
    assert_eq!(r.annual_mip_bps, 15);
    assert_eq!(r.duration, MipDuration::Years(11));
}

#[test]
fn test_fha_annual_mip_15yr_standard_ltv_over90_is_40bps_loan_term() {
    let r = store().fha_mip(&fha(180, 9500, 40_000_000), 2025).unwrap();
    assert_eq!(r.annual_mip_bps, 40);
    assert_eq!(r.duration, MipDuration::LoanTerm);
}

#[test]
fn test_fha_annual_mip_15yr_high_balance_ltv_le78_is_15bps_11yr() {
    let r = store().fha_mip(&fha(180, 7500, 80_000_000), 2025).unwrap();
    assert_eq!(r.annual_mip_bps, 15);
    assert_eq!(r.duration, MipDuration::Years(11));
}

#[test]
fn test_fha_annual_mip_15yr_high_balance_ltv_le90_is_40bps_11yr() {
    let r = store().fha_mip(&fha(180, 8800, 80_000_000), 2025).unwrap();
    assert_eq!(r.annual_mip_bps, 40);
    assert_eq!(
        r.duration,
        MipDuration::Years(11),
        "high-balance ≤15yr 78-90% LTV cancels at 11yr, not loan term"
    );
}

#[test]
fn test_fha_annual_mip_15yr_high_balance_ltv_over90_is_65bps_loan_term() {
    let r = store().fha_mip(&fha(180, 9500, 80_000_000), 2025).unwrap();
    assert_eq!(r.annual_mip_bps, 65);
    assert_eq!(r.duration, MipDuration::LoanTerm);
}

// ── Streamline pre-2009 ───────────────────────────────────────────────────────

#[test]
fn test_fha_streamline_pre2009_ltv_le90_annual_mip_55bps_11yr() {
    let input = FhaMipInput {
        term_months: 360,
        ltv_bps: 8500,
        base_loan_cents: 45_000_000,
        is_streamline_pre_2009: true,
    };
    let r = store().fha_mip(&input, 2025).unwrap();
    assert_eq!(r.annual_mip_bps, 55);
    assert_eq!(r.duration, MipDuration::Years(11));
}

#[test]
fn test_fha_streamline_pre2009_ltv_over90_annual_mip_55bps_loan_term() {
    let input = FhaMipInput {
        term_months: 360,
        ltv_bps: 9500,
        base_loan_cents: 45_000_000,
        is_streamline_pre_2009: true,
    };
    let r = store().fha_mip(&input, 2025).unwrap();
    assert_eq!(r.annual_mip_bps, 55);
    assert_eq!(r.duration, MipDuration::LoanTerm);
}

// ── FhaMipResult computation helpers ────────────────────────────────────────

#[test]
fn test_fha_monthly_mip_calculation() {
    let r = FhaMipResult {
        ufmip_bps: 175,
        annual_mip_bps: 55,
        duration: MipDuration::LoanTerm,
    };
    // $459,000 × 0.55% / 12 = $210.41 → ceiling = $211
    let monthly = r.monthly_mip(Cents(45_900_000));
    assert!(monthly.0 > 0);
    let expected_annual = 45_900_000i128 * 55 / 10_000;
    let expected_monthly = ((expected_annual + 11) / 12) as i64;
    assert_eq!(monthly.0, expected_monthly);
}

#[test]
fn test_fha_ufmip_amount_calculation() {
    let r = FhaMipResult {
        ufmip_bps: 175,
        annual_mip_bps: 50,
        duration: MipDuration::Years(11),
    };
    // $300,000 × 1.75% = $5,250
    let ufmip = r.ufmip_amount(30_000_000);
    assert_eq!(ufmip, Cents(525_000));
}

// ── USDA Guarantee Fees ───────────────────────────────────────────────────────

#[test]
fn test_usda_upfront_fee_is_1_percent() {
    let fees = store().usda_guarantee_fees(2025).unwrap();
    assert_eq!(fees.upfront_fee_bps, 100, "USDA upfront fee must be 1.00%");
}

#[test]
fn test_usda_annual_fee_is_35_bps() {
    let fees = store().usda_guarantee_fees(2025).unwrap();
    assert_eq!(fees.annual_fee_bps, 35, "USDA annual fee must be 0.35%");
}

#[test]
fn test_usda_fees_upfront_amount_calculation() {
    let fees = store().usda_guarantee_fees(2025).unwrap();
    // $100,000 × 1% = $1,000
    assert_eq!(fees.upfront_amount(10_000_000), Cents(100_000));
}

#[test]
fn test_usda_fees_monthly_annual_fee() {
    let fees = store().usda_guarantee_fees(2025).unwrap();
    // $200,000 × 0.35% / 12 = $58.33 → ceiling = $59
    let monthly = fees.monthly_annual_fee(Cents(20_000_000));
    assert!(monthly.0 > 0 && monthly.0 < 100_000);
}

// ════════════════════════════════════════════════════════════════════════════
// TASK 4.14 — VA Funding Fee
// ════════════════════════════════════════════════════════════════════════════

fn va(cat: VeteranCategory, purpose: VaLoanPurpose, use_: VaUse, dp_bps: u32) -> VaFeeInput {
    VaFeeInput {
        category: cat,
        purpose,
        use_,
        down_payment_bps: dp_bps,
    }
}

#[test]
fn test_va_regular_military_purchase_no_down_first_use_is_215bps() {
    let fee = store()
        .va_funding_fee(
            &va(
                VeteranCategory::RegularMilitary,
                VaLoanPurpose::PurchaseOrConstruction,
                VaUse::FirstTime,
                0,
            ),
            2025,
        )
        .unwrap();
    assert_eq!(fee, 215);
}

#[test]
fn test_va_regular_military_purchase_no_down_subsequent_is_330bps() {
    let fee = store()
        .va_funding_fee(
            &va(
                VeteranCategory::RegularMilitary,
                VaLoanPurpose::PurchaseOrConstruction,
                VaUse::Subsequent,
                0,
            ),
            2025,
        )
        .unwrap();
    assert_eq!(fee, 330);
}

#[test]
fn test_va_regular_military_purchase_5pct_down_is_150bps() {
    let fee = store()
        .va_funding_fee(
            &va(
                VeteranCategory::RegularMilitary,
                VaLoanPurpose::PurchaseOrConstruction,
                VaUse::FirstTime,
                500,
            ),
            2025,
        )
        .unwrap();
    assert_eq!(fee, 150);
}

#[test]
fn test_va_regular_military_purchase_10pct_down_is_125bps() {
    let fee = store()
        .va_funding_fee(
            &va(
                VeteranCategory::RegularMilitary,
                VaLoanPurpose::PurchaseOrConstruction,
                VaUse::FirstTime,
                1000,
            ),
            2025,
        )
        .unwrap();
    assert_eq!(fee, 125);
}

#[test]
fn test_va_regular_military_purchase_subsequent_5pct_is_still_150bps() {
    let fee = store()
        .va_funding_fee(
            &va(
                VeteranCategory::RegularMilitary,
                VaLoanPurpose::PurchaseOrConstruction,
                VaUse::Subsequent,
                500,
            ),
            2025,
        )
        .unwrap();
    assert_eq!(fee, 150, "5%+ down: subsequent use same as first use");
}

#[test]
fn test_va_reserves_purchase_no_down_first_use_is_240bps() {
    let fee = store()
        .va_funding_fee(
            &va(
                VeteranCategory::ReservesNationalGuard,
                VaLoanPurpose::PurchaseOrConstruction,
                VaUse::FirstTime,
                0,
            ),
            2025,
        )
        .unwrap();
    assert_eq!(fee, 240);
}

#[test]
fn test_va_reserves_purchase_no_down_subsequent_is_330bps() {
    let fee = store()
        .va_funding_fee(
            &va(
                VeteranCategory::ReservesNationalGuard,
                VaLoanPurpose::PurchaseOrConstruction,
                VaUse::Subsequent,
                0,
            ),
            2025,
        )
        .unwrap();
    assert_eq!(fee, 330);
}

#[test]
fn test_va_reserves_purchase_5pct_down_is_175bps() {
    let fee = store()
        .va_funding_fee(
            &va(
                VeteranCategory::ReservesNationalGuard,
                VaLoanPurpose::PurchaseOrConstruction,
                VaUse::FirstTime,
                500,
            ),
            2025,
        )
        .unwrap();
    assert_eq!(fee, 175);
}

#[test]
fn test_va_reserves_purchase_10pct_down_is_150bps() {
    let fee = store()
        .va_funding_fee(
            &va(
                VeteranCategory::ReservesNationalGuard,
                VaLoanPurpose::PurchaseOrConstruction,
                VaUse::FirstTime,
                1000,
            ),
            2025,
        )
        .unwrap();
    assert_eq!(fee, 150);
}

#[test]
fn test_va_cash_out_refi_regular_first_use_is_215bps() {
    let fee = store()
        .va_funding_fee(
            &va(
                VeteranCategory::RegularMilitary,
                VaLoanPurpose::CashOutRefinance,
                VaUse::FirstTime,
                0,
            ),
            2025,
        )
        .unwrap();
    assert_eq!(fee, 215);
}

#[test]
fn test_va_cash_out_refi_subsequent_is_330bps() {
    let fee = store()
        .va_funding_fee(
            &va(
                VeteranCategory::RegularMilitary,
                VaLoanPurpose::CashOutRefinance,
                VaUse::Subsequent,
                0,
            ),
            2025,
        )
        .unwrap();
    assert_eq!(fee, 330);
}

#[test]
fn test_va_irrrl_is_50bps_any_use() {
    let first = store()
        .va_funding_fee(
            &va(
                VeteranCategory::RegularMilitary,
                VaLoanPurpose::Irrrl,
                VaUse::FirstTime,
                0,
            ),
            2025,
        )
        .unwrap();
    let sub = store()
        .va_funding_fee(
            &va(
                VeteranCategory::RegularMilitary,
                VaLoanPurpose::Irrrl,
                VaUse::Subsequent,
                0,
            ),
            2025,
        )
        .unwrap();
    assert_eq!(first, 50);
    assert_eq!(sub, 50, "IRRRL fee is 50 bps regardless of use");
}

#[test]
fn test_va_assumption_is_50bps() {
    let fee = store()
        .va_funding_fee(
            &va(
                VeteranCategory::RegularMilitary,
                VaLoanPurpose::LoanAssumption,
                VaUse::FirstTime,
                0,
            ),
            2025,
        )
        .unwrap();
    assert_eq!(fee, 50);
}

#[test]
fn test_va_exempt_veteran_pays_zero() {
    let fee = store()
        .va_funding_fee(
            &va(
                VeteranCategory::Exempt,
                VaLoanPurpose::PurchaseOrConstruction,
                VaUse::FirstTime,
                0,
            ),
            2025,
        )
        .unwrap();
    assert_eq!(fee, 0, "disabled/exempt veteran must pay 0 bps");
}

#[test]
fn test_va_fee_year_fallback() {
    let fee_2025 = store()
        .va_funding_fee(
            &va(
                VeteranCategory::RegularMilitary,
                VaLoanPurpose::PurchaseOrConstruction,
                VaUse::FirstTime,
                0,
            ),
            2025,
        )
        .unwrap();
    let fee_2030 = store()
        .va_funding_fee(
            &va(
                VeteranCategory::RegularMilitary,
                VaLoanPurpose::PurchaseOrConstruction,
                VaUse::FirstTime,
                0,
            ),
            2030,
        )
        .unwrap();
    assert_eq!(
        fee_2025, fee_2030,
        "year fallback returns latest available data"
    );
}
