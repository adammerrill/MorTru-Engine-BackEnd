//! QM categorization and ATR 8-factor compliance testing engine.
//!
//! Epic 10 implements 12 CFR 1026.43 compliance: QM prohibited features (Task 10.1), points-and-fees cap calculation (Task 10.2), APR vs APOR test for General QM (Task 10.3), ATR 8 underwriting factors (Task 10.4), QM categorization across General QM / Temporary GSE QM / Small Creditor QM / Seasoned QM (Task 10.5), and HPML status with escrow requirements (Task 10.6).
//!
//! Task 1.1 ships only the empty crate scaffolding so the workspace bootstrap
//! is verifiable before any domain logic lands.

// Workspace lints declared in the root Cargo.toml apply here via the
// `[lints] workspace = true` directive in this crate's manifest.
