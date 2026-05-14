//! CI hardening tests — Task 1.10.
//!
//! These tests verify that the infrastructure required for the CI coverage
//! gate, benchmark gate, license gate, and advisory gate is correctly
//! deployed. They are **structural guards**: they will catch a developer
//! accidentally deleting a required file or silently skipping a module.
//!
//! If a test here fails, it means the CI configuration is broken, not that
//! a business-logic invariant was violated. Fix the configuration, not the
//! test.
//!
//! # Running the coverage gate locally
//!
//! ```text
//! # Install once
//! cargo install cargo-llvm-cov
//!
//! # View HTML report
//! cargo llvm-cov --workspace --all-features --html --open
//!
//! # Check threshold (same command CI uses)
//! cargo llvm-cov --workspace --all-features --fail-under-lines 100 \
//!     --ignore-filename-regex 'crates/(mismo|reso|ingest|enrich|eligibility|compliance|scenarios|solver|amort|ml|orchestrator|api)/src/lib\.rs'
//! ```
//!
//! # Running the benchmark gate locally
//!
//! ```text
//! # Full benchmark run (use for local <5 ns verification)
//! cargo bench --bench money_arithmetic
//!
//! # Quick smoke-run (same as CI — just verifies no panics)
//! cargo bench --bench money_arithmetic -- --sample-size 1 \
//!     --warm-up-time 0 --measurement-time 1
//! ```
//!
//! # Running the license gate locally
//!
//! ```text
//! cargo install cargo-deny
//! cargo deny check
//! ```

use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is crates/types; workspace root is two levels up
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn types_src() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn types_tests() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests")
}

// ─────────────────────────────────────────────────────────────────────────────
// CI configuration files exist
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_ci_workflow_exists() {
    let ci = workspace_root().join(".github/workflows/ci.yml");
    assert!(ci.exists(), ".github/workflows/ci.yml must exist — this file drives all CI gates");
}

#[test]
fn test_deny_toml_exists() {
    let deny = workspace_root().join("deny.toml");
    assert!(
        deny.exists(),
        "deny.toml must exist — required by the cargo-deny license and advisory CI gate"
    );
}

#[test]
fn test_deny_toml_has_required_sections() {
    let deny = workspace_root().join("deny.toml");
    let contents = std::fs::read_to_string(deny)
        .expect("deny.toml must be readable");
    assert!(contents.contains("[licenses]"), "deny.toml must contain [licenses] section");
    assert!(contents.contains("[advisories]"), "deny.toml must contain [advisories] section");
    assert!(contents.contains("[bans]"), "deny.toml must contain [bans] section");
    assert!(contents.contains("[sources]"), "deny.toml must contain [sources] section");
    // Copyleft licenses must be denied
    assert!(
        contents.contains("GPL-2.0") || contents.contains("GPL-3.0"),
        "deny.toml must explicitly deny GPL licences"
    );
}

