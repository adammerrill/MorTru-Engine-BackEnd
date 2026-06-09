//! Cash-to-close fee worksheet (Epic 17 / P4a).
//!
//! # What this module is
//!
//! Through P3, the pricer produced a *placeholder* cash-to-close:
//! `down_payment + LLPA_points + upfront_MI`. That is not a real number a
//! borrower brings to closing. This module replaces that single figure with a
//! **typed, itemized, explainable worksheet** — every line a borrower would see
//! on a Loan Estimate, each summable into the true cash-to-close.
//!
//! # Why a worksheet instead of a number
//!
//! Two reasons, both load-bearing for the product:
//!
//! 1. **Defensibility.** A bare `Cents` can't be audited. A [`FeeWorksheet`] can:
//!    every dollar is a [`FeeLine`] with a [`FeeKind`] category and (once wired to
//!    `Derived<T>`) a provenance trail. "Why is my cash-to-close $X?" is answerable
//!    line by line.
//! 2. **TRID shape.** The lines map to Loan-Estimate groupings, so the same
//!    structure drives both the engine math and the consumer-facing disclosure.
//!
//! # What this module is NOT
//!
//! - It does **not** classify finance charges for APR (Reg Z §1026.4). [`FeeKind`]
//!   is a *display/aggregation* category, not a finance-charge determination. The
//!   `compliance` crate (Epic 10) tags which lines are APR-bearing. P4 computes
//!   dollars; E10 interprets them.
//! - It does **not** yet compute RESPA aggregate escrow (12 CFR 1024.17). The
//!   tax-reserve line here is a simple `annual ÷ 12 × cushion_months` estimate.
//!   The jurisdiction-correct aggregate-escrow engine is P4b — see
//!   `docs/EPIC-17-P4-fee-worksheet-design.md` §4 for how that stays region-ready.
//! - It does **not** include lender origination/discount or third-party closing
//!   costs (title, appraisal, recording) — those have no non-hardcoded data source
//!   today and land in P4c as additional [`FeeLine`]s (no rewrite).
//!
//! # Region-readiness
//!
//! Lines are built only where a real, versioned data source exists. Everything
//! else is a typed slot that fills in per-jurisdiction as `ref_data` records are
//! added — never a code branch per state. See the design doc §4.

use types::Cents;

/// TRID/Loan-Estimate grouping for a worksheet line.
///
/// This is a **classification for display and aggregation only**. It is *not* a
/// Reg Z finance-charge determination — that judgment belongs to the `compliance`
/// crate (Epic 10), which reads these lines and tags the APR-bearing ones.
///
/// Variants map to Loan-Estimate sections so one structure serves both the
/// engine and the consumer disclosure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeeKind {
    /// Borrower's down payment (sale price − loan, after any basis adjustment).
    DownPayment,
    /// LLPA discount points paid at closing (P2).
    Points,
    /// Upfront mortgage insurance: FHA UFMIP, VA funding fee, USDA guarantee (P3).
    MortgageInsurance,
    /// Prepaid item collected at closing (e.g. first-year homeowner's insurance).
    Prepaid,
    /// Escrow/impound reserve deposited at closing (e.g. property-tax cushion).
    EscrowReserve,
    /// Seller/interested-party concession credited to the borrower. Negative.
    Concession,
    // ── P4c (no data source yet; reserved so the enum is stable) ──
    /// Lender fee: origination, discount beyond LLPA. (P4c)
    LenderFee,
    /// Third-party closing cost: title, appraisal, recording, etc. (P4c)
    ThirdPartyFee,
}

/// One itemized line of the cash-to-close worksheet.
///
/// The sign convention is the whole point: **positive = borrower pays**,
/// **negative = credit/offset** (e.g. a seller concession). Summing the signed
/// amounts yields cash-to-close, so the total is always reconstructable from the
/// lines — there is no separate "total" field that could drift out of sync.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeeLine {
    /// Stable label key. A *key*, not display text, so per-tenant white-label
    /// overrides resolve it to localized copy (engineering standard: user-facing
    /// strings externalized). Kept as `&'static str` here; the content layer maps
    /// it to display text.
    pub label: &'static str,
    /// Signed amount in integer cents. Positive = borrower pays; negative = credit.
    pub amount: Cents,
    /// Display/aggregation category (NOT a finance-charge determination).
    pub kind: FeeKind,
}

impl FeeLine {
    /// Construct a worksheet line. `amount` is signed (negative = credit).
    #[must_use]
    pub fn new(label: &'static str, amount: Cents, kind: FeeKind) -> Self {
        FeeLine {
            label,
            amount,
            kind,
        }
    }
}

/// The assembled cash-to-close worksheet for a single priced point.
///
/// Invariant: cash-to-close is *defined* as the signed sum of [`Self::lines`].
/// Nothing stores a precomputed total, so the displayed number and the itemized
/// lines can never disagree.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FeeWorksheet {
    /// All worksheet lines, in display order. Positive lines are costs;
    /// negative lines (concessions/credits) reduce the total.
    pub lines: Vec<FeeLine>,
}

