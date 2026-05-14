//! MISMO 3.4 loan terms schema — `MORTGAGE_TERMS` and `AMORTIZATION`.
//!
//! These two elements together describe the core financial parameters of a
//! mortgage: loan amount, rate, term, program type, lien position, and
//! purpose. They are the first structured data the engine reads from an
//! incoming MISMO document.
//!
//! # Document location
//!
//! ```text
//! MESSAGE/DEAL_SETS/DEAL_SET/DEALS/DEAL/LOANS/LOAN/
//!   ├── MORTGAGE_TERMS   ← MortgageTerms
//!   └── AMORTIZATION     ← Amortization
//! ```
//!
//! # Reference values (FHA purchase, spreadsheet scenario)
//!
//! | Field | XML value | Parsed value |
//! |---|---|---|
//! | `BaseLoanAmount` | `"434443.00"` | `Cents(43_444_300)` |
//! | `LoanAmountWithFinancedMI` | `"442046.00"` | `Cents(44_204_600)` |
//! | `NoteRatePercent` | `"6.375"` | `BasisPoints(6375)` |
//! | `LoanTermMonthsCount` | `"360"` | `TermMonths(360)` |
//! | `MortgageType` | `"FHA"` | `ProgramCode::Fha` |
//! | `LienPriorityType` | `"FirstLien"` | `LienPriority::First` |
//! | `LoanPurposeType` | `"Purchase"` | `LoanPurpose::Purchase` |
//! | `AmortizationType` | `"Fixed"` | `AmortizationType::Fixed` |

use rust_decimal::Decimal;
use std::str::FromStr;
use types::{
    AmortizationType, BasisPoints, Cents, LienPriority, LoanPurpose, ProgramCode, TermMonths,
};

// ── Parsing helpers ───────────────────────────────────────────────────────────

/// Parse a MISMO dollar-amount string (e.g. `"434443.00"`) to [`Cents`].
///
/// Rounds to the nearest cent using half-up rounding.
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

/// Parse an optional MISMO dollar-amount string to `Option<Cents>`.
/// Returns `None` if the option is absent.
fn parse_optional_cents(
    opt: &Option<String>,
    element: &'static str,
) -> crate::Result<Option<Cents>> {
    match opt.as_deref() {
        None | Some("") => Ok(None),
        Some(s) => parse_cents(s, element).map(Some),
    }
}

/// Parse a MISMO rate-percent string (e.g. `"6.375"` or `"6.375%"`) to
/// [`BasisPoints`].
fn parse_rate_bps(s: &str, element: &'static str) -> crate::Result<BasisPoints> {
    BasisPoints::from_percentage_str(s).map_err(|_| crate::MismoError::OutOfRange {
        element,
        detail: format!("'{s}' is not a valid rate percentage"),
    })
}

/// Parse a MISMO month-count string (e.g. `"360"`) to [`TermMonths`].
fn parse_term_months(s: &str) -> crate::Result<TermMonths> {
    let n: u16 = s.trim().parse().map_err(|_| crate::MismoError::OutOfRange {
        element: "LoanTermMonthsCount",
        detail: format!("'{s}' is not a valid integer month count"),
    })?;
    TermMonths::new(n).map_err(|_| crate::MismoError::OutOfRange {
        element: "LoanTermMonthsCount",
        detail: format!("{n} is outside the valid TermMonths range (120–360)"),
    })
}

/// Parse an optional integer string to `Option<u32>`.
fn parse_optional_u32(opt: &Option<String>, element: &'static str) -> crate::Result<Option<u32>> {
    match opt.as_deref() {
        None | Some("") => Ok(None),
        Some(s) => s
            .trim()
            .parse::<u32>()
            .map(Some)
            .map_err(|_| crate::MismoError::OutOfRange {
                element,
                detail: format!("'{s}' is not a valid unsigned integer"),
            }),
    }
}

/// Parse a MISMO boolean indicator string.
/// Accepts `"true"`, `"yes"`, and `"1"` (case-insensitive) as true;
/// everything else including absent fields is false.
fn parse_bool_indicator(opt: &Option<String>) -> bool {
    opt.as_deref()
        .map(|s| matches!(s.trim().to_lowercase().as_str(), "true" | "yes" | "1"))
        .unwrap_or(false)
}

