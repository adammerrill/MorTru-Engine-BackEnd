//! `CbsaCode` — Core-Based Statistical Area (CBSA) code.
//!
//! A 5-digit numeric string assigned by the U.S. Office of Management and
//! Budget (OMB) that identifies a Metropolitan or Micropolitan Statistical
//! Area. CBSA codes drive:
//!
//! - Conforming loan limits (Fannie/Freddie set limits by CBSA)
//! - FHA and VA loan limits (set by county, resolved from CBSA)
//! - AMI income limits for HomeReady/HomePossible eligibility
//!
//! # Examples
//! - `"10180"` — Abilene, TX Metropolitan Statistical Area
//! - `"19100"` — Dallas-Fort Worth-Arlington, TX MSA
//! - `"41700"` — San Antonio-New Braunfels, TX MSA
//!
//! Properties outside any CBSA are assigned `"00000"` (non-metropolitan).

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::ParseError;

/// 5-digit CBSA code.
///
/// Validated at construction: must be exactly 5 ASCII digits.
/// The sentinel value `"00000"` represents a non-metropolitan area.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CbsaCode(String);

impl CbsaCode {
    /// Sentinel value for properties outside any CBSA (non-metropolitan).
    pub const NON_METRO: &'static str = "00000";

    /// Construct from a string. Must be exactly 5 ASCII digits.
    ///
    /// Leading zeros are significant: `"08060"` and `"8060"` are different
    /// values; only `"08060"` is valid.
    pub fn new(s: impl Into<String>) -> Result<Self, ParseError> {
        let s = s.into();
        let trimmed = s.trim();
        if trimmed.len() != 5 || !trimmed.chars().all(|c| c.is_ascii_digit()) {
            return Err(ParseError::IdentifierInvalidChars {
                kind: "CbsaCode",
                value: s,
            });
        }
        Ok(CbsaCode(trimmed.to_string()))
    }

    /// The raw 5-digit string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// True when this CBSA represents a non-metropolitan area.
    pub fn is_non_metro(&self) -> bool {
        self.0 == Self::NON_METRO
    }
}

impl FromStr for CbsaCode {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl fmt::Display for CbsaCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cbsa_code_valid_parses() {
        let code = CbsaCode::new("19100").unwrap();
        assert_eq!(code.as_str(), "19100");
    }

    #[test]
    fn test_cbsa_code_display() {
        let code = CbsaCode::new("41700").unwrap();
        assert_eq!(code.to_string(), "41700");
    }

    #[test]
    fn test_cbsa_code_from_str() {
        let code: CbsaCode = "10180".parse().unwrap();
        assert_eq!(code.as_str(), "10180");
    }

    #[test]
    fn test_cbsa_code_leading_zero_valid() {
        let code = CbsaCode::new("08060").unwrap();
        assert_eq!(code.as_str(), "08060");
    }

    #[test]
    fn test_cbsa_code_non_metro_sentinel() {
        let code = CbsaCode::new("00000").unwrap();
        assert!(code.is_non_metro());
    }

    #[test]
    fn test_cbsa_code_normal_is_not_non_metro() {
        let code = CbsaCode::new("19100").unwrap();
        assert!(!code.is_non_metro());
    }

    #[test]
    fn test_cbsa_code_too_short_rejected() {
        assert!(CbsaCode::new("1234").is_err());
    }

    #[test]
    fn test_cbsa_code_too_long_rejected() {
        assert!(CbsaCode::new("123456").is_err());
    }

    #[test]
    fn test_cbsa_code_non_numeric_rejected() {
        assert!(CbsaCode::new("1234A").is_err());
    }

    #[test]
    fn test_cbsa_code_whitespace_trimmed() {
        let code = CbsaCode::new("  19100  ").unwrap();
        assert_eq!(code.as_str(), "19100");
    }

    #[test]
    fn test_cbsa_code_serde_json() {
        let code = CbsaCode::new("19100").unwrap();
        let json = serde_json::to_string(&code).unwrap();
        assert_eq!(json, "\"19100\"");
        let back: CbsaCode = serde_json::from_str(&json).unwrap();
        assert_eq!(back, code);
    }

    #[test]
    fn test_cbsa_code_ordering() {
        let a = CbsaCode::new("10180").unwrap();
        let b = CbsaCode::new("19100").unwrap();
        assert!(a < b);
    }
}
