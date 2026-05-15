//! Epic 1 deployment gate — Tasks 1.1, 1.2, 1.3.
//!
//! **This test suite must reach 100 % before work on Task 1.4 begins.**
//!
//! Run it in isolation with:
//!
//! ```text
//! cargo test --test epic_1_gate
//! ```
//!
//! The gate is organised into three modules, one per task. Each module covers
//! two concerns:
//!
//! 1. **Structure** — the source files and workspace configuration promised by
//!    the task specification exist on disk.
//! 2. **Behaviour** — the public API can be exercised end-to-end: construction,
//!    arithmetic/parsing, Display, JSON round-trip. These are quick smoke tests,
//!    not exhaustive proofs (those live in the per-type unit tests and the
//!    `properties.rs`/`identifier_properties.rs` suites).
//!
//! If any test in this file fails, the named task is not fully deployed and
//! must be remedied before 1.4 starts.
//!
//! ─────────────────────────────────────────────────────────────────────────
//!
//! ## Pass criteria summary
//!
//! ### Task 1.1 — Workspace Bootstrap
//! - All 13 crates present as workspace members
//! - Each member Cargo.toml inherits workspace lints
//! - Release profile is production-grade
//! - `unsafe_code` is workspace-forbidden
//!
//! ### Task 1.2 — Money Types
//! - 7 source files present (6 types + error.rs)
//! - Every type: construct → arithmetic → Display → serde roundtrip
//! - ParseError: all 6 Task 1.2 variants are constructible
//!
//! ### Task 1.3 — Identifier Types
//! - 7 source files present (7 types)
//! - Every type: construct → validate → Display → serde roundtrip
//! - ParseError: all 4 Task 1.3 variants are constructible
//! - UUID types: 100-ID uniqueness sweep each

#![allow(unused_qualifications)]

use std::path::{Path, PathBuf};

use cargo_metadata::MetadataCommand;
use toml::Value;

// ── helpers ──────────────────────────────────────────────────────────────────

fn workspace_root() -> PathBuf {
    MetadataCommand::new()
        .exec()
        .expect("cargo metadata must succeed")
        .workspace_root
        .into_std_path_buf()
}

fn workspace_toml() -> Value {
    let path = workspace_root().join("Cargo.toml");
    let src = std::fs::read_to_string(path).expect("workspace Cargo.toml must be readable");
    toml::from_str(&src).expect("workspace Cargo.toml must be valid TOML")
}

