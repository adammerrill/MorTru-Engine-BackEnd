//! Versioned reference data for the MorTru mortgage engine.
//!
//! # What this crate does
//!
//! Every loan program eligibility check and every payment calculation depends
//! on external reference data that changes on known schedules:
//!
//! | Data | Source | Update cadence |
//! |---|---|---|
//! | FHA loan limits | HUD | Annually (Nov → Jan 1) |
//! | GSE conforming limits | FHFA | Annually (Nov → Jan 1) |
//! | USDA rural eligibility | USDA/Census | Every 10 yrs + interim |
//! | USDA income limits (SFGH) | USDA RD | Annually (Oct/Nov) |
//! | Area Median Income (AMI) | HUD/FFIEC | Annually |
//! | FHA MIP rates | HUD | As published (rare) |
//! | VA funding fees | VA | As published (rare) |
//! | Conv MI coverage reqs | GSEs | As published |
//! | LLPA matrices | FNMA/FHLMC | Monthly |
//! | MI rate cards | MI providers | Weekly/intraday |
//! | Rate sheets | Individual lenders | Daily/intraday |
//! | Lender overlays | Per lender | As published |
//! | Texas HOI premiums | TDOI | Annually |
//! | CBSA/MSA crosswalk | Census/OMB | Every 3–5 years |
//!
//! # Versioning contract
//!
//! Every piece of data is wrapped in [`Versioned<T>`] which carries a
//! [`VersionId`] and an `effective_date`. Every analysis stores a
//! [`DataVersionManifest`] — the exact versions used. Any historical
//! analysis can be reproduced exactly by loading the same manifest.
//!
//! # Storage backends
//!
//! All data access goes through the [`RefDataStore`] trait:
//! - [`JsonFileStore`] — dev and CI (no database)
//! - `SqliteStore` — integration tests
//! - `PostgresStore` — production
//!
//! # Task roadmap
//!
//! - Task 4.1: This scaffold + `RefDataStore` trait + versioning types
//! - Task 4.2: `JsonFileStore` impl
//! - Task 4.3: Migration framework (12 SQL files)
//! - Task 4.4: `SqliteStore` impl
//! - Task 4.5: FHA loan limits loader + query
//! - Task 4.6: GSE conforming limits + high-balance designation
//! - Task 4.7: USDA rural eligibility — shapefile → census tract table
//! - Task 4.8: USDA MFH by tract — CSV loader
//! - Task 4.9: USDA income limits — SFGH 115% AMI by family size (8 sizes)
//! - Task 4.10: AMI tract data — 50/80/100/115% AMI, low-income tract flag
//! - Task 4.11: CBSA/MSA crosswalk
//! - Task 4.12: Texas HOI premiums by ZIP
//! - Task 4.13: FHA MIP rates (monthly + single premium)
//! - Task 4.14: VA funding fee table
//! - Task 4.15: MI coverage requirements + LLPA matrices
//! - Task 4.16: Lender profiles + overlays
//! - Task 4.17: MI provider + rate cards
//! - Task 4.18: Rate sheets (volatile intraday)
//! - Task 4.19: [`GeoEligibility`] unified query API
//! - Task 4.20: Epic 4 gate — all 5 fixture scenarios

pub mod error;
pub mod geo;
pub mod store;
pub mod versioning;

pub use error::{RefDataError, RefDataResult};
pub use geo::{
    AmiTractData, FhaLimitType, FhaLoanLimits, GeoEligibility, GseLoanLimits, UsdaIncomeLimit,
    UsdaMfhByTract, UsdaruralEligibility,
};
pub use store::RefDataStore;
pub use versioning::{DataVersionManifest, VersionId, Versioned};
