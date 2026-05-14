//! `LoanCasefileId` — DU / LPA loan casefile identifier.
//!
//! Identifies a specific loan submission in Fannie Mae's Desktop Underwriter
//! (DU) or Freddie Mac's Loan Product Advisor (LPA). DU returns a 10-digit
//! casefile ID; LPA returns a longer alphanumeric AUS key. The type accepts
//! either format.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

use crate::error::ParseError;

/// AUS loan casefile identifier. Storage is `SmolStr` for cheap clones.
///
/// # Validation
///
/// - Non-empty after trimming whitespace
/// - At most 64 characters
/// - Only ASCII alphanumerics, hyphen, underscore, or period
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct LoanCasefileId(SmolStr);

impl LoanCasefileId {
    /// Maximum allowed length.
    pub const MAX_LEN: usize = 64;

    /// Validating constructor.
    pub fn new(s: impl AsRef<str>) -> Result<Self, ParseError> {
        let raw = s.as_ref();
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(ParseError::IdentifierEmpty {
                kind: "LoanCasefileId",
            });
        }
        if trimmed.len() > Self::MAX_LEN {
            return Err(ParseError::IdentifierTooLong {
                kind: "LoanCasefileId",
                actual: trimmed.len(),
                max: Self::MAX_LEN,
            });
        }
        if !trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
        {
            return Err(ParseError::IdentifierInvalidChars {
                kind: "LoanCasefileId",
                value: raw.to_string(),
            });
        }
        Ok(LoanCasefileId(SmolStr::new(trimmed)))
    }

    /// Borrow the underlying string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl FromStr for LoanCasefileId {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl fmt::Display for LoanCasefileId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.as_str())
    }
}

impl AsRef<str> for LoanCasefileId {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loan_casefile_id_valid() {
        // DU format: 10 digits
        assert!(LoanCasefileId::new("1234567890").is_ok());

        // LPA format: longer alphanumeric
        assert!(LoanCasefileId::new("LPA-2024-001234567890").is_ok());
        assert!(LoanCasefileId::new("AUS_KEY.123.ABC").is_ok());

        let id = LoanCasefileId::new("1234567890").unwrap();
        assert_eq!(id.as_str(), "1234567890");
    }

    #[test]
    fn test_loan_casefile_id_empty_rejected() {
        assert!(matches!(
            LoanCasefileId::new(""),
            Err(ParseError::IdentifierEmpty {
                kind: "LoanCasefileId"
            })
        ));
        assert!(matches!(
            LoanCasefileId::new("   "),
            Err(ParseError::IdentifierEmpty {
                kind: "LoanCasefileId"
            })
        ));
    }

    #[test]
    fn test_loan_casefile_id_too_long_rejected() {
        let at_limit = "A".repeat(64);
        assert!(LoanCasefileId::new(at_limit).is_ok());

        let too_long = "A".repeat(65);
        match LoanCasefileId::new(too_long) {
            Err(ParseError::IdentifierTooLong { kind, actual, max }) => {
                assert_eq!(kind, "LoanCasefileId");
                assert_eq!(actual, 65);
                assert_eq!(max, 64);
            }
            other => panic!("expected IdentifierTooLong, got {other:?}"),
        }
    }

    #[test]
    fn test_loan_casefile_id_invalid_chars_rejected() {
        assert!(LoanCasefileId::new("ID!").is_err());
        assert!(LoanCasefileId::new("ID with space").is_err());
        assert!(LoanCasefileId::new("ID/slash").is_err());
        assert!(LoanCasefileId::new("café").is_err());
    }

    #[test]
    fn test_loan_casefile_id_whitespace_trimmed() {
        let id = LoanCasefileId::new("  1234567890  ").unwrap();
        assert_eq!(id.as_str(), "1234567890");
    }

    #[test]
    fn test_loan_casefile_id_display() {
        let id = LoanCasefileId::new("1234567890").unwrap();
        assert_eq!(id.to_string(), "1234567890");
    }

    #[test]
    fn test_loan_casefile_id_serde_json() {
        let id = LoanCasefileId::new("1234567890").unwrap();
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"1234567890\"");
        let back: LoanCasefileId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, id);
    }

    #[test]
    fn test_loan_casefile_id_from_str() {
        let id: LoanCasefileId = "1234567890".parse().unwrap();
        assert_eq!(id.as_str(), "1234567890");
    }
}