// ── MORTGAGE_TERMS ────────────────────────────────────────────────────────────

/// MISMO 3.4 `MORTGAGE_TERMS` element.
///
/// All fields are stored as raw strings exactly as they appear in the XML.
/// Call [`MortgageTerms::parse`] together with an [`Amortization`] to
/// obtain a fully typed [`LoanTermsParsed`].
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename = "MORTGAGE_TERMS")]
pub struct MortgageTerms {
    /// Base loan amount before any financed MI. e.g. `"434443.00"`.
    #[serde(rename = "BaseLoanAmount")]
    pub base_loan_amount: String,

    /// Loan amount after financed UFMIP/funding fee. e.g. `"442046.00"`.
    /// Absent when MI is not financed.
    #[serde(rename = "LoanAmountWithFinancedMI", skip_serializing_if = "Option::is_none")]
    pub loan_amount_with_financed_mi: Option<String>,

    /// Note rate as a percentage string. e.g. `"6.375"` or `"6.375%"`.
    #[serde(rename = "NoteRatePercent")]
    pub note_rate_percent: String,

    /// Loan term in months. e.g. `"360"`.
    #[serde(rename = "LoanTermMonthsCount")]
    pub loan_term_months_count: String,

    /// MISMO mortgage type. e.g. `"FHA"`, `"VA"`, `"Conventional"`.
    #[serde(rename = "MortgageType")]
    pub mortgage_type: String,

    /// MISMO lien priority. e.g. `"FirstLien"`, `"SecondLien"`.
    #[serde(rename = "LienPriorityType")]
    pub lien_priority_type: String,

    /// MISMO loan purpose. e.g. `"Purchase"`, `"Refinance"`.
    #[serde(rename = "LoanPurposeType")]
    pub loan_purpose_type: String,

    // ── Engine extension fields (not in MISMO 3.4 standard) ──────────────────

    /// Planned holding period in months. Used for break-even analysis.
    #[serde(rename = "MortgageHoldingPeriodMonthsCount", skip_serializing_if = "Option::is_none")]
    pub holding_period_months: Option<String>,

    /// Number of days from today to the expected closing date.
    /// Used to calculate prepaid interest (daily_rate × days).
    #[serde(rename = "DaysUntilClosingCount", skip_serializing_if = "Option::is_none")]
    pub days_until_closing: Option<String>,

    /// Seller-paid closing cost contribution. e.g. `"10000.00"`.
    /// Appears in Section K of the cash-to-close calculation.
    #[serde(rename = "SellerConcessionAmount", skip_serializing_if = "Option::is_none")]
    pub seller_concession_amount: Option<String>,

    /// Whether the seller pays the owner's title insurance policy.
    #[serde(rename = "SellerPaysOwnersTitleIndicator", skip_serializing_if = "Option::is_none")]
    pub seller_pays_owners_title: Option<String>,

    /// Whether the borrower has waived impound/escrow accounts.
    #[serde(rename = "EscrowWaiverIndicator", skip_serializing_if = "Option::is_none")]
    pub waive_escrow: Option<String>,

    /// Whether a temporary rate buydown subsidy is present.
    #[serde(rename = "TemporaryBuydownIndicator", skip_serializing_if = "Option::is_none")]
    pub temp_buydown: Option<String>,

    /// Whether a subordinate lien (HELOC / 2nd mortgage) is being used.
    #[serde(rename = "SubordinateFinancingIndicator", skip_serializing_if = "Option::is_none")]
    pub subordinate_financing: Option<String>,

    /// Whether the loan amount exceeds the standard conforming limit and
    /// falls in the FHFA-designated high-cost area tier.
    #[serde(rename = "HighBalanceLoanIndicator", skip_serializing_if = "Option::is_none")]
    pub high_balance: Option<String>,
}

// ── AMORTIZATION ──────────────────────────────────────────────────────────────

/// MISMO 3.4 `AMORTIZATION` element.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename = "AMORTIZATION")]
pub struct Amortization {
    /// MISMO amortization type. e.g. `"Fixed"`, `"AdjustableRate"`.
    #[serde(rename = "AmortizationType")]
    pub amortization_type: String,
}

// ── Parsed output ─────────────────────────────────────────────────────────────

