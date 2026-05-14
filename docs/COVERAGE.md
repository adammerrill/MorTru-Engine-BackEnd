# Coverage Gate Runbook

The `types` crate requires **100% line coverage** on every CI run.

---

## Running locally

```bash
# One-time install
cargo install cargo-llvm-cov

# Full run — opens HTML report in your browser
cargo llvm-cov --workspace --all-features --html --open

# Threshold check (same as CI)
cargo llvm-cov --workspace --all-features --fail-under-lines 100 \
    --ignore-filename-regex \
    'crates/(mismo|reso|ingest|enrich|eligibility|compliance|scenarios|solver|amort|ml|orchestrator|api)/src/lib\.rs'

# Per-file summary in the terminal
cargo llvm-cov --workspace --all-features --summary-only
```

---

## What the gate excludes

The `--ignore-filename-regex` flag excludes the empty stub `lib.rs` files in
crates that have not been implemented yet (Epic 2+). Once a crate is
implemented, remove it from the ignore list in `.github/workflows/ci.yml`.

---

## How to fix a coverage failure

1. Run the HTML report: `cargo llvm-cov --workspace --all-features --html --open`
2. Navigate to the red-highlighted file in the report
3. Red lines are uncovered; add a test that exercises them
4. Re-run until green

---

## Coverage by task

| Task | Modules | Gate |
|------|---------|------|
| 1.2 Money types | cents, basis_points, price_ticks, ltv, dti, credit_score | 100% |
| 1.3 Identifiers | fips_code, state_code, lender_id, mls_listing_key, loan_casefile_id, scenario_id, analysis_id | 100% |
| 1.4 Errors | errors/ingestion, errors/eligibility, errors/solver, errors/compliance | 100% |
| 1.5 Enumerations | enums/* | 100% |
| 1.6 Term primitives | term_band, term_months | 100% |
| 1.7–1.9 Scenario | scenario_key, goal_mask | 100% |
| 1.8 Decimal bridge | cents (new methods), basis_points (new methods) | 100% |
