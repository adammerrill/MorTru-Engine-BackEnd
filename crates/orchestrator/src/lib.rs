//! Epic 17 / Sub-task P1 — `StorePricer`: LTV + rate-sheet slice.
//!
//! The real `solver::ScenarioPricer`, backed by `ref_data` + `amort`. This
//! first slice computes, at a candidate starting balance:
//!   1. LTV = balance / property_value
//!   2. base note rate from the lender's rate sheet (par rate for the product)
//!   3. monthly payment + horizon cost via `amort`
//!
//! MI, LLPA, and the full fee worksheet are added in P2–P4; here MI = 0,
//! LLPA = 0, and cash-to-close is the down payment (balance-derived placeholder).
//!
//! ## Product mapping
//! A `scenarios::Scenario` maps to a rate-sheet product string (e.g.
//! "conv_30yr_fixed") + a lock period. P1 uses a fixed default lock; later
//! slices thread the borrower's chosen lock through.

use amort::{horizon_cost, monthly_payment, schedule};
use ref_data::{
    GseAgency, Ineligible, LlpaOccupancy, LlpaPricing, LlpaPropertyType, LlpaPurpose, LlpaScenario,
    RateSheet, RefDataError, RefDataStore,
};

/// Narrow LLPA slice of `RefDataStore`, blanket-impl'd over it (the
/// `RateSheetStore` pattern) so tests stub one method, not all 33.
pub trait LlpaStore {
    fn llpa_price(
        &self,
        agency: GseAgency,
        scenario: &LlpaScenario,
        lender_id: Option<&str>,
        year: u16,
    ) -> Result<Result<types::Derived<LlpaPricing>, Ineligible>, RefDataError>;
}
impl<S: RefDataStore> LlpaStore for S {
    fn llpa_price(
        &self,
        agency: GseAgency,
        scenario: &LlpaScenario,
        lender_id: Option<&str>,
        year: u16,
    ) -> Result<Result<types::Derived<LlpaPricing>, Ineligible>, RefDataError> {
        RefDataStore::llpa_price(self, agency, scenario, lender_id, year)
    }
}
use scenarios::Scenario;
use solver::{PricedPoint, ScenarioPricer};
use types::{
    BalanceType, BasisPoints, Cents, CreditScore, LoanProduct, LtvBasisPoints, ProgramCode,
    TermMonths,
};

/// Whether a loan product is fixed-rate (vs an ARM).
#[must_use]
pub fn product_is_fixed(product: LoanProduct) -> bool {
    use LoanProduct::*;
    matches!(
        product,
        FixedConv8To10
            | FixedConv11To15
            | FixedConv16To20
            | FixedConv21To30
            | FixedFha8To15
            | FixedFha16To30
            | FixedVa8To15
            | FixedVa16To30
            | FixedUsda30
            | OtcConv30
            | OtcConv15
            | OtcVa30
            | OtcVaJumbo30
    )
}

/// Borrower + property attributes the LLPA grid needs that aren't on a bare
/// `Scenario`. Supplied by the orchestrator from the wizard input + property.
#[derive(Debug, Clone)]
pub struct PricingContext {
    pub indicator_score: CreditScore,
    pub purpose: LlpaPurpose,
    pub occupancy: LlpaOccupancy,
    pub property_type: LlpaPropertyType,
    pub state: String,
    pub is_first_time_homebuyer: bool,
    pub is_high_cost_area: bool,
    pub ami_percent: Option<u16>,
    /// GSE whose grid to price against (Fannie/Freddie).
    pub agency: GseAgency,
    /// Pricing year for versioned grid lookup.
    pub year: u16,
}

/// The narrow slice of `RefDataStore` the pricer needs. Blanket-impl'd for any
/// `RefDataStore`, so production passes the real store and tests stub only this
/// one method (the `EligibilityData` pattern, applied to pricing).
pub trait RateSheetStore {
    fn rate_sheet(&self, lender_id: &str) -> Result<Option<RateSheet>, RefDataError>;
}

