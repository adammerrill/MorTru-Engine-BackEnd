//! Property enrichment: raw listing -> EnrichedProperty with FIPS, tax, HOA, HOI estimates.
//!
//! Epic 7 builds the property enrichment pipeline. Inputs are RawProperty records from Zillow/Realtor/Redfin scrapes or RESO Web API feeds; outputs are EnrichedProperty records with address parsed to MISMO-compliant components, FIPS code derived, tax normalized and validated, HOA frequency-normalized, HOI estimated, and all program-relevant limits attached.
//!
//! Task 1.1 ships only the empty crate scaffolding so the workspace bootstrap
//! is verifiable before any domain logic lands.

// Workspace lints declared in the root Cargo.toml apply here via the
// `[lints] workspace = true` directive in this crate's manifest.
