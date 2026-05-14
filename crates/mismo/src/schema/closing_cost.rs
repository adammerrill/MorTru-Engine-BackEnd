//! MISMO 3.4 closing cost schema — `CLOSING_COST_ESTIMATED_AMOUNTS`.
//!
//! # Architecture
//!
//! This module defines the **typed container** for fees on a CFPB Loan
//! Estimate. Fee *amounts* are either read from XML or computed by the Epic 11
//! fee engine using Epic 4 reference tables. This module does not compute
//! amounts.
//!
//! # Fee rules registry
//!
//! Per-fee-type rules (VA allowability, tolerance category, APR inclusion,
//! etc.) are loaded at runtime from `data/fee_rules.json`. The registry can
//! be updated via API without recompiling. See [`FeeRulesRegistry`].
//!
//! # TRID tolerance buckets
//!
//! - **0%** — origination, transfer taxes, required-provider services
//! - **10%** — recording fees, shopping-permitted services
//! - **Unlimited** — prepaids, insurance, taxes, optional services
//!
//! # Lender credits
//!
//! A lender credit is a `FeeEntry` with `fee_type = FeeType::LenderCredit`
//! and a **negative** `borrower_amount`. It appears in Section J.
//! Validation rule: total credits cannot exceed total borrower closing costs.

use std::collections::HashMap;
use std::str::FromStr;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use types::Cents;

// ── Enumerations ──────────────────────────────────────────────────────────────

/// CFPB Loan Estimate section.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FeeSection {
    /// Origination charges (0% tolerance).
    A,
    /// Services borrower did not shop for (0% tolerance).
    B,
    /// Services borrower did shop for (10% tolerance).
    C,
    /// Taxes and government recording (mixed tolerance).
    E,
    /// Prepaids (unlimited tolerance).
    F,
    /// Initial escrow payment (unlimited tolerance).
    G,
    /// Other (unlimited tolerance).
    H,
    /// Section J — lender credits and totals.
    J,
}

impl FeeSection {
    /// Parse from a MISMO `IntegratedDisclosureSectionType` string.
    pub fn from_mismo_str(s: &str) -> crate::Result<Self> {
        match s.trim() {
            "LoanCosts_OriginationCharges" => Ok(Self::A),
            "LoanCosts_ServicesYouCannotShopFor" => Ok(Self::B),
            "LoanCosts_ServicesYouCanShopFor" => Ok(Self::C),
            "OtherCosts_TaxesAndOtherGovernmentFees" => Ok(Self::E),
            "OtherCosts_Prepaids" => Ok(Self::F),
            "OtherCosts_InitialEscrowPaymentAtClosing" => Ok(Self::G),
            "OtherCosts_Other" => Ok(Self::H),
            "LenderCredit" | "TotalClosingCosts" => Ok(Self::J),
            other => Err(crate::MismoError::InvalidEnum {
                element: "IntegratedDisclosureSectionType",
                value: other.to_owned(),
            }),
        }
    }

    /// Canonical MISMO `IntegratedDisclosureSectionType` string.
    #[must_use]
    pub const fn to_mismo_str(self) -> &'static str {
        match self {
            Self::A => "LoanCosts_OriginationCharges",
            Self::B => "LoanCosts_ServicesYouCannotShopFor",
            Self::C => "LoanCosts_ServicesYouCanShopFor",
            Self::E => "OtherCosts_TaxesAndOtherGovernmentFees",
            Self::F => "OtherCosts_Prepaids",
            Self::G => "OtherCosts_InitialEscrowPaymentAtClosing",
            Self::H => "OtherCosts_Other",
            Self::J => "TotalClosingCosts",
        }
    }

    /// Single-letter label used on the CFPB Loan Estimate form.
    #[must_use]
    pub const fn label(self) -> char {
        match self {
            Self::A => 'A',
            Self::B => 'B',
            Self::C => 'C',
            Self::E => 'E',
            Self::F => 'F',
            Self::G => 'G',
            Self::H => 'H',
            Self::J => 'J',
        }
    }

    /// True if this section's fees count toward the Loan Costs (Section D).
    #[must_use]
    pub const fn is_loan_cost(self) -> bool {
        matches!(self, Self::A | Self::B | Self::C)
    }

    /// True if this section's fees count toward Other Costs (Section I).
    #[must_use]
    pub const fn is_other_cost(self) -> bool {
        matches!(self, Self::E | Self::F | Self::G | Self::H)
    }
}

