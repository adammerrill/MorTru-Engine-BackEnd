//! Epic 16 — borrower-wizard → engine contract and progressive eligibility funnel.
pub mod contract;
mod funnel;
pub mod validation;
pub use contract::*;
pub use funnel::{step, FunnelResponse, FunnelStage, ScenarioFunnel, StubFunnel};
pub use validation::{is_valid, valid_completed_steps, validate, WizardError};