#[test]
fn test_benchmark_file_exists() {
    let bench = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("benches/money_arithmetic.rs");
    assert!(
        bench.exists(),
        "benches/money_arithmetic.rs must exist — the CI bench gate compiles and runs it"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// All Epic 1 source modules are present
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_all_task_1_2_source_files_exist() {
    // Task 1.2: Money types
    let src = types_src();
    let required = [
        "error.rs",
        "cents.rs",
        "basis_points.rs",
        "price_ticks.rs",
        "ltv.rs",
        "dti.rs",
        "credit_score.rs",
    ];
    for f in required {
        assert!(src.join(f).exists(), "Task 1.2 source file missing: src/{f}");
    }
}

#[test]
fn test_all_task_1_3_source_files_exist() {
    // Task 1.3: Identifier types
    let src = types_src();
    let required = [
        "analysis_id.rs",
        "fips_code.rs",
        "lender_id.rs",
        "loan_casefile_id.rs",
        "mls_listing_key.rs",
        "scenario_id.rs",
        "state_code.rs",
    ];
    for f in required {
        assert!(src.join(f).exists(), "Task 1.3 source file missing: src/{f}");
    }
}

#[test]
fn test_all_task_1_4_source_files_exist() {
    // Task 1.4: Error hierarchy
    let src = types_src();
    assert!(src.join("errors.rs").exists(), "Task 1.4: errors.rs missing");
    assert!(src.join("errors").is_dir(), "Task 1.4: errors/ directory missing");
    for f in ["ingestion.rs", "eligibility.rs", "solver.rs", "compliance.rs"] {
        assert!(src.join("errors").join(f).exists(), "Task 1.4: errors/{f} missing");
    }
}

#[test]
fn test_all_task_1_5_source_files_exist() {
    // Task 1.5: Common enumerations
    let src = types_src();
    assert!(src.join("enums.rs").exists(), "Task 1.5: enums.rs missing");
    assert!(src.join("enums").is_dir(), "Task 1.5: enums/ directory missing");
    for f in [
        "program_code.rs", "loan_product.rs", "property_type.rs",
        "occupancy.rs", "loan_purpose.rs", "amortization_type.rs", "misc.rs",
    ] {
        assert!(src.join("enums").join(f).exists(), "Task 1.5: enums/{f} missing");
    }
}

#[test]
fn test_all_task_1_6_source_files_exist() {
    // Task 1.6: Term primitives
    let src = types_src();
    assert!(src.join("term_band.rs").exists(), "Task 1.6: term_band.rs missing");
    assert!(src.join("term_months.rs").exists(), "Task 1.6: term_months.rs missing");
}

#[test]
fn test_all_task_1_7_source_files_exist() {
    // Task 1.7: Scenario primitives
    let src = types_src();
    assert!(src.join("scenario_key.rs").exists(), "Task 1.7: scenario_key.rs missing");
    assert!(src.join("goal_mask.rs").exists(), "Task 1.7/1.9: goal_mask.rs missing");
}

// ─────────────────────────────────────────────────────────────────────────────
// All integration test files are present
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_all_integration_test_files_exist() {
    let tests = types_tests();
    let required = [
        "properties.rs",           // Task 1.2 property tests
        "identifier_properties.rs", // Task 1.3 property tests
        "error_hierarchy.rs",      // Task 1.4 integration tests
        "enumerations.rs",         // Task 1.5 integration tests
        "term_band_and_months.rs", // Task 1.6 integration tests
        "decimal_conversions.rs",  // Task 1.8 integration tests
        "epic_1_gate.rs",          // Epic 1 deployment gate
        "ci_hardening.rs",         // Task 1.10 (this file)
    ];
    for f in required {
        assert!(tests.join(f).exists(), "integration test file missing: tests/{f}");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Developer documentation is present
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_developer_docs_exist() {
    let docs = workspace_root().join("docs");
    assert!(docs.is_dir(), "docs/ directory must exist");
    assert!(
        docs.join("goal-mask-developer-guide.md").exists(),
        "docs/goal-mask-developer-guide.md must exist (Task 1.9 developer guide)"
    );
    assert!(
        docs.join("epic-1-types-crate-reference.md").exists(),
        "docs/epic-1-types-crate-reference.md must exist (Epic 1 full reference)"
    );
    assert!(
        docs.join("COVERAGE.md").exists(),
        "docs/COVERAGE.md must exist (coverage gate runbook)"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Coverage gate simulation
// ─────────────────────────────────────────────────────────────────────────────

/// Verifies the test infrastructure is wired up correctly by confirming that
/// every public module in the types crate is imported in lib.rs. If a module
/// is declared but not `pub use`d, it contributes uncovered lines that would
/// fail the coverage gate even if the module itself has inline tests.
#[test]
fn test_lib_rs_re_exports_all_public_types() {
    let lib_rs = types_src().join("lib.rs");
    let contents = std::fs::read_to_string(lib_rs)
        .expect("src/lib.rs must be readable");

    let expected_reexports = [
        // Task 1.2
        "BasisPoints", "Cents", "CreditScore", "DtiBasisPoints",
        "LtvBasisPoints", "PriceTicks",
        // Task 1.3
        "AnalysisId", "FipsCode", "LenderId", "LoanCasefileId",
        "MlsListingKey", "ScenarioId", "StateCode",
        // Task 1.4
        "ParseError", "ComplianceError", "EligibilityError",
        "IngestionError", "SolverError",
        // Task 1.5
        "AmortizationType", "AusType", "BalanceType", "LienPriority",
        "LoanProduct", "LoanPurpose", "LockPeriod", "MiCoverageType",
        "Occupancy", "ProgramCode", "PropertyType", "Tier",
        // Task 1.6
        "TermBand", "TermMonths",
        // Task 1.7 / 1.9
        "GoalMask", "ScenarioKey",
    ];

    for name in expected_reexports {
        assert!(
            contents.contains(name),
            "lib.rs must re-export `{name}` — it is a public API type"
        );
    }
}

/// Verifies the workspace Cargo.toml has the lint configuration that powers
/// the coverage-forcing `unsafe_code = forbid` rule.
#[test]
fn test_workspace_cargo_toml_has_lint_config() {
    let workspace_toml = workspace_root().join("Cargo.toml");
    let contents = std::fs::read_to_string(workspace_toml)
        .expect("workspace Cargo.toml must be readable");
    assert!(
        contents.contains("unsafe_code"),
        "workspace Cargo.toml must forbid unsafe_code"
    );
    assert!(
        contents.contains("[workspace.lints"),
        "workspace Cargo.toml must have [workspace.lints] section"
    );
}
