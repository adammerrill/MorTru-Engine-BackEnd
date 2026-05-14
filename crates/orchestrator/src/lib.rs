//! Pipeline coordination: enrich -> enumerate -> prune -> solve -> amortize -> rank.
//!
//! Epic 15 builds the Engine struct that owns all the Arc<...> stores (rate sheets, LLPAs, MI rates, reference data, ML models) and orchestrates the full analysis pipeline. Hot reload of rate sheets and models is coordinated via arc_swap so in-flight analyses see a stable snapshot while new analyses pick up the new data.
//!
//! Task 1.1 ships only the empty crate scaffolding so the workspace bootstrap
//! is verifiable before any domain logic lands.

// Workspace lints declared in the root Cargo.toml apply here via the
// `[lints] workspace = true` directive in this crate's manifest.
