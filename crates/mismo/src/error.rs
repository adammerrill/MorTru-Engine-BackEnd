/// All errors that can arise when parsing or serializing MISMO 3.4 documents.
///
/// The three structural variants (`MissingElement`, `InvalidEnum`, `OutOfRange`)
/// are constructed manually in later tasks (2.3–2.10) when individual schema
/// fields are parsed into typed domain values. `Parse` and `Serialize` wrap the
/// underlying quick-xml errors directly via `#[from]`.
#[derive(Debug, thiserror::Error)]
pub enum MismoError {
    /// quick-xml deserialization failure — malformed XML, unexpected element
    /// structure, or a type mismatch between the XML and the target Rust type.
    ///
    /// Also used for serialization failures because quick-xml 0.36 uses
    /// `DeError` for both directions.
    #[error("XML error: {0}")]
    Parse(#[from] quick_xml::DeError),

    /// A required MISMO element was absent from the document.
    ///
    /// # Example
    /// ```ignore
    /// return Err(MismoError::MissingElement { element: "MORTGAGE_TERMS" });
    /// ```
    #[error("missing required MISMO element: {element}")]
    MissingElement { element: &'static str },

    /// A MISMO string enumeration value was not recognised by the engine.
    ///
    /// # Example
    /// ```ignore
    /// return Err(MismoError::InvalidEnum {
    ///     element: "MortgageType",
    ///     value: raw.to_string(),
    /// });
    /// ```
    #[error("invalid MISMO enum value '{value}' for element <{element}>")]
    InvalidEnum {
        /// The MISMO element name that contained the unrecognised value.
        element: &'static str,
        /// The raw string value that could not be mapped.
        value: String,
    },

    /// A numeric value parsed from MISMO XML fell outside the acceptable range
    /// for the target domain type (e.g. a credit score below 300).
    ///
    /// # Example
    /// ```ignore
    /// return Err(MismoError::OutOfRange {
    ///     element: "CreditScoreValue",
    ///     detail: "280 is below the minimum of 300".to_string(),
    /// });
    /// ```
    #[error("value out of range for element <{element}>: {detail}")]
    OutOfRange {
        /// The MISMO element name whose value was out of range.
        element: &'static str,
        /// Human-readable description of the violation.
        detail: String,
    },
}

/// Convenience alias used throughout the `mismo` crate.
///
/// Functions that parse or validate MISMO data return `mismo::Result<T>`
/// rather than spelling out the full `Result<T, MismoError>` each time.
pub type Result<T> = std::result::Result<T, MismoError>;
