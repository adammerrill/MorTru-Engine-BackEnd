//! Loan-Level Price Adjustments (LLPA) and lender rate sheets.
//!
//! # LLPA overview
//!
//! Fannie Mae and Freddie Mac publish price adjustment grids that modify the
//! interest rate or points on a conventional loan based on risk factors. The
//! primary grid is Credit Score × LTV. Additional scenario adjustments apply
//! for cash-out, manufactured homes, second homes, investment properties, etc.
//!
//! LLPAs are expressed in **basis points** (positive = cost to borrower).
//! They are added to the loan's base price and ultimately affect rate/points.
//!
//! Source: FNMA LLPA Matrix (effective 2024). Updated regularly.
//!
//! # Rate sheet
//!
//! A `RateSheet` captures a lender's par rates at a point in time. Because rates
//! change throughout the day, rate sheets are the most volatile data in the system.
//! They are seeded with a test fixture for integration testing; in production they
//! are refreshed from a pricing engine.
//!
//! # Updating LLPAs
//!
//! FNMA/FHLMC publish updated LLPA matrices quarterly or on ad-hoc schedule.
//! Replace `data/llpa_matrix_fnma_{year}.json` to pick up new values.
//! No Rust code changes are needed.

use serde::{Deserialize, Serialize};

// ── LLPA matrix ───────────────────────────────────────────────────────────────

/// Parameters for a LLPA grid lookup.
#[derive(Debug, Clone)]
pub struct LlpaInput {
    pub fico: u16,
    pub ltv_bps: u32,
    /// Loan purpose: "purchase", "rate_term_refi", "cash_out_refi".
    pub loan_purpose: String,
    /// Occupancy: "primary", "second_home", "investment".
    pub occupancy: String,
    /// True for standard manufactured home (not MH Advantage).
    pub is_standard_manufactured: bool,
    /// True for loan amount above FHFA conforming limit.
    pub is_high_balance: bool,
}

/// One row in `llpa_matrix_fnma_{year}.json`.
#[derive(Debug, Clone, Deserialize)]
pub struct LlpaRow {
    pub fico_min: u16,
    pub fico_max: u16,
    pub ltv_min_bps: u32, // exclusive lower bound
    pub ltv_max_bps: u32, // inclusive upper bound
    /// Adjustment category: "credit_score_ltv", "cash_out", "second_home",
    /// "investment", "manufactured_home", "high_balance".
    pub category: String,
    /// Price adjustment in basis points. Positive = cost; negative = credit.
    pub adjustment_bps: i32,
}

/// Top-level shape of `llpa_matrix_fnma_{year}.json`.
#[derive(Debug, Deserialize)]
pub struct LlpaMatrix {
    pub agency: String,
    pub effective_date: String,
    pub rows: Vec<LlpaRow>,
}

impl LlpaMatrix {
    /// Compute total LLPA in basis points for a loan scenario.
    ///
    /// Multiple categories may apply; this sums all matching rows.
    pub fn total_llpa(&self, input: &LlpaInput) -> i32 {
        let mut total: i32 = 0;

        // 1. Credit score / LTV grid (always applied)
        if let Some(cs_ltv) = self.lookup_category(input, "credit_score_ltv") {
            total += cs_ltv;
        }

        // 2. Scenario adjustments
        if input.loan_purpose == "cash_out_refi" {
            if let Some(adj) = self.lookup_category(input, "cash_out") {
                total += adj;
            }
        }
        if input.occupancy == "second_home" {
            if let Some(adj) = self.lookup_category(input, "second_home") {
                total += adj;
            }
        }
        if input.occupancy == "investment" {
            if let Some(adj) = self.lookup_category(input, "investment") {
                total += adj;
            }
        }
        if input.is_standard_manufactured {
            if let Some(adj) = self.lookup_category(input, "manufactured_home") {
                total += adj;
            }
        }
        if input.is_high_balance {
            if let Some(adj) = self.lookup_category(input, "high_balance") {
                total += adj;
            }
        }

        total
    }

    fn lookup_category(&self, input: &LlpaInput, category: &str) -> Option<i32> {
        self.rows
            .iter()
            .find(|row| {
                row.category == category
                    && input.fico >= row.fico_min
                    && input.fico <= row.fico_max
                    && input.ltv_bps > row.ltv_min_bps
                    && input.ltv_bps <= row.ltv_max_bps
            })
            .map(|row| row.adjustment_bps)
    }
}

// ── Rate sheet ────────────────────────────────────────────────────────────────

/// One product/lock-period entry in a lender's rate sheet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateSheetEntry {
    /// Product code, e.g. "conv_30yr_fixed", "fha_30yr_fixed", "va_30yr_fixed".
    pub product: String,
    /// Lock period in days (15, 30, 45, 60).
    pub lock_days: u8,
    /// Par rate in basis points (7125 = 7.125%).
    pub par_rate_bps: u32,
    /// Price of par rate in points per $100 loan amount.
    /// Positive = discount points (borrower pays); negative = rebate (YSP).
    pub price_at_par: f32,
}

/// A lender's full rate sheet at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateSheet {
    pub lender_id: String,
    pub as_of: String, // ISO 8601 datetime
    pub entries: Vec<RateSheetEntry>,
}

impl RateSheet {
    /// Look up the par rate entry for a product and lock period.
    pub fn find(&self, product: &str, lock_days: u8) -> Option<&RateSheetEntry> {
        self.entries
            .iter()
            .find(|e| e.product == product && e.lock_days == lock_days)
    }
}

/// Top-level shape of `rate_sheet_{lender}_{date}.json`.
#[derive(Debug, Deserialize)]
pub struct RateSheetFile {
    pub sheets: Vec<RateSheet>,
}
