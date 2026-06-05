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
use ref_data::{RateSheet, RefDataError, RefDataStore};

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
use scenarios::Scenario;
use solver::{PricedPoint, ScenarioPricer};
use types::{BasisPoints, Cents, LoanProduct, LtvBasisPoints, ProgramCode, TermMonths};

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
        // Any product variant not explicitly mapped falls back to the
        // conventional 30-year fixed sheet (P1 default; refined as products
        // are added to rate sheets in ingestion, Epic 4–6).
        _ => "conv_30yr_fixed",
    }
}

/// The real pricer. Holds a `RateSheetStore`, the scenario being priced, the
/// property value (for LTV), the lender id, and the pricing year.
pub struct StorePricer<'a, S: RateSheetStore> {
    store: &'a S,
    scenario: Scenario,
    property_value: Cents,
    lender_id: String,
    /// Eligible starting-balance bounds for this scenario.
    min_balance: Cents,
    max_balance: Cents,
}

impl<S: RateSheetStore> std::fmt::Debug for StorePricer<'_, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StorePricer")
            .field("scenario", &self.scenario)
            .field("property_value", &self.property_value)
            .field("lender_id", &self.lender_id)
            .finish_non_exhaustive()
    }
}

impl<'a, S: RateSheetStore> StorePricer<'a, S> {
    #[must_use]
    pub fn new(
        store: &'a S,
        scenario: Scenario,
        property_value: Cents,
        lender_id: impl Into<String>,
        min_balance: Cents,
        max_balance: Cents,
    ) -> Self {
        StorePricer {
            store,
            scenario,
            property_value,
            lender_id: lender_id.into(),
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
}

impl<S: RateSheetStore> ScenarioPricer for StorePricer<'_, S> {
    fn price_at(&self, balance: Cents) -> Option<PricedPoint> {
        if balance.0 < self.min_balance.0 || balance.0 > self.max_balance.0 {
            return None;
        }
        let ltv = self.ltv_at(balance);
        let note_rate = self.base_rate().ok()?; // P2 adds LLPA on top
        let term = TermMonths(self.scenario.term.0);
        let payment = monthly_payment(balance, note_rate, term);
        let sched = schedule(balance, note_rate, term);
        // Hold horizon defaults to full term here; the orchestrator threads the
        // borrower's stated horizon in P5.
        let hc = horizon_cost(&sched, term.0);
        // P1 placeholder CTC: down payment only (= property_value − balance).
        let down = Cents((self.property_value.0 - balance.0).max(0));

        Some(PricedPoint {
            balance,
            ltv,
            mi: Cents::ZERO, // P3
            llpa_bps: 0,     // P2
            note_rate,
            monthly_payment: payment,
            cash_to_close: down, // P4 expands to the full worksheet
            horizon_cost: hc,
        })
    }

    fn balance_bounds(&self) -> (Cents, Cents) {
        (self.min_balance, self.max_balance)
    }
}
