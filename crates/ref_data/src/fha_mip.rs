//! FHA mortgage insurance premium rates.
//!
//! Source: HUD Handbook 4000.1 Appendix 1.0 (last revised 11/26/2025).
//!
//! # Two-part FHA MI structure
//!
//! **UFMIP** — Upfront Mortgage Insurance Premium, financed into the loan.
//! Standard rate: 175 bps (1.75%). Special streamline refi rate: 1 bps (0.01%).
//!
//! **Annual MIP** — Paid monthly as 1/12th of the annual rate applied to the
//! outstanding principal balance. Rate and duration depend on term, LTV, and
//! loan size (standard vs. high-balance).
//!
//! # Updating rates
//!
//! All rate data lives in `data/fha_mip_rates_{year}.json`. When HUD publishes
//! a Mortgagee Letter with new MIP rates:
//! 1. Add a new `fha_mip_rates_{YYYY}.json` file with updated values.
//! 2. No Rust code changes needed — the store picks up the latest file.

use serde::{Deserialize, Serialize};
use types::Cents;

// ── High-balance threshold ────────────────────────────────────────────────────

/// FHA high-balance loan threshold as of 2024-2025 (national ceiling ceiling).
/// Stored in the JSON data file as `high_balance_threshold_cents`; this
/// constant is only used as a fallback when the data file is unavailable.
pub const FHA_HIGH_BALANCE_THRESHOLD_CENTS: i64 = 72_620_000; // $726,200.00

// ── Duration ──────────────────────────────────────────────────────────────────

/// How long annual MIP is assessed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MipDuration {
    /// Fixed number of years (e.g., 11 years). Coverage cancels automatically.
    Years(u8),
    /// Remains for the full loan term. Cannot be cancelled except by refinance.
    LoanTerm,
}

// ── Input / output ────────────────────────────────────────────────────────────

/// Parameters needed to look up FHA MIP rates.
#[derive(Debug, Clone)]
pub struct FhaMipInput {
    /// Original loan term in months.
    pub term_months: u16,
    /// LTV at origination in basis points (e.g. 9650 = 96.50%).
    pub ltv_bps: u32,
    /// Base loan amount in cents (used to detect high-balance loans).
    pub base_loan_cents: i64,
    /// True for Streamline/Simple Refi of a pre-June 2009 FHA endorsement.
    /// Activates special 1-bps UFMIP and 55-bps annual MIP schedule.
    pub is_streamline_pre_2009: bool,
}

/// FHA MIP rates for a specific loan scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FhaMipResult {
    /// UFMIP in basis points (175 or 1 for special streamline refi).
    pub ufmip_bps: u16,
    /// Annual MIP in basis points (applied to outstanding principal / 12 monthly).
    pub annual_mip_bps: u16,
    /// How long annual MIP is assessed.
    pub duration: MipDuration,
}

impl FhaMipResult {
    /// Monthly MIP amount given current outstanding principal balance.
    ///
    /// Uses ceiling division — conservative for underwriting.
    #[must_use]
    pub fn monthly_mip(&self, outstanding_principal: Cents) -> Cents {
        if outstanding_principal.0 <= 0 {
            return Cents(0);
        }
        // annual_mip = principal × rate_bps / 10_000
        // monthly = annual / 12 (ceiling)
        let annual = outstanding_principal.0 as i128 * i128::from(self.annual_mip_bps) / 10_000;
        let monthly = (annual + 11) / 12;
        Cents(monthly as i64)
    }

    /// UFMIP as a Cents amount for a given base loan amount.
    #[must_use]
    pub fn ufmip_amount(&self, base_loan_cents: i64) -> Cents {
        let amount = base_loan_cents as i128 * i128::from(self.ufmip_bps) / 10_000;
        Cents(amount as i64)
    }
}

// ── JSON-deserialized row ────────────────────────────────────────────────────

/// One row from `fha_mip_rates_{year}.json`.
///
/// The JSON file encodes every scenario as a flat row. The lookup function
/// walks these rows and returns the first match for the given input.
#[derive(Debug, Clone, Deserialize)]
pub struct FhaMipTableRow {
    /// Human-readable scenario label (for debugging).
    pub scenario: String,
    /// True for Streamline/Simple Refi of pre-2009 endorsements.
    pub is_streamline_pre_2009: bool,
    /// True when this row applies to loans with term ≤ 15 years (≤ 180 months).
    pub term_le_15_years: bool,
    /// True when this row applies to loans above the high-balance threshold.
    pub high_balance: bool,
    /// Maximum LTV (inclusive) in bps that triggers this row.
    /// The row fires when `input.ltv_bps <= ltv_max_bps`.
    pub ltv_max_bps: u32,
    /// UFMIP in basis points.
    pub ufmip_bps: u16,
    /// Annual MIP in basis points.
    pub annual_mip_bps: u16,
    /// Duration in years. `null` in JSON means the rate applies for the full
    /// loan term (`MipDuration::LoanTerm`).
    pub duration_years: Option<u8>,
}

impl FhaMipTableRow {
    pub fn duration(&self) -> MipDuration {
        match self.duration_years {
            Some(y) => MipDuration::Years(y),
            None => MipDuration::LoanTerm,
        }
    }
}

/// Top-level structure of `fha_mip_rates_{year}.json`.
#[derive(Debug, Deserialize)]
pub struct FhaMipTable {
    /// High-balance threshold in cents (e.g., $726,200 → 72_620_000).
    pub high_balance_threshold_cents: i64,
    /// ISO date this table became effective.
    pub effective_date: String,
    /// All rows. Rows are evaluated in order; first match wins.
    pub rows: Vec<FhaMipTableRow>,
}

impl FhaMipTable {
    /// Look up the FHA MIP rates for a given loan scenario.
    pub fn lookup(&self, input: &FhaMipInput) -> Option<FhaMipResult> {
        let is_high_balance = input.base_loan_cents > self.high_balance_threshold_cents;
        let term_le_15 = input.term_months <= 180;

        for row in &self.rows {
            if row.is_streamline_pre_2009 != input.is_streamline_pre_2009 {
                continue;
            }
            if !input.is_streamline_pre_2009 {
                if row.term_le_15_years != term_le_15 {
                    continue;
                }
                if row.high_balance != is_high_balance {
                    continue;
                }
            }
            if input.ltv_bps <= row.ltv_max_bps {
                return Some(FhaMipResult {
                    ufmip_bps: row.ufmip_bps,
                    annual_mip_bps: row.annual_mip_bps,
                    duration: row.duration(),
                });
            }
        }
        None
    }
}
