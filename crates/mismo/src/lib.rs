//! MISMO 3.4 reference-model schema for the MorTru Engine.
//!
//! Epic 2 builds out the MISMO 3.4 container hierarchy (MESSAGE/DEAL_SETS/DEAL/LOAN, plus PARTY, COLLATERAL, QUALIFICATION, MI_DATA, CLOSING_INFORMATION). The crate is the canonical Rust representation that maps to MISMO 3.4 XML for AUS submission (Fannie Mae DU, Freddie Mac LPA).
//!
//! Task 1.1 ships only the empty crate scaffolding so the workspace bootstrap
//! is verifiable before any domain logic lands.

// Workspace lints declared in the root Cargo.toml apply here via the
// `[lints] workspace = true` directive in this crate's manifest.
