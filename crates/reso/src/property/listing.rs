//! HOA, tax, pricing, and listing helpers — Categories 13–21 (selected).
//!
//! Covers the financial fields the engine consumes during scenario analysis:
//! HOA fees, property taxes, list/close prices, and buyer financing flags.
//!
//! # HOA frequency normalization
//!
//! `AssociationFeeFrequency` has five RESO values. All are normalized to
//! a monthly equivalent `Cents` value:
//!
//! | Frequency | Divisor |
//! |---|---|
//! | Monthly | 1 |
//! | Annually | 12 |
//! | Quarterly | 3 |
//! | Semi-Annually | 6 |
//! | OneTime | — (not recurring, returns `None`) |
//!
//! # Cents conversion
//!
//! All price/tax/fee fields in RESO are `Edm.Decimal` (dollars with cents).
//! Conversion to `types::Cents` (integer ¢): `dollars × 100`, rounded half-up.
//!
//! # Flood zone — SFHA designation
//!
//! The National Flood Insurance Program (NFIP) requires flood insurance for
//! properties in a Special Flood Hazard Area (SFHA). FEMA designates SFHAs
//! with zone codes beginning with "A" or "V". Zone "X" is minimal hazard —
//! no insurance required. `is_flood_insurance_required()` checks the zone code
//! prefix to determine mandatory coverage status.

use rust_decimal::{prelude::ToPrimitive, Decimal};

use crate::property::PropertyReso;
use types::Cents;

/// Multiply a dollar `Decimal` by 100 and return integer `Cents`.
/// Rounds half-up (standard for monetary values).
fn to_cents(dollars: Decimal) -> Option<Cents> {
    let cents = (dollars * Decimal::ONE_HUNDRED).round_dp(0);
    cents.to_i64().map(Cents)
}

impl PropertyReso {
    // ── HOA ───────────────────────────────────────────────────────────────────

    /// True if property is subject to an HOA (`AssociationYN`).
    pub fn hoa_yn(&self) -> bool {
        self.association_yn.unwrap_or(false)
    }

    /// Primary HOA fee normalized to a monthly `Cents` equivalent.
    ///
    /// Returns `None` when:
    /// - `AssociationFee` is absent
    /// - Frequency is "OneTime" (not a recurring expense)
    /// - Frequency is unknown/unparseable
    pub fn hoa_monthly_cents(&self) -> Option<Cents> {
        let fee = self.association_fee?;
        let freq = self
            .association_fee_frequency
            .as_deref()
            .unwrap_or("Monthly");
        normalize_to_monthly(fee, freq)
    }

    /// Primary HOA fee as an annual `Cents` amount.
    pub fn hoa_annual_cents(&self) -> Option<Cents> {
        let monthly = self.hoa_monthly_cents()?;
        monthly.0.checked_mul(12).map(Cents)
    }

    /// True if a secondary HOA fee is present (master + sub HOA model).
    pub fn has_second_hoa(&self) -> bool {
        self.association_fee2.is_some()
    }

    /// Secondary HOA fee normalized to monthly `Cents`.
    pub fn hoa2_monthly_cents(&self) -> Option<Cents> {
        let fee = self.association_fee2?;
        let freq = self
            .association_fee_frequency2
            .as_deref()
            .unwrap_or("Monthly");
        normalize_to_monthly(fee, freq)
    }

    /// Combined monthly HOA — primary + secondary fees.
    ///
    /// Returns `None` only if both HOA fees are absent.
    pub fn total_monthly_hoa_cents(&self) -> Option<Cents> {
        let p = self.hoa_monthly_cents().map(|c| c.0).unwrap_or(0);
        let s = self.hoa2_monthly_cents().map(|c| c.0).unwrap_or(0);
        if p == 0 && s == 0 {
            None
        } else {
            Some(Cents(p + s))
        }
    }

    // ── Tax ───────────────────────────────────────────────────────────────────

    /// Annual property tax as `Cents`.
    pub fn tax_annual_cents(&self) -> Option<Cents> {
        self.tax_annual_amount.and_then(to_cents)
    }

    /// Tax-assessed value as `Cents`.
    pub fn tax_assessed_cents(&self) -> Option<Cents> {
        self.tax_assessed_value.and_then(to_cents)
    }

    /// Tax year the `TaxAnnualAmount` applies to.
    pub fn tax_year(&self) -> Option<u16> {
        self.tax_year.map(|y| y as u16)
    }

    /// True if any tax exemptions are recorded.
    pub fn has_tax_exemptions(&self) -> bool {
        self.tax_exemptions
            .as_ref()
            .map(|v| !v.is_empty())
            .unwrap_or(false)
    }

