//! Rate sheet, LLPA, MI, and reference-data ingestion pipeline.
//!
//! Epics 4-6 build the ingestion pipeline: Excel rate-sheet block detection (Task 4.1), rate-row extraction and normalization (Tasks 4.2-4.3), LLPA grid parsing (Tasks 5.1-5.2), MI rate cards (Tasks 5.4-5.6), and reference data (conforming/FHA/USDA limits, AMI, APOR, VA funding fee schedules, HOI rates) in Epic 6.
//!
//! Task 1.1 ships only the empty crate scaffolding so the workspace bootstrap
//! is verifiable before any domain logic lands.

// Workspace lints declared in the root Cargo.toml apply here via the
// `[lints] workspace = true` directive in this crate's manifest.
