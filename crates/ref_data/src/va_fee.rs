//! VA loan funding fee lookup.
//!
//! Source: VA Pamphlet VAP26-7 Chapter 8 (current as of April 2023).
//! VA Circular 26-23-11 confirmed current fee schedule.
//!
//! # Who pays?
//!
//! Every veteran must pay a funding fee unless exempt (disability compensation,
//! surviving spouse on service-connected death, pre-discharge rated ≥10%).
//! Exempt veterans get 0% — stored as `fee_bps = 0` in the result.
//!
//! # Updating fees
//!
//! Congress periodically adjusts rates. Update `data/va_funding_fees_{year}.json`
//! when VA issues a new circular. No Rust code changes needed.

use serde::{Deserialize, Serialize};

/// Veteran service branch category (affects funding fee tier).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VeteranCategory {
    RegularMilitary,
    ReservesNationalGuard,
    /// Exempt from funding fee (disability, surviving spouse, etc.).
    /// Returns fee_bps = 0 regardless of other parameters.
    Exempt,
}

/// VA loan purpose (each has different fee tiers).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VaLoanPurpose {
    PurchaseOrConstruction,
    CashOutRefinance,
    /// Interest Rate Reduction Refinance Loan (streamline refi).
    Irrrl,
    /// Manufactured home not permanently affixed to land.
    ManufacturedHomeNotPermanent,
    LoanAssumption,
}

/// Whether this is the veteran's first use of VA home loan entitlement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VaUse {
    FirstTime,
    /// All subsequent uses carry higher fees on 0-down purchase loans.
    Subsequent,
}

/// Parameters needed to look up the VA funding fee.
#[derive(Debug, Clone)]
pub struct VaFeeInput {
    pub category: VeteranCategory,
    pub purpose: VaLoanPurpose,
    pub use_: VaUse,
    /// Down payment as a percentage of purchase price, in basis points.
    /// 0 = no down payment. 500 = 5%. 1000 = 10%.
    /// Only meaningful for `PurchaseOrConstruction`.
    pub down_payment_bps: u32,
}

/// One row from `va_funding_fees_{year}.json`.
#[derive(Debug, Clone, Deserialize)]
pub struct VaFeeRow {
    /// JSON-serialized `VeteranCategory`.
    pub veteran_category: String,
    /// JSON-serialized `VaLoanPurpose`.
    pub loan_purpose: String,
    /// Minimum down payment (inclusive, bps). 0 for no-down tier.
    pub down_payment_min_bps: u32,
    /// Maximum down payment (exclusive, bps). 999_999 for top tier (10%+).
    pub down_payment_max_bps: u32,
    /// Fee in basis points for first-time use.
    pub first_use_fee_bps: u32,
    /// Fee in basis points for subsequent use.
    pub subsequent_use_fee_bps: u32,
}

/// Top-level structure of `va_funding_fees_{year}.json`.
#[derive(Debug, Deserialize)]
pub struct VaFeeTable {
    pub effective_date: String,
    pub rows: Vec<VaFeeRow>,
}

impl VaFeeTable {
    /// Look up the VA funding fee in basis points.
    ///
    /// Returns `0` for exempt veterans. Returns `None` if no matching row
    /// is found (should not happen with a complete data file).
    pub fn lookup(&self, input: &VaFeeInput) -> Option<u32> {
        if input.category == VeteranCategory::Exempt {
            return Some(0);
        }

        let cat_str = match input.category {
            VeteranCategory::RegularMilitary => "regular_military",
            VeteranCategory::ReservesNationalGuard => "reserves_national_guard",
            VeteranCategory::Exempt => unreachable!(),
        };
        let purpose_str = match input.purpose {
            VaLoanPurpose::PurchaseOrConstruction => "purchase_or_construction",
            VaLoanPurpose::CashOutRefinance => "cash_out_refinance",
            VaLoanPurpose::Irrrl => "irrrl",
            VaLoanPurpose::ManufacturedHomeNotPermanent => "manufactured_home_not_permanent",
            VaLoanPurpose::LoanAssumption => "loan_assumption",
        };

        for row in &self.rows {
            if row.veteran_category != cat_str {
                continue;
            }
            if row.loan_purpose != purpose_str {
                continue;
            }
            if input.down_payment_bps < row.down_payment_min_bps
                || input.down_payment_bps >= row.down_payment_max_bps
            {
                continue;
            }
            return Some(match input.use_ {
                VaUse::FirstTime => row.first_use_fee_bps,
                VaUse::Subsequent => row.subsequent_use_fee_bps,
            });
        }
        None
    }
}