/// CFPB TRID tolerance category.
///
/// Controls how much a fee can increase from the Loan Estimate to the
/// Closing Disclosure. Any increase beyond the tolerance requires the
/// lender to cure (reimburse) the borrower.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum FeeTolerance {
    /// Fee cannot increase by any amount.
    Zero,
    /// The aggregate of fees in this bucket cannot increase more than 10%.
    TenPct,
    /// No restriction on increase.
    Unlimited,
}

/// Which party paid or is paying this fee.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum FeePaidBy {
    Borrower,
    Seller,
    Lender,
    Other,
}

impl FeePaidBy {
    pub fn from_str_case(s: &str) -> crate::Result<Self> {
        match s.trim() {
            "Borrower" | "borrower" => Ok(Self::Borrower),
            "Seller" | "seller" => Ok(Self::Seller),
            "Lender" | "lender" => Ok(Self::Lender),
            "Other" | "other" => Ok(Self::Other),
            other => Err(crate::MismoError::InvalidEnum {
                element: "FeePaymentPaidByType",
                value: other.to_owned(),
            }),
        }
    }
}

/// How the fee amount was determined.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeeSource {
    /// Fixed dollar amount (read from XML or set by lender schedule).
    Static,
    /// Percentage of loan amount (e.g. origination points, UFMIP).
    LoanAmountPct,
    /// Percentage of sales price (e.g. transfer taxes, owner's title policy).
    SalesPricePct,
    /// Looked up from a reference data table (Epic 4).
    TableLookup,
    /// Computed via a formula (Epic 11).
    Formula,
}

/// Exhaustive enumeration of every fee that can appear on a Loan Estimate.
///
/// This enum drives runtime rules loaded from `data/fee_rules.json` —
/// VA allowability, APR inclusion, default tolerance category, and whether
/// the fee can be financed. Updating those rules requires only a JSON change,
/// not a code change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum FeeType {
    // Section A — Origination
    OriginationPoints,
    OriginationFee,
    ApplicationFee,
    ProcessingFee,
    UnderwritingFee,
    BrokerCompensation,
    CreditReportFee,

    // Section B — Required services
    AppraisalFee,
    AppraisalReviewFee,
    AppraisalAmcFee,
    FloodCertificationFee,
    FloodMonitoringFee,
    TaxServiceFee,
    TaxStatusResearchFee,
    AusFee,
    DocumentPrepFee,
    PestInspectionFee,
    VoeVodFee,
    FhaUfmip,
    VaFundingFee,
    UsdaGuaranteeFee,
    MiSinglePremium,

    // Section C — Title and settlement
    OwnersTitlePolicy,
    LendersTitlePolicy,
    TitleEndorsement,
    TitleSearchFee,
    SettlementFee,
    EscrowFee,
    AttorneyFee,
    ErecordingFee,
    CourierFee,
    WireFee,

    // Section E — Government
    DeedRecordingFee,
    MortgageRecordingFee,
    DeedTransferTax,
    MortgageStampTax,
    MansionTax,
    CityTransferTax,

    // Section F — Prepaids
    PrepaidInterest,
    HoiPrepaid,
    FloodInsurancePrepaid,

    // Section G — Initial escrow
    TaxEscrow,
    HoiEscrow,
    PmiEscrow,
    MipEscrow,
    UsdaAnnualFeeEscrow,
    FloodEscrow,
    AggregateAdjustment,

    // Section H — Other
    HoaTransferFee,
    HoaWorkingCapital,
    HoaEstoppelFee,
    HomeWarrantyFee,
    SurveyFee,

    // Section J — Credits
    LenderCredit,
    SellerConcession,

    /// Catch-all for fees not yet in the enum.
    Other,
}

impl FeeType {
    /// Parse from a string matching the enum variant name (PascalCase).
    pub fn parse_fee_type(s: &str) -> Option<Self> {
        serde_json::from_value(serde_json::Value::String(s.to_owned())).ok()
    }
}

// ── Fee rules (runtime-loadable from JSON) ────────────────────────────────────

