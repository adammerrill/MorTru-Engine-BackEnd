//! TRID-compliant amortization schedules and multi-goal Pareto ranking.
//!
//! Epic 14 builds the amortization engine with program-specific MI cancellation rules (HPA-driven 78% LTV for conventional, 132-month or life-of-loan for FHA based on initial LTV, no recurring for VA, annual recalculation for USDA). It computes horizon costs for the borrower's hold period and ranks every qualified scenario against every enabled goal in the GoalMask, producing both per-goal top-N lists and the Pareto-optimal frontier.
//!
//! Task 1.1 ships only the empty crate scaffolding so the workspace bootstrap
//! is verifiable before any domain logic lands.

// Workspace lints declared in the root Cargo.toml apply here via the
// `[lints] workspace = true` directive in this crate's manifest.
