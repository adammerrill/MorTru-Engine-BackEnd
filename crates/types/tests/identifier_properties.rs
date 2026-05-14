//! Property-based tests for the Task 1.3 identifier types.

use std::collections::HashSet;

use proptest::prelude::*;
use uuid::Uuid;

use types::{
    AnalysisId, FipsCode, LenderId, LoanCasefileId, MlsListingKey, ScenarioId, StateCode,
};

// ----- FipsCode -----

proptest! {
    /// `FipsCode::new` accepts iff state is valid (via StateCode::from_fips)
    /// and county is in 0..=999.
    #[test]
    fn prop_fips_code_accepts_iff_valid(state in 0u8..100, county in 0u16..2000) {
        let result = FipsCode::new(state, county);
        let state_valid = StateCode::from_fips(state).is_some();
        let county_valid = county <= 999;
        if state_valid && county_valid {
            prop_assert!(result.is_ok(), "expected Ok for state={state} county={county}");
            let code = result.unwrap();
            prop_assert_eq!(code.state_fips(), state);
            prop_assert_eq!(code.county_fips(), county);
        } else {
            prop_assert!(result.is_err(), "expected Err for state={state} county={county}");
        }
    }

    /// Roundtrip: FipsCode -> String -> FipsCode is identity for every valid code.
    #[test]
    fn prop_fips_code_string_roundtrip(idx in 0usize..StateCode::ALL.len(), county in 0u16..=999) {
        let state = StateCode::ALL[idx].to_fips();
        let original = FipsCode::new(state, county).unwrap();
        let s = original.to_string();
        prop_assert_eq!(s.len(), 5, "Display must always produce 5 chars");
        let parsed: FipsCode = s.parse().unwrap();
        prop_assert_eq!(parsed, original);
    }

    /// Roundtrip through JSON serialization.
    #[test]
    fn prop_fips_code_serde_roundtrip(idx in 0usize..StateCode::ALL.len(), county in 0u16..=999) {
        let state = StateCode::ALL[idx].to_fips();
        let original = FipsCode::new(state, county).unwrap();
        let json = serde_json::to_string(&original).unwrap();
        let back: FipsCode = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(original, back);
    }
}

// ----- StateCode -----

proptest! {
    /// Every value in StateCode::ALL roundtrips through to_fips -> from_fips.
    #[test]
    fn prop_state_code_fips_roundtrip(idx in 0usize..StateCode::ALL.len()) {
        let sc = StateCode::ALL[idx];
        let fips = sc.to_fips();
        let back = StateCode::from_fips(fips).unwrap();
        prop_assert_eq!(sc, back);
    }

    /// from_str roundtrips against as_str for every state.
    #[test]
    fn prop_state_code_string_roundtrip(idx in 0usize..StateCode::ALL.len()) {
        let sc = StateCode::ALL[idx];
        let s = sc.as_str();
        let back: StateCode = s.parse().unwrap();
        prop_assert_eq!(sc, back);
    }

    /// Case-insensitive parsing works for every state.
    #[test]
    fn prop_state_code_case_insensitive(idx in 0usize..StateCode::ALL.len()) {
        let sc = StateCode::ALL[idx];
        let upper = sc.as_str().to_string();
        let lower = upper.to_lowercase();
        let mixed: String = upper
            .chars()
            .enumerate()
            .map(|(i, c)| if i % 2 == 0 { c.to_ascii_lowercase() } else { c })
            .collect();

        prop_assert_eq!(upper.parse::<StateCode>().unwrap(), sc);
        prop_assert_eq!(lower.parse::<StateCode>().unwrap(), sc);
        prop_assert_eq!(mixed.parse::<StateCode>().unwrap(), sc);
    }

    /// Serde roundtrip for every state.
    #[test]
    fn prop_state_code_serde_roundtrip(idx in 0usize..StateCode::ALL.len()) {
        let sc = StateCode::ALL[idx];
        let json = serde_json::to_string(&sc).unwrap();
        let back: StateCode = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(sc, back);
    }
}

// ----- LenderId / MlsListingKey / LoanCasefileId -----
//
// Generate alphanumeric strings within length bounds and verify that
// validation and round-tripping behave consistently.

fn alphanumeric_strategy(min: usize, max: usize) -> impl Strategy<Value = String> {
    proptest::collection::vec(
        prop_oneof![
            any::<u8>().prop_map(|b| (b % 26) + b'A'),
            any::<u8>().prop_map(|b| (b % 10) + b'0'),
        ],
        min..=max,
    )
    .prop_map(|bytes| String::from_utf8(bytes).unwrap())
}

