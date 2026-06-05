//! Epic 16 / Task T1 — the borrower-wizard → engine JSON contract.
//!
//! The web-side wizard collects borrower inputs and POSTs JSON to the engine.
//! The engine deserializes into [`PartialAnalysisInput`], determines which
//! [`WizardStep`]s are complete, and returns the funnel counts unlocked so far.
//!
//! # Design
//! - **Partial by construction.** Every step's fields are `Option` (or an empty
//!   `Vec` for per-borrower data) so any prefix of the fixed step sequence
//!   deserializes successfully. The engine never sees a half-built struct it
//!   can't reason about.
//! - **Strong typing.** Domain values reuse the existing `types` newtypes/enums
//!   (`Cents`, `CreditScore`, `Occupancy`, `ProgramCode`, `TermMonths`,
//!   `StateCode`, `FipsCode`, `PropertyType`, `LoanPurpose`, `GoalMask`) — no
//!   stringly-typed domain data crosses the boundary.
//! - **Stepwise gating.** [`WizardStep`] is a fixed ordered sequence; each step
//!   has a completeness predicate over the fields it owns. Counts unlock at
//!   defined stages (see `funnel.rs`).
//!
//! # MISMO/RESO mapping (no-PII narrowing documented in module README)
//! Per-borrower data maps to the MISMO `BORROWER` container but collects ONLY
//! the risk/eligibility subset (`IndicatorScoreValue`, `BaseIncomeAmount`,
//! VA status) — never name/SSN/DOB/address. Monthly-budget, cash-budget, and
//! hold-horizon have no ULAD equivalent (they are borrower-stated analysis
//! goals, not loan-application facts) and are modeled as engine-native types.

use serde::{Deserialize, Serialize};
use types::{
    Cents, CreditScore, FipsCode, GoalMask, LoanPurpose, Occupancy, ProgramCode, PropertyType,
    StateCode, TermMonths,
};

// ── Per-borrower (MISMO BORROWER subset, no PII) ────────────────────────────

/// VA status for a borrower. Maps to MISMO `MilitaryServiceData` /
/// `VABorrowerEligibility` subset. Present only when the borrower is a veteran.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VaStatus {
    /// VA-eligible (has entitlement). MISMO `VALoanEligibilityIndicator`.
    pub eligible: bool,
    /// Has previously used VA entitlement. Drives funding-fee tier (4.27).
    pub previous_use: bool,
    /// Service-connected disability → funding-fee exempt. MISMO
    /// `VAFundingFeeExemptIndicator` basis.
    pub disability: bool,
}

/// One borrower's eligibility-relevant inputs (no identity/PII).
/// Maps to the MISMO `BORROWER` container, risk subset only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BorrowerInput {
    /// Per-borrower occupancy intent. MISMO `BorrowerResidencyType`.
    pub occupancy: Occupancy,
    /// One or more credit scores. MISMO `IndicatorScoreValue`. The engine
    /// computes the representative score (Fannie B3-5.1-02) downstream.
    pub credit_scores: Vec<CreditScore>,
    /// VA status, present only for veterans. `None` = not a veteran.
    pub va: Option<VaStatus>,
    /// Annual base income. MISMO `CurrentIncome` `BaseIncomeAmount` (annual).
    pub annual_income: Cents,
}

impl BorrowerInput {
    /// A borrower row is complete when it has at least one credit score and a
    /// non-negative income. (Occupancy is non-optional by type.)
    #[must_use]
    pub fn is_complete(&self) -> bool {
        !self.credit_scores.is_empty() && !self.annual_income.is_negative()
    }
}

// ── Seller-paid credit basket (steps 9–12) ──────────────────────────────────

/// Seller-paid credits and concessions. Feeds the cash-to-close basket and the
/// IPC cap check (Task 4.26). Maps to MISMO `SellerPaidClosingCostsAmount` /
/// `SellerConcessionsAmount` and closing-cost responsibility indicators.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SellerCredits {
    /// Buyer-agent commission the seller agrees to pay. RESO
    /// `BuyerAgencyCompensation` subset. `None` = seller pays none.
    pub agent_commission_paid_by_seller: Option<Cents>,
    /// Requested seller concessions (IPC). Subject to the 4.26 cap.
    pub concessions_requested: Option<Cents>,
    /// Seller pays owner's/lender's title. `TitleInsurancePaidByType`.
    pub pays_title: bool,
    /// Seller pays the survey.
    pub pays_survey: bool,
}

// ── The fixed ordered step sequence ─────────────────────────────────────────

/// The 12 wizard steps, in canonical order. The discriminant is the wire order;
/// never renumber a released variant (it is persisted/transmitted).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WizardStep {
    BorrowerCount = 1,
    PropertyUse = 2,
    BorrowerDetails = 3,
    PreferredTerm = 4,
    MonthlyPaymentBudget = 5,
    UpfrontCashBudget = 6,
    HoldHorizon = 7,
    BuyerAgentCommission = 8,
    SellerPaidBuyerAgentCommission = 9,
    RequestedSellerConcessions = 10,
    SellerPaysTitle = 11,
    SellerPaysSurvey = 12,
}

