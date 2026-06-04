//! Derivation provenance — every derived value carries the full trail of how it
//! was produced: which dataset, which file, which version, which record, and an
//! ordered list of the rules applied along the way.
//!
//! This is the backbone of the engine's "100% explainable" requirement. Any
//! result a borrower or auditor sees can be expanded into the exact chain of
//! reference data and rules that produced it. The pattern is generic
//! (`Derived<T>`) so it composes across every crate: a `Derived<Cents>` payment
//! can be assembled from `Derived` rate-sheet, LLPA, MIP, and tax values, each
//! carrying its own sub-trail.

use serde::{Deserialize, Serialize};

/// Identifies the exact source record a value was read from.
///
/// Captures both the *requested* version and the *resolved* version so that
/// year-fallback (e.g. asking for 2030 data and getting the latest 2025 file)
/// is itself part of the audit trail rather than a silent substitution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    /// Logical dataset name, e.g. "mcc_catalog".
    pub dataset: String,
    /// Concrete file the value was read from, e.g. "mcc_catalog_2025.json".
    pub source_file: String,
    /// Human-readable citation of the underlying authority,
    /// e.g. "TDHCA Texas MCC Program Guidelines (2025)".
    pub source_citation: String,
    /// ISO date the data became effective.
    pub effective_date: String,
    /// The specific record/row id within the file, e.g. "mcc_tx_tdhca".
    pub record_id: String,
    /// Version year the caller requested.
    pub requested_version: u16,
    /// Version year actually matched (may differ from requested via fallback).
    pub resolved_version: u16,
}

impl Provenance {
    /// True when the resolved version differs from the requested version,
    /// i.e. a fallback occurred and the caller should be aware the data is
    /// not an exact-year match.
    #[must_use]
    pub fn is_fallback(&self) -> bool {
        self.requested_version != self.resolved_version
    }
}

/// One ordered step in deriving a result. Steps read like an audit log entry:
/// "applied rule X to inputs Y, concluded Z".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DerivationStep {
    /// The rule or check applied, e.g. "first_time_homebuyer_requirement".
    pub rule: String,
    /// The inputs considered, e.g. "is_fthb=false, targeted_area=true".
    pub inputs: String,
    /// What the step concluded, e.g. "satisfied via targeted-area exemption".
    pub outcome: String,
}

impl DerivationStep {
    pub fn new(
        rule: impl Into<String>,
        inputs: impl Into<String>,
        outcome: impl Into<String>,
    ) -> Self {
        Self {
            rule: rule.into(),
            inputs: inputs.into(),
            outcome: outcome.into(),
        }
    }
}

/// A value bundled with its provenance and an ordered derivation trace.
///
/// `Derived<T>` is the universal return shape for anything the engine computes.
/// The inner `value` is the answer; `provenance` says where the governing data
/// came from; `steps` is the ordered "why".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Derived<T> {
    pub value: T,
    pub provenance: Provenance,
    pub steps: Vec<DerivationStep>,
}

impl<T> Derived<T> {
    /// Start a derivation from a value and its source provenance, with no steps yet.
    pub fn new(value: T, provenance: Provenance) -> Self {
        Self {
            value,
            provenance,
            steps: Vec::new(),
        }
    }

    /// Append a derivation step (builder style).
    #[must_use]
    pub fn with_step(
        mut self,
        rule: impl Into<String>,
        inputs: impl Into<String>,
        outcome: impl Into<String>,
    ) -> Self {
        self.steps.push(DerivationStep::new(rule, inputs, outcome));
        self
    }

    /// Append a derivation step in place.
    pub fn push_step(
        &mut self,
        rule: impl Into<String>,
        inputs: impl Into<String>,
        outcome: impl Into<String>,
    ) {
        self.steps.push(DerivationStep::new(rule, inputs, outcome));
    }

    /// Transform the inner value while preserving the full trace.
    ///
    /// Lets a higher layer build on a lower-layer result without losing the
    /// provenance — e.g. turning a `Derived<MccProgram>` into a
    /// `Derived<Cents>` credit estimate that still carries the catalog trail.
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Derived<U> {
        Derived {
            value: f(self.value),
            provenance: self.provenance,
            steps: self.steps,
        }
    }

    /// Render the full derivation as human-readable text. This is the
    /// "show me how this was derived" output an analyst or borrower sees.
    pub fn explain(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "Source: {} (record '{}') from {}\n",
            self.provenance.source_citation, self.provenance.record_id, self.provenance.source_file,
        ));
        out.push_str(&format!(
            "Effective: {} | version requested {} resolved {}{}\n",
            self.provenance.effective_date,
            self.provenance.requested_version,
            self.provenance.resolved_version,
            if self.provenance.is_fallback() {
                " (FALLBACK)"
            } else {
                ""
            },
        ));
        out.push_str("Derivation:\n");
        for (i, s) in self.steps.iter().enumerate() {
            out.push_str(&format!(
                "  {}. [{}] {} → {}\n",
                i + 1,
                s.rule,
                s.inputs,
                s.outcome
            ));
        }
        out
    }
}
