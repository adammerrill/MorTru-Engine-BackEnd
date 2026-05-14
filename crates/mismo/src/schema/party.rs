//! MISMO 3.4 party/borrower schema.
//!
//! Parses the financially-relevant borrower profile flags from MISMO.
//! No PII (names, SSNs, DOBs) is stored — only the data the engine
//! needs for eligibility, pricing, and fee calculations.
//!
//! # Document location
//!
//! ```text
//! MESSAGE/DEAL_SETS/DEAL_SET/DEALS/DEAL/PARTIES/
//!   └── PARTY (one per borrower/co-borrower)
//!         └── BORROWER_DETAIL  ← BorrowerDetail
//! ```
//!
//! # Reference values — FHA purchase, Kyle TX, single borrower, credit 720
//!
//! Single borrower, credit score 720, primary residence purchase.
//! No VA/USDA flags. No HOA. No budget constraints.

use rust_decimal::Decimal;
use std::str::FromStr;
use types::{BasisPoints, Cents, CreditScore, DtiBasisPoints, LtvBasisPoints};

use crate::enums::party::{AffordableLendingProgram, VaFundingFeeTier};

// ── Parsing helpers ───────────────────────────────────────────────────────────

fn parse_cents(s: &str, element: &'static str) -> crate::Result<Cents> {
    let decimal = Decimal::from_str(s.trim()).map_err(|_| crate::MismoError::OutOfRange {
        element,
        detail: format!("'{s}' is not a valid decimal amount"),
    })?;
    Cents::from_decimal_dollars(decimal).map_err(|_| crate::MismoError::OutOfRange {
        element,
        detail: format!("'{s}' is out of range for Cents (i64)"),
    })
}

fn parse_optional_cents(
    opt: &Option<String>,
    element: &'static str,
) -> crate::Result<Option<Cents>> {
    match opt.as_deref() {
        None | Some("") => Ok(None),
        Some(s) => parse_cents(s, element).map(Some),
    }
}

fn parse_bool_indicator(opt: &Option<String>) -> bool {
    opt.as_deref()
        .map(|s| matches!(s.trim().to_lowercase().as_str(), "true" | "yes" | "1"))
        .unwrap_or(false)
}

fn parse_optional_credit_score(opt: &Option<String>) -> crate::Result<Option<CreditScore>> {
    match opt.as_deref() {
        None | Some("") => Ok(None),
        Some(s) => {
            let n: u16 = s
                .trim()
                .parse()
                .map_err(|_| crate::MismoError::OutOfRange {
                    element: "CreditScoreValue",
                    detail: format!("'{s}' is not a valid credit score integer"),
                })?;
            CreditScore::new(n)
                .map(Some)
                .map_err(|_| crate::MismoError::OutOfRange {
                    element: "CreditScoreValue",
                    detail: format!("{n} is outside the valid CreditScore range (300–850)"),
                })
        }
    }
}

fn parse_optional_u8(opt: &Option<String>, element: &'static str) -> crate::Result<Option<u8>> {
    match opt.as_deref() {
        None | Some("") => Ok(None),
        Some(s) => s
            .trim()
            .parse::<u8>()
            .map(Some)
            .map_err(|_| crate::MismoError::OutOfRange {
                element,
                detail: format!("'{s}' is not a valid count"),
            }),
    }
}

/// Parse a percentage string (e.g. `"43.0"`) to [`DtiBasisPoints`].
///
/// `DtiBasisPoints(4300)` represents 43.00% DTI.
fn parse_optional_dti(opt: &Option<String>) -> crate::Result<Option<DtiBasisPoints>> {
    match opt.as_deref() {
        None | Some("") => Ok(None),
        Some(s) => {
            let bps =
                BasisPoints::from_percentage_str(s).map_err(|_| crate::MismoError::OutOfRange {
                    element: "TargetDTIPercent",
                    detail: format!("'{s}' is not a valid DTI percentage"),
                })?;
            // BasisPoints uses 0.001% per unit; DtiBasisPoints uses 0.01% per unit.
            // Divide by 10 to convert: BasisPoints(4300) -> DtiBasisPoints(430)? No —
            // "43.0" -> BasisPoints(43000) via from_percentage_str (×1000).
            // DtiBasisPoints(4300) = 43.00% (÷100).
            // So: raw_bps = 43000, divide by 10 to get DtiBasisPoints = 4300. ✓
            Ok(Some(DtiBasisPoints::new(bps.0 / 10)))
        }
    }
}

// ── BorrowerDetail ────────────────────────────────────────────────────────────

