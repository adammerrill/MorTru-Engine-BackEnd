//! `FipsCode` — 5-digit county FIPS code.
//!
//! Format `SSCCC`: first two digits are the state FIPS code, last three are
//! the county FIPS code within that state. Example: `"06037"` is Los Angeles
//! County (state 06 = California, county 037 = Los Angeles).
//!
//! The constructor validates that the state portion corresponds to a real US
//! state or territory by routing through [`crate::StateCode::from_fips`].
//! The county portion is bounded to `0..=999` (3 digits) but otherwise not
//! checked against a per-state county list — that finer validation belongs in
//! the reference-data ingestion (Epic 6) where the authoritative county list
//! is loaded.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::ParseError;
use crate::state_code::StateCode;

/// 5-digit county FIPS code, stored as a `u32` for compact serialization
/// and cheap comparison. The internal value is always in `1_000..=78_999`
/// because the smallest valid state FIPS is 1 (Alabama) and the largest is
/// 78 (US Virgin Islands), giving a 5-digit number with leading zero when
/// printed.
#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[repr(transparent)]
pub struct FipsCode(u32);

impl FipsCode {
    /// Construct from a state FIPS code and a 3-digit county code.
    ///
    /// Returns `Err(ParseError::InvalidFipsCode)` if:
    /// - `state_fips` does not correspond to a real US state or territory
    /// - `county_fips` exceeds 999
    pub fn new(state_fips: u8, county_fips: u16) -> Result<Self, ParseError> {
        if county_fips > 999 {
            return Err(ParseError::InvalidFipsCode(format!(
                "{state_fips:02}{county_fips}"
            )));
        }
        // Validate state portion against the StateCode allowlist
        StateCode::from_fips(state_fips).ok_or_else(|| {
            ParseError::InvalidFipsCode(format!("{state_fips:02}{county_fips:03}"))
        })?;
        Ok(FipsCode(
            u32::from(state_fips) * 1000 + u32::from(county_fips),
        ))
    }

    /// Return the 2-digit state FIPS portion (0..=99 in principle, in
    /// practice always a real US state code).
    #[must_use]
    pub const fn state_fips(self) -> u8 {
        // Inner value is at most 78_999, so dividing by 1000 yields at most 78,
        // which fits in u8.
        (self.0 / 1000) as u8
    }

    /// Return the 3-digit county FIPS portion (0..=999).
    #[must_use]
    pub const fn county_fips(self) -> u16 {
        (self.0 % 1000) as u16
    }

    /// Resolve the state portion to a [`StateCode`]. This never fails because
    /// construction validates the state portion against the StateCode
    /// allowlist.
    #[must_use]
    pub fn state_code(self) -> StateCode {
        StateCode::from_fips(self.state_fips()).expect("FIPS state validated at construction")
    }

    /// Underlying u32 representation. Mostly useful for compact serialization
    /// or hashing in tight loops.
    #[must_use]
    pub const fn as_u32(self) -> u32 {
        self.0
    }
}

impl FromStr for FipsCode {
    type Err = ParseError;

    /// Parse from a 5-character ASCII-digit string. Whitespace is trimmed.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let trimmed = s.trim();
        if trimmed.len() != 5 || !trimmed.chars().all(|c| c.is_ascii_digit()) {
            return Err(ParseError::InvalidFipsCode(s.to_string()));
        }
        // Safe: we just verified all 5 chars are ASCII digits, so parse won't fail
        let n: u32 = trimmed
            .parse()
            .map_err(|_| ParseError::InvalidFipsCode(s.to_string()))?;
        let state_fips =
            u8::try_from(n / 1000).map_err(|_| ParseError::InvalidFipsCode(s.to_string()))?;
        let county_fips = (n % 1000) as u16;
        Self::new(state_fips, county_fips)
    }
}

impl fmt::Display for FipsCode {
    /// Format as a 5-digit string with leading zero, e.g., `06037`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:05}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fips_code_from_valid_5digit_string() {
        let los_angeles: FipsCode = "06037".parse().unwrap();
        assert_eq!(los_angeles.state_fips(), 6);
        assert_eq!(los_angeles.county_fips(), 37);

        let new_york_county: FipsCode = "36061".parse().unwrap();
        assert_eq!(new_york_county.state_fips(), 36);
        assert_eq!(new_york_county.county_fips(), 61);

