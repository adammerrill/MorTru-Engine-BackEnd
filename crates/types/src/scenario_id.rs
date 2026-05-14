//! `ScenarioId` — UUID v4 identifier for an individual loan scenario.
//!
//! Distinct from the 8-byte packed `ScenarioKey` that lives in the
//! `scenarios` crate. `ScenarioKey` is the deduplication key used inside
//! the scenario enumeration pipeline; `ScenarioId` is the externally-visible
//! UUID attached to scenarios that survive enumeration, used for logging,
//! API responses, and correlation across services.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// UUID v4 identifier for a single scored scenario.
///
/// The inner field is `pub` so generating an ID from a known UUID is
/// frictionless (e.g., in tests, or when restoring from persisted state).
/// Use [`Self::new`] to generate a fresh random ID for a new scenario.
#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[repr(transparent)]
pub struct ScenarioId(pub Uuid);

impl ScenarioId {
    /// Generate a fresh random `ScenarioId` using UUID v4.
    #[must_use]
    pub fn new() -> Self {
        ScenarioId(Uuid::new_v4())
    }

    /// Wrap an existing `Uuid`. Useful for reconstructing an ID from a
    /// persisted value or a test fixture.
    #[must_use]
    pub const fn from_uuid(uuid: Uuid) -> Self {
        ScenarioId(uuid)
    }

    /// Borrow the underlying `Uuid`.
    #[must_use]
    pub const fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    /// The all-zero (nil) UUID. Used as a sentinel where required, never as
    /// a real scenario identifier.
    pub const NIL: Self = ScenarioId(Uuid::nil());
}

// `new` is the canonical constructor; `Default` delegates to it for the
// occasional case where a default value is required (e.g., `#[derive(Default)]`
// on a struct that contains a `ScenarioId`).
impl Default for ScenarioId {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Uuid> for ScenarioId {
    fn from(u: Uuid) -> Self {
        ScenarioId(u)
    }
}

impl FromStr for ScenarioId {
    type Err = uuid::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(s).map(ScenarioId)
    }
}

impl fmt::Display for ScenarioId {
    /// Format as the canonical hyphenated UUID string.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_scenario_id_new_generates_v4() {
        let id = ScenarioId::new();
        assert_eq!(id.0.get_version_num(), 4);
    }

    #[test]
    fn test_scenario_id_new_generates_unique() {
        // Statistical: 1,000 IDs in a row, expect no collisions
        let mut seen = HashSet::new();
        for _ in 0..1000 {
            let id = ScenarioId::new();
            assert!(seen.insert(id), "duplicate ScenarioId: {id}");
        }
        assert_eq!(seen.len(), 1000);
    }

    #[test]
    fn test_scenario_id_from_uuid() {
        let uuid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let id = ScenarioId::from_uuid(uuid);
        assert_eq!(*id.as_uuid(), uuid);
    }

    #[test]
    fn test_scenario_id_from_uuid_impl_via_into() {
        let uuid = Uuid::new_v4();
        let id: ScenarioId = uuid.into();
        assert_eq!(*id.as_uuid(), uuid);
    }

    #[test]
    fn test_scenario_id_display() {
        let uuid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let id = ScenarioId::from_uuid(uuid);
        assert_eq!(id.to_string(), "550e8400-e29b-41d4-a716-446655440000");
    }

    #[test]
    fn test_scenario_id_from_str() {
        let id: ScenarioId = "550e8400-e29b-41d4-a716-446655440000".parse().unwrap();
        assert_eq!(id.to_string(), "550e8400-e29b-41d4-a716-446655440000");

        // Invalid UUID rejected
        assert!("not-a-uuid".parse::<ScenarioId>().is_err());
        assert!("".parse::<ScenarioId>().is_err());
    }

    #[test]
    fn test_scenario_id_serde_json() {
        let id =
            ScenarioId::from_uuid(Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap());
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"550e8400-e29b-41d4-a716-446655440000\"");

        let back: ScenarioId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, id);
    }

    #[test]
    fn test_scenario_id_nil_sentinel() {
        assert_eq!(ScenarioId::NIL.as_uuid(), &Uuid::nil());
        assert_eq!(
            ScenarioId::NIL.to_string(),
            "00000000-0000-0000-0000-000000000000"
        );
    }

    #[test]
    fn test_scenario_id_default_generates_fresh() {
        let a: ScenarioId = Default::default();
        let b: ScenarioId = Default::default();
        // Two defaults are extremely unlikely to collide
        assert_ne!(a, b);
    }

    #[test]
    fn test_scenario_id_size() {
        // UUID is 16 bytes; transparent wrapper adds no overhead
        assert_eq!(std::mem::size_of::<ScenarioId>(), 16);
    }

    #[test]
    fn test_scenario_id_hashable() {
        let mut seen = HashSet::new();
        let id = ScenarioId::new();
        seen.insert(id);
        assert!(seen.contains(&id));
    }
}
