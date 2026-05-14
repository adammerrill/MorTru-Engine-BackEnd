//! MISMO 3.4 lender compensation schema.
//!
//! Under TRID (RESPA/Reg Z), broker/originator compensation must be disclosed
//! on the Loan Estimate. The disclosure location depends on who pays:
//!
//! - **Borrower-paid** compensation → Section A (Origination Charges)
//! - **Lender-paid** compensation → Page 3 table (not in Section A)
//!
//! # Document location
//!
//! ```text
//! MESSAGE/DEAL_SETS/DEAL_SET/DEALS/DEAL/LOANS/LOAN/
//!   └── ORIGINATION_FEE_DETAIL  ← LenderComp
//! ```
//!
//! # Reference values — FHA purchase, Kyle TX
//!
//! | Field | Value |
//! |---|---|
//! | Loan Amount | $434,443 |
//! | Comp BPS | 112.76 (fractional — stored as string) |
//! | Comp Amount | $4,899.24 |
//! | Comp Type | BorrowerPaid |
//! | In Section A | true |
//!
//! # Comp BPS precision note
//!
//! Broker compensation rates are stored as fractional basis points in the
//! XML (e.g. `"112.76"`). `LenderCompParsed.comp_bps` stores the value
//! rounded to the nearest integer `BasisPoints` for engine use. Exact
//! cent-level precision requires the Decimal computation path in
//! `compute_from_bps_decimal`, used by the closing cost engine (Epic 11).

use rust_decimal::Decimal;
use std::str::FromStr;
use types::{BasisPoints, Cents};

use crate::enums::comp::{CompDisclosure, CompType};

// ── Parsing helpers ───────────────────────────────────────────────────────────

fn parse_cents(s: &str, element: &'static str) -> crate::Result<Cents> {
    let decimal = Decimal::from_str(s.trim()).map_err(|_| crate::MismoError::OutOfRange {
        element,
        detail: format!("'{s}' is not a valid decimal amount"),
    })?;
    Cents::from_decimal_dollars(decimal).map_err(|_| crate::MismoError::OutOfRange {
        element,
        detail: format!("'{s}' is out of range for Cents"),
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

// ── LenderComp XML struct ─────────────────────────────────────────────────────

/// MISMO `ORIGINATION_FEE_DETAIL` element — broker/originator compensation.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename = "ORIGINATION_FEE_DETAIL")]
pub struct LenderComp {
    /// Total compensation dollar amount. e.g. `"4899.24"`.
    #[serde(rename = "CompensationAmount")]
    pub amount: String,

    /// Compensation expressed as basis points of loan amount.
    /// Stored as a decimal string: e.g. `"112.76"` = 1.1276%.
    #[serde(
        rename = "CompensationBasisPoints",
        skip_serializing_if = "Option::is_none"
    )]
    pub comp_bps: Option<String>,

    /// Who funds the compensation. `"BorrowerPaid"` | `"LenderPaid"` | `"Split"`.
    #[serde(rename = "CompensationType")]
    pub comp_type: String,

    /// `"true"` when the compensation appears as a line item in Section A of
    /// the Loan Estimate (required for borrower-paid comp under TRID).
    #[serde(
        rename = "DisclosedInSectionAIndicator",
        skip_serializing_if = "Option::is_none"
    )]
    pub disclose_in_section_a: Option<String>,

    /// Compensation cap amount under Reg Z § 36(d), if applicable.
    #[serde(
        rename = "CompensationCapAmount",
        skip_serializing_if = "Option::is_none"
    )]
    pub cap_amount: Option<String>,
}

// ── LenderCompParsed ──────────────────────────────────────────────────────────

/// Validated, typed lender compensation — output of [`LenderComp::parse`].
#[derive(Debug, Clone)]
pub struct LenderCompParsed {
    /// Total compensation dollar amount.
    pub amount: Cents,
    /// Compensation rate in standard-finance basis points (1 bp = 0.01%),
    /// rounded to the nearest integer. `BasisPoints(113)` ≈ 1.13%.
    pub comp_bps: Option<BasisPoints>,
    /// Raw decimal BPS value for high-precision calculation (e.g. `112.76`).
    /// Use this with [`LenderCompParsed::compute_from_bps_decimal`] when
    /// cent-level accuracy is required (Epic 11 closing costs).
    pub comp_bps_decimal: Option<Decimal>,
    /// Who funds the compensation.
    pub comp_type: CompType,
    /// Where this comp appears on the Loan Estimate.
    pub disclosure: CompDisclosure,
    /// Compensation cap amount, if present.
    pub cap_amount: Option<Cents>,
}

