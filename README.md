# MorTru Engine

Enterprise-grade product pricing eligibility engine for residential mortgages. Intakes wholesale rate sheets and produces fully-priced, eligibility-verified, QM-classified loan scenarios.

## Workspace layout

```
crates/
├── types         Foundation: money types, IDs, enums, errors                 (Epic 1)
├── mismo         MISMO 3.4 schema: LoanApplication, Party, Collateral        (Epic 2)
├── reso          RESO Data Dictionary 2.0: Property resource                 (Epic 3)
├── ingest        Rate-sheet / LLPA / MI / reference-data ingestion           (Epics 4-6)
├── enrich        Property enrichment pipeline                                (Epic 7)
├── eligibility   Loan-product + MI eligibility (Conv/HR/HP/FHA/VA/USDA)      (Epics 8-9)
├── compliance    QM & ATR testing engine                                     (Epic 10)
├── scenarios     Scenario enumeration + pruning + ML hooks                   (Epics 11-12)
├── solver        Circular pricing solver + fee worksheet                     (Epic 13)
├── amort         TRID amortization + multi-goal ranking                      (Epic 14)
├── ml            ML model interfaces and loaders                             (Epic 12)
├── orchestrator  Pipeline coordination + analysis engine                     (Epic 15)
└── api           HTTP API (Axum) + auth + observability                      (Epic 15)
```

## Build

```bash
cargo build --workspace            # debug build
cargo build --workspace --release  # production build
cargo test  --workspace            # full test suite
cargo clippy --workspace --all-targets -- -D warnings  # lint gate
cargo fmt --check                  # formatting gate
```

## CI gates

Every PR must pass:

1. **`cargo fmt --check`** — formatting matches `rustfmt.toml`
2. **`cargo clippy --workspace -- -D warnings`** — zero warnings tolerated
3. **`cargo test --workspace`** — all tests pass on Linux and macOS
4. **`cargo llvm-cov --fail-under-lines 100`** — 100% line coverage on non-stub crates
5. **`cargo deny check`** — license allowlist + dependency advisories + source policy
6. **`cargo audit`** — no known-vulnerable dependencies

## Task tracking

This workspace is built epic-by-epic against the plan in `docs/epic-task-plan-part-{1,2,3}.md`. Each task ships with its own test suite that must pass at 100% before the next task begins.

Current status: **Epic 1, Task 1.1 — Workspace Bootstrap** complete.
