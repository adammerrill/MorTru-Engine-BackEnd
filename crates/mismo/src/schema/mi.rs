//! MISMO 3.4 mortgage insurance schema — `MI_DATA_DETAIL`.
//!
//! Covers all four MI program types:
//!
//! | Program | Upfront | Monthly | Notes |
//! |---|---|---|---|
//! | FHA MIP | 1.75% (UFMIP) | 0.55% annual (FHA purchase ref) | Life-of-loan when LTV > 90% |
//! | VA Funding Fee | 0.50–3.30% | None | Rate depends on tier; see Epic 7 |
//! | USDA Guarantee | 1.00% upfront | 0.35% annual | Both required |
//! | Conventional PMI | None | Varies by LTV/score/plan | Cancels at 80% LTV |
//!
//! # Document location
//!
//! ```text
//! MESSAGE/DEAL_SETS/DEAL_SET/DEALS/DEAL/LOANS/LOAN/
//!   └── MI_DATA_DETAIL  ← MiDataDetail
//! ```
//!
//! # Reference values — FHA purchase, Kyle TX, $434,443 base loan
//!
//! | Field | XML value | Parsed value |
//! |---|---|---|
//! | `MIPremiumSourceType` | `"FHAUpfrontMIP"` | `MismoMiProgramType::FhaMip` |
//! | `MIUpfrontRatePercent` | `"1.75"` | `BasisPoints(175)` |
//! | `MIUpfrontPremiumAmount` | `"7602.7525"` | `Cents(760_275)` |
//! | `MIFinancedIndicator` | `"true"` | `is_financed = true` |
//! | `MIMonthlyPremiumRatePercent` | `"0.55"` | `BasisPoints(55)` |
//! | `MICancellationLTVPercent` | `"0.0"` | `is_life_of_loan = true` |
//! | `MIRequiredMonthsCount` | `"24"` | `required_months = Some(24)` |
//! | `MIPremiumCalculationMethodType` | `"Declining"` | `is_declining = true` |
//!
//! # BasisPoints scale for MI / upfront fee rates
//!
//! MI rates use the standard-finance convention (1 bp = 0.01%):
//! - 1.75% UFMIP → `BasisPoints(175)`
//! - 0.55% annual MIP → `BasisPoints(55)`
//!
//! This matches the VA funding fee convention already established in
//! `enums::party::VaFundingFeeTier::rate_bps()`.

use rust_decimal::Decimal;
use std::str::FromStr;
use types::{BasisPoints, Cents, LtvBasisPoints};

use crate::enums::mi::{MiFirstPremiumType, MiRenewalType, MismoMiProgramType};

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

/// Parse a percentage string to [`BasisPoints`] using the standard-finance
/// ×100 convention (1 bp = 0.01%).
///
/// `"1.75"` → `BasisPoints(175)` (= 1.75%)
/// `"0.55"` → `BasisPoints(55)` (= 0.55%)
fn parse_fee_rate_bps(s: &str, element: &'static str) -> crate::Result<BasisPoints> {
    let d = Decimal::from_str(s.trim()).map_err(|_| crate::MismoError::OutOfRange {
        element,
        detail: format!("'{s}' is not a valid rate percentage"),
    })?;
    let bps =
        (d * Decimal::from(100))
            .round()
            .try_into()
            .map_err(|_| crate::MismoError::OutOfRange {
                element,
                detail: format!("'{s}' produces a BasisPoints value out of u32 range"),
            })?;
    Ok(BasisPoints(bps))
}

fn parse_optional_fee_rate_bps(
    opt: &Option<String>,
    element: &'static str,
) -> crate::Result<Option<BasisPoints>> {
    match opt.as_deref() {
        None | Some("") => Ok(None),
        Some(s) => parse_fee_rate_bps(s, element).map(Some),
    }
}

fn parse_bool_indicator(opt: &Option<String>) -> bool {
    opt.as_deref()
        .map(|s| matches!(s.trim().to_lowercase().as_str(), "true" | "yes" | "1"))
        .unwrap_or(false)
}

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

