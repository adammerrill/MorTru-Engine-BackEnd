//! Epic 16 — borrower-wizard → engine contract and progressive eligibility funnel.
pub mod contract;
pub mod validation;
pub use contract::*;
pub use validation::{is_valid, valid_completed_steps, validate, WizardError};
