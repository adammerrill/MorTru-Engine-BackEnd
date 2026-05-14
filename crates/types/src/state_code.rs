//! `StateCode` — closed enumeration of the 50 US states, DC, and 5 US territories.
//!
//! The set of valid state codes is closed and stable, so an enum is the
//! correct representation: type-safe, no runtime validation, exhaustive `match`
//! ergonomics for downstream code that needs per-state logic (e.g., bond
//! programs, state-specific MI rates). The enum carries the standard 2-letter
//! postal abbreviation as its serde representation.
//!
//! Each variant has a corresponding FIPS state numeric code accessible via
//! [`StateCode::to_fips`]; the inverse mapping is [`StateCode::from_fips`].

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::ParseError;

/// US state or territory. The serde representation is the 2-letter postal
/// abbreviation (e.g., `"CA"`, `"NY"`, `"PR"`).
#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[allow(clippy::upper_case_acronyms)] // postal codes are intentionally uppercase
pub enum StateCode {
    AL, AK, AZ, AR, CA, CO, CT, DE, DC, FL, GA, HI, ID, IL, IN, IA,
    KS, KY, LA, ME, MD, MA, MI, MN, MS, MO, MT, NE, NV, NH, NJ, NM,
    NY, NC, ND, OH, OK, OR, PA, RI, SC, SD, TN, TX, UT, VT, VA, WA,
    WV, WI, WY,
    // US territories
    AS, GU, MP, PR, VI,
}