/// Path to the `types` crate root (the package that owns all the types).
fn types_crate() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn assert_file_exists(path: &Path) {
    assert!(
        path.exists(),
        "required file is missing: {}",
        path.display()
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// TASK 1.1 — Workspace Bootstrap
// ─────────────────────────────────────────────────────────────────────────────

mod task_1_1 {
    use super::*;

    const EXPECTED_CRATES: &[&str] = &[
        "types",
        "mismo",
        "reso",
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
        "ref_data",
    ];

    #[test]
    fn t1_1_all_13_crates_are_workspace_members() {
        let meta = MetadataCommand::new()
            .exec()
            .expect("cargo metadata must succeed");

        let member_names: Vec<&str> = meta
            .workspace_packages()
            .iter()
            .map(|p| p.name.as_str())
            .collect();

        for expected in EXPECTED_CRATES {
            assert!(
                member_names.contains(expected),
                "crate `{expected}` is missing from workspace members; \
                 found: {member_names:?}"
            );
        }
        assert_eq!(
            member_names.len(),
            14,
            "expected exactly 14 workspace members, found {}; \
             members: {member_names:?}",
            member_names.len()
        );
    }

    #[test]
    fn t1_1_all_crate_directories_exist_on_disk() {
        let root = workspace_root();
        for name in EXPECTED_CRATES {
            let crate_dir = root.join("crates").join(name);
            assert!(
                crate_dir.is_dir(),
                "crate directory missing: {}",
                crate_dir.display()
            );
            assert!(
                crate_dir.join("Cargo.toml").exists(),
                "Cargo.toml missing in crate `{name}`"
            );
            assert!(
                crate_dir.join("src").is_dir(),
                "`src/` directory missing in crate `{name}`"
            );
        }
    }

    #[test]
    fn t1_1_every_member_crate_inherits_workspace_lints() {
        let root = workspace_root();
        for name in EXPECTED_CRATES {
            let path = root.join("crates").join(name).join("Cargo.toml");
            let src = std::fs::read_to_string(&path)
                .unwrap_or_else(|_| panic!("cannot read {}", path.display()));
            let toml: Value = toml::from_str(&src)
                .unwrap_or_else(|_| panic!("invalid TOML in {}", path.display()));

            let workspace_flag = toml
                .get("lints")
                .and_then(|l| l.get("workspace"))
                .and_then(Value::as_bool)
                .unwrap_or(false);

            assert!(
                workspace_flag,
                "crate `{name}` must have `[lints] workspace = true` \
                 so it inherits the workspace-level clippy/rustc strictness"
            );
        }
    }

    #[test]
    fn t1_1_release_profile_is_production_grade() {
        let toml = workspace_toml();
        let release = toml
            .get("profile")
            .and_then(|p| p.get("release"))
            .expect("workspace Cargo.toml must have [profile.release]");

        assert_eq!(
            release.get("lto").and_then(Value::as_str),
            Some("fat"),
            "[profile.release] lto must be \"fat\""
        );
        assert_eq!(
            release.get("codegen-units").and_then(Value::as_integer),
            Some(1),
            "[profile.release] codegen-units must be 1"
        );
        assert_eq!(
            release.get("opt-level").and_then(Value::as_integer),
            Some(3),
            "[profile.release] opt-level must be 3"
        );
        assert_eq!(
            release.get("panic").and_then(Value::as_str),
            Some("abort"),
            "[profile.release] panic must be \"abort\""
        );
    }

    #[test]
    fn t1_1_unsafe_code_is_workspace_forbidden() {
        let toml = workspace_toml();
        let unsafe_setting = toml
            .get("workspace")
            .and_then(|w| w.get("lints"))
            .and_then(|l| l.get("rust"))
            .and_then(|r| r.get("unsafe_code"))
            .expect("[workspace.lints.rust.unsafe_code] must be set");

        // May be the string "forbid" or a table {level="forbid",...}
        let level = if let Some(s) = unsafe_setting.as_str() {
            s.to_string()
        } else {
            unsafe_setting
                .get("level")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string()
        };
        assert_eq!(
            level, "forbid",
            "unsafe_code must be `forbid` at workspace level, got `{level}`"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// TASK 1.2 — Money Types
// ─────────────────────────────────────────────────────────────────────────────

mod task_1_2 {
    use super::*;
    use rust_decimal_macros::dec;
    use types::{
        BasisPoints, Cents, CreditScore, DtiBasisPoints, LtvBasisPoints, ParseError, PriceTicks,
    };

    // ── file structure ────────────────────────────────────────────────────────

    #[test]
    fn t1_2_all_source_files_exist() {
        let src = types_crate().join("src");
        let required = [
            "error.rs",
            "cents.rs",
            "basis_points.rs",
            "price_ticks.rs",
            "ltv.rs",
            "dti.rs",
            "credit_score.rs",
        ];
        for file in required {
            assert_file_exists(&src.join(file));
        }
    }

    #[test]
    fn t1_2_properties_test_file_exists() {
        assert_file_exists(&types_crate().join("tests").join("properties.rs"));
    }

    #[test]
    fn t1_2_benchmark_file_exists() {
        assert_file_exists(&types_crate().join("benches").join("money_arithmetic.rs"));
    }

    // ── Cents ─────────────────────────────────────────────────────────────────

    #[test]
    fn t1_2_cents_api() {
        // Construction
        assert_eq!(Cents(150), Cents(150));
        assert_eq!(Cents::from_dollars(5), Cents(500));
        assert_eq!(Cents::ZERO, Cents(0));

        // Arithmetic — checked
        assert_eq!(Cents(100).checked_add(Cents(50)), Some(Cents(150)));
        assert_eq!(Cents(100).checked_sub(Cents(50)), Some(Cents(50)));
        assert_eq!(Cents(100).checked_mul(3), Some(Cents(300)));
        assert_eq!(Cents(i64::MAX).checked_add(Cents(1)), None);
        assert_eq!(Cents(i64::MIN).checked_sub(Cents(1)), None);

        // Arithmetic — saturating
        assert_eq!(Cents(i64::MAX).saturating_add(Cents(1)), Cents(i64::MAX));

        // Decimal conversion
        assert_eq!(Cents(12345).to_decimal(), dec!(123.45));
        assert_eq!(
            Cents::from_decimal_round_half_up(dec!(123.455)).unwrap(),
            Cents(12346)
        );

        // Display
        assert_eq!(Cents(123456).to_string(), "$1,234.56");
        assert_eq!(Cents(-150).to_string(), "-$1.50");

        // Parse
        assert_eq!("$1,234.56".parse::<Cents>().unwrap(), Cents(123456));
        assert!("garbage".parse::<Cents>().is_err());

        // Serde
        let json = serde_json::to_string(&Cents(99)).unwrap();
        assert_eq!(json, "99");
        assert_eq!(serde_json::from_str::<Cents>("99").unwrap(), Cents(99));

        // Predicates
        assert!(Cents(1).is_positive());
        assert!(Cents(-1).is_negative());
        assert!(Cents(0).is_zero());
        assert_eq!(Cents(-100).abs(), Cents(100));

        // Zero-overhead repr
        assert_eq!(std::mem::size_of::<Cents>(), std::mem::size_of::<i64>());
    }

    // ── BasisPoints ───────────────────────────────────────────────────────────

    #[test]
    fn t1_2_basis_points_api() {
        // Parse from percentage string
        assert_eq!(
            BasisPoints::from_percentage_str("6.875").unwrap(),
            BasisPoints(6875)
        );
        assert_eq!(
            BasisPoints::from_percentage_str("7%").unwrap(),
            BasisPoints(7000)
        );
        assert!(BasisPoints::from_percentage_str("-1").is_err());

        // Rate conversion: 6.875% → 0.06875
        assert_eq!(BasisPoints(6875).to_decimal_rate(), dec!(0.06875));
        assert_eq!(BasisPoints(6875).to_decimal_percent(), dec!(6.875));

        // Display: always 3 decimal places
        assert_eq!(BasisPoints(6875).to_string(), "6.875%");
        assert_eq!(BasisPoints(7000).to_string(), "7.000%");

        // Arithmetic
        assert_eq!(
            BasisPoints(6875).checked_add(BasisPoints(125)),
            Some(BasisPoints(7000))
        );
        assert_eq!(BasisPoints(u32::MAX).checked_add(BasisPoints(1)), None);

        // Serde
        let json = serde_json::to_string(&BasisPoints(6875)).unwrap();
        assert_eq!(json, "6875");
        assert_eq!(
            serde_json::from_str::<BasisPoints>("6875").unwrap(),
            BasisPoints(6875)
        );
    }

    // ── PriceTicks ────────────────────────────────────────────────────────────

    #[test]
    fn t1_2_price_ticks_api() {
        // Signed type — discounts are negative
        assert!(PriceTicks(-32810).is_discount());
        assert!(PriceTicks(22500).is_premium());
        assert!(PriceTicks(0).is_par());

        // Parse
        assert_eq!(
            PriceTicks::from_percentage_points_str("-3.281").unwrap(),
            PriceTicks(-32810)
        );

        // Apply to loan: $200k × -3.281% = -$6,562.00
        let cost = PriceTicks(-32810).apply_to_loan(Cents(20_000_000));
        assert_eq!(cost, Cents(-656_200));

        // Display
        assert_eq!(PriceTicks(-32810).to_string(), "-3.2810");
        assert_eq!(PriceTicks(0).to_string(), "0.0000");

        // Serde
        let json = serde_json::to_string(&PriceTicks(-32810)).unwrap();
        assert_eq!(json, "-32810");
        assert_eq!(
            serde_json::from_str::<PriceTicks>("-32810").unwrap(),
            PriceTicks(-32810)
        );

        // Zero overhead
        assert_eq!(
            std::mem::size_of::<PriceTicks>(),
            std::mem::size_of::<i32>()
        );
    }

    // ── LtvBasisPoints ────────────────────────────────────────────────────────

    #[test]
    fn t1_2_ltv_basis_points_api() {
        // Validating constructor
        assert_eq!(LtvBasisPoints::new(9500).unwrap(), LtvBasisPoints(9500));
        assert_eq!(LtvBasisPoints::new(11000).unwrap(), LtvBasisPoints(11_000)); // boundary OK
        assert!(
            LtvBasisPoints::new(11001).is_err(),
            "11001 bp = 110.01% must be rejected"
        );

        // From loan / value
        let ltv =
            LtvBasisPoints::from_loan_and_value(Cents(28_500_000), Cents(30_000_000)).unwrap();
        assert_eq!(ltv, LtvBasisPoints(9500)); // 95.00%

        // Decimal conversions
        assert_eq!(LtvBasisPoints(9500).to_decimal_percent(), dec!(95.00));
        assert_eq!(LtvBasisPoints(9700).to_decimal_rate(), dec!(0.9700));

        // Display
        assert_eq!(LtvBasisPoints(9500).to_string(), "95.00%");

        // Serde
        let original = LtvBasisPoints::new(9700).unwrap();
        let json = serde_json::to_string(&original).unwrap();
        assert_eq!(json, "9700");
        assert_eq!(
            serde_json::from_str::<LtvBasisPoints>("9700").unwrap(),
            original
        );
    }

    // ── DtiBasisPoints ────────────────────────────────────────────────────────

    #[test]
    fn t1_2_dti_basis_points_api() {
        // No hard cap — construction always succeeds
        let normal = DtiBasisPoints::new(4300);
        assert_eq!(normal.0, 4300);

        let high = DtiBasisPoints::new(7000); // above 60% threshold
        assert_eq!(high.0, 7000);
        assert!(high.exceeds_typical_max());
        assert!(!DtiBasisPoints::new(4300).exceeds_typical_max());

        // Display
        assert_eq!(DtiBasisPoints::new(4300).to_string(), "43.00%");

        // Serde
        let d = DtiBasisPoints::new(5000);
        let json = serde_json::to_string(&d).unwrap();
        assert_eq!(json, "5000");
        assert_eq!(serde_json::from_str::<DtiBasisPoints>("5000").unwrap(), d);
    }

    // ── CreditScore ───────────────────────────────────────────────────────────

    #[test]
    fn t1_2_credit_score_api() {
        // Range: 300..=850
        assert!(CreditScore::new(299).is_err());
        assert!(CreditScore::new(300).is_ok());
        assert!(CreditScore::new(720).is_ok());
        assert!(CreditScore::new(850).is_ok());
        assert!(CreditScore::new(851).is_err());

        // Error variant
        match CreditScore::new(900) {
            Err(ParseError::CreditScoreOutOfRange(v)) => assert_eq!(v, 900),
            other => panic!("expected CreditScoreOutOfRange, got {other:?}"),
        }

        // Middle of three
        let mid = CreditScore::middle_of_three(
            CreditScore::new(720).unwrap(),
            CreditScore::new(740).unwrap(),
            CreditScore::new(700).unwrap(),
        );
        assert_eq!(mid, CreditScore(720));

        // Display
        assert_eq!(CreditScore::new(720).unwrap().to_string(), "720");

        // Serde
        let cs = CreditScore::new(720).unwrap();
        let json = serde_json::to_string(&cs).unwrap();
        assert_eq!(json, "720");
        assert_eq!(serde_json::from_str::<CreditScore>("720").unwrap(), cs);
    }

    // ── ParseError Task 1.2 variants ─────────────────────────────────────────

    #[test]
    fn t1_2_parse_error_variants_all_present() {
        // Verify every Task 1.2 error variant can be constructed and
        // matched. If a variant was removed or renamed, this test fails.
        let errors: Vec<ParseError> = vec![
            ParseError::LtvOutOfRange(15_000),
            ParseError::CreditScoreOutOfRange(900),
            ParseError::InvalidPercentageString("bad".to_string()),
            ParseError::InvalidMoneyString("bad".to_string()),
            ParseError::DecimalOutOfRange("overflow".to_string()),
            ParseError::ZeroPropertyValue,
        ];
        // All 6 Task 1.2 variants must be distinct (PartialEq is derived)
        assert_eq!(errors.len(), 6);
        // Each error must produce a non-empty Display message
        for err in &errors {
            let msg = err.to_string();
            assert!(!msg.is_empty(), "ParseError variant produced empty message");
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// TASK 1.3 — Identifier Types
// ─────────────────────────────────────────────────────────────────────────────

mod task_1_3 {
    use std::collections::HashSet;

    use super::*;
    use types::{
        AnalysisId, FipsCode, LenderId, LoanCasefileId, MlsListingKey, ParseError, ScenarioId,
        StateCode,
    };

    // ── file structure ────────────────────────────────────────────────────────

    #[test]
    fn t1_3_all_source_files_exist() {
        let src = types_crate().join("src");
        let required = [
            "state_code.rs",
            "fips_code.rs",
            "lender_id.rs",
            "mls_listing_key.rs",
            "loan_casefile_id.rs",
            "scenario_id.rs",
            "analysis_id.rs",
        ];
        for file in required {
            assert_file_exists(&src.join(file));
        }
    }

    #[test]
    fn t1_3_identifier_properties_test_file_exists() {
        assert_file_exists(&types_crate().join("tests").join("identifier_properties.rs"));
    }

    // ── StateCode ─────────────────────────────────────────────────────────────

    #[test]
    fn t1_3_state_code_api() {
        // All 56 entries present
        assert_eq!(StateCode::ALL.len(), 56);

        // Parse — case-insensitive
        assert_eq!("CA".parse::<StateCode>().unwrap(), StateCode::CA);
        assert_eq!("ca".parse::<StateCode>().unwrap(), StateCode::CA);
        assert!("XX".parse::<StateCode>().is_err());

        // Territories included
        assert_eq!("PR".parse::<StateCode>().unwrap(), StateCode::PR);
        assert_eq!("DC".parse::<StateCode>().unwrap(), StateCode::DC);

        // FIPS roundtrip for every state
        for sc in StateCode::ALL {
            let fips = sc.to_fips();
            let back = StateCode::from_fips(fips)
                .unwrap_or_else(|| panic!("from_fips failed for {sc:?} (fips={fips})"));
            assert_eq!(*sc, back);
        }

        // is_state / is_territory
        assert!(StateCode::CA.is_state());
        assert!(StateCode::PR.is_territory()); // PR is a territory
        let territory_count = StateCode::ALL.iter().filter(|s| s.is_territory()).count();
        assert_eq!(territory_count, 5);
        let state_count = StateCode::ALL.iter().filter(|s| s.is_state()).count();
        assert_eq!(state_count, 50);

        // Display
        assert_eq!(StateCode::CA.to_string(), "CA");

        // Serde — canonical uppercase representation
        let json = serde_json::to_string(&StateCode::CA).unwrap();
        assert_eq!(json, "\"CA\"");
        assert_eq!(
            serde_json::from_str::<StateCode>("\"CA\"").unwrap(),
            StateCode::CA
        );
    }

    // ── FipsCode ──────────────────────────────────────────────────────────────

    #[test]
    fn t1_3_fips_code_api() {
        // Construct from parts
        let la = FipsCode::new(6, 37).unwrap();
        assert_eq!(la.state_fips(), 6);
        assert_eq!(la.county_fips(), 37);
        assert_eq!(la.state_code(), StateCode::CA);

        // Construct from string
        let parsed: FipsCode = "06037".parse().unwrap();
        assert_eq!(parsed, la);

        // Display: always 5 digits with leading zero
        assert_eq!(la.to_string(), "06037");
        assert_eq!(FipsCode::new(1, 1).unwrap().to_string(), "01001");

        // Validation: unassigned state rejected
        assert!(FipsCode::new(3, 1).is_err()); // FIPS 3 is unassigned
        assert!(FipsCode::new(6, 1000).is_err()); // county > 999

        // Invalid string
        assert!("0603".parse::<FipsCode>().is_err()); // too short
        assert!("06A37".parse::<FipsCode>().is_err()); // non-digit

        // Serde
        let json = serde_json::to_string(&la).unwrap();
        let back: FipsCode = serde_json::from_str(&json).unwrap();
        assert_eq!(back, la);

        // Zero overhead
        assert_eq!(std::mem::size_of::<FipsCode>(), std::mem::size_of::<u32>());
    }

    // ── LenderId ──────────────────────────────────────────────────────────────

    #[test]
    fn t1_3_lender_id_api() {
        // Valid construction
        let id = LenderId::new("UWM").unwrap();
        assert_eq!(id.as_str(), "UWM");

        // Whitespace trimmed
        assert_eq!(LenderId::new("  ROCKET  ").unwrap().as_str(), "ROCKET");

        // Empty rejected
        assert!(matches!(
            LenderId::new(""),
            Err(ParseError::IdentifierEmpty { kind: "LenderId" })
        ));

        // Too long rejected
        match LenderId::new("A".repeat(33)) {
            Err(ParseError::IdentifierTooLong { kind, max, .. }) => {
                assert_eq!(kind, "LenderId");
                assert_eq!(max, 32);
            }
            other => panic!("expected IdentifierTooLong, got {other:?}"),
        }

        // Invalid chars rejected
        assert!(matches!(
            LenderId::new("UWM!"),
            Err(ParseError::IdentifierInvalidChars {
                kind: "LenderId",
                ..
            })
        ));

        // Display + serde
        assert_eq!(id.to_string(), "UWM");
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"UWM\"");
        assert_eq!(serde_json::from_str::<LenderId>("\"UWM\"").unwrap(), id);
    }

    // ── MlsListingKey ─────────────────────────────────────────────────────────

    #[test]
    fn t1_3_mls_listing_key_api() {
        let key = MlsListingKey::new("OC24123456").unwrap();
        assert_eq!(key.as_str(), "OC24123456");

        // RESO 2.0: up to 128 chars, printable ASCII
        assert!(MlsListingKey::new("A".repeat(128)).is_ok());
        assert!(matches!(
            MlsListingKey::new("A".repeat(129)),
            Err(ParseError::IdentifierTooLong {
                kind: "MlsListingKey",
                ..
            })
        ));
        assert!(MlsListingKey::new("").is_err());
        // Non-printable ASCII rejected
        assert!(MlsListingKey::new("key\x01").is_err());

        // Display + serde
        assert_eq!(key.to_string(), "OC24123456");
        let json = serde_json::to_string(&key).unwrap();
        assert_eq!(json, "\"OC24123456\"");
        assert_eq!(
            serde_json::from_str::<MlsListingKey>("\"OC24123456\"").unwrap(),
            key
        );
    }

    // ── LoanCasefileId ────────────────────────────────────────────────────────

    #[test]
    fn t1_3_loan_casefile_id_api() {
        // DU 10-digit format
        let id = LoanCasefileId::new("1234567890").unwrap();
        assert_eq!(id.as_str(), "1234567890");

        assert!(LoanCasefileId::new("A".repeat(64)).is_ok());
        assert!(matches!(
            LoanCasefileId::new("A".repeat(65)),
            Err(ParseError::IdentifierTooLong {
                kind: "LoanCasefileId",
                ..
            })
        ));
        assert!(LoanCasefileId::new("").is_err());
        // Internal spaces rejected
        assert!(LoanCasefileId::new("ID WITH SPACE").is_err());

        // Serde
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"1234567890\"");
        assert_eq!(
            serde_json::from_str::<LoanCasefileId>("\"1234567890\"").unwrap(),
            id
        );
    }

    // ── ScenarioId ────────────────────────────────────────────────────────────

    #[test]
    fn t1_3_scenario_id_api() {
        use uuid::Uuid;

        // new() generates UUID v4
        let id = ScenarioId::new();
        assert_eq!(id.0.get_version_num(), 4);

        // from_uuid
        let uuid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let from = ScenarioId::from_uuid(uuid);
        assert_eq!(*from.as_uuid(), uuid);

        // From<Uuid>
        let via_from: ScenarioId = uuid.into();
        assert_eq!(via_from, from);

        // Display — hyphenated UUID
        assert_eq!(from.to_string(), "550e8400-e29b-41d4-a716-446655440000");

        // FromStr
        let parsed: ScenarioId = "550e8400-e29b-41d4-a716-446655440000".parse().unwrap();
        assert_eq!(parsed, from);
        assert!("not-a-uuid".parse::<ScenarioId>().is_err());

        // NIL sentinel
        assert_eq!(ScenarioId::NIL.0, Uuid::nil());

        // Serde
        let json = serde_json::to_string(&from).unwrap();
        assert_eq!(json, "\"550e8400-e29b-41d4-a716-446655440000\"");
        assert_eq!(serde_json::from_str::<ScenarioId>(&json).unwrap(), from);

        // 100-ID uniqueness sweep
        let ids: HashSet<ScenarioId> = (0..100).map(|_| ScenarioId::new()).collect();
        assert_eq!(
            ids.len(),
            100,
            "ScenarioId::new() must not produce duplicates"
        );

        // Zero overhead — same size as Uuid (16 bytes)
        assert_eq!(std::mem::size_of::<ScenarioId>(), 16);
    }

    // ── AnalysisId ────────────────────────────────────────────────────────────

    #[test]
    fn t1_3_analysis_id_api() {
        use uuid::Uuid;

        let id = AnalysisId::new();
        assert_eq!(id.0.get_version_num(), 4);

        let uuid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let from = AnalysisId::from_uuid(uuid);
        assert_eq!(from.to_string(), "550e8400-e29b-41d4-a716-446655440000");

        // Serde
        let json = serde_json::to_string(&from).unwrap();
        assert_eq!(serde_json::from_str::<AnalysisId>(&json).unwrap(), from);

        // 100-ID uniqueness sweep
        let ids: HashSet<AnalysisId> = (0..100).map(|_| AnalysisId::new()).collect();
        assert_eq!(
            ids.len(),
            100,
            "AnalysisId::new() must not produce duplicates"
        );
    }

    // ── ParseError Task 1.3 variants ──────────────────────────────────────────

    #[test]
    fn t1_3_parse_error_variants_all_present() {
        // Verify every Task 1.3 error variant can be constructed. If a
        // variant was removed or renamed, this test will not compile.
        let errors: Vec<ParseError> = vec![
            ParseError::InvalidFipsCode("99001".to_string()),
            ParseError::InvalidStateCode("XX".to_string()),
            ParseError::IdentifierEmpty { kind: "LenderId" },
            ParseError::IdentifierTooLong {
                kind: "LenderId",
                actual: 33,
                max: 32,
            },
            ParseError::IdentifierInvalidChars {
                kind: "LenderId",
                value: "bad!".to_string(),
            },
        ];
        assert_eq!(errors.len(), 5);
        for err in &errors {
            let msg = err.to_string();
            assert!(!msg.is_empty());
        }
    }
}
