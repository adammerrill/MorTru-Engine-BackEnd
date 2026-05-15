# MorTru Mortgage Engine — Full Revised Build Plan
## All 14 epics · Updated post Epic 2 completion

**Repository:** https://github.com/adammerrill/MorTru-Engine-BackEnd  
**Toolchain:** Rust 1.85.0  
**Current state:** Epic 1 ✅ (404 tests) · Epic 2 ✅ (405 tests) · CI #33 green  

---

## Progress summary

| Epic | Crate | Status | Tests | Notes |
|---|---|---|---|---|
| 1 — Domain Types | `types` | ✅ Complete | 404 | Coverage gate ≥97% active |
| 2 — MISMO Schema | `mismo` | ✅ Complete | 405 | Coverage baseline pending |
| 3 — RESO Property | `reso` | ⬜ Ready to build | — | Parallel with Epic 4 |
| 4 — Reference Data | `ref_data` (new) | ⬜ Ready to build | — | Parallel with Epic 3 |
| 5 — Geo Enrichment | `enrich` | ⬜ Blocked on 3+4 | — | FCC FIPS, HOI, loan limits |
| 6 — Ingest/Bridge | `ingest` | ⬜ Blocked on 3+4+5 | — | Critical junction |
| 7 — Eligibility | `eligibility` | ⬜ Blocked on 6 | — | 4-layer model |
| 8 — MI Pricing | *(in ref_data+solver)* | ⬜ Blocked on 7 | — | Multi-provider |
| 9 — LLPA/Rate | `solver` | ⬜ Blocked on 7 | — | Multi-lender loop |
| 10 — Amortization | `amort` | ⬜ Blocked on 6 | — | P&I, APR |
| 11 — Closing Costs | *(in ingest+enrich)* | ⬜ Blocked on 4+6 | — | Fee engine |
| 12 — Scenarios | `scenarios` | ⬜ Blocked on 9+10+11 | — | N lender × M MI |
| 13 — Compliance | `compliance` | ⬜ Blocked on 12 | — | QM/ATR/HOEPA |
| 14 — API | `api` | ⬜ Blocked on 12+13 | — | Axum REST |

---

## Dependency graph

```
Epic 1 (types) ──────────────────────────────────────────────────────────┐
      │                                                                   │
      ├──→ Epic 2 (mismo) ──✅ Complete                                   │
      │                                                                   │
      ├──→ Epic 3 (reso) ──────────────┐   ← START NOW                   │
      │                               │                                   │
      └──→ Epic 4 (ref_data) ─────────┤   ← START NOW (parallel)         │
                                      │                                   │
                                      └──→ Epic 5 (enrich) ──────────────┘
                                                  │
                                          Epic 6 (ingest) ◄── CRITICAL JUNCTION
                                          LoanScenario produced here
                                                  │
                              ┌───────────────────┼──────────────────────┐
                              │                   │                      │
                        Epic 7 (eligibility)  Epic 10 (amort)    Epic 11 (costs)
                        4-layer model         P&I + APR          Fee engine
                              │
                    ┌─────────┼─────────┐
                    │                   │
              Epic 8 (MI)          Epic 9 (LLPA/rate)
              Multi-provider       Multi-lender loop
                    │                   │
                    └─────────┬─────────┘
                              │
                       Epic 12 (scenarios)
                       N lender × M MI matrix
                              │
                       Epic 13 (compliance)
                       QM / ATR / HOEPA
                              │
                        Epic 14 (API)
                        Axum REST endpoints
```

---

## Epic 3 — RESO Property Data

**Crate:** `reso`  **Target:** ~165 tests  **Duration:** ~2 weeks  
**Parallel:** Yes — can build simultaneously with Epic 4  

See `epic-3-build-plan.md` for full task specifications.

| Task | Deliverable | Tests |
|---|---|---|
| 3.1 | `PropertyReso` raw struct (65 fields) + `ResoError` | 25 |
| 3.2 | RESO PropertyType/SubType → `types::PropertyType` mapping | 30 |
| 3.3 | `PropertyReso::to_enriched()` → `PropertyEnriched` | 40 |
| 3.4 | FCC FIPS geocoding async client + `FipsGeocoderClient` trait | 25 |
| 3.5 | `ResoRepository` trait + `InMemoryResoRepo` | 20 |
| 3.6 | RESO ↔ MISMO address reconciliation | 20 |
| 3.7 | Epic 3 gate | 15 |

