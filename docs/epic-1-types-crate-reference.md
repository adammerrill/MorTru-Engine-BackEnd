# Epic 1 — `types` Crate Reference

**Crate:** `crates/types`  
**Tasks:** 1.1 – 1.10  
**Purpose:** Every primitive value object the engine depends on lives here.  
No business logic. No I/O. Just types, their invariants, and the conversions
between them.

---

## Table of Contents

1. [Architecture](#architecture)
2. [Precision Contract](#precision-contract)
3. [Task 1.1 — Workspace Bootstrap](#task-11--workspace-bootstrap)
4. [Task 1.2 — Money Types](#task-12--money-types)
5. [Task 1.3 — Identifier Types](#task-13--identifier-types)
6. [Task 1.4 — Error Hierarchy](#task-14--error-hierarchy)
7. [Task 1.5 — Common Enumerations](#task-15--common-enumerations)
8. [Task 1.6 — Term Primitives](#task-16--term-primitives)
9. [Task 1.7 — ScenarioKey](#task-17--scenariokey)
10. [Task 1.8 — Decimal Conversions](#task-18--decimal-conversions)
11. [Task 1.9 — GoalMask](#task-19--goalmask)
12. [Task 1.10 — CI Hardening](#task-110--ci-hardening)
13. [Type Index](#type-index)
14. [Common Patterns](#common-patterns)

---

## Architecture

```
crates/types/src/
├── lib.rs                    # Re-exports all public types (flat namespace)
│
├── cents.rs                  # Task 1.2 — Money types
├── basis_points.rs
├── price_ticks.rs
├── ltv.rs
├── dti.rs
├── credit_score.rs
├── error.rs                  # ParseError (shared validation error)
│
├── analysis_id.rs            # Task 1.3 — Identifiers
├── fips_code.rs
├── lender_id.rs
├── loan_casefile_id.rs
├── mls_listing_key.rs
├── scenario_id.rs
├── state_code.rs
│
├── errors/                   # Task 1.4 — Domain error hierarchy
│   ├── mod.rs  (= errors.rs)
│   ├── ingestion.rs
│   ├── eligibility.rs
│   ├── solver.rs
│   └── compliance.rs
│
├── enums/                    # Task 1.5 — Common enumerations
│   ├── mod.rs  (= enums.rs)
│   ├── program_code.rs
│   ├── loan_product.rs
│   ├── property_type.rs
│   ├── occupancy.rs
│   ├── loan_purpose.rs
│   ├── amortization_type.rs
│   └── misc.rs
│
├── term_band.rs              # Task 1.6 — Term primitives
├── term_months.rs
│
├── scenario_key.rs           # Task 1.7 — Scenario identifier
└── goal_mask.rs              # Tasks 1.7 / 1.9 — Goal bitflags
```

All types are re-exported flat from `types::*`. Downstream crates write
`use types::Cents;` not `use types::cents::Cents;`.

---

## Precision Contract

The engine never uses `f64` for stored financial values. This table is
binding — every calculation that crosses a crate boundary must use these types.

| Quantity | Type | Unit | Example |
|----------|------|------|---------|
| Money | `Cents` | 1 cent | `$1.50 = Cents(150)` |
| Interest rate | `BasisPoints` | 0.001% | `6.875% = BasisPoints(6875)` |
| Rate-sheet price | `PriceTicks` | 0.0001 pp | `-3.281 pt = PriceTicks(-32810)` |
| LTV ratio | `LtvBasisPoints` | 0.01% | `95.00% = LtvBasisPoints(9500)` |
| DTI ratio | `DtiBasisPoints` | 0.01% | `43.00% = DtiBasisPoints(4300)` |
| Credit score | `CreditScore` | 1 point | `720 = CreditScore(720)` |
| Loan term | `TermMonths` | 1 month | `30 yr = TermMonths(360)` |

`f64` is permitted **only** inside the Newton–Raphson APR solver as an
intermediate value. Use `Cents::as_f64_dollars()` and
`BasisPoints::as_f64_rate()` to enter the f64 domain, and
`BasisPoints::from_apr_f64()` / `Cents::from_decimal_dollars()` to leave it.

---

## Task 1.1 — Workspace Bootstrap

**File:** `Cargo.toml` (workspace root), `.github/workflows/ci.yml`

The workspace contains 13 crates, all under `crates/`:

`types` `mismo` `reso` `ingest` `enrich` `eligibility` `compliance`
`scenarios` `solver` `amort` `ml` `orchestrator` `api`

### Workspace-level lints

All crates inherit lints via `[lints] workspace = true`:

```toml
[workspace.lints.rust]
unsafe_code    = "forbid"   # no unsafe anywhere in the engine
unused_imports = "warn"
dead_code      = "warn"

[workspace.lints.clippy]
all         = { level = "warn", priority = -1 }
correctness = { level = "deny", priority = -1 }
```

### Release profile

```toml
[profile.release]
lto           = "fat"
codegen-units = 1
opt-level     = 3
panic         = "abort"
```

---

## Task 1.2 — Money Types

**Files:** `cents.rs`, `basis_points.rs`, `price_ticks.rs`, `ltv.rs`,
`dti.rs`, `credit_score.rs`, `error.rs`

### `Cents` — `i64`, unit = 1 cent

The canonical money type. $1.50 = `Cents(150)`.

**Construction:**
```rust
let amount = Cents(15000);              // direct (trusted context)
let amount = Cents::from_dollars(150);  // whole dollars
let amount = Cents::from_decimal_dollars(dec!(150.00)).unwrap(); // from Decimal
let amount: Cents = "$1,500.00".parse().unwrap();  // from string
```

**Arithmetic (all checked — no silent overflow):**
```rust
let sum = a.checked_add(b).expect("overflow");
let diff = a.saturating_sub(b);          // clamps at i64::MIN/MAX
let scaled = a.checked_mul(12).unwrap(); // multiply by integer scalar
```

**Conversion:**
```rust
let d: Decimal = amount.to_decimal_dollars();     // exact, for TRID forms
let f: f64     = amount.as_f64_dollars();          // lossy, for Newton-Raphson
```

**Display:** `Cents(123456).to_string()` → `"$1,234.56"`

**Validation:** `from_decimal_round_half_up` uses TRID half-up rounding.
`ParseError::DecimalOutOfRange` if value exceeds `i64`.

---

### `BasisPoints` — `u32`, unit = 0.001%

Interest rates with 4-digit precision. 6.875% = `BasisPoints(6875)`.

> **Why 0.001%?** US mortgage rates are quoted to 3 decimal places
> (6.875%, 6.999%, 7.125%). Storing at 0.001% per unit gives exact integer
> representation for every real-world rate.

**Construction:**
```rust
let rate = BasisPoints(6875);
let rate = BasisPoints::from_percentage_str("6.875").unwrap();
let rate = BasisPoints::from_decimal_rate(dec!(0.06875)).unwrap();
let rate = BasisPoints::from_apr_f64(0.068742_f64).unwrap(); // APR output
```

**Conversion:**
```rust
let decimal_rate: Decimal = rate.to_decimal_rate();    // 0.06875
let percent:      Decimal = rate.to_decimal_percent(); // 6.875
let f64_rate:     f64     = rate.as_f64_rate();         // 0.06875_f64
```

**Display:** `BasisPoints(6875).to_string()` → `"6.875%"`

---

### `PriceTicks` — `i32`, unit = 0.0001 pp (signed)

Rate-sheet prices. -3.281 percentage points = `PriceTicks(-32810)`.

Negative = discount (borrower pays points). Positive = premium (lender credit).

```rust
let price = PriceTicks(-32810);
assert!(price.is_discount());

// Apply to loan: $200k × -3.281 pt = -$6,562 cost to borrower
let cost: Cents = price.apply_to_loan(Cents(20_000_000));
assert_eq!(cost, Cents(-656_200));
```

---

### `LtvBasisPoints` — `u32`, unit = 0.01% (true basis points)

Validated to max 11,000 (110.00%). 95.00% = `LtvBasisPoints(9500)`.

```rust
let ltv = LtvBasisPoints::new(9500).unwrap();                          // validated
let ltv = LtvBasisPoints::from_loan_and_value(loan, value).unwrap();   // computed
assert!(LtvBasisPoints::new(11001).is_err()); // > 110% rejected
```

---

### `DtiBasisPoints` — `u32`, unit = 0.01%

No hard cap. Above 6000 (60%) emits a `tracing::debug!` log. 43.00% = `DtiBasisPoints(4300)`.

```rust
let dti = DtiBasisPoints::new(6500); // succeeds, emits debug log
assert!(dti.exceeds_typical_max());
```

---

### `CreditScore` — `u16`, range 300–850

```rust
let score = CreditScore::new(720).unwrap();
assert!(CreditScore::new(299).is_err());
assert!(CreditScore::new(851).is_err());

// Middle-of-three (industry underwriting convention)
let rep = CreditScore::middle_of_three(
    CreditScore::new(700).unwrap(),
    CreditScore::new(720).unwrap(),
    CreditScore::new(740).unwrap(),
);
assert_eq!(rep, CreditScore(720));
```

---

### `ParseError`

Shared validation error returned by every failing constructor.

| Variant | When raised |
|---------|-------------|
| `LtvOutOfRange(u32)` | LTV > 11000 (110%) |
| `CreditScoreOutOfRange(u16)` | Score outside 300–850 |
| `InvalidPercentageString(String)` | Bad percentage text |
| `InvalidMoneyString(String)` | Bad dollar amount text |
| `DecimalOutOfRange(String)` | Decimal → Cents overflow |
| `ZeroPropertyValue` | LTV from zero-value property |
| `InvalidFipsCode(String)` | Bad FIPS string |
| `InvalidStateCode(String)` | Bad state abbreviation |
| `IdentifierEmpty { kind }` | Empty string identifier |
| `IdentifierTooLong { kind, actual, max }` | Identifier too long |
| `IdentifierInvalidChars { kind, value }` | Bad characters |
| `TermMonthsOutOfRange(u16)` | Term outside 120–360 |

---

## Task 1.3 — Identifier Types

**Files:** `state_code.rs`, `fips_code.rs`, `lender_id.rs`,
`mls_listing_key.rs`, `loan_casefile_id.rs`, `scenario_id.rs`, `analysis_id.rs`

### `StateCode` — 50 states + DC + 5 territories

A `Copy` enum. Serde form: `"CA"`.

```rust
let sc: StateCode = "CA".parse().unwrap();  // case-insensitive
assert_eq!(sc.as_str(), "CA");
assert_eq!(sc.to_fips(), 6);
assert_eq!(StateCode::from_fips(6), Some(StateCode::CA));
assert!(StateCode::CA.is_state());
assert!(StateCode::PR.is_territory());

// Enumerate all 56
for state in StateCode::ALL { /* ... */ }
```

---

### `FipsCode` — 5-digit county FIPS, stored as `u32`

Validates the state component against `StateCode::ALL`.

```rust
let fips: FipsCode = "06037".parse().unwrap(); // Los Angeles County, CA
assert_eq!(fips.state_fips(), 6);
assert_eq!(fips.county_fips(), 37);
assert_eq!(fips.state_code(), StateCode::CA);
assert_eq!(fips.to_string(), "06037");

let fips = FipsCode::new(6, 37).unwrap();   // from components
assert!(FipsCode::new(3, 1).is_err());       // FIPS 3 not assigned
```

---

### String identifier types

Three `SmolStr`-backed types with private inner field (validated construction):

| Type | Max len | Allowed chars | Example |
|------|---------|---------------|---------|
| `LenderId` | 32 | `[A-Za-z0-9_-]` | `"UWM"`, `"ROCKET"` |
| `MlsListingKey` | 128 | Printable ASCII | `"OC24123456"` |
| `LoanCasefileId` | 64 | `[A-Za-z0-9_.-]` | `"1234567890"` |

```rust
let id = LenderId::new("UWM").unwrap();
assert!(LenderId::new("").is_err());         // empty
assert!(LenderId::new("A".repeat(33)).is_err()); // too long
assert!(LenderId::new("UWM!").is_err());     // invalid char
```

---

### `ScenarioId` and `AnalysisId` — UUID v4 newtypes

```rust
let id = ScenarioId::new();          // generates UUID v4
let id = ScenarioId::from_uuid(uuid);
let id: ScenarioId = str.parse()?;   // from hyphenated UUID string
assert_eq!(id.as_uuid(), &uuid);
assert_eq!(ScenarioId::NIL.0, Uuid::nil()); // sentinel
```

`AnalysisId` is identical in shape; the types are distinct to prevent
accidentally passing a `ScenarioId` where an `AnalysisId` is expected.

---

## Task 1.4 — Error Hierarchy

**Files:** `errors/ingestion.rs`, `errors/eligibility.rs`,
`errors/solver.rs`, `errors/compliance.rs`

**Rule:** No `anyhow::Error` in library crates. Every error is a concrete,
matchable enum so callers can make programmatic decisions.

### `IngestionError` — rate sheet and data feed errors

| Variant | When raised |
|---------|-------------|
| `SchemaMismatch { file, expected, found }` | Wrong cell type in rate sheet |
| `UnrecognizedBlock { row, col, hint }` | Unknown rate-table header |
| `AmbiguousFicoBand { input }` | Overlapping FICO band strings |
| `MalformedLtvBand { input }` | Bad LTV band separator |
| `InvalidExcelDate { serial }` | Out-of-range Excel date |
| `MismoValidation(String)` | MISMO 3.4 XML schema failure |
| `InvalidResoLookup { field, value }` | Unknown RESO 2.0 lookup |
| `Io(#[from] std::io::Error)` | Filesystem/network failure |

The `#[from]` on `Io` means functions returning `Result<_, IngestionError>`
can use `?` directly on `std::io::Result`:

```rust
fn load(path: &Path) -> Result<String, IngestionError> {
    let s = std::fs::read_to_string(path)?; // io::Error auto-converts
    Ok(s)
}
```

---

### `EligibilityError` — program guideline rejections

Policy violations, not data errors. Eight variants covering score, LTV, DTI,
loan amount, property type, occupancy, reserves, and missing fields.

```rust
return Err(EligibilityError::CreditScoreBelowMinimum {
    score: 619,
    minimum: 620,
    program: "HomeReady 97".to_string(),
});
```

---

### `SolverError` — pricing computation failures

Five variants: `RateNotFound`, `AprIterationLimitExceeded`,
`AmortizationFailed`, `InvalidScenario`, `NumericalOverflow`.

---

### `ComplianceError` — regulatory violations

Six variants with mandatory regulatory citations in every Display message:

| Variant | Regulation |
|---------|-----------|
| `HoepaAprThresholdExceeded` | Reg Z § 1026.32 |
| `HoepaPointsAndFeesExceeded` | Reg Z § 1026.32(a)(1)(ii) |
| `QmSafeHarborFailed` | Reg Z § 1026.43(e) |
| `AtrFailed` | Reg Z § 1026.43(c) |
| `StateLicensingRequirementNotMet` | State NMLS |
| `FloodZoneRequirementNotMet` | 42 U.S.C. § 4012a |

---

## Task 1.5 — Common Enumerations

**Files:** `enums/` directory (7 modules)

All enums derive `Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize,
Deserialize` and use `#[serde(rename_all = "snake_case")]`.
MISMO and RESO mapping methods are provided where applicable.

### `ProgramCode` (12 variants)

`Conventional` `HomeReady` `HomePossible` `HomeOne` `Fha` `FhaDpa`
`Va` `VaJumbo` `Usda` `Bond` `Jumbo` `NonQm`

```rust
assert_eq!(ProgramCode::Fha.to_mismo_mortgage_type(), "FHA");
assert!(ProgramCode::from_mismo_mortgage_type("fha").is_err()); // case-sensitive
assert!(ProgramCode::Conventional.is_agency());
assert!(ProgramCode::Va.is_government());
```

---

### `LoanProduct` (18 variants) — `#[repr(u8)]`

Encodes product type + term band on a single `u8` for `ScenarioKey` packing.

| Variant range | Examples |
|---------------|---------|
| Fixed conventional | `FixedConv8To10` through `FixedConv21To30` |
| Fixed government | `FixedFha8To15`, `FixedFha16To30`, `FixedVa8To15`, `FixedVa16To30`, `FixedUsda30` |
| ARM (SOFR-indexed) | `Arm5_6Sofr`, `Arm7_6Sofr`, `Arm10_6Sofr` |
| ARM (legacy) | `Arm5_1`, `Arm7_1`, `Arm10_1` |
| One-Time-Close | `OtcConv30`, `OtcConv15`, `OtcVa30`, `OtcVaJumbo30` |

```rust
assert_eq!(LoanProduct::FixedConv21To30.term_range_months(), (241, 360));
assert!(LoanProduct::Arm5_6Sofr.is_arm());
assert!(LoanProduct::OtcConv30.is_construction());
```

**Updated band ranges (user spec):**

| Variant | Months |
|---------|--------|
| `FixedConv8To10` | 96–120 |
| `FixedConv11To15` | **121**–180 |
| `FixedConv16To20` | **181**–240 |
| `FixedConv21To30` | **241**–360 |
| `FixedFha16To30` / `FixedVa16To30` | **181**–360 |

---

### `PropertyType` (10 variants)

`SingleFamilyDetached` `SingleFamilyAttached` `Townhouse` `Condominium`
`Cooperative` `PlannedUnitDevelopment` `ManufacturedHome`
`TwoUnit` `ThreeUnit` `FourUnit`

```rust
assert_eq!(PropertyType::SingleFamilyDetached.to_reso_lookup(), "Single Family Residence");
assert_eq!(PropertyType::from_reso_lookup("Duplex").unwrap(), PropertyType::TwoUnit);
assert!(!PropertyType::Cooperative.is_conventional_eligible());
```

---

### `Occupancy` (3 variants)

`PrimaryResidence` `SecondHome` `Investment`

```rust
let o = Occupancy::PrimaryResidence;
assert_eq!(o.to_mismo(), "PrimaryResidence");
assert_eq!(Occupancy::from_mismo("Investor").unwrap(), Occupancy::Investment);
```

---

### `LoanPurpose` (5 variants)

`Purchase` `RateAndTermRefinance` `CashOutRefinance`
`Construction` `ConstructionToPermanent`

```rust
assert!(LoanPurpose::CashOutRefinance.is_refinance());
assert!(LoanPurpose::ConstructionToPermanent.is_construction());
```

---

### `AmortizationType` (5 variants)

`Fixed` `Arm` `InterestOnly` `GraduatedPayment` `PaymentOption`

Only `Fixed` and `Arm` are QM-eligible under Reg Z § 1026.43(e):

```rust
assert!(AmortizationType::Fixed.is_qm_eligible());
assert!(!AmortizationType::InterestOnly.is_qm_eligible());
```

---

### Smaller enums in `misc.rs`

| Type | Variants | Key method |
|------|----------|-----------|
| `LockPeriod` | Day15/21/30/45/60/75/90 | `.days()` |
| `LienPriority` | First/Second/Third | `.to_mismo()` |
| `BalanceType` | Conforming/HighBalance/SuperConforming/Jumbo | — |
| `Tier` | Elite/Standard | — |
| `MiCoverageType` | 8 variants | `.has_monthly_premium()`, `.has_upfront_premium()` |
| `AusType` | DU/LPA/GOT/GUS/Manual | `.to_mismo()` |

`Tier` and `BalanceType` are `#[repr(u8)]` for packing into `ScenarioKey`.

---

## Task 1.6 — Term Primitives

**Files:** `term_band.rs`, `term_months.rs`

### Band boundaries (user spec — contiguous, no gaps)

| Band | Months | Label |
|------|--------|-------|
| `Band8To10` | 96–120 | "8-10 YEAR" |
| `Band11To15` | **121**–180 | "10 Year 1 Month–15 YEAR" |
| `Band16To20` | **181**–240 | "15 Year 1 Month–20 YEAR" |
| `Band21To30` | **241**–360 | "20 Year 1 Month–30 YEAR" |
| `GovtBand8To15` | 96–180 | "8-15 YEAR" |
| `GovtBand16To30` | **181**–360 | "15 Year 1 Month–30 YEAR" |
| `Usda30Only` | 360 | "30 YEAR" |

**Critical design point:** bands determine which rate row to look up.
The engine analyzes every individual `TermMonths` within the band independently
— they share a rate but produce different payments.

```rust
// Rate lookup: use the band
let band = TermMonths(241).band_for_conv().unwrap(); // Band21To30
let rate = rate_sheet.lookup(product, band, fico_band, ltv_band);

// Amortisation: use the exact month
let payment = amortize(loan, rate, TermMonths(241));
```

### `TermBand` methods

```rust
let band = TermBand::Band21To30;
assert_eq!(band.range(), (241, 360));
assert_eq!(band.month_count(), 120);
assert!(band.contains(TermMonths(300)));

// Iterate every term for bulk pricing
for term in band.all_months() {
    let payment = amortize(loan, rate, term);
}
```

### `TermMonths` — validated 120–360

```rust
let t = TermMonths::new(360).unwrap();         // validated
let t = TermMonths::from_years(30).unwrap();   // from whole years
assert_eq!(t.years_floor(), 30);
assert!(TermMonths::new(361).is_err());

// Band lookup
assert_eq!(TermMonths(241).band_for_conv(), Some(TermBand::Band21To30));
assert_eq!(TermMonths(180).band_for_govt(), Some(TermBand::GovtBand8To15));
assert_eq!(TermMonths(360).band_for_usda(), Some(TermBand::Usda30Only));

// Total valid engine range: 120..=360 = 241 terms
assert_eq!(TermMonths::all_valid().count(), 241);
```

---

## Task 1.7 — ScenarioKey

**File:** `scenario_key.rs`

An 8-byte packed identifier for one pricing scenario. Fits in a single CPU
register. Hashes and compares in O(1).

### Bit layout

| Bytes | Bits | Field | Type |
|-------|------|-------|------|
| 0 | 0–7 | `product` | `LoanProduct as u8` |
| 1 | 8–15 | `tier` | `Tier as u8` |
| 2 | 16–23 | `balance_type` | `BalanceType as u8` |
| 3–4 | 24–39 | `term_months` | `TermMonths.0` (u16) |
| 5–6 | 40–55 | `rate_quarter_bps` | u16 (rate × 4 in bps) |
| 7 | 56–63 | `mi_option` | u8 (0–15) |

```rust
let key = ScenarioKey::new(
    LoanProduct::FixedConv21To30,
    Tier::Standard,
    BalanceType::Conforming,
    TermMonths(360),
    6875,  // 6.875% stored as 6875 quarter-bps
    0,
);
assert_eq!(std::mem::size_of::<ScenarioKey>(), 8);
assert_eq!(key.term_months(), 360);
assert_eq!(key.rate_quarter_bps(), 6875);
```

### Why not `#[repr(C, packed)]`?

The spec originally called for `#[repr(C, packed)]`, but `packed` structs with
`u16` fields create unaligned references under safe Rust (UB in
`#[derive(Hash)]`). We use `#[repr(transparent)]` over `u64` instead —
identical layout, safe derives, single-register performance.

---

## Task 1.8 — Decimal Conversions

**Files:** `cents.rs` and `basis_points.rs` (new methods added in Task 1.8)

These methods bridge integer types and `Decimal`/`f64` for the APR
Newton–Raphson solver and the amortisation engine.

### `Cents`

```rust
// To Decimal (exact)
let d: Decimal = Cents(12345).to_decimal_dollars();  // Decimal("123.45")

// From Decimal (TRID half-up rounding)
let c = Cents::from_decimal_dollars(dec!(1.005)).unwrap(); // Cents(101) = $1.01

// To f64 (lossy — Newton-Raphson only)
let f: f64 = Cents(15000).as_f64_dollars(); // 150.0_f64
```

### `BasisPoints`

```rust
// From decimal rate (exact)
let bp = BasisPoints::from_decimal_rate(dec!(0.06875)).unwrap(); // BasisPoints(6875)

// From f64 APR output (rounds to 0.001% precision)
let bp = BasisPoints::from_apr_f64(0.0606442).unwrap(); // BasisPoints(6064) = 6.064%

// To f64 (lossy — Newton-Raphson only)
let f: f64 = BasisPoints(6875).as_f64_rate(); // 0.06875_f64
```

### Precision note on APR

The spec example `0.0606442 → BasisPoints(606)` used traditional 0.01%
basis points. This engine stores at 0.001% per unit, so the same value
yields `BasisPoints(6064)` — the same APR expressed at higher precision.

---

## Task 1.9 — GoalMask

**File:** `goal_mask.rs`  
**Full guide:** `docs/goal-mask-developer-guide.md`

A `u64` bitfield with 34 assigned goal bits. Each bit enables one optimisation
dimension for the Pareto frontier.

### Quick reference

```rust
// Use a built-in default
let goals = GoalMask::DEFAULT_CONSUMER;   // 5 consumer goals
let goals = GoalMask::DEFAULT_INVESTOR;   // 4 investor goals

// Customise
let goals = GoalMask::DEFAULT_CONSUMER
    .enable(GoalMask::FASTEST_BREAK_EVEN)
    .disable(GoalMask::LOWEST_APR);

// Inspect
assert_eq!(goals.active_count(), 5);
assert!(goals.is_consumer_mode());
assert!(!goals.is_investor_mode());

// Per-goal metadata
for g in goals.iter_goals() {
    println!("{}", GoalMask::name_of(g).unwrap());
}

// Persona filtering
let investor_only = goals.investor_goals();
let consumer_only = goals.consumer_goals();
```

### Goal bit assignments

| Bits | Category |
|------|----------|
| 0–2 | Consumer: cost |
| 3–6 | Consumer/Investor: payment |
| 7–10 | Consumer: cash & debt |
| 11–12 | Consumer: rate / APR |
| 13–15 | Consumer: compound |
| 16–22 | Consumer: equity, tax, MI |
| 23–29 | Investor: yield, leverage, velocity |
| 30–33 | Shared: liability & exit |
| 34–63 | Reserved for future goals |

See `docs/goal-mask-developer-guide.md` for the complete per-goal
documentation, usage patterns, and instructions for adding new goals.

---

## Task 1.10 — CI Hardening

**Files:** `deny.toml`, `.github/workflows/ci.yml`  
**Runbook:** `docs/COVERAGE.md`

### CI gates (all required to merge)

| Gate | Tool | What it catches |
|------|------|-----------------|
| `fmt` | `rustfmt --check` | Unformatted code |
| `clippy` | `-D warnings` | Every clippy warning |
| `test` (Ubuntu + macOS) | `cargo test` | Test regressions |
| `coverage` | `cargo-llvm-cov` | Uncovered lines (100% gate on `types`) |
| `bench` | criterion smoke-run | Benchmark compilation failures |
| `deny` | `cargo-deny` | GPL licences, known-bad crates, yanked deps |
| `audit` | `cargo-audit` | RUSTSEC advisories |

### Running all gates locally

```bash
cargo fmt --all                                                      # fmt
cargo clippy --workspace --all-targets -- -D warnings               # clippy
cargo test --workspace                                              # test
cargo llvm-cov --workspace --all-features --fail-under-lines 100   # coverage
cargo bench --bench money_arithmetic                               # bench
cargo deny check                                                    # deny
cargo audit                                                        # audit
```

### License policy (deny.toml)

Allowed: MIT, Apache-2.0, BSD-2/3-Clause, ISC, Zlib, Unicode-3.0, Unlicense.  
Denied: GPL-2.0, GPL-3.0, LGPL-*, AGPL-3.0.

---

## Type Index

| Type | Task | Stored as | Key constraint |
|------|------|-----------|----------------|
| `Cents` | 1.2 | `i64` cents | Arithmetic checked |
| `BasisPoints` | 1.2 | `u32` × 0.001% | Non-negative |
| `PriceTicks` | 1.2 | `i32` × 0.0001 pp | Signed |
| `LtvBasisPoints` | 1.2 | `u32` × 0.01% | ≤ 11000 (110%) |
| `DtiBasisPoints` | 1.2 | `u32` × 0.01% | No cap; warn > 6000 |
| `CreditScore` | 1.2 | `u16` | 300–850 |
| `ParseError` | 1.2 | `enum` | — |
| `StateCode` | 1.3 | `enum` (56 variants) | — |
| `FipsCode` | 1.3 | `u32` | Valid state prefix |
| `LenderId` | 1.3 | `SmolStr` | ≤ 32 chars, `[A-Za-z0-9_-]` |
| `MlsListingKey` | 1.3 | `SmolStr` | ≤ 128 chars, printable ASCII |
| `LoanCasefileId` | 1.3 | `SmolStr` | ≤ 64 chars, `[A-Za-z0-9_.-]` |
| `ScenarioId` | 1.3 | `Uuid` v4 | — |
| `AnalysisId` | 1.3 | `Uuid` v4 | — |
| `IngestionError` | 1.4 | `enum` | `#[from] io::Error` |
| `EligibilityError` | 1.4 | `enum` | — |
| `SolverError` | 1.4 | `enum` | — |
| `ComplianceError` | 1.4 | `enum` | — |
| `ProgramCode` | 1.5 | `enum` (12) | — |
| `LoanProduct` | 1.5 | `enum` (18) `#[repr(u8)]` | — |
| `PropertyType` | 1.5 | `enum` (10) | — |
| `Occupancy` | 1.5 | `enum` (3) | — |
| `LoanPurpose` | 1.5 | `enum` (5) | — |
| `AmortizationType` | 1.5 | `enum` (5) | — |
| `LockPeriod` | 1.5 | `enum` (7) | — |
| `LienPriority` | 1.5 | `enum` (3) | — |
| `BalanceType` | 1.5 | `enum` (4) `#[repr(u8)]` | — |
| `Tier` | 1.5 | `enum` (2) `#[repr(u8)]` | — |
| `MiCoverageType` | 1.5 | `enum` (8) | — |
| `AusType` | 1.5 | `enum` (5) | — |
| `TermBand` | 1.6 | `enum` (7) | Contiguous 96–360 |
| `TermMonths` | 1.6 | `u16` | 120–360 |
| `ScenarioKey` | 1.7 | `u64` transparent | Exactly 8 bytes |
| `GoalMask` | 1.7/1.9 | `u64` bitflags | 34 assigned bits |

---

## Common Patterns

### Parsing external input at the boundary

```rust
fn ingest_rate_sheet_row(
    rate_str: &str,
    ltv_str: &str,
    score: u16,
) -> Result<(BasisPoints, LtvBasisPoints, CreditScore), IngestionError> {
    let rate = BasisPoints::from_percentage_str(rate_str)
        .map_err(|_| IngestionError::AmbiguousFicoBand { input: rate_str.to_string() })?;
    let ltv = LtvBasisPoints::new(ltv_str.parse().unwrap_or(0))
        .map_err(|_| IngestionError::MalformedLtvBand { input: ltv_str.to_string() })?;
    let cs = CreditScore::new(score)
        .map_err(|e| IngestionError::MismoValidation(e.to_string()))?;
    Ok((rate, ltv, cs))
}
```

### Building a ScenarioKey

```rust
fn make_key(
    product: LoanProduct,
    term: TermMonths,
    rate: BasisPoints,
    mi: u8,
) -> ScenarioKey {
    ScenarioKey::new(
        product,
        Tier::Standard,
        BalanceType::Conforming,
        term,
        rate.0 as u16,  // rate.0 fits in u16 for normal rates
        mi,
    )
}
```

### Applying price adjustments

```rust
fn net_price(base: PriceTicks, llpas: &[PriceTicks]) -> PriceTicks {
    llpas.iter().fold(base, |acc, &adj| {
        acc.checked_add(adj).unwrap_or(PriceTicks(i32::MAX))
    })
}

fn closing_cost(price: PriceTicks, loan: Cents) -> Cents {
    price.apply_to_loan(loan) // negative = discount (cost), positive = credit
}
```

### APR calculation result

```rust
// After Newton-Raphson converges to f64 APR:
fn finalise_apr(apr_f64: f64) -> Result<BasisPoints, SolverError> {
    BasisPoints::from_apr_f64(apr_f64).ok_or(SolverError::AprIterationLimitExceeded {
        iterations: 0,
        last_residual: apr_f64,
    })
}
```

### Goal-aware analysis request

```rust
fn default_goals_for(occupancy: Occupancy) -> GoalMask {
    match occupancy {
        Occupancy::Investment => GoalMask::DEFAULT_INVESTOR,
        _ => GoalMask::DEFAULT_CONSUMER,
    }
}
```