impl LenderCompParsed {
    /// Compute compensation from a loan amount and an integer BPS rate.
    ///
    /// Uses standard-finance basis points (100 bps = 1%).
    ///
    /// ```text
    /// result = loan_cents × bps / 10_000
    /// ```
    ///
    /// For exact cent-level precision with fractional BPS (e.g. `"112.76"`),
    /// use [`compute_from_bps_decimal`] instead.
    #[must_use]
    pub fn compute_from_bps(loan_amount: Cents, comp_bps: BasisPoints) -> Cents {
        Cents((loan_amount.0 as i128 * comp_bps.0 as i128 / 10_000_i128) as i64)
    }

    /// Compute compensation from a loan amount and a fractional BPS decimal.
    ///
    /// Rounds half-up to the nearest cent. Use this when parsing the raw
    /// `comp_bps` string from the XML field (which may contain decimals like
    /// `"112.76"`).
    ///
    /// ```text
    /// result = round(loan_dollars × bps_decimal / 10_000, 2)
    /// ```
    pub fn compute_from_bps_decimal(loan_amount: Cents, comp_bps: Decimal) -> crate::Result<Cents> {
        use rust_decimal::prelude::ToPrimitive;
        let loan_d = Decimal::from(loan_amount.0);
        let result_cents = (loan_d * comp_bps / Decimal::from(10_000_i32)).round();
        let c = result_cents
            .to_i64()
            .ok_or_else(|| crate::MismoError::OutOfRange {
                element: "CompensationBasisPoints",
                detail: "computed comp overflows i64".to_string(),
            })?;
        Ok(Cents(c))
    }
}

// ── Parse implementation ──────────────────────────────────────────────────────

impl LenderComp {
    /// Parse raw XML strings into a typed [`LenderCompParsed`].
    ///
    /// # Errors
    /// Returns [`crate::MismoError`] for unknown compensation types or
    /// non-numeric amount fields.
    pub fn parse(&self) -> crate::Result<LenderCompParsed> {
        let amount = parse_cents(&self.amount, "CompensationAmount")?;
        let comp_type = CompType::try_from_str(&self.comp_type)?;
        let disclosure = CompDisclosure::from_comp_type(comp_type);
        let cap_amount = parse_optional_cents(&self.cap_amount, "CompensationCapAmount")?;

        // Section A indicator: if absent, infer from comp_type
        let section_a_explicit = self
            .disclose_in_section_a
            .as_deref()
            .filter(|s| !s.is_empty());
        let disclose_in_section_a = match section_a_explicit {
            Some(_) => parse_bool_indicator(&self.disclose_in_section_a),
            None => comp_type.disclosed_in_section_a(),
        };

        // Confirm consistency: BorrowerPaid must be in Section A
        if comp_type == CompType::BorrowerPaid && !disclose_in_section_a {
            return Err(crate::MismoError::InvalidEnum {
                element: "DisclosedInSectionAIndicator",
                value: "BorrowerPaid compensation must be disclosed in Section A".to_string(),
            });
        }

        // Parse comp BPS — both integer and decimal forms
        let (comp_bps, comp_bps_decimal) = match self.comp_bps.as_deref() {
            None | Some("") => (None, None),
            Some(s) => {
                let d = Decimal::from_str(s.trim()).map_err(|_| crate::MismoError::OutOfRange {
                    element: "CompensationBasisPoints",
                    detail: format!("'{s}' is not a valid decimal"),
                })?;
                use rust_decimal::prelude::ToPrimitive;
                let bps_int = d
                    .round()
                    .to_u32()
                    .ok_or_else(|| crate::MismoError::OutOfRange {
                        element: "CompensationBasisPoints",
                        detail: format!("'{s}' is out of range for BasisPoints"),
                    })?;
                (Some(BasisPoints(bps_int)), Some(d))
            }
        };

        // disclosure field shadows struct field so use local variable
        let _ = disclose_in_section_a;
        Ok(LenderCompParsed {
            amount,
            comp_bps,
            comp_bps_decimal,
            comp_type,
            disclosure,
            cap_amount,
        })
    }
}
