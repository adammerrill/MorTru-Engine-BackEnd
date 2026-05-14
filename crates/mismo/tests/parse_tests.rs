//! Task 2.1 gate tests — crate scaffolding and XML toolchain.
//!
//! These tests verify:
//! - `MismoError` and `Result` are correctly defined and accessible
//! - All error variants construct, display, and satisfy std::error::Error
//! - `MismoError` is Send + Sync (required for use in async contexts)
//! - `xml::parse::from_xml` returns the correct error type on bad input
//! - `xml::parse::from_xml` successfully parses valid XML into a typed struct
//! - `xml::serialize::to_xml` produces valid XML from a typed struct
//! - Round-trip (parse → serialize → parse) preserves values
//! - All module paths are accessible

use mismo::{MismoError, Result};

// ── Compile-time trait assertions ────────────────────────────────────────────

#[test]
fn test_mismo_error_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<MismoError>();
}

#[test]
fn test_mismo_error_implements_std_error() {
    fn assert_std_error<T: std::error::Error>() {}
    assert_std_error::<MismoError>();
}

#[test]
fn test_mismo_error_implements_debug() {
    fn assert_debug<T: std::fmt::Debug>() {}
    assert_debug::<MismoError>();
}

#[test]
fn test_mismo_error_implements_display() {
    fn assert_display<T: std::fmt::Display>() {}
    assert_display::<MismoError>();
}

#[test]
fn test_result_type_alias_is_correct() {
    // Verify Ok variant carries the value through.
    let ok: Result<u32> = Ok(42);
    assert!(ok.is_ok());
    if let Ok(v) = ok {
        assert_eq!(v, 42);
    }

    let err: Result<u32> = Err(MismoError::MissingElement { element: "TEST" });
    assert!(err.is_err());
}

// ── Error variant construction and display ────────────────────────────────────

#[test]
fn test_missing_element_display_contains_element_name() {
    let e = MismoError::MissingElement {
        element: "MORTGAGE_TERMS",
    };
    let msg = e.to_string();
    assert!(
        msg.contains("MORTGAGE_TERMS"),
        "display should contain element name, got: {msg}"
    );
}

#[test]
fn test_invalid_enum_display_contains_element_and_value() {
    let e = MismoError::InvalidEnum {
        element: "MortgageType",
        value: "NotARealMortgageType".to_string(),
    };
    let msg = e.to_string();
    assert!(
        msg.contains("MortgageType"),
        "display should contain element name, got: {msg}"
    );
    assert!(
        msg.contains("NotARealMortgageType"),
        "display should contain the bad value, got: {msg}"
    );
}

#[test]
fn test_out_of_range_display_contains_element_and_detail() {
    let e = MismoError::OutOfRange {
        element: "CreditScoreValue",
        detail: "280 is below the minimum of 300".to_string(),
    };
    let msg = e.to_string();
    assert!(
        msg.contains("CreditScoreValue"),
        "display should contain element name, got: {msg}"
    );
    assert!(
        msg.contains("280"),
        "display should contain the detail, got: {msg}"
    );
}

#[test]
fn test_parse_error_wraps_quick_xml_de_error() {
    #[derive(Debug, serde::Deserialize)]
    #[serde(rename = "X")]
    struct Dummy {
        #[serde(rename = "V")]
        _v: String,
    }

    let result = mismo::xml::parse::from_xml::<Dummy>("");
    assert!(result.is_err());
    assert!(
        matches!(result.unwrap_err(), MismoError::Parse(_)),
        "empty input should produce MismoError::Parse"
    );
}

#[test]
fn test_parse_error_from_invalid_xml() {
    #[derive(Debug, serde::Deserialize)]
    #[serde(rename = "X")]
    struct Dummy {
        #[serde(rename = "V")]
        _v: String,
    }

    let err = mismo::xml::parse::from_xml::<Dummy>(">>> clearly not xml <<<").unwrap_err();
    assert!(
        matches!(err, MismoError::Parse(_)),
        "invalid XML should produce MismoError::Parse"
    );
}

#[test]
fn test_parse_error_source_chain_is_accessible() {
    #[derive(Debug, serde::Deserialize)]
    #[serde(rename = "X")]
    struct Dummy {
        #[serde(rename = "V")]
        _v: String,
    }

    let err = mismo::xml::parse::from_xml::<Dummy>("").unwrap_err();
    use std::error::Error;
    assert!(
        err.source().is_some(),
        "MismoError::Parse should expose its source via std::error::Error::source"
    );
}

// ── XML parse success path ────────────────────────────────────────────────────

#[test]
fn test_from_xml_parses_simple_struct() {
    #[derive(Debug, PartialEq, serde::Deserialize)]
    #[serde(rename = "LOAN")]
    struct SimpleLoan {
        #[serde(rename = "Amount")]
        amount: String,
        #[serde(rename = "Rate")]
        rate: String,
    }

    let xml = "<LOAN><Amount>434443.00</Amount><Rate>6.375</Rate></LOAN>";
    let loan = mismo::xml::parse::from_xml::<SimpleLoan>(xml).unwrap();
    assert_eq!(loan.amount, "434443.00");
    assert_eq!(loan.rate, "6.375");
}

