//! Versioned reference data for the MorTru mortgage engine.
//!
//! # Update cadences
//!
//! | Data | Source | Cadence |
//! |---|---|---|
//! | FHA loan limits | HUD | Annually (Nov → Jan 1) |
//! | GSE conforming limits | FHFA | Annually (Nov → Jan 1) |
//! | USDA rural eligibility | USDA/Census | Every 10 yrs + interim |
//! | USDA income limits (SFGH) | USDA RD | Annually (Oct/Nov) |
//! | Area Median Income (AMI) | HUD/FFIEC | Annually |
//! | Program eligibility rules | Agencies | As published |
//! | State HOI rates | NAIC/state depts | Annually |
//! | FHA MIP rates | HUD | As published (rare) |
//! | VA funding fees | VA | As published (rare) |
//! | Conv MI coverage reqs | GSEs | As published |
//! | LLPA matrices | FNMA/FHLMC | Monthly |
//! | MI rate cards | MI providers | Weekly/intraday |
//! | Rate sheets | Individual lenders | Daily/intraday |
//! | Texas HOI premiums | TDOI | Annually |
//! | CBSA/MSA crosswalk | Census/OMB | Every 3–5 years |

pub mod error;
pub mod geo;
pub mod hoi_rates;
pub mod program_rules;
pub mod store;
pub mod versioning;

pub use error::{RefDataError, RefDataResult};
pub use geo::{
    AmiTractData, FhaLimitType, FhaLoanLimits, GeoEligibility, GseLoanLimits, UsdaIncomeLimit,
    UsdaMfhByTract, UsdaruralEligibility,
};
pub use hoi_rates::{StateHoiRate, NATIONAL_FALLBACK_RATE_BPS};
pub use program_rules::{AllProgramRules, ProgramEligibilityRules};
pub use store::{JsonFileStore, RefDataStore};
pub use versioning::{DataVersionManifest, VersionId, Versioned};
