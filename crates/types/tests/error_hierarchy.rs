//! Integration tests for the Task 1.4 error hierarchy.
//!
//! Spec-required tests:
//! - `test_error_display_includes_context`
//! - `test_error_chain_preserves_source`
//! - `test_io_error_auto_converts`

use std::error::Error;

use types::{ComplianceError, EligibilityError, IngestionError, SolverError};

// ─────────────────────────────────────────────────────────────────────────────
// test_error_display_includes_context
//
// Every error variant's Display string must include the diagnostic fields
// the operator needs to identify the problem. We test one representative
// variant per enum here; the exhaustive per-variant tests live inline in
// each module's #[cfg(test)] block.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_error_display_includes_context() {
    // IngestionError — schema mismatch should name the file and both values
    let ingestion = IngestionError::SchemaMismatch {
        file: "uwm_conventional_30y.xlsx".to_string(),
        expected: "interest_rate_percentage".to_string(),
        found: "raw_cell_ref_abc123".to_string(),
    };
    let msg = ingestion.to_string();
    assert!(
        msg.contains("uwm_conventional_30y.xlsx"),
        "IngestionError::SchemaMismatch must include the filename; got: {msg}"
    );
    assert!(
        msg.contains("interest_rate_percentage"),
        "IngestionError::SchemaMismatch must include expected type; got: {msg}"
    );
    assert!(
        msg.contains("raw_cell_ref_abc123"),
        "IngestionError::SchemaMismatch must include found value; got: {msg}"
    );

    // EligibilityError — credit score rejection should name score, minimum, program
    let eligibility = EligibilityError::CreditScoreBelowMinimum {
        score: 619,
        minimum: 620,
        program: "HomeReady 97 LTV".to_string(),
    };
    let msg = eligibility.to_string();
    assert!(msg.contains("619"), "score must appear; got: {msg}");
    assert!(msg.contains("620"), "minimum must appear; got: {msg}");
    assert!(
        msg.contains("HomeReady 97 LTV"),
        "program must appear; got: {msg}"
    );

    // SolverError — no rate found must include the scenario key
    let solver = SolverError::RateNotFound {
        scenario_key: "CONV30Y_fico720_ltv9500".to_string(),
    };
    let msg = solver.to_string();
    assert!(
        msg.contains("CONV30Y_fico720_ltv9500"),
        "SolverError::RateNotFound must include scenario key; got: {msg}"
    );

    // ComplianceError — HOEPA APR trigger must name APR, threshold, and citation
    let compliance = ComplianceError::HoepaAprThresholdExceeded {
        apr_bps: 14_000,
        apr_display: 14.000,
        threshold_bps: 13_125,
        threshold_display: 13.125,
    };
    let msg = compliance.to_string();
    assert!(msg.contains("14000"), "APR bps must appear; got: {msg}");
    assert!(
        msg.contains("13125"),
        "threshold bps must appear; got: {msg}"
    );
    assert!(
        msg.contains("1026.32"),
        "regulatory citation must appear; got: {msg}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// test_error_chain_preserves_source
//
// The IngestionError::Io variant wraps std::io::Error via #[from]. The
// wrapped error must be accessible through std::error::Error::source()
// so tooling and logging libraries can walk the full chain.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_error_chain_preserves_source() {
    let io_err = std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        "rate_sheet.xlsx: permission denied",
    );

    let ingestion_err = IngestionError::Io(io_err);

    // source() must return the inner io::Error
    let source = ingestion_err
        .source()
        .expect("IngestionError::Io must expose its source via Error::source()");

    // Downcast back to std::io::Error to verify identity
    let io_source = source
        .downcast_ref::<std::io::Error>()
        .expect("source must be a std::io::Error");

    assert_eq!(
        io_source.kind(),
        std::io::ErrorKind::PermissionDenied,
        "ErrorKind must survive the chain"
    );
    assert!(
        io_source.to_string().contains("permission denied"),
        "error message must survive the chain; got: {}",
        io_source
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// test_io_error_auto_converts
//
// The #[from] attribute on IngestionError::Io must generate a
// From<std::io::Error> impl so the ? operator works in functions that
// return Result<_, IngestionError>.
// ─────────────────────────────────────────────────────────────────────────────

fn load_rate_sheet(path: &str) -> Result<String, IngestionError> {
    // This uses ? to convert std::io::Error → IngestionError automatically.
    let content = std::fs::read_to_string(path)?;
    Ok(content)
}

#[test]
fn test_io_error_auto_converts() {
    let result = load_rate_sheet("/nonexistent/path/that/cannot/exist/rate_sheet.xlsx");

    assert!(result.is_err(), "reading a nonexistent file must fail");

    let err = result.unwrap_err();
    assert!(
        matches!(err, IngestionError::Io(_)),
        "std::io::Error must auto-convert to IngestionError::Io via ?; got: {err:?}"
    );

    // The error chain must still be intact after the ? conversion
    let source = err
        .source()
        .expect("converted Io error must preserve source");
    assert!(
        source.downcast_ref::<std::io::Error>().is_some(),
        "source must be a std::io::Error"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Additional coverage: all four enums implement std::error::Error
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_all_error_types_implement_std_error() {
    fn assert_std_error<E: Error>(_: &E) {}

    assert_std_error(&IngestionError::MismoValidation("test".to_string()));
    assert_std_error(&EligibilityError::MissingRequiredField { field: "ltv" });
    assert_std_error(&SolverError::InvalidScenario {
        reason: "zero loan amount".to_string(),
    });
    assert_std_error(&ComplianceError::AtrFailed {
        reason: "no income docs".to_string(),
    });
}

#[test]
fn test_ingestion_error_non_io_variants_have_no_source() {
    // Non-Io variants don't wrap another error, so source() returns None
    let errors: Vec<Box<dyn Error>> = vec![
        Box::new(IngestionError::SchemaMismatch {
            file: "x".to_string(),
            expected: "y".to_string(),
            found: "z".to_string(),
        }),
        Box::new(IngestionError::AmbiguousFicoBand {
            input: "680+".to_string(),
        }),
        Box::new(IngestionError::MismoValidation("bad".to_string())),
    ];
    for err in &errors {
        assert!(
            err.source().is_none(),
            "non-Io variant must not have a source; variant: {err}"
        );
    }
}

#[test]
fn test_every_ingestion_error_display_is_non_empty() {
    let io_err = std::io::Error::new(std::io::ErrorKind::Other, "test");
    let variants: Vec<Box<dyn Error>> = vec![
        Box::new(IngestionError::SchemaMismatch {
            file: "f".to_string(),
            expected: "e".to_string(),
            found: "g".to_string(),
        }),
        Box::new(IngestionError::UnrecognizedBlock {
            row: 0,
            col: 0,
            hint: "h".to_string(),
        }),
        Box::new(IngestionError::AmbiguousFicoBand {
            input: "i".to_string(),
        }),
        Box::new(IngestionError::MalformedLtvBand {
            input: "j".to_string(),
        }),
        Box::new(IngestionError::InvalidExcelDate { serial: 0.0 }),
        Box::new(IngestionError::MismoValidation("k".to_string())),
        Box::new(IngestionError::InvalidResoLookup {
            field: "l".to_string(),
            value: "m".to_string(),
        }),
        Box::new(IngestionError::Io(io_err)),
    ];
    assert_eq!(
        variants.len(),
        8,
        "test must cover all 8 IngestionError variants"
    );
    for v in &variants {
        assert!(
            !v.to_string().is_empty(),
            "Display must not be empty: {v:?}"
        );
    }
}

#[test]
fn test_every_eligibility_error_display_is_non_empty() {
    let variants: Vec<Box<dyn Error>> = vec![
        Box::new(EligibilityError::CreditScoreBelowMinimum {
            score: 600,
            minimum: 620,
            program: "p".to_string(),
        }),
        Box::new(EligibilityError::LtvExceedsLimit {
            ltv_bps: 9701,
            ltv_display: 97.01,
            limit_bps: 9700,
            limit_display: 97.00,
            program: "p".to_string(),
        }),
        Box::new(EligibilityError::DtiExceedsLimit {
            dti_bps: 4501,
            dti_display: 45.01,
            limit_bps: 4500,
            limit_display: 45.00,
            program: "p".to_string(),
        }),
        Box::new(EligibilityError::LoanAmountOutOfRange {
            amount_dollars: 800_000.0,
            min_dollars: 50_000.0,
            max_dollars: 726_200.0,
            program: "p".to_string(),
        }),
        Box::new(EligibilityError::IneligiblePropertyType {
            property_type: "Condotel".to_string(),
            program: "p".to_string(),
        }),
        Box::new(EligibilityError::IneligibleOccupancy {
            occupancy: "Investment".to_string(),
            program: "p".to_string(),
        }),
        Box::new(EligibilityError::InsufficientReserves {
            required_months: 6,
            available_months: 2,
            program: "p".to_string(),
        }),
        Box::new(EligibilityError::MissingRequiredField { field: "ltv" }),
    ];
    assert_eq!(
        variants.len(),
        8,
        "test must cover all 8 EligibilityError variants"
    );
    for v in &variants {
        assert!(
            !v.to_string().is_empty(),
            "Display must not be empty: {v:?}"
        );
    }
}

#[test]
fn test_every_solver_error_display_is_non_empty() {
    let variants: Vec<Box<dyn Error>> = vec![
        Box::new(SolverError::RateNotFound {
            scenario_key: "k".to_string(),
        }),
        Box::new(SolverError::AprIterationLimitExceeded {
            iterations: 100,
            last_residual: 0.001,
        }),
        Box::new(SolverError::AmortizationFailed {
            term_months: 360,
            reason: "r".to_string(),
        }),
        Box::new(SolverError::InvalidScenario {
            reason: "r".to_string(),
        }),
        Box::new(SolverError::NumericalOverflow {
            context: "c".to_string(),
        }),
    ];
    assert_eq!(
        variants.len(),
        5,
        "test must cover all 5 SolverError variants"
    );
    for v in &variants {
        assert!(
            !v.to_string().is_empty(),
            "Display must not be empty: {v:?}"
        );
    }
}

#[test]
fn test_every_compliance_error_display_is_non_empty() {
    let variants: Vec<Box<dyn Error>> = vec![
        Box::new(ComplianceError::HoepaAprThresholdExceeded {
            apr_bps: 14_000,
            apr_display: 14.0,
            threshold_bps: 13_000,
            threshold_display: 13.0,
        }),
        Box::new(ComplianceError::HoepaPointsAndFeesExceeded {
            fee_dollars: 15_000.0,
            threshold_dollars: 12_000.0,
        }),
        Box::new(ComplianceError::QmSafeHarborFailed {
            reason: "r".to_string(),
        }),
        Box::new(ComplianceError::AtrFailed {
            reason: "r".to_string(),
        }),
        Box::new(ComplianceError::StateLicensingRequirementNotMet {
            state: "NY".to_string(),
            requirement: "r".to_string(),
        }),
        Box::new(ComplianceError::FloodZoneRequirementNotMet {
            fips: "06037".to_string(),
            requirement: "r".to_string(),
        }),
    ];
    assert_eq!(
        variants.len(),
        6,
        "test must cover all 6 ComplianceError variants"
    );
    for v in &variants {
        assert!(
            !v.to_string().is_empty(),
            "Display must not be empty: {v:?}"
        );
    }
}