/// Per-fee-type rules loaded from `data/fee_rules.json`.
///
/// Stored externally so VA/FHA/USDA rule changes can be applied by
/// updating the JSON file via the `/api/v1/fee-rules` endpoint without
/// redeployment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeTypeRules {
    pub fee_type: FeeType,
    pub display_name: String,
    pub default_section: FeeSection,
    pub default_tolerance: FeeTolerance,
    pub can_be_financed: bool,
    pub va_allowable: bool,
    pub fha_allowable: bool,
    pub usda_allowable: bool,
    pub is_apr_included: bool,
    #[serde(default)]
    pub notes: String,
}

/// Runtime registry of fee type rules.
///
/// # Loading
///
/// ```no_run
/// use mismo::schema::closing_cost::FeeRulesRegistry;
/// // Default — uses the JSON bundled at compile time:
/// let registry = FeeRulesRegistry::default();
///
/// // From an externally updated file:
/// let json = std::fs::read_to_string("/etc/mortru/fee_rules.json").unwrap();
/// let registry = FeeRulesRegistry::from_json(&json).unwrap();
/// ```
///
/// The Epic 14 API exposes `GET /api/v1/fee-rules` (read current rules) and
/// `PUT /api/v1/fee-rules` (replace rules file and reload). This allows
/// updating VA non-allowable status or APR inclusion without redeployment.
#[derive(Debug)]
pub struct FeeRulesRegistry {
    rules: HashMap<FeeType, FeeTypeRules>,
    /// Raw JSON source (returned as-is by the API read endpoint).
    source_json: String,
}

impl FeeRulesRegistry {
    /// The JSON bundled at compile time via `include_str!`.
    const EMBEDDED_JSON: &'static str = include_str!("../../data/fee_rules.json");

    /// Load from a JSON string. Used for runtime override via API.
    pub fn from_json(json: &str) -> crate::Result<Self> {
        #[derive(Deserialize)]
        struct Root {
            fees: Vec<FeeTypeRules>,
        }
        let root: Root = serde_json::from_str(json).map_err(|e| crate::MismoError::OutOfRange {
            element: "fee_rules.json",
            detail: format!("invalid fee rules JSON: {e}"),
        })?;
        let rules = root.fees.into_iter().map(|r| (r.fee_type, r)).collect();
        Ok(Self {
            rules,
            source_json: json.to_owned(),
        })
    }

    /// Return the raw JSON (for the API read endpoint).
    pub fn as_json(&self) -> &str {
        &self.source_json
    }

    /// Look up rules for a fee type. Returns `None` for unknown types.
    pub fn get(&self, fee_type: FeeType) -> Option<&FeeTypeRules> {
        self.rules.get(&fee_type)
    }

    /// True if this fee is allowable in the borrower's column for VA loans.
    pub fn va_allowable(&self, fee_type: FeeType) -> bool {
        self.rules
            .get(&fee_type)
            .map(|r| r.va_allowable)
            .unwrap_or(true)
    }

    /// True if this fee is allowable in the borrower's column for FHA loans.
    pub fn fha_allowable(&self, fee_type: FeeType) -> bool {
        self.rules
            .get(&fee_type)
            .map(|r| r.fha_allowable)
            .unwrap_or(true)
    }

    /// True if this fee affects the APR calculation.
    pub fn is_apr_included(&self, fee_type: FeeType) -> bool {
        self.rules
            .get(&fee_type)
            .map(|r| r.is_apr_included)
            .unwrap_or(false)
    }

    /// Default tolerance for this fee type.
    pub fn default_tolerance(&self, fee_type: FeeType) -> FeeTolerance {
        self.rules
            .get(&fee_type)
            .map(|r| r.default_tolerance)
            .unwrap_or(FeeTolerance::Unlimited)
    }

    /// True if this fee can be rolled into the loan balance.
    pub fn can_be_financed(&self, fee_type: FeeType) -> bool {
        self.rules
            .get(&fee_type)
            .map(|r| r.can_be_financed)
            .unwrap_or(false)
    }

    /// All registered fee types in section order.
    pub fn all_rules(&self) -> Vec<&FeeTypeRules> {
        let mut rules: Vec<&FeeTypeRules> = self.rules.values().collect();
        rules.sort_by_key(|r| format!("{:?}{}", r.default_section, r.display_name));
        rules
    }
}

impl Default for FeeRulesRegistry {
    /// Load from the compile-time embedded `data/fee_rules.json`.
    ///
    /// Always succeeds — the embedded JSON is validated at test time.
    fn default() -> Self {
        Self::from_json(Self::EMBEDDED_JSON).expect("embedded fee_rules.json must be valid")
    }
}

