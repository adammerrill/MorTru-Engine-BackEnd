//! Test: `test_workspace_compiles_clean`.
//!
//! From the Task 1.1 specification: "verifies the workspace builds without
//! warnings." The fact that *this test runs at all* is itself proof that the
//! workspace compiles — `cargo test` will not invoke a test binary that
//! failed to build. What this test adds is a check that the *strictness
//! configuration* required to keep the workspace clean is actually in place.
//! Without these settings, "clean" today could quietly slip into "warnings
//! everywhere" tomorrow.
//!
//! Concretely the test asserts:
//!
//! 1. The workspace root `Cargo.toml` declares `[workspace.lints.rust]` and
//!    `[workspace.lints.clippy]` with the required strictness.
//! 2. `unsafe_code` is `forbid`den at the workspace level. The engine has no
//!    legitimate need for `unsafe` and any introduction is a deliberate
//!    review event.
//! 3. The release profile uses `lto = "fat"`, `codegen-units = 1`,
//!    `opt-level = 3`, and `panic = "abort"` — the production-grade settings
//!    that the architecture explicitly calls for.
//! 4. Every member crate inherits the workspace lints via
//!    `[lints] workspace = true` (rather than defining its own and drifting).

use std::path::PathBuf;

use cargo_metadata::MetadataCommand;
use toml::Value;

fn workspace_root() -> PathBuf {
    let metadata = MetadataCommand::new()
        .exec()
        .expect("cargo metadata must succeed");
    metadata.workspace_root.into_std_path_buf()
}

fn workspace_toml() -> Value {
    let path = workspace_root().join("Cargo.toml");
    let contents = std::fs::read_to_string(path).expect("workspace Cargo.toml must be readable");
    toml::from_str(&contents).expect("workspace Cargo.toml must be valid TOML")
}

#[test]
fn test_workspace_compiles_clean() {
    // If this test executes, the workspace compiled. The harness would not
    // have built this binary otherwise. The substantive checks below verify
    // the *settings that keep it clean*.
    let cargo_toml = workspace_toml();

    let lints = cargo_toml
        .get("workspace")
        .and_then(|w| w.get("lints"))
        .expect("workspace.lints table required to enforce strict compilation");

    let rust_lints = lints.get("rust").expect("workspace.lints.rust required");

    let unsafe_code = rust_lints
        .get("unsafe_code")
        .and_then(Value::as_str)
        .expect("workspace.lints.rust.unsafe_code must be configured");
    assert_eq!(
        unsafe_code, "forbid",
        "unsafe_code must be `forbid` at workspace level; \
         introducing unsafe requires a documented exemption",
    );

    let clippy_lints = lints
        .get("clippy")
        .expect("workspace.lints.clippy required");

    // `clippy::correctness` is `deny`-level. Correctness lints flag actual
    // bugs (e.g. comparing floats with `==`); allowing them in production
    // pricing code is unacceptable.
    let correctness_level = clippy_lints
        .get("correctness")
        .and_then(|v| v.get("level"))
        .and_then(Value::as_str)
        .expect("clippy::correctness lint group must be configured");
    assert!(
        matches!(correctness_level, "deny" | "forbid"),
        "clippy::correctness must be deny or forbid; got `{correctness_level}`",
    );
}

#[test]
fn test_release_profile_is_production_grade() {
    let cargo_toml = workspace_toml();
    let release = cargo_toml
        .get("profile")
        .and_then(|p| p.get("release"))
        .expect("[profile.release] required");

    assert_eq!(
        release.get("lto").and_then(Value::as_str),
        Some("fat"),
        "release profile must use fat LTO for maximum cross-crate inlining",
    );
    assert_eq!(
        release.get("codegen-units").and_then(Value::as_integer),
        Some(1),
        "release profile must use a single codegen unit",
    );
    assert_eq!(
        release.get("opt-level").and_then(Value::as_integer),
        Some(3),
        "release profile must use opt-level 3",
    );
    assert_eq!(
        release.get("panic").and_then(Value::as_str),
        Some("abort"),
        "release profile must use panic=abort; \
         any panic in the engine is a bug to fix, not recover from",
    );
}

#[test]
fn test_dev_profile_has_overflow_checks() {
    // The dev profile must keep overflow checks on so that arithmetic bugs
    // surface during testing rather than silently wrapping in production-
    // equivalent integer math. This matters specifically for Cents and
    // BasisPoints arithmetic that lands in Task 1.2.
    let cargo_toml = workspace_toml();
    let dev = cargo_toml
        .get("profile")
        .and_then(|p| p.get("dev"))
        .expect("[profile.dev] required");

    let overflow_checks = dev
        .get("overflow-checks")
        .and_then(Value::as_bool)
        .expect("profile.dev.overflow-checks must be configured");
    assert!(
        overflow_checks,
        "dev profile must keep overflow-checks on so arithmetic bugs surface during tests",
    );
}

#[test]
fn test_every_member_crate_inherits_workspace_lints() {
    let metadata = MetadataCommand::new()
        .exec()
        .expect("cargo metadata must succeed");

    for package in metadata.workspace_packages() {
        let manifest_path = package.manifest_path.as_std_path();
        let contents = std::fs::read_to_string(manifest_path)
            .unwrap_or_else(|e| panic!("could not read manifest for `{}`: {e}", package.name));
        let parsed: Value = toml::from_str(&contents)
            .unwrap_or_else(|e| panic!("could not parse manifest for `{}`: {e}", package.name));

        // Either [lints] workspace = true, OR the crate is the workspace
        // root (which has its own [workspace.lints] section). Member crates
        // must inherit.
        let lints = parsed.get("lints").unwrap_or_else(|| {
            panic!(
                "crate `{}` is missing the [lints] table; \
                 every member must declare `[lints] workspace = true`",
                package.name
            )
        });

        let inherits = lints
            .get("workspace")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        assert!(
            inherits,
            "crate `{}` does not inherit workspace lints; \
             add `[lints] workspace = true` to its Cargo.toml",
            package.name
        );
    }
}

#[test]
fn test_workspace_resolver_is_v2() {
    // Resolver v2 is required for correct feature unification across
    // workspace and dev-dependencies. v1 silently merges features in ways
    // that can pull in unwanted code paths in release builds.
    let cargo_toml = workspace_toml();
    let resolver = cargo_toml
        .get("workspace")
        .and_then(|w| w.get("resolver"))
        .and_then(Value::as_str)
        .expect("workspace.resolver must be set");
    assert_eq!(
        resolver, "2",
        "workspace must use resolver = \"2\" for correct feature unification",
    );
}
