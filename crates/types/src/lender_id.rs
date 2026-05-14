//! `LenderId` — internal short-string identifier for a lender.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

use crate::error::ParseError;

/// Identifier for a wholesale lender. Short strings like `"UWM"`, `"ROCKET"`,
/// `"PENN"`. Storage is `SmolStr` so identifiers up to 23 ASCII bytes live
/// inline (no heap allocation) and longer values use a reference-counted
/// representation that clones cheaply.
///
/// # Validation
///
/// - Non-empty after trimming whitespace
/// - At most 32 characters
/// - Only ASCII letters (A–Z, a–z), digits, hyphen, and underscore
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct LenderId(SmolStr);

impl LenderId {
    /// Maximum allowed length.
    pub const MAX_LEN: usize = 32;

    /// Validating constructor. Returns `Err` if the input is empty, too long,
    /// or contains disallowed characters.
    pub fn new(s: impl AsRef<str>) -> Result<Self, ParseError> {
        let raw = s.as_ref();
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(ParseError::IdentifierEmpty { kind: "LenderId" });
        }
        if trimmed.len() > Self::MAX_LEN {
            return Err(ParseError::IdentifierTooLong {
                kind: "LenderId",
                actual: trimmed.len(),
                max: Self::MAX_LEN,
            });
        }
        if !trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(ParseError::IdentifierInvalidChars {
                kind: "LenderId",
                value: raw.to_string(),
            });
        }
        Ok(LenderId(SmolStr::new(trimmed)))
    }

    /// Borrow the underlying string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl FromStr for LenderId {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl fmt::Display for LenderId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.as_str())
    }
}

impl AsRef<str> for LenderId {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lender_id_valid() {
        assert_eq!(LenderId::new("UWM").unwrap().as_str(), "UWM");
        assert_eq!(LenderId::new("ROCKET").unwrap().as_str(), "ROCKET");
        assert_eq!(LenderId::new("WELLS_FARGO").unwrap().as_str(), "WELLS_FARGO");
        assert_eq!(LenderId::new("LENDER-123").unwrap().as_str(), "LENDER-123");
        assert_eq!(LenderId::new("a").unwrap().as_str(), "a");
    }

    #[test]
    fn test_lender_id_whitespace_trimmed() {
        assert_eq!(LenderId::new("  UWM  ").unwrap().as_str(), "UWM");
        assert_eq!(LenderId::new("\tUWM\n").unwrap().as_str(), "UWM");
    }

    #[test]
    fn test_lender_id_empty_rejected() {
        assert!(matches!(
            LenderId::new(""),
            Err(ParseError::IdentifierEmpty { kind: "LenderId" })
        ));
        assert!(matches!(
            LenderId::new("   "),
            Err(ParseError::IdentifierEmpty { kind: "LenderId" })
        ));
        assert!(matches!(
            LenderId::new("\t\n"),
            Err(ParseError::IdentifierEmpty { kind: "LenderId" })
        ));
    }

    #[test]
    fn test_lender_id_too_long_rejected() {
        let too_long = "A".repeat(33);
        match LenderId::new(too_long) {
            Err(ParseError::IdentifierTooLong { kind, actual, max }) => {
                assert_eq!(kind, "LenderId");
                assert_eq!(actual, 33);
                assert_eq!(max, 32);
            }
            other => panic!("expected IdentifierTooLong, got {other:?}"),
        }

        // Exactly at the limit is OK
        let at_limit = "A".repeat(32);
        assert!(LenderId::new(at_limit).is_ok());

        // 1 over the limit fails
        assert!(LenderId::new("A".repeat(33)).is_err());
    }

    #[test]
    fn test_lender_id_invalid_chars_rejected() {
        assert!(LenderId::new("UWM!").is_err());
        assert!(LenderId::new("UWM 1").is_err()); // internal space
        assert!(LenderId::new("UWM.X").is_err());
        assert!(LenderId::new("UWM/X").is_err());
        assert!(LenderId::new("café").is_err()); // non-ASCII
    }

    #[test]
    fn test_lender_id_display() {
        let id = LenderId::new("ROCKET").unwrap();
        assert_eq!(id.to_string(), "ROCKET");
    }

    #[test]
    fn test_lender_id_serde_json() {
        let id = LenderId::new("UWM").unwrap();
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"UWM\"");

        let back: LenderId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, id);
    }

    #[test]
    fn test_lender_id_from_str() {
        let id: LenderId = "UWM".parse().unwrap();
        assert_eq!(id.as_str(), "UWM");

        assert!("UWM!".parse::<LenderId>().is_err());
    }

    #[test]
    fn test_lender_id_clone_is_cheap() {
        // Inline storage (under 23 bytes) — clone is a memcpy
        let id = LenderId::new("UWM").unwrap();
        let cloned = id.clone();
        assert_eq!(id, cloned);
    }

    #[test]
    fn test_lender_id_ordering() {
        let a = LenderId::new("AAA").unwrap();
        let b = LenderId::new("BBB").unwrap();
        let c = LenderId::new("CCC").unwrap();
        let mut v = vec![c.clone(), a.clone(), b.clone()];
        v.sort();
        assert_eq!(v, vec![a, b, c]);
    }
}
