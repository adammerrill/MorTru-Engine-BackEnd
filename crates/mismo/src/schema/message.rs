//! MISMO 3.4 root `MESSAGE` document struct.
//!
//! This module is the **entry point** for the `mismo` crate. It composes all
//! sub-schemas (Tasks 2.3–2.9) into a single typed representation of a full
//! MISMO 3.4 loan document.
//!
//! # Usage
//!
//! ```ignore
//! let xml = std::fs::read_to_string("fha_purchase.xml")?;
//! let msg = MismoMessage::from_xml(&xml)?;
//! let deal = msg.parse_all()?;
//!
//! // All reference values verified against the FHA purchase spreadsheet:
//! assert_eq!(deal.loan_terms.base_loan_amount, Cents(43_444_300));
//! assert_eq!(deal.collateral.state, StateCode::TX);
//! ```
//!
//! # Document hierarchy
//!
//! ```text
//! MismoMessage
//! └── DealSets → DealSet → Deals → Deal
//!     ├── Loans → MismoLoan
//!     │   ├── MortgageTerms + Amortization  → LoanTermsParsed
//!     │   ├── MiDataDetail (opt)            → MiParsed
//!     │   ├── LenderComp (opt)              → LenderCompParsed
//!     │   ├── ClosingCost (opt)             → ClosingCostBlock
//!     │   ├── AusSystems (opt)              → AusParsed
//!     │   └── Qualification (opt)           → QualificationParsed
//!     ├── Parties
//!     │   ├── Vec<Party> → BorrowerDetail   → PartiesParsed
//!     │   └── ClosingContext (opt)
//!     └── Collaterals → Collateral
//!         └── SubjectProperty              → CollateralParsed
//! ```

use crate::schema::{
    aus::{AusParsed, AusSystems, MismoQualification, QualificationParsed},
    closing_cost::{parse_closing_costs, ClosingCostBlock, FeeRulesRegistry},
    collateral::{CollateralParsed, SubjectProperty},
    lender_comp::{LenderComp, LenderCompParsed},
    loan_terms::{Amortization, LoanTermsParsed, MortgageTerms},
    mi::{MiDataDetail, MiParsed},
    party::{BorrowerDetail, ClosingContext, PartiesParsed},
};

// ── Nested container structs ──────────────────────────────────────────────────

/// Closing cost fee wrapper — holds the repeated fee amount elements.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename = "CLOSING_COST")]
pub struct MismoClosingCostContainer {
    /// Container for individual fee lines.
    #[serde(
        rename = "CLOSING_COST_ESTIMATED_AMOUNTS",
        skip_serializing_if = "Option::is_none"
    )]
    pub amounts: Option<ClosingCostAmounts>,
}

/// Wrapper around the repeating `CLOSING_COST_ESTIMATED_AMOUNT` elements.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ClosingCostAmounts {
    /// Individual fee entries. `default` allows zero fees without parse error.
    #[serde(rename = "CLOSING_COST_ESTIMATED_AMOUNT", default)]
    pub fees: Vec<crate::schema::closing_cost::MismoClosingCostFee>,
}

/// One borrower party in the MISMO document.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename = "PARTY")]
pub struct MismoParty {
    /// Borrower detail, if this party is a borrower.
    #[serde(rename = "BORROWER_DETAIL", skip_serializing_if = "Option::is_none")]
    pub borrower_detail: Option<BorrowerDetail>,
}

/// `PARTIES` container — holds all party elements plus the closing context.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename = "PARTIES")]
pub struct MismoParties {
    /// All party elements (borrower, co-borrower, seller, agent, etc.).
    /// Engine uses only the first two borrower parties.
    #[serde(rename = "PARTY", default)]
    pub parties: Vec<MismoParty>,

    /// Engine-specific closing inputs (earnest money, option fee, target DTI).
    #[serde(rename = "CLOSING_CONTEXT", skip_serializing_if = "Option::is_none")]
    pub closing_context: Option<ClosingContext>,
}

/// `LOAN` element — all loan-level data.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename = "LOAN")]
pub struct MismoLoan {
    /// Core financial terms and amortisation type.
    #[serde(rename = "MORTGAGE_TERMS")]
    pub mortgage_terms: MortgageTerms,

    /// Amortisation type (Fixed, ARM, etc.).
    #[serde(rename = "AMORTIZATION")]
    pub amortization: Amortization,

    /// Mortgage insurance data (absent for conventional at LTV ≤ 80%).
    #[serde(rename = "MI_DATA_DETAIL", skip_serializing_if = "Option::is_none")]
    pub mi_data_detail: Option<MiDataDetail>,

