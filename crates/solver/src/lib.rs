//! Circular pricing solver and TRID-compliant fee worksheet.
//!
//! Epic 13 builds the circular pricing solver in three modes: Mode A (payment-target), Mode B (cash-target), and Mode C (hybrid maximum-purchasing-power). The solver produces a fully-populated TRID Loan Estimate fee worksheet with sections A through H, an APR calculated via Newton-Raphson to Reg Z precision, MI applied recursively (including financed UFMIP/funding-fee/guarantee-fee), and net pricing cap enforcement.
//!
//! Task 1.1 ships only the empty crate scaffolding so the workspace bootstrap
//! is verifiable before any domain logic lands.

// Workspace lints declared in the root Cargo.toml apply here via the
// `[lints] workspace = true` directive in this crate's manifest.