// ── MISMO XML struct ──────────────────────────────────────────────────────────

/// One fee line from a MISMO `CLOSING_COST_ESTIMATED_AMOUNT` element.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename = "CLOSING_COST_ESTIMATED_AMOUNT")]
pub struct MismoClosingCostFee {
    /// MISMO section type string.
    /// e.g. `"LoanCosts_OriginationCharges"` for Section A.
    #[serde(rename = "IntegratedDisclosureSectionType")]
    pub section_type: String,

    /// Human-readable fee name. e.g. `"Origination Fee"`.
    #[serde(rename = "FeeDescription")]
    pub description: String,

    /// Total fee amount (all parties). e.g. `"1095.00"`.
    #[serde(
        rename = "FeeActualTotalAmount",
        skip_serializing_if = "Option::is_none"
    )]
    pub total_amount: Option<String>,

    /// Amount paid by borrower.
    #[serde(
        rename = "BorrowerChoiceClosingCostAmount",
        skip_serializing_if = "Option::is_none"
    )]
    pub borrower_amount: Option<String>,

    /// Amount paid by seller.
    #[serde(
        rename = "SellerChoiceClosingCostAmount",
        skip_serializing_if = "Option::is_none"
    )]
    pub seller_amount: Option<String>,

    /// Amount paid by lender (absorbed or from rate premium).
    #[serde(
        rename = "LenderChoiceClosingCostAmount",
        skip_serializing_if = "Option::is_none"
    )]
    pub lender_amount: Option<String>,

    /// Primary payer. `"Borrower"` | `"Seller"` | `"Lender"` | `"Other"`.
    #[serde(
        rename = "FeePaymentPaidByType",
        skip_serializing_if = "Option::is_none"
    )]
    pub paid_by: Option<String>,

    /// `"true"` when this fee is rolled into the loan balance (UFMIP, VA fee).
    #[serde(
        rename = "IntegratedDisclosureFinancedIndicator",
        skip_serializing_if = "Option::is_none"
    )]
    pub financed: Option<String>,

    /// `"true"` when this fee affects the APR.
    #[serde(
        rename = "APRAffectedIndicator",
        skip_serializing_if = "Option::is_none"
    )]
    pub apr_affected: Option<String>,

    /// Line sequence number within its section.
    #[serde(
        rename = "IntegratedDisclosureLineItemSequenceNumber",
        skip_serializing_if = "Option::is_none"
    )]
    pub sequence_number: Option<String>,

    /// Engine-internal fee type hint (not standard MISMO — engine extension).
    #[serde(rename = "FeeTypeCode", skip_serializing_if = "Option::is_none")]
    pub fee_type_code: Option<String>,
}

// ── FeeEntry (typed parsed output) ───────────────────────────────────────────

/// A single parsed, typed fee entry — output of [`MismoClosingCostFee::parse`].
#[derive(Debug, Clone)]
pub struct FeeEntry {
    /// LE section this fee belongs to.
    pub section: FeeSection,
    /// Known fee type, or `FeeType::Other` if unrecognised.
    pub fee_type: FeeType,
    /// Human-readable description from the XML.
    pub description: String,
    /// Amount borne by the borrower (never negative except for lender credits).
    pub borrower_amount: Cents,
    /// Amount borne by the seller.
    pub seller_amount: Cents,
    /// Amount borne by the lender (absorbed or from rate premium credit).
    pub lender_amount: Cents,
    /// Amount borne by another party (e.g. gift, grant).
    pub other_amount: Cents,
    /// CFPB tolerance category.
    pub tolerance: FeeTolerance,
    /// True when the fee is rolled into the loan balance.
    pub is_financed: bool,
    /// True when the fee is included in the APR calculation.
    pub is_apr_included: bool,
    /// How the fee amount was determined.
    pub source: FeeSource,
    /// Display order within the section.
    pub sequence: u16,
}

impl FeeEntry {
    /// Total across all payers.
    #[must_use]
    pub fn total_amount(&self) -> Cents {
        Cents(
            self.borrower_amount.0
                + self.seller_amount.0
                + self.lender_amount.0
                + self.other_amount.0,
        )
    }
}

// ── ClosingCostBlock ──────────────────────────────────────────────────────────