#[test]
fn test_from_xml_parses_nested_elements() {
    #[derive(Debug, serde::Deserialize)]
    #[serde(rename = "DEAL")]
    struct Deal {
        #[serde(rename = "LOAN")]
        loan: Loan,
    }

    #[derive(Debug, serde::Deserialize)]
    struct Loan {
        #[serde(rename = "Amount")]
        amount: String,
    }

    let xml = "<DEAL><LOAN><Amount>434443.00</Amount></LOAN></DEAL>";
    let deal = mismo::xml::parse::from_xml::<Deal>(xml).unwrap();
    assert_eq!(deal.loan.amount, "434443.00");
}

#[test]
fn test_from_xml_handles_optional_absent_field() {
    #[derive(Debug, serde::Deserialize)]
    #[serde(rename = "LOAN")]
    struct Loan {
        #[serde(rename = "Amount")]
        amount: String,
        #[serde(rename = "Notes")]
        notes: Option<String>,
    }

    let xml = "<LOAN><Amount>434443.00</Amount></LOAN>";
    let loan = mismo::xml::parse::from_xml::<Loan>(xml).unwrap();
    assert_eq!(loan.amount, "434443.00");
    assert!(loan.notes.is_none());
}

// ── XML serialize path ────────────────────────────────────────────────────────

#[test]
fn test_to_xml_produces_valid_xml_string() {
    #[derive(serde::Serialize)]
    #[serde(rename = "LOAN")]
    struct Loan {
        #[serde(rename = "Amount")]
        amount: String,
        #[serde(rename = "Rate")]
        rate: String,
    }

    let loan = Loan {
        amount: "434443.00".to_string(),
        rate: "6.375".to_string(),
    };

    let xml = mismo::xml::serialize::to_xml(&loan).unwrap();
    assert!(
        xml.contains("434443.00"),
        "serialized XML should contain amount"
    );
    assert!(xml.contains("6.375"), "serialized XML should contain rate");
    assert!(
        xml.contains("LOAN"),
        "serialized XML should have root element"
    );
    assert!(
        xml.contains("Amount"),
        "serialized XML should have Amount element"
    );
}

#[test]
fn test_to_xml_omits_none_optional_fields() {
    #[derive(serde::Serialize)]
    #[serde(rename = "LOAN")]
    struct Loan {
        #[serde(rename = "Amount")]
        amount: String,
        #[serde(rename = "Notes", skip_serializing_if = "Option::is_none")]
        notes: Option<String>,
    }

    let loan = Loan {
        amount: "434443.00".to_string(),
        notes: None,
    };
    let xml = mismo::xml::serialize::to_xml(&loan).unwrap();
    assert!(
        !xml.contains("Notes"),
        "None field with skip_serializing_if should be absent"
    );
}

// ── Round-trip ────────────────────────────────────────────────────────────────

#[test]
fn test_parse_serialize_parse_roundtrip() {
    #[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
    #[serde(rename = "MORTGAGE_TERMS")]
    struct MortgageTerms {
        #[serde(rename = "BaseLoanAmount")]
        base_loan_amount: String,
        #[serde(rename = "NoteRatePercent")]
        note_rate_percent: String,
        #[serde(rename = "LoanTermMonthsCount")]
        loan_term_months: String,
    }

    let original = MortgageTerms {
        base_loan_amount: "434443.00".to_string(),
        note_rate_percent: "6.375".to_string(),
        loan_term_months: "360".to_string(),
    };

    let xml = mismo::xml::serialize::to_xml(&original).unwrap();
    let parsed = mismo::xml::parse::from_xml::<MortgageTerms>(&xml).unwrap();
    assert_eq!(
        parsed, original,
        "round-trip should preserve all field values"
    );
}

// ── Module accessibility ──────────────────────────────────────────────────────

#[test]
fn test_xml_module_paths_are_accessible() {
    // Verify the module hierarchy compiles by naming concrete monomorphizations.
    // Using `assert` to prevent dead-code elimination rather than just let bindings.
    #[derive(Debug, serde::Deserialize, serde::Serialize)]
    #[serde(rename = "X")]
    struct Probe {
        #[serde(rename = "V")]
        v: String,
    }

    // Both directions should work end-to-end.
    let xml = mismo::xml::serialize::to_xml(&Probe { v: "probe".into() }).unwrap();
    let parsed = mismo::xml::parse::from_xml::<Probe>(&xml).unwrap();
    assert_eq!(parsed.v, "probe");
}

#[test]
fn test_enums_module_is_accessible() {
    // The enums module path resolves (Tasks 2.2+ populate it).
    // Referencing the module in a use item is sufficient to prove accessibility.
    #[allow(unused_imports)]
    use mismo::enums;
}

#[test]
fn test_schema_module_is_accessible() {
    // The schema module path resolves (Tasks 2.3+ populate it).
    #[allow(unused_imports)]
    use mismo::schema;
}

#[test]
fn test_mismo_error_and_result_are_crate_root_exports() {
    // MismoError and Result are pub-use re-exported at the crate root.
    // If this compiles, the `pub use` in lib.rs is correct.
    let _: fn() -> Result<u8> = || Ok(0);
    let _e: MismoError = MismoError::MissingElement { element: "X" };
}
