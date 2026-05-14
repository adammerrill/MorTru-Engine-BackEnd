//! Scenario enumeration, full month-granular term expansion, and pruning pipeline.
//!
//! Epic 11 builds the scenario enumeration pipeline including the critical month-granular term expansion (Task 11.4) that produces every possible term from 120 to 360 within each band rather than just the band boundaries. Epic 12 builds the five-gate pruning pipeline (payment capacity, cash floor, loan limits, MI feasibility, net pricing cap) plus the ML integration hooks for feasibility classifier, warm-start regressor, and term optimizer.
//!
//! Task 1.1 ships only the empty crate scaffolding so the workspace bootstrap
//! is verifiable before any domain logic lands.

// Workspace lints declared in the root Cargo.toml apply here via the
// `[lints] workspace = true` directive in this crate's manifest.
