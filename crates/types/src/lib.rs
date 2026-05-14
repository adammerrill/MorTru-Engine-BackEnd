//! Foundation types for the Meridian Mortgage Engine.
//!
//! # Money & rate types (Task 1.2)
//! [`Cents`], [`BasisPoints`], [`PriceTicks`], [`LtvBasisPoints`],
//! [`DtiBasisPoints`], [`CreditScore`]
//!
//! # Identifier types (Task 1.3)
//! [`FipsCode`], [`StateCode`], [`MlsListingKey`], [`LenderId`],
//! [`LoanCasefileId`], [`ScenarioId`], [`AnalysisId`]
//!
//! # Error hierarchy (Task 1.4)
//! [`ParseError`], [`IngestionError`], [`EligibilityError`],
//! [`SolverError`], [`ComplianceError`]
//!
//! # Common enumerations (Task 1.5)
//! [`ProgramCode`], [`LoanProduct`], [`PropertyType`], [`Occupancy`],
//! [`LoanPurpose`], [`AmortizationType`], [`LockPeriod`], [`LienPriority`],
//! [`BalanceType`], [`Tier`], [`MiCoverageType`], [`AusType`]
//!
//! # Term primitives (Task 1.6)
//! [`TermBand`], [`TermMonths`]
//!
//! # Scenario primitives (Task 1.7)
//! [`ScenarioKey`] — 8-byte packed scenario identifier
//! [`GoalMask`] — u64 bitflags for optimization goals (34 consumer + investor goals)

// ── Task 1.2: Money & rate types ─────────────────────────────────────────────
mod basis_points;
mod cents;
mod credit_score;
mod dti;
mod ltv;
mod price_ticks;

// ── Task 1.3: Identifier types ────────────────────────────────────────────────
mod analysis_id;
mod fips_code;
mod lender_id;
mod loan_casefile_id;
mod mls_listing_key;
mod scenario_id;
mod state_code;

// ── Shared validation error ───────────────────────────────────────────────────
mod error;

// ── Task 1.4: Domain error hierarchy ─────────────────────────────────────────
pub mod errors;

// ── Task 1.5: Common enumerations ────────────────────────────────────────────
pub mod enums;

// ── Task 1.6: Term primitives ─────────────────────────────────────────────────
mod term_band;
mod term_months;

// ── Task 1.7: Scenario primitives ─────────────────────────────────────────────
mod goal_mask;
mod scenario_key;

// ── Re-exports: money & rate ──────────────────────────────────────────────────
pub use basis_points::BasisPoints;
pub use cents::Cents;
pub use credit_score::CreditScore;
pub use dti::DtiBasisPoints;
pub use ltv::LtvBasisPoints;
pub use price_ticks::PriceTicks;

// ── Re-exports: identifiers ───────────────────────────────────────────────────
pub use analysis_id::AnalysisId;
pub use fips_code::FipsCode;
pub use lender_id::LenderId;
pub use loan_casefile_id::LoanCasefileId;
pub use mls_listing_key::MlsListingKey;
pub use scenario_id::ScenarioId;
pub use state_code::StateCode;

// ── Re-exports: errors ───────────────────────────────────────────────────────
pub use error::ParseError;
pub use errors::compliance::ComplianceError;
pub use errors::eligibility::EligibilityError;
pub use errors::ingestion::IngestionError;
pub use errors::solver::SolverError;

// ── Re-exports: enumerations ─────────────────────────────────────────────────
pub use enums::amortization_type::AmortizationType;
pub use enums::loan_product::LoanProduct;
pub use enums::loan_purpose::LoanPurpose;
pub use enums::misc::{AusType, BalanceType, LienPriority, LockPeriod, MiCoverageType, Tier};
pub use enums::occupancy::Occupancy;
pub use enums::program_code::ProgramCode;
pub use enums::property_type::PropertyType;

// ── Re-exports: term primitives ──────────────────────────────────────────────
pub use term_band::TermBand;
pub use term_months::TermMonths;

// ── Re-exports: scenario primitives ──────────────────────────────────────────
pub use goal_mask::GoalMask;
pub use scenario_key::ScenarioKey;
