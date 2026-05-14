//! Machine-learning integration hooks for feasibility, warm-start, term optimization, and anomaly detection.
//!
//! Epic 12 builds the ML layer as an additive accelerator. The feasibility classifier (>=99.9% recall gate) prunes the search space before the solver runs; the warm-start regressor accelerates solver convergence from 10-20 iterations to 2-4; the term optimizer predicts in-band optima for goal-directed search; and the rate-sheet anomaly detector flags suspicious values at ingestion. Every model has a recall/accuracy gate that bypasses the model when measured performance degrades.
//!
//! Task 1.1 ships only the empty crate scaffolding so the workspace bootstrap
//! is verifiable before any domain logic lands.

// Workspace lints declared in the root Cargo.toml apply here via the
// `[lints] workspace = true` directive in this crate's manifest.