    /// Broker/originator compensation (absent when zero or lender-paid only).
    #[serde(
        rename = "ORIGINATION_FEE_DETAIL",
        skip_serializing_if = "Option::is_none"
    )]
    pub origination_fee_detail: Option<LenderComp>,

    /// Closing cost fee lines (absent for incomplete/pre-qualification scenarios).
    #[serde(rename = "CLOSING_COST", skip_serializing_if = "Option::is_none")]
    pub closing_cost: Option<MismoClosingCostContainer>,

    /// AUS submissions (absent for manual underwriting).
    #[serde(
        rename = "AUTOMATED_UNDERWRITING_SYSTEMS",
        skip_serializing_if = "Option::is_none"
    )]
    pub aus_systems: Option<AusSystems>,

    /// Qualifying ratios from AUS or underwriter.
    #[serde(rename = "QUALIFICATION", skip_serializing_if = "Option::is_none")]
    pub qualification: Option<MismoQualification>,
}

/// `LOANS` container.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename = "LOANS")]
pub struct MismoLoans {
    /// Engine processes a single loan per document.
    #[serde(rename = "LOAN")]
    pub loan: MismoLoan,
}

/// `COLLATERAL` element.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename = "COLLATERAL")]
pub struct MismoCollateral {
    #[serde(rename = "SUBJECT_PROPERTY")]
    pub subject_property: SubjectProperty,
}

/// `COLLATERALS` container.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename = "COLLATERALS")]
pub struct MismoCollaterals {
    #[serde(rename = "COLLATERAL")]
    pub collateral: MismoCollateral,
}

/// `DEAL` — the top-level loan package.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename = "DEAL")]
pub struct MismoDeal {
    #[serde(rename = "LOANS")]
    pub loans: MismoLoans,
    #[serde(rename = "PARTIES")]
    pub parties: MismoParties,
    #[serde(rename = "COLLATERALS")]
    pub collaterals: MismoCollaterals,
}

/// `DEALS` container.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename = "DEALS")]
pub struct MismoDeals {
    #[serde(rename = "DEAL")]
    pub deal: MismoDeal,
}

/// `DEAL_SET` container.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename = "DEAL_SET")]
pub struct MismoDealSet {
    #[serde(rename = "DEALS")]
    pub deals: MismoDeals,
}

/// `DEAL_SETS` container.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename = "DEAL_SETS")]
pub struct MismoDealSets {
    #[serde(rename = "DEAL_SET")]
    pub deal_set: MismoDealSet,
}

// ── ParsedDeal ────────────────────────────────────────────────────────────────

/// Fully typed loan deal — output of [`MismoMessage::parse_all`].
///
/// All monetary values are `Cents`, all rates are `BasisPoints`. No `f64`
/// in any field. Ready for the ingest bridge (Epic 6) to combine with
/// enriched RESO property data and produce a `LoanScenario`.
///
/// # Notes on optional fields
///
/// - `mi`: absent for conventional loans at LTV ≤ 80%, or pre-qualification
/// - `lender_comp`: absent when no broker comp is disclosed
/// - `closing_costs`: may be empty for early-stage scenarios
/// - `aus`: absent for manual underwriting
/// - `qualification`: absent for pre-qualification scenarios
/// - `parties.va_funding_fee_tier`: always `None` after `parse_all()` —
///   call `parties.with_va_tier(ltv, ...)` in Epic 6 once LTV is computed
#[derive(Debug, Clone)]
pub struct ParsedDeal {
    /// Loan financial terms (amount, rate, term, program, lien, purpose).
    pub loan_terms: LoanTermsParsed,
    /// Property address, type, valuation, taxes, HOI, HOA.
    pub collateral: CollateralParsed,
    /// Borrower profile, income, VA/USDA data, budget constraints.
    pub parties: PartiesParsed,
    /// Mortgage insurance parameters (FHA/VA/USDA/PMI). `None` if absent.
    pub mi: Option<MiParsed>,
    /// Broker/originator compensation. `None` if absent.
    pub lender_comp: Option<LenderCompParsed>,
    /// All Loan Estimate fee sections A–H with aggregation helpers.
    pub closing_costs: ClosingCostBlock,
    /// AUS submission result (DU/LPA/FHA TOTAL/GUS). `None` for manual.
    pub aus: Option<AusParsed>,
    /// Qualifying ratios (rate, housing DTI, total DTI). `None` if absent.
    pub qualification: Option<QualificationParsed>,
}

