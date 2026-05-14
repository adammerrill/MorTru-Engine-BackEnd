//! HTTP API (Axum): analyze, batch, rate-sheet upload, lender admin.
//!
//! Epic 15 also builds the Axum HTTP API exposing /v1/analyze, /v1/batch, /v1/rate-sheets, /v1/lenders, /v1/health, and /v1/metrics. The analyze handler runs the CPU-bound engine via spawn_blocking to avoid stalling the async runtime. Authentication is JWT-based with per-lender authorization scoping; observability is structured logging via tracing plus Prometheus metrics.
//!
//! Task 1.1 ships only the empty crate scaffolding so the workspace bootstrap
//! is verifiable before any domain logic lands.

// Workspace lints declared in the root Cargo.toml apply here via the
// `[lints] workspace = true` directive in this crate's manifest.
