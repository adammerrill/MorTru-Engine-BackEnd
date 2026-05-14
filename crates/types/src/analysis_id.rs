//! `AnalysisId` — UUID v4 identifier for a complete analysis run.
//!
//! An "analysis" is one invocation of the engine's `analyze()` method: given
//! a property and a borrower, produce the ranked set of qualifying scenarios.
//! The `AnalysisId` correlates logs, metrics, API responses, and database
//! records across the duration of that one invocation.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// UUID v4 identifier for one analysis invocation.
///
/// The inner field is `pub` for ergonomic construction from a known UUID
/// (e.g., persisted state, test fixture). Use [`Self::new`] for a fresh ID.
#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[repr(transparent)]
pub struct AnalysisId(pub Uuid);

impl AnalysisId {
    /// Generate a fresh random `AnalysisId` using UUID v4.
    #[must_use]
    pub fn new() -> Self {
        AnalysisId(Uuid::new_v4())
    }

    /// Wrap an existing `Uuid`.
    #[must_use]
    pub const fn from_uuid(uuid: Uuid) -> Self {
        AnalysisId(uuid)
    }

    /// Borrow the underlying `Uuid`.
    #[must_use]
    pub const fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    /// The all-zero (nil) UUID. Sentinel value only.
    pub const NIL: Self = AnalysisId(Uuid::nil());
}

impl Default for AnalysisId {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Uuid> for AnalysisId {
    fn from(u: Uuid) -> Self {
        AnalysisId(u)
    }
}

impl FromStr for AnalysisId {
    type Err = uuid::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(s).map(AnalysisId)
    }
}

impl fmt::Display for AnalysisId {
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
    fn test_analysis_id_new_generates_v4() {
        let id = AnalysisId::new();
        assert_eq!(id.0.get_version_num(), 4);
    }

    #[test]
    fn test_analysis_id_new_generates_unique() {
        let mut seen = HashSet::new();
        for _ in 0..1000 {
            let id = AnalysisId::new();
            assert!(seen.insert(id), "duplicate AnalysisId: {id}");
        }
    }

    #[test]
    fn test_analysis_id_from_uuid() {
        let uuid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let id = AnalysisId::from_uuid(uuid);
        assert_eq!(*id.as_uuid(), uuid);
    }

    #[test]
    fn test_analysis_id_display() {
        let uuid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let id = AnalysisId::from_uuid(uuid);
        assert_eq!(id.to_string(), "550e8400-e29b-41d4-a716-446655440000");
    }

    #[test]
    fn test_analysis_id_from_str() {
        let id: AnalysisId = "550e8400-e29b-41d4-a716-446655440000".parse().unwrap();
        assert_eq!(id.to_string(), "550e8400-e29b-41d4-a716-446655440000");
        assert!("garbage".parse::<AnalysisId>().is_err());
    }

    #[test]
    fn test_analysis_id_serde_json() {
        let id =
            AnalysisId::from_uuid(Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap());
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"550e8400-e29b-41d4-a716-446655440000\"");
        let back: AnalysisId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, id);
    }

    #[test]
    fn test_analysis_id_nil_sentinel() {
        assert_eq!(AnalysisId::NIL.as_uuid(), &Uuid::nil());
    }

    #[test]
    fn test_analysis_id_size() {
        assert_eq!(size_of::<AnalysisId>(), 16);
    }

    #[test]
    fn test_analysis_id_distinct_from_scenario_id() {
        // Both wrap Uuid but are different types — passing one where the
        // other is expected must be a compile error. We can't test that
        // directly in a unit test, but we can verify they don't compare
        // equal via PartialEq across types (different types can't compare
        // at all by default in Rust).
        let uuid = Uuid::new_v4();
        let analysis = AnalysisId::from_uuid(uuid);
        let _scenario = crate::ScenarioId::from_uuid(uuid);
        // The compiler enforces type distinctness; we can only assert that
        // we constructed both successfully.
        assert_eq!(analysis.as_uuid(), &uuid);
    }
}