/// Parse a cancellation LTV percentage string to [`LtvBasisPoints`].
///
/// `"80.0"` → `LtvBasisPoints(8000)` = 80.00% LTV
/// `"0.0"` → `LtvBasisPoints(0)` → triggers `is_life_of_loan = true`
fn parse_cancellation_ltv(opt: &Option<String>) -> crate::Result<Option<LtvBasisPoints>> {
    match opt.as_deref() {
        None | Some("") => Ok(None),
        Some(s) => {
            let d = Decimal::from_str(s.trim()).map_err(|_| crate::MismoError::OutOfRange {
                element: "MICancellationLTVPercent",
                detail: format!("'{s}' is not a valid LTV percentage"),
            })?;
            let bps: u32 = (d * Decimal::from(100)).round().try_into().map_err(|_| {
                crate::MismoError::OutOfRange {
                    element: "MICancellationLTVPercent",
                    detail: format!("'{s}' is out of range"),
                }
            })?;
            LtvBasisPoints::new(bps)
                .map(Some)
                .map_err(|_| crate::MismoError::OutOfRange {
                    element: "MICancellationLTVPercent",
                    detail: format!("{bps} exceeds maximum plausible LTV"),
                })
        }
    }
}

// ── MiDataDetail XML struct ───────────────────────────────────────────────────

/// MISMO 3.4 `MI_DATA_DETAIL` element.
///
/// All rate/amount fields are optional strings at the XML level because
/// not every program populates every field (e.g. VA has no monthly rate).
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename = "MI_DATA_DETAIL")]
pub struct MiDataDetail {
    /// Payment responsibility. `"BorrowerPaid"` | `"LenderPaid"` | `"SplitPremium"`.
    #[serde(rename = "MIType")]
    pub mi_type: String,

    /// MI program. `"FHAUpfrontMIP"` | `"VAFundingFee"` | `"USDAGuaranteeFee"` | `"PrivateMI"`.
    #[serde(rename = "MIPremiumSourceType")]
    pub mi_program_type: String,

    /// Upfront premium rate as a percentage. e.g. `"1.75"` for FHA UFMIP.
    #[serde(
        rename = "MIUpfrontRatePercent",
        skip_serializing_if = "Option::is_none"
    )]
    pub upfront_rate_percent: Option<String>,

    /// Computed upfront premium dollar amount. e.g. `"7602.7525"`.
    #[serde(
        rename = "MIUpfrontPremiumAmount",
        skip_serializing_if = "Option::is_none"
    )]
    pub upfront_amount: Option<String>,

    /// `"true"` when the upfront premium is added to the loan balance.
    #[serde(
        rename = "MIFinancedIndicator",
        skip_serializing_if = "Option::is_none"
    )]
    pub financed: Option<String>,

    /// Annual monthly premium rate as a percentage. e.g. `"0.55"` = 55 bps/yr.
    #[serde(
        rename = "MIMonthlyPremiumRatePercent",
        skip_serializing_if = "Option::is_none"
    )]
    pub monthly_rate_percent: Option<String>,

    /// LTV at which MI cancels. `"80.0"` for conventional; `"0.0"` for FHA
    /// life-of-loan (LTV > 90% at origination).
    #[serde(
        rename = "MICancellationLTVPercent",
        skip_serializing_if = "Option::is_none"
    )]
    pub cancellation_ltv_percent: Option<String>,

    /// Minimum months MI must be collected regardless of LTV.
    #[serde(
        rename = "MIRequiredMonthsCount",
        skip_serializing_if = "Option::is_none"
    )]
    pub required_months: Option<String>,

    /// How the premium is recalculated. `"Declining"` | `"Level"`.
    #[serde(
        rename = "MIPremiumCalculationMethodType",
        skip_serializing_if = "Option::is_none"
    )]
    pub calculation_method: Option<String>,

    /// First premium timing. `"AtClosing"` | `"FirstPayment"` | `"Deferred"`.
    #[serde(
        rename = "MIPaymentRemittanceType",
        skip_serializing_if = "Option::is_none"
    )]
    pub remittance_type: Option<String>,
}