impl WizardStep {
    /// All steps in canonical order.
    pub const ALL: [WizardStep; 12] = [
        WizardStep::BorrowerCount,
        WizardStep::PropertyUse,
        WizardStep::BorrowerDetails,
        WizardStep::PreferredTerm,
        WizardStep::MonthlyPaymentBudget,
        WizardStep::UpfrontCashBudget,
        WizardStep::HoldHorizon,
        WizardStep::BuyerAgentCommission,
        WizardStep::SellerPaidBuyerAgentCommission,
        WizardStep::RequestedSellerConcessions,
        WizardStep::SellerPaysTitle,
        WizardStep::SellerPaysSurvey,
    ];
}

// ── The contract ────────────────────────────────────────────────────────────

/// A monotonically-completed borrower-wizard submission. Any prefix of the
/// step sequence deserializes; the engine reasons over whatever is present.
///
/// Field groups are ordered to mirror [`WizardStep`]. The "gap" fields the MVP
/// omitted (location, purchase price, property type, loan purpose) are included
/// because real eligibility counts require them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PartialAnalysisInput {
    // Step 1
    pub borrower_count: Option<u8>,
    // Step 2 — property use / occupancy at the deal level
    pub property_use: Option<Occupancy>,
    // Step 3 — per borrower (len should match borrower_count when complete)
    pub borrowers: Vec<BorrowerInput>,
    // Step 4 — preferred term (optional even when "complete"; None = engine
    // sweeps all available terms). Validated against rate-card terms downstream.
    pub preferred_term: Option<TermMonths>,
    // Step 5
    pub monthly_payment_budget: Option<Cents>,
    // Step 6
    pub upfront_cash_budget: Option<Cents>,
    // Step 7 — hold horizon in months (no ULAD equivalent; analysis goal)
    pub hold_horizon_months: Option<u16>,
    // Step 8
    pub buyer_agent_commission: Option<Cents>,
    // Steps 9–12
    #[serde(default)]
    pub seller_credits: SellerCredits,

    // ── Gap fields (not a wizard "step" each, but required for real counts) ──
    /// Property state. MISMO `PropertyState` / RESO `StateOrProvince`.
    pub property_state: Option<StateCode>,
    /// County FIPS. MISMO `CountyFIPSCode` / RESO `CountyOrParish`.
    pub property_county_fips: Option<FipsCode>,
    /// Purchase price (≠ budget). ULAD `PurchasePriceAmount`.
    pub purchase_price: Option<Cents>,
    /// Property type/units. RESO `PropertyType` / MISMO property type.
    pub property_type: Option<PropertyType>,
    /// Loan purpose. MISMO `LoanPurposeType`.
    pub loan_purpose: Option<LoanPurpose>,
    /// Program filter, if the borrower constrained it; else engine considers all.
    pub program: Option<ProgramCode>,
    /// Optimization goals. Defaults applied by the engine if absent
    /// (`GoalMask::DEFAULT_CONSUMER` / `DEFAULT_INVESTOR` by persona).
    pub goals: Option<GoalMask>,
}

impl PartialAnalysisInput {
    /// Is the given step's owned fields complete?
    #[must_use]
    pub fn step_complete(&self, step: WizardStep) -> bool {
        match step {
            WizardStep::BorrowerCount => self.borrower_count.is_some_and(|n| n >= 1),
            WizardStep::PropertyUse => self.property_use.is_some(),
            WizardStep::BorrowerDetails => {
                // complete when borrower_count is known and that many complete rows exist
                self.borrower_count.is_some_and(|n| {
                    let n = n as usize;
                    self.borrowers.len() == n
                        && n >= 1
                        && self.borrowers.iter().all(BorrowerInput::is_complete)
                })
            }
            // Preferred term is optional: the step is "complete" either way once
            // reached. Completeness here means "the user has passed this step".
            WizardStep::PreferredTerm => true,
            WizardStep::MonthlyPaymentBudget => self.monthly_payment_budget.is_some(),
            WizardStep::UpfrontCashBudget => self.upfront_cash_budget.is_some(),
            WizardStep::HoldHorizon => self.hold_horizon_months.is_some_and(|m| m >= 1),
            WizardStep::BuyerAgentCommission => self.buyer_agent_commission.is_some(),
            WizardStep::SellerPaidBuyerAgentCommission => true, // optional toggle+amount
            WizardStep::RequestedSellerConcessions => true,     // optional
            WizardStep::SellerPaysTitle => true,                // bool, always answerable
            WizardStep::SellerPaysSurvey => true,               // bool
        }
    }

    /// The furthest step reached: the last step in canonical order for which
    /// that step AND every prior non-optional step is complete. Stepwise gating
    /// — a later step cannot be "reached" until earlier required ones are done.
    #[must_use]
    pub fn furthest_step(&self) -> Option<WizardStep> {
        let mut reached = None;
        for step in WizardStep::ALL {
            // Required (non-optional) gates that must hold to advance.
            let required_ok = match step {
                WizardStep::PreferredTerm
                | WizardStep::SellerPaidBuyerAgentCommission
                | WizardStep::RequestedSellerConcessions
                | WizardStep::SellerPaysTitle
                | WizardStep::SellerPaysSurvey => true,
                other => self.step_complete(other),
            };
            if required_ok {
                reached = Some(step);
            } else {
                break;
            }
        }
        reached
    }
}