        // Smallest valid: Alabama county 001
        let small: FipsCode = "01001".parse().unwrap();
        assert_eq!(small.state_fips(), 1);
        assert_eq!(small.county_fips(), 1);
    }

    #[test]
    fn test_fips_code_invalid_length_rejected() {
        // Too short
        assert!("0603".parse::<FipsCode>().is_err());
        assert!("".parse::<FipsCode>().is_err());

        // Too long
        assert!("060370".parse::<FipsCode>().is_err());
        assert!("12345678".parse::<FipsCode>().is_err());
    }

    #[test]
    fn test_fips_code_invalid_characters_rejected() {
        assert!("06A37".parse::<FipsCode>().is_err());
        assert!("AAAAA".parse::<FipsCode>().is_err());
        assert!("06-37".parse::<FipsCode>().is_err());
        assert!("06 37".parse::<FipsCode>().is_err());
    }

    #[test]
    fn test_fips_code_rejects_unassigned_state() {
        // State FIPS 03 is unassigned (was American Samoa, now uses 60)
        assert!("03001".parse::<FipsCode>().is_err());
        assert!("07001".parse::<FipsCode>().is_err()); // gap between CO(8) and CT(9)
        assert!("99001".parse::<FipsCode>().is_err()); // beyond all valid states

        // Error variant should give the offending string back
        match "99001".parse::<FipsCode>() {
            Err(ParseError::InvalidFipsCode(s)) => assert_eq!(s, "99001"),
            other => panic!("expected InvalidFipsCode, got {other:?}"),
        }
    }

    #[test]
    fn test_fips_code_new_validates_state() {
        // Valid
        assert!(FipsCode::new(6, 37).is_ok());
        assert!(FipsCode::new(1, 1).is_ok());
        assert!(FipsCode::new(78, 1).is_ok());

        // Invalid state
        assert!(FipsCode::new(0, 1).is_err());
        assert!(FipsCode::new(3, 1).is_err());
        assert!(FipsCode::new(99, 1).is_err());

        // Invalid county (over 999)
        assert!(FipsCode::new(6, 1000).is_err());
        assert!(FipsCode::new(6, u16::MAX).is_err());
    }

    #[test]
    fn test_fips_code_state_and_county_components() {
        let code = FipsCode::new(48, 201).unwrap(); // Harris County, TX
        assert_eq!(code.state_fips(), 48);
        assert_eq!(code.county_fips(), 201);

        // County FIPS can be 0
        let code = FipsCode::new(6, 0).unwrap();
        assert_eq!(code.county_fips(), 0);

        // County FIPS up to 999
        let code = FipsCode::new(6, 999).unwrap();
        assert_eq!(code.county_fips(), 999);
    }

    #[test]
    fn test_fips_code_state_code_accessor() {
        let la = FipsCode::new(6, 37).unwrap();
        assert_eq!(la.state_code(), StateCode::CA);

        let harris = FipsCode::new(48, 201).unwrap();
        assert_eq!(harris.state_code(), StateCode::TX);

        let dc = FipsCode::new(11, 1).unwrap();
        assert_eq!(dc.state_code(), StateCode::DC);

        let pr = FipsCode::new(72, 1).unwrap();
        assert_eq!(pr.state_code(), StateCode::PR);
    }

    #[test]
    fn test_fips_code_display_pads_to_five_digits() {
        assert_eq!(FipsCode::new(6, 37).unwrap().to_string(), "06037");
        assert_eq!(FipsCode::new(1, 1).unwrap().to_string(), "01001");
        assert_eq!(FipsCode::new(48, 201).unwrap().to_string(), "48201");
        assert_eq!(FipsCode::new(78, 999).unwrap().to_string(), "78999");
    }

    #[test]
    fn test_fips_code_whitespace_trimmed() {
        let code: FipsCode = "  06037  ".parse().unwrap();
        assert_eq!(code.state_fips(), 6);
        assert_eq!(code.county_fips(), 37);
    }

    #[test]
    fn test_fips_code_serde_json() {
        let code = FipsCode::new(6, 37).unwrap();
        let json = serde_json::to_string(&code).unwrap();
        // Tuple struct serializes as its inner value
        assert_eq!(json, "6037");

        let back: FipsCode = serde_json::from_str(&json).unwrap();
        assert_eq!(back, code);
    }

    #[test]
    fn test_fips_code_as_u32() {
        let code = FipsCode::new(6, 37).unwrap();
        assert_eq!(code.as_u32(), 6037);
    }

    #[test]
    fn test_fips_code_ordering() {
        let codes = [
            FipsCode::new(6, 37).unwrap(),   // 06037
            FipsCode::new(1, 1).unwrap(),    // 01001
            FipsCode::new(48, 201).unwrap(), // 48201
            FipsCode::new(6, 1).unwrap(),    // 06001
        ];
        let mut sorted: Vec<_> = codes.to_vec();
        sorted.sort();
        // Lex-order by full 5-digit value
        assert_eq!(sorted[0], FipsCode::new(1, 1).unwrap());
        assert_eq!(sorted[1], FipsCode::new(6, 1).unwrap());
        assert_eq!(sorted[2], FipsCode::new(6, 37).unwrap());
        assert_eq!(sorted[3], FipsCode::new(48, 201).unwrap());
    }

    #[test]
    fn test_fips_code_repr_transparent() {
        assert_eq!(size_of::<FipsCode>(), size_of::<u32>());
    }

    #[test]
    fn test_fips_code_roundtrip_through_string() {
        let original = FipsCode::new(6, 37).unwrap();
        let s = original.to_string();
        let back: FipsCode = s.parse().unwrap();
        assert_eq!(original, back);
    }
}
