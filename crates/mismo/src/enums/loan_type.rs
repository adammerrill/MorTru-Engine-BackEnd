//! Loan-type MISMO string enumerations.
//!
//! Converts the four core loan-type MISMO XML strings into their
//! corresponding `types` crate domain values.
//!
//! Types that already expose `from_mismo()` in the `types` crate
//! (`LoanPurpose`, `AmortizationType`, `ProgramCode`) are wrapped here
//! to adapt the `types::ParseError` into `crate::MismoError::InvalidEnum`.
//!
//! `LienPriority` does not have `from_mismo()` in the `types` crate, so
//! the inverse mapping is implemented here.

use types::{AmortizationType, LienPriority, LoanPurpose, ProgramCode};

// ── LoanPurpose ──────────────────────────────────────────────────────────────

/// Convert a MISMO 3.4 `LoanPurposeType` string to `types::LoanPurpose`.
///
/// Accepted values: `"Purchase"`, `"Refinance"`, `"LimitedCashOutRefinance"`,
/// `"CashOutRefinance"`, `"CashOut"`, `"Construction"`,
/// `"ConstructionToPermanent"`.
///
/// # Errors
/// Returns `MismoError::InvalidEnum` for any unrecognised value.
pub fn try_loan_purpose(s: &str) -> crate::Result<LoanPurpose> {
    LoanPurpose::from_mismo(s).map_err(|_| crate::MismoError::InvalidEnum {
        element: "LoanPurposeType",
        value: s.to_owned(),
    })
}

// ── AmortizationType ─────────────────────────────────────────────────────────

/// Convert a MISMO 3.4 `AmortizationType` string to `types::AmortizationType`.
///
/// Accepted values: `"Fixed"`, `"AdjustableRate"`, `"ARM"`,
/// `"InterestOnly"`, `"NegativeAmortization"`, `"GraduatedPayment"`,
/// `"PaymentOption"`.
///
/// # Errors
/// Returns `MismoError::InvalidEnum` for any unrecognised value.
pub fn try_amortization_type(s: &str) -> crate::Result<AmortizationType> {
    AmortizationType::from_mismo(s).map_err(|_| crate::MismoError::InvalidEnum {
        element: "AmortizationType",
        value: s.to_owned(),
    })
}

// ── LienPriority ─────────────────────────────────────────────────────────────

/// Convert a MISMO 3.4 `LienPriorityType` string to `types::LienPriority`.
///
/// Accepted values: `"FirstLien"`, `"SecondLien"`, `"ThirdLien"`.
///
/// This is the inverse of `LienPriority::to_mismo()`.
///
/// # Errors
/// Returns `MismoError::InvalidEnum` for any unrecognised value.
pub fn try_lien_priority(s: &str) -> crate::Result<LienPriority> {
    match s.trim() {
        "FirstLien" => Ok(LienPriority::First),
        "SecondLien" => Ok(LienPriority::Second),
        "ThirdLien" => Ok(LienPriority::Third),
        _ => Err(crate::MismoError::InvalidEnum {
            element: "LienPriorityType",
            value: s.to_owned(),
        }),
    }
}

// ── ProgramCode ──────────────────────────────────────────────────────────────

/// Convert a MISMO 3.4 `MortgageType` string to `types::ProgramCode`.
///
/// Accepted values: `"Conventional"`, `"FHA"`, `"VA"`,
/// `"USDARuralDevelopment"`, `"USDA"`.
///
/// Returns the most-general program code for each MISMO type. The caller
/// is responsible for refining `Conventional` to `HomeReady`, `HomePossible`,
/// etc. via eligibility logic.
///
/// # Errors
/// Returns `MismoError::InvalidEnum` for any unrecognised value.
pub fn try_program_code(s: &str) -> crate::Result<ProgramCode> {
    ProgramCode::from_mismo_mortgage_type(s).map_err(|_| crate::MismoError::InvalidEnum {
        element: "MortgageType",
        value: s.to_owned(),
    })
}

// Callers that need to serialize LienPriority call lien.to_mismo() directly.