/// MISMO `BORROWER_DETAIL` element — financially-relevant borrower profile.
///
/// All fields are optional at the XML level. The parser normalises absent
/// flags to `false` / `None` in [`PartiesParsed`].
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename = "BORROWER_DETAIL")]
pub struct BorrowerDetail {
    /// Representative credit score. `"720"` → `CreditScore(720)`.
    #[serde(rename = "CreditScoreValue", skip_serializing_if = "Option::is_none")]
    pub credit_score: Option<String>,

    /// Monthly gross qualifying income. e.g. `"8500.00"`.
    #[serde(
        rename = "TotalMonthlyIncomeAmount",
        skip_serializing_if = "Option::is_none"
    )]
    pub monthly_income: Option<String>,

    // ── Homebuyer profile ─────────────────────────────────────────────────────
    /// First-time homebuyer indicator.
    #[serde(
        rename = "FirstTimeHomebuyerIndicator",
        skip_serializing_if = "Option::is_none"
    )]
    pub first_time_homebuyer: Option<String>,

    /// Experienced (not first-time) homebuyer indicator.
    #[serde(
        rename = "ExperiencedHomebuyerIndicator",
        skip_serializing_if = "Option::is_none"
    )]
    pub experienced_homebuyer: Option<String>,

    /// Self-employed indicator (affects income documentation type).
    #[serde(
        rename = "SelfEmployedIndicator",
        skip_serializing_if = "Option::is_none"
    )]
    pub self_employed: Option<String>,

    // ── VA-specific ───────────────────────────────────────────────────────────
    /// Veteran is eligible for VA loan guarantee.
    #[serde(
        rename = "VABenefitsEligibleIndicator",
        skip_serializing_if = "Option::is_none"
    )]
    pub va_eligible: Option<String>,

    /// First-time use of VA guarantee.
    /// Affects the VA funding fee tier (2.15% vs 3.30% for < 5% down).
    #[serde(
        rename = "VAFirstTimeUseIndicator",
        skip_serializing_if = "Option::is_none"
    )]
    pub va_first_use: Option<String>,

    /// Full entitlement available (no prior VA loan outstanding).
    #[serde(
        rename = "VAFullEntitlementIndicator",
        skip_serializing_if = "Option::is_none"
    )]
    pub va_full_entitlement: Option<String>,

    /// Outstanding VA loan balance — reduces available entitlement.
    #[serde(
        rename = "VAOutstandingLoanBalance",
        skip_serializing_if = "Option::is_none"
    )]
    pub va_outstanding_balance: Option<String>,

    /// Veteran has 10%+ service-connected disability — funding fee waived.
    #[serde(
        rename = "VAFundingFeeExemptIndicator",
        skip_serializing_if = "Option::is_none"
    )]
    pub va_fee_exempt: Option<String>,

    // ── USDA-specific ─────────────────────────────────────────────────────────
    /// Total people in household during first 12 months (all ages).
    /// Used for USDA household size guideline compliance.
    #[serde(
        rename = "USDATotalHouseholdSize",
        skip_serializing_if = "Option::is_none"
    )]
    pub usda_household_size: Option<String>,

    /// Combined annual income of all adults (18+) in household.
    /// Used for USDA 115% AMI eligibility check.
    #[serde(
        rename = "USDATotalAdultHouseholdAnnualIncome",
        skip_serializing_if = "Option::is_none"
    )]
    pub usda_adult_household_income: Option<String>,

    // ── Affordable lending ────────────────────────────────────────────────────
    /// Borrower meets income eligibility for an affordable lending program.
    #[serde(
        rename = "AffordableLendingEligibleIndicator",
        skip_serializing_if = "Option::is_none"
    )]
    pub affordable_lending_eligible: Option<String>,

    /// Specific affordable lending program. e.g. `"HomeReady"`.
    #[serde(
        rename = "AffordableLendingProgramType",
        skip_serializing_if = "Option::is_none"
    )]
    pub affordable_lending_program: Option<String>,

    // ── Budget constraints (engine extension) ─────────────────────────────────
    /// Maximum total cash-to-close budget (down payment + all closing costs).
    #[serde(
        rename = "MaxCashToCloseBudgetAmount",
        skip_serializing_if = "Option::is_none"
    )]
    pub max_cash_to_close: Option<String>,

    /// Maximum monthly PITIA budget (P&I + taxes + insurance + HOA).
    #[serde(
        rename = "MaxMonthlyPITIABudgetAmount",
        skip_serializing_if = "Option::is_none"
    )]
    pub max_monthly_pitia: Option<String>,
}

// ── ClosingContext ────────────────────────────────────────────────────────────

