//! Test: `test_all_crates_present`.
//!
//! From the Task 1.1 specification: "programmatically verifies every expected
//! crate exists." This test reads the workspace metadata via `cargo metadata`
//! and asserts:
//!
//! 1. The 13 expected crates listed in the architecture are all members of
//!    the workspace.
//! 2. No unexpected crates have been added without being included in this
//!    list (which would indicate the architecture document is out of date).
//! 3. Each member crate has the required file structure: a `Cargo.toml`,
//!    a `src/` directory, and a `src/lib.rs`.
//!
//! Using `cargo_metadata` rather than parsing `Cargo.toml` directly means
//! this test stays correct even if the workspace `members` glob changes
//! shape (e.g., `["crates/*"]` becomes `["crates/*", "internal/*"]`).

use std::collections::HashSet;
use std::path::Path;

use cargo_metadata::MetadataCommand;

/// The complete, authoritative list of crates that must exist in the
/// workspace per the Meridian Mortgage Engine architecture. Adding a crate
/// to the workspace without adding it here is a deliberate failure mode: a
/// new crate is a structural change that the test enforces a deliberate
/// update for.
const EXPECTED_CRATES: &[&str] = &[
    "types",
    "mismo",
    "reso",
    "ref_data",
    "ingest",
    "enrich",
    "eligibility",
    "compliance",
    "scenarios",
    "solver",
    "amort",
    "ml",
    "orchestrator",
    "api",
];

#[test]
fn test_all_crates_present() {
    let metadata = MetadataCommand::new()
        .exec()
        .expect("cargo metadata must succeed in a valid workspace");

    let actual: HashSet<String> = metadata
        .workspace_packages()
        .into_iter()
        .map(|p| p.name.clone())
        .collect();

    let expected: HashSet<String> = EXPECTED_CRATES.iter().map(|s| (*s).to_string()).collect();

    let missing: Vec<&String> = expected.difference(&actual).collect();
    let extra: Vec<&String> = actual.difference(&expected).collect();

    assert!(
        missing.is_empty(),
        "workspace is missing expected crates: {missing:?}.\n\
         The architecture requires all of {EXPECTED_CRATES:?} to exist.",
    );

    assert!(
        extra.is_empty(),
        "workspace contains unexpected crates: {extra:?}.\n\
         If these are intentional additions, update the EXPECTED_CRATES list \
         in this test and the architecture document at docs/architecture.md.",
    );

    assert_eq!(
        actual.len(),
        EXPECTED_CRATES.len(),
        "workspace must contain exactly {} crates",
        EXPECTED_CRATES.len(),
    );
}

#[test]
fn test_every_crate_has_required_file_structure() {
    let metadata = MetadataCommand::new()
        .exec()
        .expect("cargo metadata must succeed");

    let workspace_root: &Path = metadata.workspace_root.as_std_path();

    for crate_name in EXPECTED_CRATES {
        let crate_dir = workspace_root.join("crates").join(crate_name);
        let manifest = crate_dir.join("Cargo.toml");
        let src_dir = crate_dir.join("src");
        let lib_rs = src_dir.join("lib.rs");

        assert!(
            crate_dir.is_dir(),
            "crate directory missing: {}",
            crate_dir.display()
        );
        assert!(
            manifest.is_file(),
            "Cargo.toml missing for crate `{crate_name}`: {}",
            manifest.display()
        );
        assert!(
            src_dir.is_dir(),
            "src/ directory missing for crate `{crate_name}`: {}",
            src_dir.display()
        );
        assert!(
            lib_rs.is_file(),
            "src/lib.rs missing for crate `{crate_name}`: {}",
            lib_rs.display()
        );
    }
}

#[test]
fn test_every_crate_inherits_workspace_package_metadata() {
    // Members must inherit version/edition/authors/license/rust-version from
    // the workspace root to keep them aligned. A crate that hardcodes its own
    // version drifts from the rest of the workspace at the next release.
    let metadata = MetadataCommand::new()
        .exec()
        .expect("cargo metadata must succeed");

    let workspace_version = "0.1.0";
    let workspace_edition = "2021";

    for package in metadata.workspace_packages() {
        assert_eq!(
            package.version.to_string(),
            workspace_version,
            "crate `{}` has version {} but workspace is at {workspace_version}",
            package.name,
            package.version,
        );
        assert_eq!(
            package.edition.to_string(),
            workspace_edition,
            "crate `{}` uses edition {} but workspace standardizes on {workspace_edition}",
            package.name,
            package.edition,
        );
    }
}