/// Fully validated, typed loan terms.
///
/// Produced by [`MortgageTerms::parse`]. All string fields from the XML
/// have been converted to domain types from the `types` crate. Any
/// conversion failure surfaces as [`crate::MismoError`].
#[derive(Debug, Clone)]
pub struct LoanTermsParsed {
    // ── Core financial parameters ─────────────────────────────────────────────

    /// Base loan amount before any financed MI premium.
    pub base_loan_amount: Cents,

    /// Loan amount after financed UFMIP / VA funding fee.
    /// `None` when MI is not financed into the loan balance.
    pub adjusted_loan_amount: Option<Cents>,

    /// Note rate.
    pub note_rate: BasisPoints,

    /// Loan term.
    pub term: TermMonths,

    /// Loan program (FHA, VA, Conventional, USDA, …).
    pub program: ProgramCode,

    /// Lien position.
    pub lien: LienPriority,

    /// Transaction purpose.
    pub purpose: LoanPurpose,

    /// Amortization schedule type.
    pub amortization: AmortizationType,

    // ── Closing context (optional engine extensions) ──────────────────────────

    /// Planned holding period for break-even calculation.
    pub holding_period_months: Option<u32>,

    /// Days from today to closing — drives prepaid interest calculation.
    pub days_until_closing: Option<u32>,

    /// Seller-paid closing cost contribution.
    pub seller_concession: Option<Cents>,

    /// True if seller is paying the owner's title insurance policy.
    pub seller_pays_title: bool,

    /// True if borrower has waived escrow/impound accounts.
    pub waive_escrow: bool,

    /// True if a temporary rate buydown subsidy applies.
    pub temp_buydown: bool,

    /// True if a subordinate lien accompanies this first mortgage.
    pub is_subordinate: bool,

    /// True if the loan amount falls in the FHFA high-balance tier.
    pub is_high_balance: bool,
}

// ── Parse implementation ──────────────────────────────────────────────────────

impl MortgageTerms {
    /// Convert raw XML strings into a fully typed [`LoanTermsParsed`].
    ///
    /// Requires an [`Amortization`] element because the amortization type
    /// lives in a sibling element in the MISMO hierarchy.
    ///
    /// # Errors
    /// Returns [`crate::MismoError`] if any required field is missing,
    /// contains an unrecognised enum value, or falls outside the valid
    /// range for its target type.
    ///
    /// # Example
    /// ```ignore
    /// let terms = MortgageTerms { /* ... */ };
    /// let amort = Amortization { amortization_type: "Fixed".into() };
    /// let parsed = terms.parse(&amort)?;
    /// assert_eq!(parsed.program, ProgramCode::Fha);
    /// ```
    pub fn parse(&self, amort: &Amortization) -> crate::Result<LoanTermsParsed> {
        Ok(LoanTermsParsed {
            base_loan_amount: parse_cents(&self.base_loan_amount, "BaseLoanAmount")?,
            adjusted_loan_amount: parse_optional_cents(
                &self.loan_amount_with_financed_mi,
                "LoanAmountWithFinancedMI",
            )?,
            note_rate: parse_rate_bps(&self.note_rate_percent, "NoteRatePercent")?,
            term: parse_term_months(&self.loan_term_months_count)?,
            program: crate::enums::loan_type::try_program_code(&self.mortgage_type)?,
            lien: crate::enums::loan_type::try_lien_priority(&self.lien_priority_type)?,
            purpose: crate::enums::loan_type::try_loan_purpose(&self.loan_purpose_type)?,
            amortization: crate::enums::loan_type::try_amortization_type(
                &amort.amortization_type,
            )?,
            holding_period_months: parse_optional_u32(
                &self.holding_period_months,
                "MortgageHoldingPeriodMonthsCount",
            )?,
            days_until_closing: parse_optional_u32(
                &self.days_until_closing,
                "DaysUntilClosingCount",
            )?,
            seller_concession: parse_optional_cents(
                &self.seller_concession_amount,
                "SellerConcessionAmount",
            )?,
            seller_pays_title: parse_bool_indicator(&self.seller_pays_owners_title),
            waive_escrow: parse_bool_indicator(&self.waive_escrow),
            temp_buydown: parse_bool_indicator(&self.temp_buydown),
            is_subordinate: parse_bool_indicator(&self.subordinate_financing),
            is_high_balance: parse_bool_indicator(&self.high_balance),
        })
    }
}
