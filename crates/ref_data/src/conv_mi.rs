//! Conventional mortgage insurance coverage requirements and provider rates.
//!
//! # Coverage requirements
//!
//! Fannie Mae (B7-1-02, 08/07/2019) and Freddie Mac (4701.1, 02/04/2026) mandate
//! minimum MI coverage by LTV and loan type. These are stored in
//! `data/conv_mi_coverage_{year}.json` and almost never change.
//!
//! # Standard vs. minimum (custom) coverage
//!
//! Both GSEs offer a **minimum coverage** option at a lower percentage, but only
//! when the lender accepts an LLPA (Loan-Level Price Adjustment) surcharge. For
//! underwriting, always compute using **standard coverage** unless the lender has
//! explicitly opted into minimum coverage. The `ConvMiCoverage` struct carries both.
//!
//! # MI provider rates
//!
//! MI premium rates come from individual MI providers. National Mortgage Insurance
//! (National MI) monthly premium rates are seeded from
//! `data/mi_rates_nmi_monthly_{year}.json`.
//!
//! Additional providers (MGIC, Radian, Essent, Arch, etc.) can be added by
//! dropping a new JSON file and registering the provider name. No code changes needed.
//!
//! # Updating rates
//!
//! - **GSE coverage table**: update `conv_mi_coverage_{YYYY}.json` if Fannie/Freddie
//!   revise their published tables (very rare).
//! - **MI provider rates**: replace or add a new `mi_rates_{provider}_monthly_{YYYY}.json`.
//!   Rates are updated quarterly.

use serde::{Deserialize, Serialize};
use types::Cents;

// ── Program types ─────────────────────────────────────────────────────────────

/// Conventional MI program — affects required coverage percentage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConvMiProgram {
    /// Standard conventional loan (FNMA/FHLMC).
    Standard,
    /// Fannie Mae HomeReady® — reduced MI at 97% LTV.
    HomeReady,
    /// Freddie Mac Home Possible® — same reduced MI as HomeReady.
    HomePossible,
}

// ── Input / output ────────────────────────────────────────────────────────────

/// Parameters for looking up required MI coverage percentage.
#[derive(Debug, Clone)]
pub struct ConvMiInput {
    pub program: ConvMiProgram,
    /// Loan term in months (≤ 240 = short-term bucket; > 240 = standard bucket).
    pub term_months: u16,
    /// LTV in basis points at origination.
    pub ltv_bps: u32,
    /// True for ARMs — ARMs follow the same coverage table as fixed >20yr.
    pub is_arm: bool,
    /// Standard manufactured home (not MH Advantage / CHOICEHome).
    pub is_standard_manufactured: bool,
}

/// Required MI coverage percentages from Fannie Mae / Freddie Mac.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvMiCoverage {
    /// Standard (agency-mandated) coverage percentage.
    pub standard_pct: u8,
    /// Minimum coverage allowed with a corresponding MI LLPA surcharge.
    /// Equal to `standard_pct` when no minimum-coverage option is offered.
    pub minimum_pct: u8,
    /// True when using `minimum_pct` triggers a Loan-Level Price Adjustment.
    pub llpa_with_minimum: bool,
}

// ── JSON row ──────────────────────────────────────────────────────────────────

/// One row from `conv_mi_coverage_{year}.json`.
#[derive(Debug, Deserialize)]
pub struct ConvMiCoverageRow {
    pub program: String, // "standard" | "home_ready" | "home_possible"
    /// True for fixed-rate loans with term ≤ 20 years.
    pub term_le_20yr: bool,
    /// True for ARMs and fixed loans with term > 20 years.
    pub is_standard_term: bool, // >20yr fixed or ARMs
    /// True for standard manufactured homes (limited to ≤ 95% LTV).
    pub is_manufactured: bool,
    pub ltv_min_bps: u32, // exclusive lower bound
    pub ltv_max_bps: u32, // inclusive upper bound
    pub standard_coverage_pct: u8,
    pub minimum_coverage_pct: u8,
    pub llpa_with_minimum: bool,
}

#[derive(Debug, Deserialize)]
pub struct ConvMiCoverageTable {
    pub effective_date: String,
    pub rows: Vec<ConvMiCoverageRow>,
}

impl ConvMiCoverageTable {
    pub fn lookup(&self, input: &ConvMiInput) -> Option<ConvMiCoverage> {
        let prog_str = match input.program {
            ConvMiProgram::Standard => "standard",
            ConvMiProgram::HomeReady => "home_ready",
            ConvMiProgram::HomePossible => "home_possible",
        };
        let is_le_20yr = !input.is_arm && input.term_months <= 240;
        let is_standard_term = input.is_arm || input.term_months > 240;

        for row in &self.rows {
            if row.program != prog_str {
                continue;
            }
            if input.is_standard_manufactured && !row.is_manufactured {
                continue;
            }
            if !input.is_standard_manufactured && row.is_manufactured {
                continue;
            }
            // Only one of term_le_20yr / is_standard_term should be true
            if row.term_le_20yr && !is_le_20yr {
                continue;
            }
            if row.is_standard_term && !is_standard_term {
                continue;
            }
            if input.ltv_bps <= row.ltv_min_bps || input.ltv_bps > row.ltv_max_bps {
                continue;
            }
            return Some(ConvMiCoverage {
                standard_pct: row.standard_coverage_pct,
                minimum_pct: row.minimum_coverage_pct,
                llpa_with_minimum: row.llpa_with_minimum,
            });
        }
        None
    }
}

// ── MI provider monthly rates ─────────────────────────────────────────────────

