//! `RefDataStore` trait — the single access point for all reference data.
//!
//! The engine never talks to a database directly. Every data fetch goes
//! through this trait, which enables:
//!
//! - `JsonFileStore` in dev and CI (no database, fast tests)
//! - `SqliteStore` for integration tests
//! - `PostgresStore` for production
//!
//! Switching environments is one line of startup configuration.

use crate::{
    error::RefDataResult,
    geo::{
        AmiTractData, FhaLoanLimits, GeoEligibility, GseLoanLimits, UsdaIncomeLimit,
        UsdaMfhByTract, UsdaruralEligibility,
    },
    versioning::VersionId,
};

/// The single interface for all reference data access.
///
/// All methods return owned data; the store handles caching internally.
/// Every method returns the data as-of the requested effective year,
/// using the most recent version available that is ≤ the year.
pub trait RefDataStore: Send + Sync {
    // ── Loan limits ──────────────────────────────────────────────────────────

    /// FHA loan limits for a county.
    fn fha_loan_limits(
        &self,
        fips_code: &str,
        year: u16,
    ) -> RefDataResult<crate::versioning::Versioned<FhaLoanLimits>>;

    /// GSE conforming loan limits for a county.
    fn gse_loan_limits(
        &self,
        fips_code: &str,
        year: u16,
    ) -> RefDataResult<crate::versioning::Versioned<GseLoanLimits>>;

    // ── USDA ─────────────────────────────────────────────────────────────────

    /// USDA rural eligibility for a census tract.
    /// Returns None if no tract data is available (treat as ineligible).
    fn usda_rural_eligibility(&self, geoid: &str) -> RefDataResult<Option<UsdaruralEligibility>>;

    /// USDA SFGH income limits for a county.
    fn usda_income_limits(
        &self,
        fips_code: &str,
        effective_date: chrono::NaiveDate,
    ) -> RefDataResult<crate::versioning::Versioned<UsdaIncomeLimit>>;

    /// USDA MFH projects for a census tract (may be None).
    fn usda_mfh_by_tract(&self, geoid: &str) -> RefDataResult<Option<UsdaMfhByTract>>;

    // ── AMI ──────────────────────────────────────────────────────────────────

    /// Area Median Income data for a census tract.
    fn ami_tract_data(
        &self,
        geoid: &str,
        year: u16,
    ) -> RefDataResult<Option<crate::versioning::Versioned<AmiTractData>>>;

    // ── Unified geo-eligibility query ─────────────────────────────────────────

    /// Assemble all geographic eligibility data for one property in a single
    /// call. Implementations should batch the underlying data fetches.
    ///
    /// `tract_geoid` is the 11-digit census tract GEOID from FCC resolution.
    /// Pass `None` if FCC has not been called yet — USDA and AMI checks will
    /// return conservative (ineligible / no limit) results.
    fn geo_eligibility(
        &self,
        fips_code: &str,
        tract_geoid: Option<&str>,
        year: u16,
    ) -> RefDataResult<GeoEligibility>;

    // ── Version tracking ─────────────────────────────────────────────────────

    /// Current version ID for a named dataset. Used to build a
    /// [`DataVersionManifest`] for each analysis.
    fn current_version(&self, dataset: &str) -> RefDataResult<VersionId>;
}

// ── Stub JsonFileStore (Tasks 4.1-4.2) ───────────────────────────────────────

/// JSON-file-backed store for development and CI.
///
/// Data lives in `data/ref_data/*.json`. No database required.
/// This is the store used in ALL `ref_data` tests.
#[derive(Debug)]
pub struct JsonFileStore {
    pub data_dir: std::path::PathBuf,
}

impl JsonFileStore {
    /// Create a store pointing at `data_dir`.
    #[must_use]
    pub fn new(data_dir: impl Into<std::path::PathBuf>) -> Self {
        Self {
            data_dir: data_dir.into(),
        }
    }
}
