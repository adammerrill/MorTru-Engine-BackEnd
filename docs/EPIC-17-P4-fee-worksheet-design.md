# Epic 17 / P4 — Cash-to-Close Fee Worksheet (design)

**Status:** P4a building · P4b (escrow engine) and P4c (lender/3rd-party fees) scoped, deferred
**Crate:** `orchestrator` (extends `StorePricer`)
**Depends on:** P1 (rate), P2 (LLPA), P3 (MI) — all complete
**Owner doc:** this file. Inline docs live in `crates/orchestrator/src/worksheet.rs`.

---

## 1. Why this exists

Through P3, `StorePricer::price_at` produced a *placeholder* cash-to-close:

```
cash_to_close = down_payment + LLPA_points + upfront_MI
```

That is not a real cash-to-close. A borrower's actual cash at the table also includes
prepaids (homeowner's insurance, property-tax reserves), escrow setup, lender fees,
and third-party closing costs — less any seller concession. P4 replaces the single
`Cents` placeholder with a **typed, itemized, explainable fee worksheet**: every line a
borrower would see on a Loan Estimate, each carrying its own provenance.

This is the artifact that makes the cash-to-close number *defensible* (every dollar
traces to a source) and *TRID-shaped* (the lines map to LE sections), while the Reg Z
**finance-charge classification** (which lines are APR-bearing) is deliberately left to
the `compliance` crate (Epic 10) — P4 computes the dollars; E10 tags them.

---

## 2. What ships in each slice

P4 is split by **data availability**, not by preference. A line is built only when a
real, non-hardcoded source exists for it; everything else is a typed slot that fills in
without a rewrite.

### P4a — lines with real sources today (THIS SLICE)

| Worksheet line | Source | Direction |
|----------------|--------|-----------|
| Down payment | `property_value − balance` | + CTC |
| LLPA discount points | P2 `llpa_bps_at` | + CTC |
| Upfront MI (UFMIP / VA fee / USDA guarantee) | P3 `mi_at` | + CTC |
| Prepaid homeowner's insurance (first year) | `zip_hoi_rate` ?? `state_hoi_rate` × value | + CTC |
| Property-tax reserve | `PropertyEnriched.tax_annual_cents` × cushion-month token | + CTC |
| Seller-concession offset (capped) | `evaluate_seller_concession` | − CTC |
| Monthly escrow (HOI + tax + HOA) added to payment | HOI / tax / HOA ÷ 12 | + payment |

### P4b — RESPA aggregate-escrow engine (NEXT, region-ready — see §4)
Jurisdiction-agnostic 12 CFR 1024.17 engine + per-FIPS `TaxCalendar` ref_data records.

### P4c — lender + third-party fees (LATER)
Origination, discount, title, appraisal, recording, per-diem interest — each blocked on a
lender-fee config / `enrich` source. Appended as `FeeLine`s, no rewrite.

---

## 3. The worksheet type

```rust
/// One itemized line of the cash-to-close worksheet. Every cash-to-close
/// figure is a sum of these — never a bare number — so the total is always
/// explainable line-by-line.
pub struct FeeLine {
    /// Stable, externalizable label key (per-tenant white-label override).
    pub label: FeeLabel,
    /// Signed amount. Positive = borrower pays; negative = credit/offset.
    pub amount: Cents,
    /// TRID/finance-charge category. Dollars only here; APR classification
    /// (which lines are finance charges) is the compliance crate's job.
    pub kind: FeeKind,
}

/// The assembled cash-to-close worksheet for one priced point.
pub struct FeeWorksheet {
    pub lines: Vec<FeeLine>,
}
impl FeeWorksheet {
    /// Cash to close = signed sum of all lines.
    pub fn cash_to_close(&self) -> Cents { /* Σ line.amount */ }
}
```

`FeeKind` mirrors Loan-Estimate groupings (`DownPayment`, `Points`, `MortgageInsurance`,
`Prepaid`, `EscrowReserve`, `Concession`, and — P4c — `LenderFee`, `ThirdPartyFee`). It is
a **classification for display/aggregation**, not a Reg-Z finance-charge determination.

`WorksheetInput` is the seam that carries borrower/property facts the worksheet needs that
aren't on a bare `Scenario`:

```rust
pub struct WorksheetInput {
    /// Annual property tax in cents (from PropertyEnriched today; enrich later).
    pub tax_annual: Cents,
    /// Monthly HOA dues in cents.
    pub hoa_monthly: Cents,
    /// Seller concession the borrower negotiated (0 if none).
    pub proposed_concession: Cents,
    /// Tax-reserve cushion months. A CONFIG TOKEN, never hardcoded — RESPA
    /// caps this at 2 for most loans; the real per-jurisdiction count comes
    /// from the P4b escrow engine. Defaulted from tenant config until then.
    pub tax_reserve_months: u8,
}
```

Sourced from the wizard + `PropertyEnriched` now; `enrich`-fed later — an adapter swap,
not a rewrite (mock-data-first standard).

---

## 4. Region-readiness — how a new state becomes a data-drop, not code

The escrow/tax-reserve calculation has two layers, and **only the data layer changes per
region**:

**Layer 1 — federal engine (build once, P4b).** RESPA aggregate accounting
(12 CFR 1024.17) is national law: the 2-month cushion cap, the aggregate adjustment, the
trial-balance method are identical in all 50 states. Pure calculation over
(monthly escrow items, disbursement dates, first payment date). No regional branching.

**Layer 2 — jurisdiction parameters (data, dropped per region).** What varies by
state/county is *data the engine reads*: property-tax due dates / installment schedule,
transfer & recording taxes. Each is a **versioned `ref_data` record keyed by FIPS/state** —
`tax_calendar_{fips}_{year}.json` — the same pattern as `gse_loan_limits` and
`state_hoi_rates`.

```
WorksheetInput (closing date, FIPS)
      │
      ▼
EscrowEngine  ◄─ reads ─  TaxCalendar{fips}    (ref_data, versioned, Derived<T>)
(RESPA 1024.17,           HoiRate{zip|state}    (ref_data — exists today)
 jurisdiction-blind)      TransferTax{fips}     (ref_data, per-region drop)
      │
      ▼
FeeWorksheet (typed lines, each Derived<T> citing its jurisdiction source)
```

Two rules make new regions a data-drop:

1. **The engine hardcodes no number or date.** Cushion months, due dates, rates — all from
   typed input or a versioned `ref_data` record. The engine just applies 1024.17 to
   whatever it's handed.
2. **Jurisdiction lookups return `Derived<T>` (provenance) or a typed
   `RefDataError::NotFound{fips, year}`.** A supported region cites its county file +
   version in `.explain()`. An **unsupported** region **fails loudly** — it never silently
   computes one state's math for another's property.

Texas launches with TX `tax_calendar` records. Florida = a Florida file. No escrow code
changes. This is the LOS-grade pattern (national RESPA core + per-jurisdiction parameter
tables).

---

## 5. Seller-concession over-cap handling (agency-correct + convergence-safe)

`ref_data::evaluate_seller_concession` returns `ConcessionOutcome { max_allowed, excess,
within_limit, … }`. Agency selling guides (FNMA/FHLMC/FHA/VA) and every LOS treat an
**over-cap** concession the same way: the **excess is deducted from the sales price**,
which lowers the loan basis → recalculates LTV → **re-prices MI and LLPA**.

P4a implements this correctly *and* safely by treating the concession as a **fixed
borrower input** (not a solver-optimized variable):

1. `proposed_concession` is known → `evaluate_seller_concession` → `excess` is deterministic.
2. Effective sales price = `property_value − excess` (one adjustment, no loop).
3. LTV / MI / LLPA price off the **adjusted** basis.
4. Worksheet shows the capped concession as a `Concession` credit line; the excess is
   surfaced (it changed the basis), not hidden.

**Why fixed-input matters:** the solver already iterates balance↔LTV↔MI↔LLPA↔CTC. A
concession that depends on sales price adds a second feedback path. Deterministic
(fixed-input) excess = one basis adjustment = convergence-safe. Letting the solver
*optimize* concession is a separate epic with real loop-stability work — flagged, not
silently built.

---

## 6. Test plan (P4a)

- `worksheet_cash_to_close_is_line_sum` — CTC equals Σ lines.
- `down_payment_line_present` / `llpa_line_present` / `upfront_mi_line_present`.
- `prepaid_hoi_uses_zip_then_state_fallback` — ZIP rate when present, else state.
- `tax_reserve_scales_with_cushion_months` — reserve = tax_annual ÷ 12 × months.
- `monthly_escrow_added_to_payment` — payment includes HOI+tax+HOA ÷ 12.
- `concession_within_cap_reduces_ctc` — capped credit subtracts.
- `concession_over_cap_recalcs_basis` — excess lowers basis; MI/LLPA re-price; LTV rises.
- `zero_concession_no_credit_line` — no `Concession` line when proposed = 0.

## 7. Explicitly deferred (flagged, not silently dropped)

- RESPA aggregate-escrow engine + `TaxCalendar` ref_data shape → **P4b** (next).
- Lender origination/discount + third-party closing costs → **P4c** (needs fee-config source).
- Per-diem interest (needs closing date + first-payment date) → P4c.
- Reg Z finance-charge classification of worksheet lines → **Epic 10 (compliance)**.
- Solver-optimized seller concession → separate epic (loop-stability work).