/// All Loan Estimate fees organised by section.
#[derive(Debug, Clone, Default)]
pub struct ClosingCostBlock {
    /// Section A — Origination charges.
    pub section_a: Vec<FeeEntry>,
    /// Section B — Services borrower did not shop for.
    pub section_b: Vec<FeeEntry>,
    /// Section C — Services borrower did shop for (title).
    pub section_c: Vec<FeeEntry>,
    /// Section E — Taxes and government recording.
    pub section_e: Vec<FeeEntry>,
    /// Section F — Prepaids.
    pub section_f: Vec<FeeEntry>,
    /// Section G — Initial escrow payment.
    pub section_g: Vec<FeeEntry>,
    /// Section H — Other.
    pub section_h: Vec<FeeEntry>,
    /// Section J — Lender credits (stored as positive; subtracted in totals).
    pub lender_credits: Vec<FeeEntry>,
}

impl ClosingCostBlock {
    fn section_borrower_total(fees: &[FeeEntry]) -> Cents {
        Cents(fees.iter().map(|f| f.borrower_amount.0).sum())
    }

    /// Section A borrower total.
    pub fn section_a_borrower(&self) -> Cents {
        Self::section_borrower_total(&self.section_a)
    }

    /// Section B borrower total (excludes financed fees from cash-to-close).
    pub fn section_b_borrower(&self) -> Cents {
        Cents(
            self.section_b
                .iter()
                .filter(|f| !f.is_financed)
                .map(|f| f.borrower_amount.0)
                .sum(),
        )
    }

    /// Section C borrower total.
    pub fn section_c_borrower(&self) -> Cents {
        Self::section_borrower_total(&self.section_c)
    }

    /// Section D — Total Loan Costs (A + B + C), borrower column.
    pub fn total_loan_costs_borrower(&self) -> Cents {
        Cents(
            self.section_a_borrower().0 + self.section_b_borrower().0 + self.section_c_borrower().0,
        )
    }

    /// Section E borrower total.
    pub fn section_e_borrower(&self) -> Cents {
        Self::section_borrower_total(&self.section_e)
    }

    /// Section F borrower total.
    pub fn section_f_borrower(&self) -> Cents {
        Self::section_borrower_total(&self.section_f)
    }

    /// Section G borrower total.
    pub fn section_g_borrower(&self) -> Cents {
        Self::section_borrower_total(&self.section_g)
    }

    /// Section H borrower total.
    pub fn section_h_borrower(&self) -> Cents {
        Self::section_borrower_total(&self.section_h)
    }

    /// Section I — Total Other Costs (E + F + G + H), borrower column.
    pub fn total_other_costs_borrower(&self) -> Cents {
        Cents(
            self.section_e_borrower().0
                + self.section_f_borrower().0
                + self.section_g_borrower().0
                + self.section_h_borrower().0,
        )
    }

    /// Total lender credits (sum of positive credit amounts).
    pub fn total_lender_credits(&self) -> Cents {
        Cents(
            self.lender_credits
                .iter()
                .map(|c| c.borrower_amount.0)
                .sum(),
        )
    }

    /// Section J — Total Closing Costs (D + I − lender credits), borrower column.
    pub fn total_closing_costs_borrower(&self) -> Cents {
        Cents(
            self.total_loan_costs_borrower().0 + self.total_other_costs_borrower().0
                - self.total_lender_credits().0,
        )
    }

    /// Total of all APR-included fees (for Reg Z APR calculation in Epic 10).
    pub fn total_apr_fees(&self) -> Cents {
        let all = self
            .section_a
            .iter()
            .chain(&self.section_b)
            .chain(&self.section_c)
            .chain(&self.section_e)
            .chain(&self.section_f)
            .chain(&self.section_g)
            .chain(&self.section_h);
        Cents(
            all.filter(|f| f.is_apr_included)
                .map(|f| f.borrower_amount.0)
                .sum(),
        )
    }

    /// Total of fees rolled into the loan balance (UFMIP, VA fee, USDA fee).
    pub fn total_financed(&self) -> Cents {
        let all = self.section_b.iter();
        Cents(
            all.filter(|f| f.is_financed)
                .map(|f| f.borrower_amount.0)
                .sum(),
        )
    }