/// FICO band boundaries used in MI rate tables.
/// Bands (min, max_inclusive): (760+)=760-999, then 740-759, 720-739, …, 620-639.
pub const FICO_BANDS: [(u16, u16); 8] = [
    (760, 999),
    (740, 759),
    (720, 739),
    (700, 719),
    (680, 699),
    (660, 679),
    (640, 659),
    (620, 639),
];

/// Parameters for looking up a MI provider's monthly premium rate.
#[derive(Debug, Clone)]
pub struct MiRateInput {
    /// LTV at origination in bps.
    pub ltv_bps: u32,
    /// MI coverage percentage required (from `ConvMiCoverage.standard_pct`).
    pub coverage_pct: u8,
    /// Representative credit score (median of borrowers).
    pub fico: u16,
    /// Loan term in months (determines rate table).
    pub term_months: u16,
    /// True for ARMs and loans with payment changes in the first 5 years.
    pub is_non_fixed: bool,
}

/// One row from `mi_rates_{provider}_monthly_{year}.json`.
#[derive(Debug, Deserialize)]
pub struct MiMonthlyRow {
    /// Exclusive lower bound of LTV range in bps.
    pub ltv_min_bps: u32,
    /// Inclusive upper bound of LTV range in bps.
    pub ltv_max_bps: u32,
    /// Coverage percentage this row applies to.
    pub coverage_pct: u8,
    /// True for loans with term > 20 years (> 240 months); false for ≤ 20 years.
    pub term_gt_20yr: bool,
    /// Annual rates in basis points for the 8 standard FICO bands (760+, 740-759, ..., 620-639).
    pub rates_by_fico: [u16; 8],
    /// Minimum annual rate in bps (floor after all adjustments). Per National MI: 14 bps.
    #[serde(default = "default_mi_floor")]
    pub floor_bps: u16,
}

fn default_mi_floor() -> u16 {
    14 // National MI minimum: 0.14% per year
}

/// Top-level structure of `mi_rates_{provider}_monthly_{year}.json`.
#[derive(Debug, Deserialize)]
pub struct MiMonthlyTable {
    pub provider: String,
    pub effective_date: String,
    /// Annual floor after all adjustments (from provider notes).
    pub floor_bps: u16,
    pub rows: Vec<MiMonthlyRow>,
}

impl MiMonthlyTable {
    /// Look up the base annual MI rate in basis points for a loan scenario.
    ///
    /// Returns `None` if no matching row is found. For non-fixed rate loans,
    /// callers must multiply the result by 1.25 and round to the nearest bps.
    pub fn lookup_annual_bps(&self, input: &MiRateInput) -> Option<u16> {
        let term_gt_20yr = input.term_months > 240;

        for row in &self.rows {
            if row.coverage_pct != input.coverage_pct {
                continue;
            }
            if row.term_gt_20yr != term_gt_20yr {
                continue;
            }
            if input.ltv_bps <= row.ltv_min_bps || input.ltv_bps > row.ltv_max_bps {
                continue;
            }
            // Find FICO band
            let rate_idx = FICO_BANDS
                .iter()
                .position(|&(min, max)| input.fico >= min && input.fico <= max)?;
            let base_rate = row.rates_by_fico[rate_idx];
            // Apply non-fixed multiplier (125%, round to nearest bps)
            let rate = if input.is_non_fixed {
                ((u32::from(base_rate) * 125 + 50) / 100) as u16
            } else {
                base_rate
            };
            return Some(rate.max(self.floor_bps));
        }
        None
    }

    /// Monthly MI amount for a loan given current outstanding balance.
    pub fn monthly_mi(&self, input: &MiRateInput, outstanding_principal: Cents) -> Option<Cents> {
        let annual_bps = self.lookup_annual_bps(input)?;
        if outstanding_principal.0 <= 0 {
            return Some(Cents(0));
        }
        let annual = outstanding_principal.0 as i128 * i128::from(annual_bps) / 10_000;
        let monthly = (annual + 11) / 12;
        Some(Cents(monthly as i64))
    }
}

// ── USDA guarantee fees ───────────────────────────────────────────────────────

/// USDA Section 502 Guaranteed Loan fee structure.
///
/// Source: HB 1-3555 Chapter 16; current FY2025 rates from RD Instruction 440.1 Exhibit K.
///
/// Both fees are financed into the loan (or paid at closing).
/// Annual fee is paid to the Agency each year on the outstanding principal balance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsdaGuaranteeFees {
    /// Upfront guarantee fee in basis points (e.g., 100 = 1.00%).
    pub upfront_fee_bps: u32,
    /// Annual fee in basis points (e.g., 35 = 0.35%).
    pub annual_fee_bps: u32,
    pub effective_date: String,
    pub fiscal_year: u16,
}

impl UsdaGuaranteeFees {
    /// Upfront fee amount for a given base loan amount.
    #[must_use]
    pub fn upfront_amount(&self, base_loan_cents: i64) -> Cents {
        let fee = base_loan_cents as i128 * i128::from(self.upfront_fee_bps) / 10_000;
        Cents(fee as i64)
    }

    /// Monthly annual fee amount given current outstanding principal.
    #[must_use]
    pub fn monthly_annual_fee(&self, outstanding_principal: Cents) -> Cents {
        if outstanding_principal.0 <= 0 {
            return Cents(0);
        }
        let annual = outstanding_principal.0 as i128 * i128::from(self.annual_fee_bps) / 10_000;
        let monthly = (annual + 11) / 12;
        Cents(monthly as i64)
    }
}
