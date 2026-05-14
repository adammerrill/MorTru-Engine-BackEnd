//! Validation errors returned by the constructors and parsers of this crate.

use thiserror::Error;

/// Errors raised when a value cannot be constructed or parsed.
///
/// Every variant carries enough context for the caller to surface a helpful
/// error message without needing to look at internal state.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum ParseError {
    // ----- Task 1.2 errors -----
    /// LTV exceeded the plausible maximum of 110.00% (11000 basis points).
    /// VA loans can finance the funding fee into the loan and go slightly
    /// above 100% LTV; anything above 110% is almost certainly a data error.
    #[error("invalid LTV: {0} basis points exceeds maximum 11000 (110.00%)")]
    LtvOutOfRange(u32),

    /// Credit score was outside the valid FICO/VantageScore range of 300–850.
    #[error("invalid credit score: {0} not in range 300..=850")]
    CreditScoreOutOfRange(u16),

    /// Percentage string could not be parsed (non-numeric, multiple decimal
    /// points, negative when negatives are not allowed, etc.).
    #[error("invalid percentage string: `{0}`")]
    InvalidPercentageString(String),

    /// Money-format string could not be parsed.
    #[error("invalid money string: `{0}`")]
    InvalidMoneyString(String),

    /// Decimal value was too large or too small to fit in the target integer
    /// representation.
    #[error("decimal value out of range: `{0}`")]
    DecimalOutOfRange(String),

    /// LTV computation attempted with a zero or negative property value.
    /// Division by zero is undefined and any sub-zero value indicates the
    /// upstream property data is corrupt.
    #[error("property value must be positive (cannot compute LTV with zero or negative value)")]
    ZeroPropertyValue,

    // ----- Task 1.3 errors -----
    /// FIPS code could not be parsed. Must be exactly 5 ASCII digits and the
    /// state portion (first two digits) must correspond to a real US state or
    /// territory.
    #[error("invalid FIPS code: `{0}` (expected 5 digits with valid state prefix)")]
    InvalidFipsCode(String),

    /// State code could not be parsed. Must be a 2-letter abbreviation for
    /// one of the 50 US states, DC, or a US territory (AS, GU, MP, PR, VI).
    #[error("invalid state code: `{0}` (expected 2-letter US state or territory abbreviation)")]
    InvalidStateCode(String),

    /// An identifier required by a downstream system was empty after trimming
    /// whitespace. The `kind` field names which identifier this was for so
    /// the operator can locate the source of the bad data.
    #[error("identifier `{kind}` cannot be empty")]
    IdentifierEmpty { kind: &'static str },

    /// An identifier exceeded its maximum allowed length. The `actual` and
    /// `max` fields give the exact lengths so the operator can see how far
    /// over the limit the input ran.
    #[error("identifier `{kind}` is too long: {actual} chars (max {max})")]
    IdentifierTooLong {
        /// Name of the identifier kind (e.g., `"LenderId"`).
        kind: &'static str,
        /// Length of the offending input.
        actual: usize,
        /// Maximum permitted length.
        max: usize,
    },

    /// An identifier contained characters outside the allowed set for its
    /// kind. The `value` field gives back the offending input so the operator
    /// can see exactly what came in.
    #[error("identifier `{kind}` contains disallowed characters: `{value}`")]
    IdentifierInvalidChars {
        /// Name of the identifier kind (e.g., `"LenderId"`).
        kind: &'static str,
        /// The offending input.
        value: String,
    },

    // ----- Task 1.6 errors -----
    /// A term expressed in months was outside the valid engine range of
    /// 120..=360 (10-year minimum through 30-year maximum).
    #[error("term {0} months is outside the valid engine range 120..=360")]
    TermMonthsOutOfRange(u16),
}