/// Engine extension element carrying closing-specific borrower inputs.
///
/// These fields are not in the MISMO 3.4 core schema; they are populated
/// by the ingest layer (Epic 4) from user input or default values.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename = "CLOSING_CONTEXT")]
pub struct ClosingContext {
    /// Earnest money deposit already paid — reduces cash-to-close (Section L).
    #[serde(rename = "EarnestMoneyAmount", skip_serializing_if = "Option::is_none")]
    pub earnest_money: Option<String>,

    /// Option fee paid (Texas-specific). Appears in Section L as a credit.
    #[serde(rename = "OptionFeeAmount", skip_serializing_if = "Option::is_none")]
    pub option_fee: Option<String>,

    /// Target DTI constraint for scenario generation. e.g. `"43.0"`.
    #[serde(rename = "TargetDTIPercent", skip_serializing_if = "Option::is_none")]
    pub target_dti: Option<String>,

    /// Requested loan term (months). e.g. `"360"`.
    #[serde(
        rename = "RequestedLoanTermMonths",
        skip_serializing_if = "Option::is_none"
    )]
    pub requested_term_months: Option<String>,
}

// ── PartiesParsed ─────────────────────────────────────────────────────────────

/// Fully typed borrower profile — output of [`PartiesParsed::parse`].
#[derive(Debug, Clone)]
pub struct PartiesParsed {
    // ── Credit ────────────────────────────────────────────────────────────────
    /// Representative qualifying credit score.
    /// Single borrower: their score. Co-borrower: lower of the two.
    pub qualifying_credit_score: Option<CreditScore>,

    /// Number of borrowers (1 or 2).
    pub borrower_count: u8,

    // ── Homebuyer profile ─────────────────────────────────────────────────────
    /// True if any borrower is a first-time homebuyer.
    pub first_time_homebuyer: bool,

    /// True if the primary borrower is an experienced (repeat) homebuyer.
    pub experienced_homebuyer: bool,

    /// Combined monthly gross qualifying income.
    pub monthly_gross_income: Option<Cents>,

    // ── VA ────────────────────────────────────────────────────────────────────
    /// Borrower is eligible for VA loan guarantee.
    pub va_eligible: bool,

    /// First-time use of VA guarantee (drives funding fee tier).
    pub va_first_use: bool,

    /// Full entitlement available.
    pub va_full_entitlement: bool,

    /// Outstanding VA loan balance (reduces available entitlement).
    pub va_outstanding_balance: Option<Cents>,

    /// Funding fee is waived (10%+ service-connected disability).
    pub va_fee_exempt: bool,

    /// Funding fee tier, derivable once LTV is known.
    /// `None` after initial parse; computed via [`PartiesParsed::with_va_tier`]
    /// when LTV is available in the ingest layer (Epic 4).
    pub va_funding_fee_tier: Option<VaFundingFeeTier>,

    // ── USDA ──────────────────────────────────────────────────────────────────
    /// Total household size (all ages, first 12 months).
    pub usda_household_size: Option<u8>,

    /// Combined annual income of all adult household members.
    pub usda_adult_household_income: Option<Cents>,

    // ── Affordable lending ────────────────────────────────────────────────────
    /// Borrower meets AMI income limit for an affordable program.
    pub affordable_lending_eligible: bool,

    /// Specific affordable lending program (HomeReady, HomePossible, etc.).
    pub affordable_lending_program: AffordableLendingProgram,

    // ── Budget constraints ────────────────────────────────────────────────────
    /// Maximum total cash-to-close budget.
    pub max_cash_to_close: Option<Cents>,

    /// Maximum monthly PITIA budget.
    pub max_monthly_pitia: Option<Cents>,

    // ── Closing context ───────────────────────────────────────────────────────
    /// Earnest money deposit (reduces cash-to-close).
    pub earnest_money: Option<Cents>,

    /// Option fee paid (Texas — reduces cash-to-close).
    pub option_fee: Option<Cents>,

    /// Target DTI for scenario generation.
    pub target_dti: Option<DtiBasisPoints>,
}

// ── Parse implementation ──────────────────────────────────────────────────────

