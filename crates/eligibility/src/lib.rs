//! Loan-product and mortgage-insurance eligibility engine.
//!
//! Epic 8 implements eligibility checks for Conventional, HomeReady, Home Possible, FHA, VA, USDA, and Bond programs. Epic 9 implements MI eligibility (LPMI, BorrowerPaidMonthly, Single Premium, Split, FHA MIP, VA Funding Fee, USDA guarantee fee). Every rule traces back to its authoritative source (Fannie Mae Selling Guide, HUD 4000.1, VA Lender's Handbook, 7 CFR 3555).
//!
//! Task 1.1 ships only the empty crate scaffolding so the workspace bootstrap
//! is verifiable before any domain logic lands.

// Workspace lints declared in the root Cargo.toml apply here via the
// `[lints] workspace = true` directive in this crate's manifest.