impl<S: RefDataStore> RateSheetStore for S {
    fn rate_sheet(&self, lender_id: &str) -> Result<Option<RateSheet>, RefDataError> {
        RefDataStore::rate_sheet(self, lender_id)
    }
}

/// Default rate lock period (days) until the borrower's choice is threaded in.
const DEFAULT_LOCK_DAYS: u8 = 30;

/// Map a scenario's program/product to the rate-sheet product code.
#[must_use]
pub fn rate_sheet_product(program: ProgramCode, product: LoanProduct) -> &'static str {
    use LoanProduct::*;
    match product {
        FixedConv8To10 | FixedConv11To15 | FixedConv16To20 | FixedConv21To30 => match program {
            ProgramCode::HomeReady => "conv_homeready_fixed",
            ProgramCode::HomePossible => "conv_homepossible_fixed",
            _ => "conv_30yr_fixed",
        },
        FixedFha8To15 | FixedFha16To30 => "fha_30yr_fixed",
        FixedVa8To15 | FixedVa16To30 => "va_30yr_fixed",
        FixedUsda30 => "usda_30yr_fixed",
        Arm5_6Sofr | Arm5_1 => "conv_5yr_arm",
        Arm7_6Sofr | Arm7_1 => "conv_7yr_arm",
        Arm10_6Sofr | Arm10_1 => "conv_10yr_arm",
        OtcConv30 => "otc_conv_30yr_fixed",
        OtcConv15 => "otc_conv_15yr_fixed",
        OtcVa30 => "otc_va_30yr_fixed",
        OtcVaJumbo30 => "otc_va_jumbo_30yr_fixed",
    }
}

/// The real pricer. Holds a `RateSheetStore`, the scenario being priced, the
/// property value (for LTV), the lender id, and the pricing year.
pub struct StorePricer<'a, S: RateSheetStore> {
    store: &'a S,
    scenario: Scenario,
    property_value: Cents,
    lender_id: String,
    /// Borrower/property LLPA attributes (P2).
    ctx: PricingContext,
    /// Eligible starting-balance bounds for this scenario.
    min_balance: Cents,
    max_balance: Cents,
}

impl<S: RateSheetStore + LlpaStore> std::fmt::Debug for StorePricer<'_, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StorePricer")
            .field("scenario", &self.scenario)
            .field("property_value", &self.property_value)
            .field("lender_id", &self.lender_id)
            .finish_non_exhaustive()
    }
}