impl PartiesParsed {
    /// Parse one or two borrower details into a typed [`PartiesParsed`].
    ///
    /// # Parameters
    /// - `primary` — the primary borrower
    /// - `secondary` — the co-borrower, if present
    /// - `closing` — closing context inputs (earnest money, option fee, etc.)
    ///
    /// # Credit score convention
    /// - Single borrower: use their representative score directly.
    /// - Co-borrower: use the **lower** of the two scores (industry convention).
    ///
    /// # VA funding fee tier
    /// [`PartiesParsed::va_funding_fee_tier`] is always `None` after this call.
    /// Use [`PartiesParsed::with_va_tier`] to attach the tier once LTV is known.
    ///
    /// # Errors
    /// Returns [`crate::MismoError`] if any field contains an out-of-range or
    /// invalid value (e.g. credit score 200 or a non-numeric amount).
    pub fn parse(
        primary: &BorrowerDetail,
        secondary: Option<&BorrowerDetail>,
        closing: Option<&ClosingContext>,
    ) -> crate::Result<Self> {
        let primary_score = parse_optional_credit_score(&primary.credit_score)?;
        let secondary_score = secondary
            .map(|b| parse_optional_credit_score(&b.credit_score))
            .transpose()?
            .flatten();

        let qualifying_credit_score = match (primary_score, secondary_score) {
            (Some(p), Some(s)) => Some(CreditScore::lower_of_two(p, s)),
            (Some(p), None) => Some(p),
            (None, Some(s)) => Some(s),
            (None, None) => None,
        };

        let borrower_count = if secondary.is_some() { 2 } else { 1 };

        // Combined monthly income (sum primary + secondary)
        let primary_income =
            parse_optional_cents(&primary.monthly_income, "TotalMonthlyIncomeAmount")?;
        let secondary_income = secondary
            .map(|b| parse_optional_cents(&b.monthly_income, "TotalMonthlyIncomeAmount"))
            .transpose()?
            .flatten();
        let monthly_gross_income = match (primary_income, secondary_income) {
            (Some(p), Some(s)) => Some(Cents(p.0 + s.0)),
            (Some(p), None) => Some(p),
            (None, Some(s)) => Some(s),
            (None, None) => None,
        };

        let affordable_lending_program = primary
            .affordable_lending_program
            .as_deref()
            .map(AffordableLendingProgram::try_from_str)
            .transpose()?
            .unwrap_or(AffordableLendingProgram::None);

        let (earnest_money, option_fee, target_dti) = if let Some(ctx) = closing {
            (
                parse_optional_cents(&ctx.earnest_money, "EarnestMoneyAmount")?,
                parse_optional_cents(&ctx.option_fee, "OptionFeeAmount")?,
                parse_optional_dti(&ctx.target_dti)?,
            )
        } else {
            (None, None, None)
        };

        Ok(PartiesParsed {
            qualifying_credit_score,
            borrower_count,
            first_time_homebuyer: parse_bool_indicator(&primary.first_time_homebuyer),
            experienced_homebuyer: parse_bool_indicator(&primary.experienced_homebuyer),
            monthly_gross_income,
            va_eligible: parse_bool_indicator(&primary.va_eligible),
            va_first_use: parse_bool_indicator(&primary.va_first_use),
            va_full_entitlement: parse_bool_indicator(&primary.va_full_entitlement),
            va_outstanding_balance: parse_optional_cents(
                &primary.va_outstanding_balance,
                "VAOutstandingLoanBalance",
            )?,
            va_fee_exempt: parse_bool_indicator(&primary.va_fee_exempt),
            va_funding_fee_tier: None,
            usda_household_size: parse_optional_u8(
                &primary.usda_household_size,
                "USDATotalHouseholdSize",
            )?,
            usda_adult_household_income: parse_optional_cents(
                &primary.usda_adult_household_income,
                "USDATotalAdultHouseholdAnnualIncome",
            )?,
            affordable_lending_eligible: parse_bool_indicator(&primary.affordable_lending_eligible),
            affordable_lending_program,
            max_cash_to_close: parse_optional_cents(
                &primary.max_cash_to_close,
                "MaxCashToCloseBudgetAmount",
            )?,
            max_monthly_pitia: parse_optional_cents(
                &primary.max_monthly_pitia,
                "MaxMonthlyPITIABudgetAmount",
            )?,
            earnest_money,
            option_fee,
            target_dti,
        })
    }

    /// Attach the VA funding fee tier once LTV is known.
    ///
    /// Called by the ingest layer (Epic 4) after computing LTV from
    /// `CollateralParsed::appraised_value` and `LoanTermsParsed::base_loan_amount`.
    ///
    /// Returns `self` with `va_funding_fee_tier` populated (or still `None`
    /// if the borrower is not VA eligible).
    #[must_use]
    pub fn with_va_tier(
        mut self,
        ltv: LtvBasisPoints,
        is_cash_out_refi: bool,
        is_irrrl: bool,
    ) -> Self {
        if self.va_eligible {
            self.va_funding_fee_tier = Some(VaFundingFeeTier::from_inputs(
                self.va_first_use,
                ltv,
                is_cash_out_refi,
                is_irrrl,
                self.va_fee_exempt,
            ));
        }
        self
    }
}