// ── MiParsed ─────────────────────────────────────────────────────────────────

/// Validated, typed MI parameters — output of [`MiDataDetail::parse`].
#[derive(Debug, Clone)]
pub struct MiParsed {
    /// Which MI program applies.
    pub program: MismoMiProgramType,
    /// Upfront premium rate. `BasisPoints(175)` = 1.75% FHA UFMIP.
    pub upfront_rate: Option<BasisPoints>,
    /// Upfront premium dollar amount. `Cents(760_275)` = $7,602.75.
    pub upfront_amount: Option<Cents>,
    /// True when the upfront premium is rolled into the loan balance.
    pub is_financed: bool,
    /// Annual MI rate. `BasisPoints(55)` = 0.55%/yr FHA MIP.
    pub monthly_annual_rate: Option<BasisPoints>,
    /// LTV at which MI cancels. `LtvBasisPoints(8000)` = 80.00% for conventional.
    /// `LtvBasisPoints(0)` when life-of-loan (see `is_life_of_loan`).
    pub cancellation_ltv: Option<LtvBasisPoints>,
    /// Minimum months MI must be collected.
    pub required_months: Option<u32>,
    /// How the monthly premium is recalculated (true = on declining balance).
    pub is_declining: bool,
    /// True when `cancellation_ltv` is zero — FHA 30yr with LTV > 90% at
    /// origination collects MIP for the full loan term with no cancellation.
    pub is_life_of_loan: bool,
    /// When the first premium is due.
    pub first_premium_timing: Option<MiFirstPremiumType>,
    /// How the premium renews annually.
    pub renewal_type: Option<MiRenewalType>,
}

// ── Parse implementation ──────────────────────────────────────────────────────

impl MiDataDetail {
    /// Parse raw XML strings into a typed [`MiParsed`].
    ///
    /// # Errors
    /// Returns [`crate::MismoError`] for unknown program types or
    /// out-of-range numeric fields.
    pub fn parse(&self) -> crate::Result<MiParsed> {
        let program = MismoMiProgramType::try_from_str(&self.mi_program_type)?;
        let upfront_rate =
            parse_optional_fee_rate_bps(&self.upfront_rate_percent, "MIUpfrontRatePercent")?;
        let upfront_amount = parse_optional_cents(&self.upfront_amount, "MIUpfrontPremiumAmount")?;
        let is_financed = parse_bool_indicator(&self.financed);
        let monthly_annual_rate =
            parse_optional_fee_rate_bps(&self.monthly_rate_percent, "MIMonthlyPremiumRatePercent")?;
        let cancellation_ltv = parse_cancellation_ltv(&self.cancellation_ltv_percent)?;
        let is_declining = self
            .calculation_method
            .as_deref()
            .map(|s| s.trim().eq_ignore_ascii_case("declining"))
            .unwrap_or(false);
        // Life-of-loan when cancellation LTV is explicitly zero.
        // FHA 30yr with original LTV > 90% never cancels MI.
        let is_life_of_loan = cancellation_ltv.map(|ltv| ltv.0 == 0).unwrap_or(false);
        let required_months = parse_optional_u32(&self.required_months, "MIRequiredMonthsCount")?;
        let first_premium_timing = self
            .remittance_type
            .as_deref()
            .filter(|s| !s.is_empty())
            .map(MiFirstPremiumType::try_from_str)
            .transpose()?;
        let renewal_type = self
            .calculation_method
            .as_deref()
            .filter(|s| !s.is_empty())
            .map(MiRenewalType::try_from_str)
            .transpose()?;

        Ok(MiParsed {
            program,
            upfront_rate,
            upfront_amount,
            is_financed,
            monthly_annual_rate,
            cancellation_ltv,
            required_months,
            is_declining,
            is_life_of_loan,
            first_premium_timing,
            renewal_type,
        })
    }
}