proptest! {
    /// LenderId accepts valid alphanumeric strings up to MAX_LEN and rejects
    /// strings that exceed it.
    #[test]
    fn prop_lender_id_length_validation(s in alphanumeric_strategy(1, 64)) {
        let result = LenderId::new(&s);
        if s.len() <= LenderId::MAX_LEN {
            prop_assert!(result.is_ok());
            let id = result.unwrap();
            prop_assert_eq!(id.as_str(), s.as_str());
        } else {
            prop_assert!(result.is_err());
        }
    }

    #[test]
    fn prop_lender_id_serde_roundtrip(s in alphanumeric_strategy(1, 32)) {
        let id = LenderId::new(s.as_str()).unwrap();
        let json = serde_json::to_string(&id).unwrap();
        let back: LenderId = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(id, back);
    }

    #[test]
    fn prop_mls_listing_key_length_validation(s in alphanumeric_strategy(1, 200)) {
        let result = MlsListingKey::new(s.as_str());
        if s.len() <= MlsListingKey::MAX_LEN {
            prop_assert!(result.is_ok());
        } else {
            prop_assert!(result.is_err());
        }
    }

    #[test]
    fn prop_mls_listing_key_serde_roundtrip(s in alphanumeric_strategy(1, 128)) {
        let key = MlsListingKey::new(s.as_str()).unwrap();
        let json = serde_json::to_string(&key).unwrap();
        let back: MlsListingKey = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(key, back);
    }

    #[test]
    fn prop_loan_casefile_id_length_validation(s in alphanumeric_strategy(1, 100)) {
        let result = LoanCasefileId::new(s.as_str());
        if s.len() <= LoanCasefileId::MAX_LEN {
            prop_assert!(result.is_ok());
        } else {
            prop_assert!(result.is_err());
        }
    }

    #[test]
    fn prop_loan_casefile_id_serde_roundtrip(s in alphanumeric_strategy(1, 64)) {
        let id = LoanCasefileId::new(s.as_str()).unwrap();
        let json = serde_json::to_string(&id).unwrap();
        let back: LoanCasefileId = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(id, back);
    }
}

// ----- ScenarioId / AnalysisId -----

proptest! {
    /// Wrapping any 16 bytes as a UUID and putting it into a ScenarioId
    /// roundtrips through serde.
    #[test]
    fn prop_scenario_id_serde_roundtrip(bytes in any::<[u8; 16]>()) {
        let uuid = Uuid::from_bytes(bytes);
        let id = ScenarioId::from_uuid(uuid);
        let json = serde_json::to_string(&id).unwrap();
        let back: ScenarioId = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(id, back);
    }

    #[test]
    fn prop_analysis_id_serde_roundtrip(bytes in any::<[u8; 16]>()) {
        let uuid = Uuid::from_bytes(bytes);
        let id = AnalysisId::from_uuid(uuid);
        let json = serde_json::to_string(&id).unwrap();
        let back: AnalysisId = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(id, back);
    }

    /// Display roundtrips through FromStr for ScenarioId.
    #[test]
    fn prop_scenario_id_display_parse_roundtrip(bytes in any::<[u8; 16]>()) {
        let uuid = Uuid::from_bytes(bytes);
        let id = ScenarioId::from_uuid(uuid);
        let s = id.to_string();
        let parsed: ScenarioId = s.parse().unwrap();
        prop_assert_eq!(id, parsed);
    }

    #[test]
    fn prop_analysis_id_display_parse_roundtrip(bytes in any::<[u8; 16]>()) {
        let uuid = Uuid::from_bytes(bytes);
        let id = AnalysisId::from_uuid(uuid);
        let s = id.to_string();
        let parsed: AnalysisId = s.parse().unwrap();
        prop_assert_eq!(id, parsed);
    }
}

/// Outside the proptest macro because it doesn't need fuzzing — just a
/// strong statistical check that ScenarioId::new() avoids collisions.
#[test]
fn test_scenario_id_uniqueness_at_10k() {
    let mut seen = HashSet::with_capacity(10_000);
    for _ in 0..10_000 {
        let id = ScenarioId::new();
        assert!(seen.insert(id), "ScenarioId::new() produced a duplicate");
    }
    assert_eq!(seen.len(), 10_000);
}

#[test]
fn test_analysis_id_uniqueness_at_10k() {
    let mut seen = HashSet::with_capacity(10_000);
    for _ in 0..10_000 {
        let id = AnalysisId::new();
        assert!(seen.insert(id), "AnalysisId::new() produced a duplicate");
    }
    assert_eq!(seen.len(), 10_000);
}
