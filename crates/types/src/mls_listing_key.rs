//! `MlsListingKey` — RESO Data Dictionary 2.0 listing identifier.
//!
//! Per the RESO 2.0 schema, `ListingKey` is a variable-length string up to
//! 128 characters that uniquely identifies a listing within a single MLS.
//! Format varies by MLS (some use numeric IDs, others alphanumeric with
//! hyphens or underscores). We accept a permissive character set since the
//! identifier comes directly from upstream MLS systems and reformatting it
//! risks breaking lookups.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

use crate::error::ParseError;

/// RESO 2.0 ListingKey. Variable-length, up to 128 characters.
///
/// # Validation
///
/// - Non-empty after trimming whitespace
/// - At most 128 characters (per RESO 2.0 spec)
/// - Only printable ASCII (chars 0x20..=0x7E)
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct MlsListingKey(SmolStr);

impl MlsListingKey {
    /// Maximum allowed length per the RESO 2.0 specification.
    pub const MAX_LEN: usize = 128;

    /// Validating constructor.
    pub fn new(s: impl AsRef<str>) -> Result<Self, ParseError> {
        let raw = s.as_ref();
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(ParseError::IdentifierEmpty {
                kind: "MlsListingKey",
            });
        }
        if trimmed.len() > Self::MAX_LEN {
            return Err(ParseError::IdentifierTooLong {
                kind: "MlsListingKey",
                actual: trimmed.len(),
                max: Self::MAX_LEN,
            });
        }
        // Printable ASCII only (0x20..=0x7E). MLS keys can include letters,
        // digits, hyphens, underscores, dots, sometimes colons or slashes.
        if !trimmed.bytes().all(|b| (0x20..=0x7E).contains(&b)) {
            return Err(ParseError::IdentifierInvalidChars {
                kind: "MlsListingKey",
                value: raw.to_string(),
            });
        }
        Ok(MlsListingKey(SmolStr::new(trimmed)))
    }

    /// Borrow the underlying string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl FromStr for MlsListingKey {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl fmt::Display for MlsListingKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.as_str())
    }
}

impl AsRef<str> for MlsListingKey {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mls_listing_key_valid() {
        // Typical MLS key formats
        assert!(MlsListingKey::new("12345678").is_ok());
        assert!(MlsListingKey::new("MLS-123-ABC").is_ok());
        assert!(MlsListingKey::new("CRMLS_2024_0001234").is_ok());
        assert!(MlsListingKey::new("a.b.c").is_ok());
        assert!(MlsListingKey::new("listing:42").is_ok());

        let key = MlsListingKey::new("OC24123456").unwrap();
        assert_eq!(key.as_str(), "OC24123456");
    }

    #[test]
    fn test_mls_listing_key_empty_rejected() {
        assert!(matches!(
            MlsListingKey::new(""),
            Err(ParseError::IdentifierEmpty { kind: "MlsListingKey" })
        ));
        assert!(matches!(
            MlsListingKey::new("   "),
            Err(ParseError::IdentifierEmpty { kind: "MlsListingKey" })
        ));
    }

    #[test]
    fn test_mls_listing_key_too_long_rejected() {
        // Exactly at limit OK
        let at_limit = "A".repeat(128);
        assert!(MlsListingKey::new(at_limit).is_ok());

        // 1 over fails
        let too_long = "A".repeat(129);
        match MlsListingKey::new(too_long) {
            Err(ParseError::IdentifierTooLong { kind, actual, max }) => {
                assert_eq!(kind, "MlsListingKey");
                assert_eq!(actual, 129);
                assert_eq!(max, 128);
            }
            other => panic!("expected IdentifierTooLong, got {other:?}"),
        }
    }

    #[test]
    fn test_mls_listing_key_non_ascii_rejected() {
        assert!(MlsListingKey::new("listing-café").is_err());
        assert!(MlsListingKey::new("k\u{00e9}y").is_err());

        // Control characters (below 0x20)
        assert!(MlsListingKey::new("key\x01").is_err());
        assert!(MlsListingKey::new("key\nkey").is_err()); // newline = 0x0A
    }

    #[test]
    fn test_mls_listing_key_whitespace_trimmed() {
        let key = MlsListingKey::new("  ABC123  ").unwrap();
        assert_eq!(key.as_str(), "ABC123");
    }

    #[test]
    fn test_mls_listing_key_display() {
        let key = MlsListingKey::new("OC24123456").unwrap();
        assert_eq!(key.to_string(), "OC24123456");
    }

    #[test]
    fn test_mls_listing_key_serde_json() {
        let key = MlsListingKey::new("OC24123456").unwrap();
        let json = serde_json::to_string(&key).unwrap();
        assert_eq!(json, "\"OC24123456\"");

        let back: MlsListingKey = serde_json::from_str(&json).unwrap();
        assert_eq!(back, key);
    }

    #[test]
    fn test_mls_listing_key_from_str() {
        let key: MlsListingKey = "OC24123456".parse().unwrap();
        assert_eq!(key.as_str(), "OC24123456");
    }

    #[test]
    fn test_mls_listing_key_ordering() {
        let a = MlsListingKey::new("AAA").unwrap();
        let b = MlsListingKey::new("BBB").unwrap();
        let c = MlsListingKey::new("CCC").unwrap();
        let mut v = vec![c.clone(), a.clone(), b.clone()];
        v.sort();
        assert_eq!(v, vec![a, b, c]);
    }
}