    /// Validate that lender credits do not exceed total borrower closing costs.
    ///
    /// Per CFPB TRID: a lender credit in excess of closing costs is not
    /// permissible — it would result in impermissible cash back to borrower.
    pub fn validate_lender_credits(&self) -> crate::Result<()> {
        let credits = self.total_lender_credits();
        let costs = Cents(self.total_loan_costs_borrower().0 + self.total_other_costs_borrower().0);
        if credits.0 > costs.0 {
            return Err(crate::MismoError::OutOfRange {
                element: "LenderCredit",
                detail: format!(
                    "lender credits ({}) exceed total closing costs ({}) — TRID violation",
                    credits, costs
                ),
            });
        }
        Ok(())
    }

    /// Add a fee entry to the appropriate section.
    pub fn add(&mut self, entry: FeeEntry) {
        match entry.section {
            FeeSection::A => self.section_a.push(entry),
            FeeSection::B => self.section_b.push(entry),
            FeeSection::C => self.section_c.push(entry),
            FeeSection::E => self.section_e.push(entry),
            FeeSection::F => self.section_f.push(entry),
            FeeSection::G => self.section_g.push(entry),
            FeeSection::H => self.section_h.push(entry),
            FeeSection::J => self.lender_credits.push(entry),
        }
    }
}

// ── Parsing helpers ───────────────────────────────────────────────────────────

fn parse_optional_cents(s: Option<&str>, element: &'static str) -> crate::Result<Cents> {
    match s.filter(|v| !v.trim().is_empty()) {
        None => Ok(Cents(0)),
        Some(v) => {
            let d = Decimal::from_str(v.trim()).map_err(|_| crate::MismoError::OutOfRange {
                element,
                detail: format!("'{v}' is not a valid decimal amount"),
            })?;
            Cents::from_decimal_dollars(d).map_err(|_| crate::MismoError::OutOfRange {
                element,
                detail: format!("'{v}' is out of range for Cents"),
            })
        }
    }
}

fn parse_bool_indicator(opt: Option<&str>) -> bool {
    opt.map(|s| matches!(s.trim().to_lowercase().as_str(), "true" | "yes" | "1"))
        .unwrap_or(false)
}

// ── Parse implementation ──────────────────────────────────────────────────────

impl MismoClosingCostFee {
    /// Parse one MISMO fee element into a typed [`FeeEntry`].
    ///
    /// Uses the provided registry for tolerance and APR lookup. When no
    /// registry is available, call `parse_with_registry(FeeRulesRegistry::default())`.
    pub fn parse_with_registry(&self, registry: &FeeRulesRegistry) -> crate::Result<FeeEntry> {
        let section = FeeSection::from_mismo_str(&self.section_type)?;

        // Resolve fee type from engine hint or default to Other
        let fee_type = self
            .fee_type_code
            .as_deref()
            .and_then(FeeType::parse_fee_type)
            .unwrap_or(FeeType::Other);

        let borrower_amount = parse_optional_cents(
            self.borrower_amount.as_deref(),
            "BorrowerChoiceClosingCostAmount",
        )?;
        let seller_amount = parse_optional_cents(
            self.seller_amount.as_deref(),
            "SellerChoiceClosingCostAmount",
        )?;
        let lender_amount = parse_optional_cents(
            self.lender_amount.as_deref(),
            "LenderChoiceClosingCostAmount",
        )?;

        let is_financed = parse_bool_indicator(self.financed.as_deref());

        // APR: prefer explicit XML flag, fall back to registry
        let is_apr_included = if self
            .apr_affected
            .as_deref()
            .map(|s| !s.is_empty())
            .unwrap_or(false)
        {
            parse_bool_indicator(self.apr_affected.as_deref())
        } else {
            registry.is_apr_included(fee_type)
        };

        let tolerance = registry.default_tolerance(fee_type);

        let sequence = self
            .sequence_number
            .as_deref()
            .and_then(|s| s.trim().parse::<u16>().ok())
            .unwrap_or(0);

        Ok(FeeEntry {
            section,
            fee_type,
            description: self.description.clone(),
            borrower_amount,
            seller_amount,
            lender_amount,
            other_amount: Cents(0),
            tolerance,
            is_financed,
            is_apr_included,
            source: FeeSource::Static,
            sequence,
        })
    }
}

/// Parse a list of MISMO fee elements into a [`ClosingCostBlock`].
pub fn parse_closing_costs(
    fees: &[MismoClosingCostFee],
    registry: &FeeRulesRegistry,
) -> crate::Result<ClosingCostBlock> {
    let mut block = ClosingCostBlock::default();
    for fee in fees {
        let entry = fee.parse_with_registry(registry)?;
        block.add(entry);
    }
    Ok(block)
}
