//! Versioning types for auditable, reproducible loan analysis.
//!
//! # Why this matters
//!
//! CFPB examination requires perfect replay of any historical loan analysis.
//! Every reference data element (loan limits, AMI, USDA eligibility, MI rates,
//! LLPAs) is timestamped and version-identified. The [`DataVersionManifest`]
//! stored with each analysis records exactly which version of every data element
//! was used. Combined with the engine git SHA, any analysis can be reproduced.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// Opaque identifier for a specific version of a reference data element.
///
/// Format: `"{table}:{effective_date}"`, e.g. `"fha_loan_limits:2025-01-01"`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VersionId(pub String);

impl VersionId {
    /// Construct a version ID from table name and effective date.
    #[must_use]
    pub fn new(table: &str, effective_date: NaiveDate) -> Self {
        Self(format!("{}:{}", table, effective_date.format("%Y-%m-%d")))
    }

    /// The raw string identifier.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for VersionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A piece of reference data paired with its version identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Versioned<T> {
    pub version_id: VersionId,
    pub effective_date: NaiveDate,
    pub data: T,
}

impl<T> Versioned<T> {
    /// Wrap data with a version identity.
    #[must_use]
    pub fn new(table: &str, effective_date: NaiveDate, data: T) -> Self {
        Self {
            version_id: VersionId::new(table, effective_date),
            effective_date,
            data,
        }
    }
}

/// Records the exact version of every reference data element used in one
/// loan analysis. Stored with the analysis result for audit/replay.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DataVersionManifest {
    /// Engine git commit SHA at analysis time.
    pub engine_sha: Option<String>,

    /// FHA loan limits version used.
    pub fha_loan_limits: Option<VersionId>,

    /// GSE conforming loan limits version used.
    pub gse_loan_limits: Option<VersionId>,

    /// USDA rural eligibility dataset version used.
    pub usda_rural_eligibility: Option<VersionId>,

    /// USDA SFGH income limits version used.
    pub usda_income_limits: Option<VersionId>,

    /// AMI tract data version used (HUD/FFIEC).
    pub ami_tract_data: Option<VersionId>,

    /// FHA MIP rate table version used.
    pub fha_mip_rates: Option<VersionId>,

    /// VA funding fee table version used.
    pub va_funding_fees: Option<VersionId>,

    /// Conventional MI coverage requirements version used.
    pub mi_coverage_reqs: Option<VersionId>,

    /// LLPA matrix versions used (one per lender).
    /// Key = lender_id, value = version of their LLPA matrix.
    pub llpa_matrices: std::collections::HashMap<String, VersionId>,

    /// MI provider rate card versions used.
    /// Key = provider_id, value = version.
    pub mi_rate_cards: std::collections::HashMap<String, VersionId>,

    /// Rate sheet versions used (one per lender).
    /// Key = lender_id, value = rate sheet version.
    pub rate_sheets: std::collections::HashMap<String, VersionId>,

    /// Timestamp when this manifest was created (analysis run time).
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl DataVersionManifest {
    /// Create a new empty manifest, capturing the current timestamp.
    #[must_use]
    pub fn new() -> Self {
        Self {
            created_at: Some(chrono::Utc::now()),
            ..Default::default()
        }
    }

    /// True if every required data element has a version recorded.
    /// Used to validate manifest completeness before storing an analysis.
    #[must_use]
    pub fn is_complete_for_program(&self, is_usda: bool, is_va: bool) -> bool {
        let base = self.fha_loan_limits.is_some()
            && self.gse_loan_limits.is_some()
            && self.ami_tract_data.is_some()
            && self.fha_mip_rates.is_some()
            && self.mi_coverage_reqs.is_some();

        let usda_ok = !is_usda
            || (self.usda_rural_eligibility.is_some() && self.usda_income_limits.is_some());

        let va_ok = !is_va || self.va_funding_fees.is_some();

        base && usda_ok && va_ok
    }
}