// ── MismoMessage ──────────────────────────────────────────────────────────────

/// Root MISMO 3.4 `MESSAGE` document.
///
/// # Parse a MISMO document
///
/// ```ignore
/// use mismo::schema::message::MismoMessage;
///
/// let xml = include_str!("../../tests/fixtures/fha_purchase.xml");
/// let msg = MismoMessage::from_xml(xml).expect("valid MISMO XML");
/// let deal = msg.parse_all().expect("all fields valid");
///
/// assert_eq!(deal.loan_terms.base_loan_amount, types::Cents(43_444_300));
/// assert_eq!(deal.collateral.state, types::StateCode::TX);
/// ```
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename = "MESSAGE")]
pub struct MismoMessage {
    #[serde(rename = "DEAL_SETS")]
    pub deal_sets: MismoDealSets,
}

impl MismoMessage {
    /// Parse a MISMO 3.4 XML string into a typed document.
    pub fn from_xml(xml: &str) -> crate::Result<Self> {
        crate::xml::parse::from_xml(xml)
    }

    /// Serialize this document to a MISMO 3.4 XML string.
    pub fn to_xml(&self) -> crate::Result<String> {
        crate::xml::serialize::to_xml(self)
    }

    /// Extract all validated, typed data from this document.
    ///
    /// Calls every sub-schema `.parse()` method and returns a [`ParsedDeal`].
    ///
    /// # Errors
    ///
    /// Returns [`crate::MismoError`] if any required field is missing or
    /// any value is out of range.
    ///
    /// # VA funding fee tier
    ///
    /// [`ParsedDeal::parties`]`.va_funding_fee_tier` is always `None` after
    /// this call. The tier requires LTV which is computed in the ingest layer
    /// (Epic 6) from `loan_terms.base_loan_amount` and `collateral.appraised_value`.
    /// Call `parties.with_va_tier(ltv, is_cash_out, is_irrrl)` at that point.
    pub fn parse_all(&self) -> crate::Result<ParsedDeal> {
        let deal = &self.deal_sets.deal_set.deals.deal;
        let loan = &deal.loans.loan;
        let parties_elem = &deal.parties;
        let collateral_elem = &deal.collaterals.collateral;

        // ── Loan terms ──────────────────────────────────────────────────────
        let loan_terms = loan.mortgage_terms.parse(&loan.amortization)?;

        // ── Collateral ──────────────────────────────────────────────────────
        let collateral = collateral_elem.subject_property.parse()?;

        // ── Parties ─────────────────────────────────────────────────────────
        // Collect borrower parties in document order, ignoring non-borrowers
        let borrowers: Vec<&BorrowerDetail> = parties_elem
            .parties
            .iter()
            .filter_map(|p| p.borrower_detail.as_ref())
            .collect();

        let primary = borrowers.first().ok_or(crate::MismoError::MissingElement {
            element: "PARTY/BORROWER_DETAIL",
        })?;
        let secondary = borrowers.get(1).copied();
        let closing_ctx = parties_elem.closing_context.as_ref();
        let parties = PartiesParsed::parse(primary, secondary, closing_ctx)?;

        // ── MI (optional) ───────────────────────────────────────────────────
        let mi = loan
            .mi_data_detail
            .as_ref()
            .map(|m| m.parse())
            .transpose()?;

        // ── Lender comp (optional) ──────────────────────────────────────────
        let lender_comp = loan
            .origination_fee_detail
            .as_ref()
            .map(|c| c.parse())
            .transpose()?;

        // ── Closing costs ───────────────────────────────────────────────────
        let registry = FeeRulesRegistry::default();
        let closing_costs = match loan.closing_cost.as_ref() {
            Some(cc) => {
                let fees = cc
                    .amounts
                    .as_ref()
                    .map(|a| a.fees.as_slice())
                    .unwrap_or(&[]);
                parse_closing_costs(fees, &registry)?
            }
            None => ClosingCostBlock::default(),
        };

        // ── AUS (optional, first submission used) ───────────────────────────
        let aus = loan
            .aus_systems
            .as_ref()
            .and_then(|s| s.systems.first())
            .map(|a| a.parse())
            .transpose()?;

        // ── Qualification (optional) ────────────────────────────────────────
        let qualification = loan.qualification.as_ref().map(|q| q.parse()).transpose()?;

        Ok(ParsedDeal {
            loan_terms,
            collateral,
            parties,
            mi,
            lender_comp,
            closing_costs,
            aus,
            qualification,
        })
    }
}