---

## Epic 4 — Reference Data (Versioned)

**Crate:** `ref_data` (new)  **Target:** ~200 tests  **Duration:** ~3 weeks  
**Parallel:** Yes — can build simultaneously with Epic 3  

### Architecture (decided post-Epic 2)

All reference data uses:
- **`Versioned<T>` wrapper** — every record has a `version_id: VersionId` (UUID v4),
  `effective_at: DateTime<Utc>`, `expires_at: Option<DateTime<Utc>>`.
  Records are never updated — only superseded by new versions.
- **`RefDataStore` trait** — abstracts storage backend.
  Three implementations: `JsonFileStore` (dev), `SqliteStore` (CI),
  `PostgresStore` (production).
- **`DataVersionManifest`** — recorded alongside every analysis.
  Captures every `VersionId` consumed, plus engine git SHA.
  Enables exact replay for CFPB regulatory audit.

| Task | Deliverable | Tests |
|---|---|---|
| 4.1 | Crate scaffold: `RefDataStore` trait, `VersionId`, `Versioned<T>`, `DataVersionManifest` | 20 |
| 4.2 | `JsonFileStore` impl — loads all versioned JSON from `data/` dir | 20 |
| 4.3 | SQL migration framework — 12 migration files, `SqliteStore` for integration tests | 15 |
| 4.4 | `PostgresStore` impl — production-ready with connection pooling | 15 |
| 4.5 | Fee rules, agency eligibility, conforming/FHA/VA/USDA loan limits | 20 |
| 4.6 | FHA MIP rate tables, VA funding fee tables, USDA guarantee fee | 15 |
| 4.7 | Conventional MI coverage requirements (FNMA/FHLMC minimums by LTV tier) | 10 |
| 4.8 | LLPA matrices — FNMA/FHLMC/Jumbo JSON-backed rule engine | 20 |
| 4.9 | Title insurance rate tables (TX promulgated + state coverage map) | 15 |
| 4.10 | `LenderProfile` + `LenderOverlays` (versioned, per-lender) | 15 |
| 4.11 | `BrokerPanel` — lender relationship management | 10 |
| 4.12 | `MiProvider` + `MiRateCard` JSON loader | 15 |
| 4.13 | `MiProviderOverlays` | 10 |
| 4.14 | `RateSheet` — volatile intraday data model (DB-backed even in dev) | 15 |
| 4.15 | `DataVersionManifest` capture + storage | 10 |
| 4.16 | Epic 4 gate | 15 |

### Data files structure

```
crates/ref_data/
├── data/
│   ├── agency/
│   │   ├── eligibility_v2025.json
│   │   ├── loan_limits_v2025.json
│   │   └── mi_coverage_requirements_v2025.json
│   ├── mi/
│   │   ├── fha_mip_v2025.json
│   │   ├── va_funding_fee_v2025.json
│   │   ├── usda_guarantee_v2025.json
│   │   ├── mgic_monthly_v2024-Q4.json
│   │   ├── arch_mi_monthly_v2024-Q4.json
│   │   ├── radian_monthly_v2024-Q4.json
│   │   ├── essent_monthly_v2024-Q4.json
│   │   ├── enact_monthly_v2024-Q4.json
│   │   └── nmi_monthly_v2024-Q4.json
│   ├── llpa/
│   │   ├── fnma_conv_v2024-08.json
│   │   └── fhlmc_conv_v2024-08.json
│   └── title/
│       ├── tx_promulgated_v2021.json
│       └── state_coverage_map_v2024.json
└── migrations/
    ├── 0001_versioned_records_base.sql
    ├── 0002_llpa_matrices.sql
    ├── 0003_mi_rate_cards.sql
    ├── 0004_rate_sheets.sql
    ├── 0005_lender_profiles.sql
    ├── 0006_lender_overlays.sql
    ├── 0007_mi_provider_overlays.sql
    ├── 0008_agency_eligibility.sql
    ├── 0009_loan_limits.sql
    ├── 0010_ami_data.sql
    ├── 0011_analysis_results.sql
    └── 0012_data_version_manifests.sql
```