impl StateCode {
    /// All valid state codes in alphabetical order. Useful for iteration in
    /// reference-data ingestion and for property tests that need to enumerate
    /// every state.
    pub const ALL: &'static [Self] = &[
        Self::AL, Self::AK, Self::AZ, Self::AR, Self::CA, Self::CO, Self::CT,
        Self::DE, Self::DC, Self::FL, Self::GA, Self::HI, Self::ID, Self::IL,
        Self::IN, Self::IA, Self::KS, Self::KY, Self::LA, Self::ME, Self::MD,
        Self::MA, Self::MI, Self::MN, Self::MS, Self::MO, Self::MT, Self::NE,
        Self::NV, Self::NH, Self::NJ, Self::NM, Self::NY, Self::NC, Self::ND,
        Self::OH, Self::OK, Self::OR, Self::PA, Self::RI, Self::SC, Self::SD,
        Self::TN, Self::TX, Self::UT, Self::VT, Self::VA, Self::WA, Self::WV,
        Self::WI, Self::WY,
        Self::AS, Self::GU, Self::MP, Self::PR, Self::VI,
    ];

    /// 2-letter postal abbreviation. `'static` so it can be used in `const`
    /// contexts and returned without allocation.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AL => "AL", Self::AK => "AK", Self::AZ => "AZ", Self::AR => "AR",
            Self::CA => "CA", Self::CO => "CO", Self::CT => "CT", Self::DE => "DE",
            Self::DC => "DC", Self::FL => "FL", Self::GA => "GA", Self::HI => "HI",
            Self::ID => "ID", Self::IL => "IL", Self::IN => "IN", Self::IA => "IA",
            Self::KS => "KS", Self::KY => "KY", Self::LA => "LA", Self::ME => "ME",
            Self::MD => "MD", Self::MA => "MA", Self::MI => "MI", Self::MN => "MN",
            Self::MS => "MS", Self::MO => "MO", Self::MT => "MT", Self::NE => "NE",
            Self::NV => "NV", Self::NH => "NH", Self::NJ => "NJ", Self::NM => "NM",
            Self::NY => "NY", Self::NC => "NC", Self::ND => "ND", Self::OH => "OH",
            Self::OK => "OK", Self::OR => "OR", Self::PA => "PA", Self::RI => "RI",
            Self::SC => "SC", Self::SD => "SD", Self::TN => "TN", Self::TX => "TX",
            Self::UT => "UT", Self::VT => "VT", Self::VA => "VA", Self::WA => "WA",
            Self::WV => "WV", Self::WI => "WI", Self::WY => "WY",
            Self::AS => "AS", Self::GU => "GU", Self::MP => "MP", Self::PR => "PR",
            Self::VI => "VI",
        }
    }

    /// FIPS state numeric code. Used as the first two digits of a 5-digit
    /// county [`crate::FipsCode`]. Values follow the official FIPS State Code
    /// list (not sequential — there are gaps, e.g., FIPS 3, 7, 14 are unused).
    #[must_use]
    pub const fn to_fips(self) -> u8 {
        match self {
            Self::AL => 1,  Self::AK => 2,  Self::AZ => 4,  Self::AR => 5,
            Self::CA => 6,  Self::CO => 8,  Self::CT => 9,  Self::DE => 10,
            Self::DC => 11, Self::FL => 12, Self::GA => 13, Self::HI => 15,
            Self::ID => 16, Self::IL => 17, Self::IN => 18, Self::IA => 19,
            Self::KS => 20, Self::KY => 21, Self::LA => 22, Self::ME => 23,
            Self::MD => 24, Self::MA => 25, Self::MI => 26, Self::MN => 27,
            Self::MS => 28, Self::MO => 29, Self::MT => 30, Self::NE => 31,
            Self::NV => 32, Self::NH => 33, Self::NJ => 34, Self::NM => 35,
            Self::NY => 36, Self::NC => 37, Self::ND => 38, Self::OH => 39,
            Self::OK => 40, Self::OR => 41, Self::PA => 42, Self::RI => 44,
            Self::SC => 45, Self::SD => 46, Self::TN => 47, Self::TX => 48,
            Self::UT => 49, Self::VT => 50, Self::VA => 51, Self::WA => 53,
            Self::WV => 54, Self::WI => 55, Self::WY => 56,
            Self::AS => 60, Self::GU => 66, Self::MP => 69, Self::PR => 72,
            Self::VI => 78,
        }
    }

    /// Look up a state by its FIPS numeric code. Returns `None` for codes
    /// that are not assigned to any state or territory.
    #[must_use]
    pub const fn from_fips(fips: u8) -> Option<Self> {
        match fips {
            1  => Some(Self::AL),  2  => Some(Self::AK),  4  => Some(Self::AZ),
            5  => Some(Self::AR),  6  => Some(Self::CA),  8  => Some(Self::CO),
            9  => Some(Self::CT),  10 => Some(Self::DE),  11 => Some(Self::DC),
            12 => Some(Self::FL),  13 => Some(Self::GA),  15 => Some(Self::HI),
            16 => Some(Self::ID),  17 => Some(Self::IL),  18 => Some(Self::IN),
            19 => Some(Self::IA),  20 => Some(Self::KS),  21 => Some(Self::KY),
            22 => Some(Self::LA),  23 => Some(Self::ME),  24 => Some(Self::MD),
            25 => Some(Self::MA),  26 => Some(Self::MI),  27 => Some(Self::MN),
            28 => Some(Self::MS),  29 => Some(Self::MO),  30 => Some(Self::MT),
            31 => Some(Self::NE),  32 => Some(Self::NV),  33 => Some(Self::NH),
            34 => Some(Self::NJ),  35 => Some(Self::NM),  36 => Some(Self::NY),
            37 => Some(Self::NC),  38 => Some(Self::ND),  39 => Some(Self::OH),
            40 => Some(Self::OK),  41 => Some(Self::OR),  42 => Some(Self::PA),
            44 => Some(Self::RI),  45 => Some(Self::SC),  46 => Some(Self::SD),
            47 => Some(Self::TN),  48 => Some(Self::TX),  49 => Some(Self::UT),
            50 => Some(Self::VT),  51 => Some(Self::VA),  53 => Some(Self::WA),
            54 => Some(Self::WV),  55 => Some(Self::WI),  56 => Some(Self::WY),
            60 => Some(Self::AS),  66 => Some(Self::GU),  69 => Some(Self::MP),
            72 => Some(Self::PR),  78 => Some(Self::VI),
            _  => None,
        }
    }

    /// True if this is one of the 50 US states (excludes DC and territories).
    #[must_use]
    pub const fn is_state(self) -> bool {
        !matches!(self, Self::DC | Self::AS | Self::GU | Self::MP | Self::PR | Self::VI)
    }

    /// True if this is a US territory (AS, GU, MP, PR, VI).
    #[must_use]
    pub const fn is_territory(self) -> bool {
        matches!(self, Self::AS | Self::GU | Self::MP | Self::PR | Self::VI)
    }
}

impl FromStr for StateCode {
    type Err = ParseError;

