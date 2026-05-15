//! Versioned reference data for the MorTru mortgage engine.

pub mod cbsa;
pub mod error;
pub mod geo;
pub mod hoi_rates;
pub mod program_rules;
pub mod store;
pub mod versioning;
pub mod zip_hoi;

pub use cbsa::{CbsaDesignation, CbsaEntry};
pub use error::{RefDataError, RefDataResult};
pub use geo::{
    AmiTractData, FhaLimitType, FhaLoanLimits, GeoEligibility, GseLoanLimits, UsdaIncomeLimit,
    UsdaMfhByTract, UsdaruralEligibility,
};
pub use hoi_rates::{StateHoiRate, NATIONAL_FALLBACK_RATE_BPS};
pub use program_rules::{AllProgramRules, ProgramEligibilityRules};
pub use store::{JsonFileStore, RefDataStore};
pub use versioning::{DataVersionManifest, VersionId, Versioned};
pub use zip_hoi::ZipHoiRate;

#[cfg(feature = "sqlite")]
pub use store::sqlite::SqliteStore;