impl FeeWorksheet {
    /// Create an empty worksheet. Lines are added via [`Self::push`].
    #[must_use]
    pub fn new() -> Self {
        FeeWorksheet { lines: Vec::new() }
    }

    /// Append a line. Zero-amount lines are skipped so the worksheet shows only
    /// charges that actually apply (e.g. no "Concession: $0" line when the
    /// borrower negotiated none).
    pub fn push(&mut self, label: &'static str, amount: Cents, kind: FeeKind) {
        if amount.0 != 0 {
            self.lines.push(FeeLine::new(label, amount, kind));
        }
    }

    /// Cash to close = signed sum of every line.
    ///
    /// This is the single source of truth for the cash-to-close figure; callers
    /// must not cache a copy that could diverge from the lines.
    #[must_use]
    pub fn cash_to_close(&self) -> Cents {
        Cents(self.lines.iter().map(|l| l.amount.0).sum())
    }

    /// Total of all positive (borrower-pays) lines, ignoring credits. Useful for
    /// disclosures that show gross costs separately from credits.
    #[must_use]
    pub fn total_charges(&self) -> Cents {
        Cents(self.lines.iter().map(|l| l.amount.0.max(0)).sum())
    }

    /// Total of all credit (negative) lines, returned as a positive magnitude.
    #[must_use]
    pub fn total_credits(&self) -> Cents {
        Cents(-self.lines.iter().map(|l| l.amount.0.min(0)).sum::<i64>())
    }
}

/// First-year prepaid homeowner's-insurance premium, in cents.
///
/// HOI rates in `ref_data` are expressed as **annual basis points of property
/// value** ([`ref_data::ZipHoiRate`] / [`ref_data::StateHoiRate`]). The first-year
/// premium is collected as a prepaid at closing, so this is simply
/// `value × annual_rate_bps / 10_000`.
///
/// The ZIP rate is preferred when present; the caller falls back to the state
/// rate (mirroring the `ref_data` documented fall-through).
#[must_use]
pub fn prepaid_hoi(property_value: Cents, annual_rate_bps: u16) -> Cents {
    Cents(property_value.0 * i64::from(annual_rate_bps) / 10_000)
}

/// Property-tax reserve (impound cushion) collected at closing, in cents.
///
/// **This is a P4a estimate, not RESPA aggregate accounting.** It collects
/// `cushion_months` of the monthly tax amount (`tax_annual ÷ 12`). The
/// jurisdiction-correct figure — which depends on the closing date, the county
/// tax-installment calendar, and the RESPA 2-month cushion cap (12 CFR 1024.17) —
/// is the P4b escrow engine's job. `cushion_months` is a **config token** supplied
/// by the caller, never hardcoded here.
#[must_use]
pub fn tax_reserve(tax_annual: Cents, cushion_months: u8) -> Cents {
    Cents(tax_annual.0 / 12 * i64::from(cushion_months))
}

/// Monthly escrow added to the housing payment: (HOI + tax + HOA) spread monthly.
///
/// HOI and tax are annual figures divided by 12; HOA is already monthly. This is
/// the recurring escrow portion of the payment, distinct from the one-time
/// prepaid/reserve amounts collected at closing.
#[must_use]
pub fn monthly_escrow(hoi_annual: Cents, tax_annual: Cents, hoa_monthly: Cents) -> Cents {
    Cents(hoi_annual.0 / 12 + tax_annual.0 / 12 + hoa_monthly.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cash_to_close_is_signed_line_sum() {
        let mut w = FeeWorksheet::new();
        w.push("down_payment", Cents(10_000_000), FeeKind::DownPayment);
        w.push("llpa_points", Cents(500_000), FeeKind::Points);
        w.push("seller_concession", Cents(-300_000), FeeKind::Concession);
        assert_eq!(w.cash_to_close(), Cents(10_200_000));
        assert_eq!(w.total_charges(), Cents(10_500_000));
        assert_eq!(w.total_credits(), Cents(300_000));
    }

    #[test]
    fn zero_lines_are_skipped() {
        let mut w = FeeWorksheet::new();
        w.push("seller_concession", Cents::ZERO, FeeKind::Concession);
        assert!(w.lines.is_empty());
    }

    #[test]
    fn prepaid_hoi_is_bps_of_value() {
        // $500k value at 35 bps = $1,750/yr.
        assert_eq!(prepaid_hoi(Cents(50_000_000), 35), Cents(175_000));
    }

    #[test]
    fn tax_reserve_scales_with_cushion() {
        // $6,000/yr tax, 2-month cushion = $1,000.
        assert_eq!(tax_reserve(Cents(600_000), 2), Cents(100_000));
        assert_eq!(tax_reserve(Cents(600_000), 3), Cents(150_000));
    }

    #[test]
    fn monthly_escrow_sums_components() {
        // $1,800 HOI/yr ($150/mo) + $6,000 tax/yr ($500/mo) + $50 HOA/mo = $700/mo.
        assert_eq!(
            monthly_escrow(Cents(180_000), Cents(600_000), Cents(5_000)),
            Cents(70_000)
        );
    }
}