impl<'a, S: RateSheetStore + LlpaStore> StorePricer<'a, S> {
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        store: &'a S,
        scenario: Scenario,
        property_value: Cents,
        lender_id: impl Into<String>,
        ctx: PricingContext,
        min_balance: Cents,
        max_balance: Cents,
    ) -> Self {
        StorePricer {
            store,
            scenario,
            property_value,
            lender_id: lender_id.into(),
            ctx,
            min_balance,
            max_balance,
        }
    }

    /// LTV in basis points for a given balance against the property value.
    fn ltv_at(&self, balance: Cents) -> LtvBasisPoints {
        if self.property_value.0 <= 0 {
            return LtvBasisPoints(0);
        }
        // bps = balance / value * 10000
        let bps = (balance.0 as i128 * 10_000 / self.property_value.0 as i128) as u32;
        LtvBasisPoints(bps)
    }

    /// Base note rate from the lender's rate sheet for this scenario's product.
    fn base_rate(&self) -> Result<BasisPoints, RefDataError> {
        let product = rate_sheet_product(self.scenario.program, self.scenario.product);
        let sheet = self
            .store
            .rate_sheet(&self.lender_id)?
            .ok_or_else(|| RefDataError::Storage(format!("rate_sheet {}", self.lender_id)))?;
        let entry = sheet
            .find(product, DEFAULT_LOCK_DAYS)
            .ok_or_else(|| RefDataError::Storage(format!("{product}@{DEFAULT_LOCK_DAYS}d")))?;
        Ok(BasisPoints(entry.par_rate_bps))
    }

    /// P2 — LLPA price adjustment for this scenario at a given balance/LTV.
    /// Returns the net price adjustment in basis points (of price). Ineligible
    /// or non-GSE programs (FHA/VA/USDA) return 0 — their pricing is the
    /// agency's own MIP/fee, handled in P3, not the GSE LLPA grid.
    fn llpa_bps_at(&self, balance: Cents, ltv: LtvBasisPoints) -> i32 {
        // GSE LLPA only applies to conventional/affordable-conventional.
        if self.scenario.program.is_government()
            || matches!(self.scenario.program, ProgramCode::Usda)
        {
            return 0;
        }
        let scenario = LlpaScenario {
            purpose: self.ctx.purpose,
            occupancy: self.ctx.occupancy,
            property_type: self.ctx.property_type,
            indicator_score: self.ctx.indicator_score,
            ltv,
            loan_amount: balance,
            is_arm: !product_is_fixed(self.scenario.product),
            is_high_balance: matches!(self.scenario.balance_type, BalanceType::HighBalance),
            is_super_conforming: matches!(self.scenario.balance_type, BalanceType::SuperConforming),
            has_subordinate_financing: false,
            heloc_balance_at_closing: Cents::ZERO,
            has_affordable_second: false,
            state: self.ctx.state.clone(),
            ami_percent: self.ctx.ami_percent,
            is_first_time_homebuyer: self.ctx.is_first_time_homebuyer,
            is_high_cost_area: self.ctx.is_high_cost_area,
            is_duty_to_serve: false,
            is_home_ready_or_possible: matches!(
                self.scenario.program,
                ProgramCode::HomeReady | ProgramCode::HomePossible
            ),
        };
        match self.store.llpa_price(
            self.ctx.agency,
            &scenario,
            Some(&self.lender_id),
            self.ctx.year,
        ) {
            Ok(Ok(pricing)) => pricing.value.total_bps,
            // Ineligible or lookup error → no adjustment (scenario is pruned
            // elsewhere by eligibility; pricing just contributes nothing).
            Ok(Err(_)) | Err(_) => 0,
        }
    }
}

impl<S: RateSheetStore + LlpaStore> ScenarioPricer for StorePricer<'_, S> {
    fn price_at(&self, balance: Cents) -> Option<PricedPoint> {
        if balance.0 < self.min_balance.0 || balance.0 > self.max_balance.0 {
            return None;
        }
        let ltv = self.ltv_at(balance);
        let note_rate = self.base_rate().ok()?;
        let term = TermMonths(self.scenario.term.0);
        let payment = monthly_payment(balance, note_rate, term);
        let sched = schedule(balance, note_rate, term);
        let hc = horizon_cost(&sched, term.0);

        // P2 — LLPA as discount points at closing. total_bps is price in bps
        // (hundredths of a point); cost = balance × total_bps / 10_000.
        // Negative (rebate) reduces cash-to-close. The rate-buydown alternative
        // needs a price/rate ladder the rate sheet doesn't carry yet (Epic 4–6).
        let llpa_bps = self.llpa_bps_at(balance, ltv);
        let llpa_cost = Cents((balance.0 as i128 * llpa_bps as i128 / 10_000) as i64);

        // Down payment + LLPA points (P3 adds MI, P4 the full worksheet).
        let down = Cents((self.property_value.0 - balance.0).max(0));
        let ctc = Cents(down.0 + llpa_cost.0);

        Some(PricedPoint {
            balance,
            ltv,
            mi: Cents::ZERO, // P3
            llpa_bps,
            note_rate,
            monthly_payment: payment,
            cash_to_close: ctc, // P4 expands to the full worksheet
            horizon_cost: hc,
        })
    }

    fn balance_bounds(&self) -> (Cents, Cents) {
        (self.min_balance, self.max_balance)
    }
}