    // ── Pricing ───────────────────────────────────────────────────────────────

    /// Current list price as `Cents`.
    pub fn list_price_cents(&self) -> Option<Cents> {
        self.list_price.and_then(to_cents)
    }

    /// Final close/sale price as `Cents`.
    pub fn close_price_cents(&self) -> Option<Cents> {
        self.close_price.and_then(to_cents)
    }

    /// Original list price when first listed as `Cents`.
    pub fn original_list_price_cents(&self) -> Option<Cents> {
        self.original_list_price.and_then(to_cents)
    }

    /// Dollar amount of price reduction from original to current list price.
    ///
    /// Returns `None` if either price is absent or current ≥ original.
    pub fn price_reduction_amount(&self) -> Option<Cents> {
        let original = self.original_list_price_cents()?;
        let current = self.list_price_cents()?;
        if original > current {
            Some(Cents(original.0 - current.0))
        } else {
            None
        }
    }

    /// True if `ListPrice` is below `OriginalListPrice`.
    pub fn is_price_reduced(&self) -> bool {
        self.price_reduction_amount().is_some()
    }

    /// Price per square foot as `Cents` (¢ per sq ft).
    ///
    /// Computed as `ListPrice ÷ LivingArea`, rounded to nearest cent.
    /// Returns `None` if either is absent or LivingArea is zero.
    pub fn price_per_sqft_cents(&self) -> Option<Cents> {
        let price = self.list_price?;
        let area = self.living_area.filter(|a| *a > Decimal::ZERO)?;
        to_cents(price / area)
    }

    // ── Listing ───────────────────────────────────────────────────────────────

    /// Days on market for the current listing period.
    pub fn days_on_market(&self) -> Option<u32> {
        self.days_on_market.map(|n| n as u32)
    }

    /// True if `BuyerFinancing` collection includes "FHA".
    pub fn has_buyer_financing_fha(&self) -> bool {
        collection_contains_ci(&self.buyer_financing, "fha")
    }

    /// True if `BuyerFinancing` collection includes "VA".
    pub fn has_buyer_financing_va(&self) -> bool {
        collection_contains_ci(&self.buyer_financing, "va")
    }

    /// True if `BuyerFinancing` collection includes "USDA".
    pub fn has_buyer_financing_usda(&self) -> bool {
        collection_contains_ci(&self.buyer_financing, "usda")
    }

    /// True if `BuyerFinancing` collection includes "Cash".
    pub fn has_buyer_financing_cash(&self) -> bool {
        collection_contains_ci(&self.buyer_financing, "cash")
    }

    /// True if seller concessions were provided (`Concessions = "Yes"`).
    pub fn has_seller_concessions(&self) -> bool {
        self.concessions
            .as_deref()
            .map(|s| s.eq_ignore_ascii_case("yes"))
            .unwrap_or(false)
    }

    // ── Flood ─────────────────────────────────────────────────────────────────

    /// True if the property is in a FEMA Special Flood Hazard Area (SFHA)
    /// requiring mandatory flood insurance under NFIP.
    ///
    /// SFHA zones start with "A" or "V":
    /// - A, AE, AO, AH, AR, A99 — inland flooding
    /// - V, VE — coastal/wave-action flooding
    ///
    /// Zone "X" (minimal hazard), "B", "C", and "D" do NOT require insurance.
    pub fn is_flood_insurance_required(&self) -> bool {
        self.flood_zone
            .as_deref()
            .map(|z| {
                let z = z.trim().to_ascii_uppercase();
                z.starts_with('A') || z.starts_with('V')
            })
            .unwrap_or(false)
    }
}

// ── Private helpers ───────────────────────────────────────────────────────────

/// Normalize a fee `Decimal` (dollars) to monthly `Cents` based on frequency.
fn normalize_to_monthly(fee: Decimal, frequency: &str) -> Option<Cents> {
    let freq = frequency.trim().to_ascii_lowercase();
    let monthly = match freq.as_str() {
        "monthly" => fee,
        "annually" | "annual" | "yearly" => fee / Decimal::from(12u32),
        "quarterly" => fee / Decimal::from(3u32),
        "semiannually" | "semi-annually" | "semi annually" | "biannually" => {
            fee / Decimal::from(6u32)
        }
        "onetime" | "one time" | "one-time" => return None,
        _ => return None,
    };
    to_cents(monthly)
}

fn collection_contains_ci(v: &Option<Vec<String>>, needle: &str) -> bool {
    let needle_lower = needle.to_ascii_lowercase();
    v.as_ref()
        .map(|items| {
            items
                .iter()
                .any(|s| s.to_ascii_lowercase().contains(&needle_lower))
        })
        .unwrap_or(false)
}
