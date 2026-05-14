//! Domain-specific error types for the Meridian Mortgage Engine.
//!
//! Each enum targets one subsystem. They live in the `types` crate so all
//! downstream crates can `use types::IngestionError` etc. without creating
//! circular dependencies.
//!
//! # Design rule
//!
//! No `anyhow::Error` in library crates. `anyhow` is reserved for the
//! `orchestrator` crate where all domain errors converge at the CLI/API
//! boundary and a single opaque error type is acceptable. In library code
//! every error must be a concrete, matchable enum so callers can make
//! programmatic decisions (e.g., retry an I/O error, surface a compliance
//! violation to the LO, log a solver convergence failure to telemetry).
//!
//! # Error chains
//!
//! Where a variant wraps an underlying error, `thiserror` automatically
//! implements `std::error::Error::source()` so the full chain is accessible:
//!
//! ```ignore
//! use std::error::Error;
//! if let Some(cause) = ingestion_err.source() {
//!     eprintln!("caused by: {cause}");
//! }
//! ```

pub mod compliance;
pub mod eligibility;
pub mod ingestion;
pub mod solver;
