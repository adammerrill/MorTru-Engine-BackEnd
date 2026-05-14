//! RESO Data Dictionary 2.0 Property resource for listing data interchange.
//!
//! Epic 3 implements the RESO 2.0 Property resource (~150 fields actually consumed by the engine out of the 1,700+ in the full dictionary), the lookup enumerations, the OData v4 query builder, and the bidirectional RESO <-> MISMO bridge.
//!
//! Task 1.1 ships only the empty crate scaffolding so the workspace bootstrap
//! is verifiable before any domain logic lands.

// Workspace lints declared in the root Cargo.toml apply here via the
// `[lints] workspace = true` directive in this crate's manifest.
