//! Versioned reference data for the MorTru mortgage engine.

pub mod cbsa;
pub mod condo_approval;
pub mod conv_mi;
pub mod error;
pub mod fha_mip;
pub mod geo;
pub mod hoi_rates;
pub mod lender;
pub mod mcc_catalog;
pub mod program_rules;
pub mod rate_sheet;
pub mod store;
pub mod va_fee;
pub mod versioning;
pub mod zip_hoi;

pub use cbsa::{CbsaDesignation, CbsaEntry};
pub use condo_approval::{CondoApprovalStatus, FhaCondoProject};
pub use conv_mi::{
    ConvMiCoverage, ConvMiInput, ConvMiProgram, MiMonthlyTable, MiRateInput, UsdaGuaranteeFees,
};
pub use error::{RefDataError, RefDataResult};
pub use fha_mip::{FhaMipInput, FhaMipResult, MipDuration};
pub use geo::{
    AmiTractData, FhaLimitType, FhaLoanLimits, GeoEligibility, GseLoanLimits, UsdaIncomeLimit,
    UsdaMfhByTract, UsdaruralEligibility,
};
pub use hoi_rates::{StateHoiRate, NATIONAL_FALLBACK_RATE_BPS};
pub use lender::{LenderOverlays, LenderProfile};
pub use mcc_catalog::{
    estimate_annual_credit, MccCatalogFile, MccEligibilityInput, MccOutcome, MccProgram,
};
pub use program_rules::{AllProgramRules, ProgramEligibilityRules};
pub use rate_sheet::{LlpaInput, LlpaMatrix, RateSheet, RateSheetEntry};
pub use store::{JsonFileStore, RefDataStore};
pub use va_fee::{VaFeeInput, VaLoanPurpose, VaUse, VeteranCategory};
pub use versioning::{DataVersionManifest, VersionId, Versioned};
pub use zip_hoi::ZipHoiRate;

#[cfg(feature = "sqlite")]
pub use store::sqlite::SqliteStore;