    /// Parse a 2-letter state code, case-insensitive. Whitespace is trimmed.
    /// Returns `Err(ParseError::InvalidStateCode)` for any non-matching input.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let trimmed = s.trim();
        if trimmed.len() != 2 || !trimmed.is_ascii() {
            return Err(ParseError::InvalidStateCode(s.to_string()));
        }
        // Build an uppercase 2-char comparison key on the stack.
        let bytes = trimmed.as_bytes();
        let upper = [bytes[0].to_ascii_uppercase(), bytes[1].to_ascii_uppercase()];
        let upper_str = std::str::from_utf8(&upper)
            .map_err(|_| ParseError::InvalidStateCode(s.to_string()))?;
        for state in Self::ALL {
            if state.as_str() == upper_str {
                return Ok(*state);
            }
        }
        Err(ParseError::InvalidStateCode(s.to_string()))
    }
}

impl fmt::Display for StateCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_code_all_has_56_entries() {
        // 50 states + DC + 5 territories
        assert_eq!(StateCode::ALL.len(), 56);
    }

    #[test]
    fn test_state_code_from_str_valid_states() {
        assert_eq!(StateCode::from_str("CA").unwrap(), StateCode::CA);
        assert_eq!(StateCode::from_str("NY").unwrap(), StateCode::NY);
        assert_eq!(StateCode::from_str("TX").unwrap(), StateCode::TX);
        assert_eq!(StateCode::from_str("WY").unwrap(), StateCode::WY);
        assert_eq!(StateCode::from_str("AL").unwrap(), StateCode::AL);
    }

    #[test]
    fn test_state_code_from_str_valid_territories() {
        assert_eq!(StateCode::from_str("DC").unwrap(), StateCode::DC);
        assert_eq!(StateCode::from_str("PR").unwrap(), StateCode::PR);
        assert_eq!(StateCode::from_str("GU").unwrap(), StateCode::GU);
        assert_eq!(StateCode::from_str("AS").unwrap(), StateCode::AS);
        assert_eq!(StateCode::from_str("VI").unwrap(), StateCode::VI);
        assert_eq!(StateCode::from_str("MP").unwrap(), StateCode::MP);
    }

    #[test]
    fn test_state_code_from_str_case_insensitive() {
        assert_eq!(StateCode::from_str("ca").unwrap(), StateCode::CA);
        assert_eq!(StateCode::from_str("Ca").unwrap(), StateCode::CA);
        assert_eq!(StateCode::from_str("cA").unwrap(), StateCode::CA);
        assert_eq!(StateCode::from_str("CA").unwrap(), StateCode::CA);
    }

    #[test]
    fn test_state_code_from_str_whitespace_trimmed() {
        assert_eq!(StateCode::from_str("  CA  ").unwrap(), StateCode::CA);
        assert_eq!(StateCode::from_str("\tNY\n").unwrap(), StateCode::NY);
    }

    #[test]
    fn test_state_code_from_str_rejects_invalid() {
        // Not a real state
        assert!(StateCode::from_str("XX").is_err());
        assert!(StateCode::from_str("ZZ").is_err());

        // Wrong length
        assert!(StateCode::from_str("").is_err());
        assert!(StateCode::from_str("C").is_err());
        assert!(StateCode::from_str("CAL").is_err());

        // Non-ASCII (e.g., diacritics)
        assert!(StateCode::from_str("Cá").is_err());

        // Numeric
        assert!(StateCode::from_str("12").is_err());
    }

    #[test]
    fn test_state_code_to_fips() {
        assert_eq!(StateCode::CA.to_fips(), 6);
        assert_eq!(StateCode::NY.to_fips(), 36);
        assert_eq!(StateCode::TX.to_fips(), 48);
        assert_eq!(StateCode::AL.to_fips(), 1);
        assert_eq!(StateCode::DC.to_fips(), 11);
        assert_eq!(StateCode::PR.to_fips(), 72);
        assert_eq!(StateCode::VI.to_fips(), 78);
    }

    #[test]
    fn test_state_code_from_fips() {
        assert_eq!(StateCode::from_fips(6), Some(StateCode::CA));
        assert_eq!(StateCode::from_fips(36), Some(StateCode::NY));
        assert_eq!(StateCode::from_fips(48), Some(StateCode::TX));
        assert_eq!(StateCode::from_fips(1), Some(StateCode::AL));
        assert_eq!(StateCode::from_fips(11), Some(StateCode::DC));
        assert_eq!(StateCode::from_fips(72), Some(StateCode::PR));

        // Gaps in FIPS numbering — these are not assigned
        assert_eq!(StateCode::from_fips(3), None);
        assert_eq!(StateCode::from_fips(7), None);
        assert_eq!(StateCode::from_fips(14), None);
        assert_eq!(StateCode::from_fips(43), None);  // between PA(42) and RI(44)
        assert_eq!(StateCode::from_fips(52), None);  // between VA(51) and WA(53)
        assert_eq!(StateCode::from_fips(57), None);  // above WY(56), below AS(60)
        assert_eq!(StateCode::from_fips(0), None);
        assert_eq!(StateCode::from_fips(255), None);
    }

    #[test]
    fn test_state_code_fips_roundtrip_for_all_states() {
        for sc in StateCode::ALL {
            let fips = sc.to_fips();
            let back = StateCode::from_fips(fips).expect("every state must roundtrip");
            assert_eq!(*sc, back, "FIPS roundtrip failed for {sc:?} (fips {fips})");
        }
    }

    #[test]
    fn test_state_code_is_state_predicate() {
        assert!(StateCode::CA.is_state());
        assert!(StateCode::TX.is_state());
        assert!(StateCode::WY.is_state());

        assert!(!StateCode::DC.is_state());
        assert!(!StateCode::PR.is_state());
        assert!(!StateCode::GU.is_state());
        assert!(!StateCode::AS.is_state());
        assert!(!StateCode::VI.is_state());
        assert!(!StateCode::MP.is_state());

        // Exactly 50 entries in ALL should be is_state() == true
        let state_count = StateCode::ALL.iter().filter(|s| s.is_state()).count();
        assert_eq!(state_count, 50);
    }

    #[test]
    fn test_state_code_is_territory_predicate() {
        assert!(StateCode::AS.is_territory());
        assert!(StateCode::GU.is_territory());
        assert!(StateCode::MP.is_territory());
        assert!(StateCode::PR.is_territory());
        assert!(StateCode::VI.is_territory());

        assert!(!StateCode::DC.is_territory());  // DC is not a territory
        assert!(!StateCode::CA.is_territory());

        let territory_count = StateCode::ALL.iter().filter(|s| s.is_territory()).count();
        assert_eq!(territory_count, 5);
    }

    #[test]
    fn test_state_code_display() {
        assert_eq!(StateCode::CA.to_string(), "CA");
        assert_eq!(StateCode::NY.to_string(), "NY");
        assert_eq!(StateCode::PR.to_string(), "PR");
        assert_eq!(StateCode::DC.to_string(), "DC");
    }

    #[test]
    fn test_state_code_serde_json() {
        let sc = StateCode::CA;
        let json = serde_json::to_string(&sc).unwrap();
        assert_eq!(json, "\"CA\"");

        let back: StateCode = serde_json::from_str(&json).unwrap();
        assert_eq!(back, sc);

        let pr = StateCode::PR;
        let json = serde_json::to_string(&pr).unwrap();
        assert_eq!(json, "\"PR\"");
    }

    #[test]
    fn test_state_code_all_unique() {
        // No accidental duplicates in the ALL table
        let mut seen = std::collections::HashSet::new();
        for sc in StateCode::ALL {
            assert!(seen.insert(*sc), "duplicate in ALL: {sc:?}");
        }
        assert_eq!(seen.len(), 56);
    }

    #[test]
    fn test_state_code_every_variant_has_str_and_fips() {
        // Touch as_str and to_fips for every variant — guards against
        // forgetting to update one of the match arms when adding a state.
        for sc in StateCode::ALL {
            let s = sc.as_str();
            assert_eq!(s.len(), 2);
            assert!(s.chars().all(|c| c.is_ascii_uppercase()));
            let f = sc.to_fips();
            assert!((1..=78).contains(&f), "FIPS {f} out of expected range for {sc:?}");
        }
    }

    #[test]
    fn test_state_code_all_contains_every_variant() {
        // Every variant must appear in ALL exactly once. Using a HashSet
        // prevents an editing mistake (forgetting to add a new variant or
        // duplicating one) from going unnoticed.
        let unique: std::collections::HashSet<_> = StateCode::ALL.iter().collect();
        assert_eq!(unique.len(), StateCode::ALL.len());
        assert_eq!(unique.len(), 56);
    }
}