---

## Epic 5 — Geo Enrichment

**Crate:** `enrich`  **Target:** ~130 tests  **Blocked on:** Epics 3+4  

| Task | Deliverable |
|---|---|
| 5.1 | FIPS resolution: lat/lng → `FipsCode` (calls Epic 3's FCC client) |
| 5.2 | FIPS resolution: address/ZIP fallback chain |
| 5.3 | CBSA enrichment: `FipsCode` → `CbsaCode` + MSA name |
| 5.4 | HOI enrichment: ZIP → annual HOI estimate |
| 5.5 | Tax enrichment: RESO tax fields → `Cents/yr` with tiered fallback |
| 5.6 | HOA normalization (delegates to Epic 3, verifies consistency) |
| 5.7 | Loan limits: `CbsaCode` → conforming/FHA/VA/USDA limits (Epic 4 data) |
| 5.8 | AMI: `FipsCode` → 80% AMI limit (Epic 4 data) |
| 5.9 | USDA eligibility: location flag + household income check |
| 5.10 | `EnrichedProperty` composite struct |
| 5.11 | Epic 5 gate |

---

## Epic 6 — Ingest / Bridge (Critical Junction)

**Crate:** `ingest`  **Target:** ~100 tests  **Blocked on:** 2+3+4+5  

The `LoanScenario` struct produced here is what ALL downstream engines consume.

| Task | Deliverable |
|---|---|
| 6.1 | `LoanScenario` struct definition |
| 6.2 | MISMO `ParsedDeal` → `LoanScenario` bridge |
| 6.3 | RESO `EnrichedProperty` → `LoanScenario` bridge |
| 6.4 | LTV calculation: `(base_loan, appraised_value)` → `LtvBasisPoints` |
| 6.5 | VA tier derivation: `parties.with_va_tier(ltv, is_cash_out, is_irrrl)` |
| 6.6 | CLTV / HCLTV calculation (for 2nd liens) |
| 6.7 | Scenario validation: field consistency checks |
| 6.8 | Epic 6 gate |

---

## Epic 7 — Eligibility (Four-Layer Model)

**Crate:** `eligibility`  **Target:** ~220 tests  **Blocked on:** Epic 6  

Four sequential layers — a scenario must pass all four before pricing.

### Layer 1: Agency / Program Guidelines (static, from Epic 4)

| Task | Deliverable |
|---|---|
| 7.1 | Conventional: FICO, LTV, DTI, property type, occupancy, units |
| 7.2 | FHA: FICO/LTV matrix, DTI, condo approval, FHA case number |
| 7.3 | VA: residual income check, entitlement calculation, overlays |
| 7.4 | USDA: income limits, property eligibility, DTI 29/41 |
| 7.5 | High-balance / jumbo: county-based loan limit checks |
| 7.6 | New construction: stage-specific eligibility rules |
| 7.7 | Investment / second home: LTV/FICO restrictions by program |

### Layer 2: Lender Overlays (from Epic 4 LenderOverlays)

| Task | Deliverable |
|---|---|
| 7.8 | Lender overlay eligibility check (per-lender from Epic 4.10) |
| 7.9 | Lender state licensing check |
| 7.10 | Overlay conflict reporting |

### Layer 3: MI Provider Overlays (from Epic 4 MiProviderOverlays)

| Task | Deliverable |
|---|---|
| 7.11 | MI provider eligibility check (per-provider from Epic 4.13) |
| 7.12 | MI coverage requirement validation |

### Layer 4: Product Validation

| Task | Deliverable |
|---|---|
| 7.13 | Adjusted loan ≤ county FHA limit |
| 7.14 | ARM index / margin validation |
| 7.15 | Epic 7 gate |

---

## Epic 8 — MI Pricing (Multi-Provider)

**Lives in:** `solver`  **Target:** ~80 tests  **Blocked on:** Epic 7  

| Task | Deliverable |
|---|---|
| 8.1 | FHA MIP calculation: monthly amount, UFMIP, life-of-loan determination |
| 8.2 | VA funding fee: tier lookup, exempt check, financed amount |
| 8.3 | USDA guarantee + annual fee calculation |
| 8.4 | Conventional PMI: LTV band → coverage requirement → rate card lookup |
| 8.5 | Multi-provider MI rating loop: rate all approved providers |
| 8.6 | MI cancellation projection: month when LTV reaches 80% |
| 8.7 | Single-premium PMI: financed vs. paid-at-closing |
| 8.8 | LPMI (Lender-Paid MI): rate premium trade-off calculation |
| 8.9 | Epic 8 gate |

---

## Epic 9 — LLPA / Rate Pricing (Multi-Lender)

**Crate:** `solver`  **Target:** ~100 tests  **Blocked on:** Epics 7+8  

| Task | Deliverable |
|---|---|
| 9.1 | `LlpaRule` condition evaluation engine |
| 9.2 | LLPA aggregation: sum all matching rules for a scenario |
| 9.3 | Rate sheet lookup: base price for given rate + lock period |
| 9.4 | Adjusted price: base + LLPA total |
| 9.5 | Par rate resolution: rate where adjusted price = 10000 |
| 9.6 | Lender credit computation: price above par → Section J credit |
| 9.7 | Discount points: price below par → Section A points |
| 9.8 | Multi-lender pricing loop: rate all lenders on panel |
| 9.9 | Rate sheet staleness check: refuse to quote expired sheets |
| 9.10 | Epic 9 gate |

---

## Epic 10 — Amortization Engine

**Crate:** `amort`  **Target:** ~80 tests  **Blocked on:** Epic 6  

| Task | Deliverable |
|---|---|
| 10.1 | Monthly P&I payment: `(loan, rate, term)` → `Cents/month` |
| 10.2 | Full 360-month amortization schedule |
| 10.3 | ARM amortization: initial period + adjustment cap model |
| 10.4 | APR calculation: Newton-Raphson IRR on cash flow stream |
| 10.5 | APR from Regulation Z § 1026.18 definition |
| 10.6 | Interest-only period handling |
| 10.7 | Remaining balance at any month |
| 10.8 | Epic 10 gate |

---

## Epic 11 — Closing Cost Engine

**Lives in:** `ingest`/`enrich`  **Target:** ~120 tests  **Blocked on:** 4+6  

| Task | Deliverable |
|---|---|
| 11.1 | Appraisal fee matrix: program × property type × units × geography |
| 11.2 | Title insurance engine: TX promulgated rates + state map |
| 11.3 | Recording fee engine: county-level per-doc/per-page rates |
| 11.4 | Transfer tax engine: state/county matrix + mansion tax |
| 11.5 | Prepaid interest calculation: actual/365, closing date → EOM |
| 11.6 | Initial escrow: RESPA aggregate adjustment (12-month projection) |
| 11.7 | Section A fees: origination, broker comp, credit report |
| 11.8 | Section B fees: appraisal, flood cert, tax service, pest inspection |
| 11.9 | Section C fees: title, settlement, endorsements |
| 11.10 | TRID tolerance validation: 0%, 10%, unlimited buckets |
| 11.11 | Lender credit application and validation |
| 11.12 | Epic 11 gate |

---

## Epic 12 — Scenario Engine (Multi-Lender × Multi-MI)

**Crate:** `scenarios`  **Target:** ~150 tests  **Blocked on:** 9+10+11  

| Task | Deliverable |
|---|---|
| 12.1 | `LoanScenarioVariant` — one priced combination |
| 12.2 | Single-lender, single-program pricing pass |
| 12.3 | Multi-program expansion (all eligible programs per lender) |
| 12.4 | Multi-MI expansion (all approved providers per lender) |
| 12.5 | Broker panel loop: N lenders × programs × MI = full matrix |
| 12.6 | `GoalMask`-driven ranking: PITIA, cash-to-close, APR, total interest |
| 12.7 | Budget filtering: max PITIA, max cash-to-close constraints |
| 12.8 | Deduplication: remove dominated results |
| 12.9 | `AnalysisResult` struct — ranked variants + manifest |
| 12.10 | Epic 12 gate: FHA fixture, 3 lenders, 3 MI providers |

---

## Epic 13 — Compliance (QM/ATR/HOEPA)

**Crate:** `compliance`  **Target:** ~80 tests  **Blocked on:** Epic 12  

| Task | Deliverable |
|---|---|
| 13.1 | QM safe harbor: points-and-fees test (3% of loan amount) |
| 13.2 | QM balloon payment prohibition |
| 13.3 | QM loan term limit (≤30 years) |
| 13.4 | ATR 8-factor ability-to-repay verification |
| 13.5 | HOEPA APR threshold check |
| 13.6 | HOEPA points-and-fees threshold check |
| 13.7 | Texas-specific: HELOC 3% fee cap, homestead protections |
| 13.8 | VA IRRRL net tangible benefit test |
| 13.9 | FHA UFMIP refund calculation for early payoff |
| 13.10 | Epic 13 gate |

---

## Epic 14 — API Layer

**Crate:** `api`  **Target:** ~100 tests  **Blocked on:** 12+13  

### Core analysis endpoints

```
POST /api/v1/analyze                 Run full multi-lender analysis
GET  /api/v1/analyze/{analysis_id}   Fetch results
POST /api/v1/analyze/{id}/replay     Re-run with exact historical versions
```

### Configuration management (no redeployment)

```
GET  /api/v1/fee-rules               Read current fee_rules.json
PUT  /api/v1/fee-rules               Replace and reload

GET  /api/v1/lenders                 List all lenders
POST /api/v1/lenders                 Create lender profile
PUT  /api/v1/lenders/{id}/rate-sheet Push intraday rate sheet
PUT  /api/v1/lenders/{id}/llpa       Update LLPA matrix
PUT  /api/v1/lenders/{id}/overlays   Update eligibility overlays
PUT  /api/v1/lenders/{id}/mi-providers Approved MI provider list

GET  /api/v1/brokers/{id}/panel      List broker's lender panel
PUT  /api/v1/brokers/{id}/panel      Update panel

PUT  /api/v1/mi-providers/{id}/rate-card   Update MI rate card
PUT  /api/v1/mi-providers/{id}/overlays    Update MI overlays
```

| Task | Deliverable |
|---|---|
| 14.1 | Axum server setup, middleware stack, error formatting |
| 14.2 | `POST /api/v1/analyze` endpoint |
| 14.3 | Analysis result persistence + retrieval |
| 14.4 | Replay endpoint: manifest → reconstruct store → re-run |
| 14.5 | Lender management endpoints |
| 14.6 | Broker panel endpoints |
| 14.7 | MI provider management endpoints |
| 14.8 | Fee rules read/write endpoints |
| 14.9 | Rate sheet push endpoint (intraday, high frequency) |
| 14.10 | Authentication + rate limiting |
| 14.11 | OpenAPI spec generation |
| 14.12 | Epic 14 gate |

---

## Revised total estimates

| Epic | Tests | Duration |
|---|---|---|
| 1 ✅ | 404 | — |
| 2 ✅ | 405 | — |
| 3 | ~165 | 2 weeks |
| 4 | ~200 | 3 weeks |
| 5 | ~130 | 2 weeks |
| 6 | ~100 | 1.5 weeks |
| 7 | ~220 | 3 weeks |
| 8 | ~80 | 1 week |
| 9 | ~100 | 1.5 weeks |
| 10 | ~80 | 1 week |
| 11 | ~120 | 2 weeks |
| 12 | ~150 | 2.5 weeks |
| 13 | ~80 | 1 week |
| 14 | ~100 | 2 weeks |
| **Total** | **~2,334** | **~23 weeks remaining** |

---

## Immediate next steps (priority order)

1. **Epic 3** — Start now. `reso` crate, 7 tasks, ~2 weeks.
2. **Epic 4** — Start now in parallel. `ref_data` crate, 16 tasks, ~3 weeks.
3. **Epic 5** — Begins once 3+4 reach Task 3.4 (FCC client) + Task 4.5 (loan limits).
4. **Epic 6** — Critical junction. Begin only after 3+4+5 complete.
5. **mismo coverage baseline** — `cargo llvm-cov --package mismo`, measure %, add gate.

