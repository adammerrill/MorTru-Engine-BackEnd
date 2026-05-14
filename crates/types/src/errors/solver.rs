//! `SolverError` — errors raised by the pricing solver, APR calculator,
//! and amortisation engine.
//!
//! Unlike [`super::eligibility::EligibilityError`], which is a clean policy
//! rejection, a `SolverError` indicates that the engine itself could not
//! complete a computation. These errors typically surface in telemetry and
//! operator dashboards rather than in borrower-facing messaging.

use thiserror::Error;

/// An error occurred inside the pricing solver or amortisation engine.
#[derive(Debug, Error)]
pub enum SolverError {
    /// The rate sheet contained no row matching the scenario's combination
    /// of program, term band, FICO band, and LTV band. This usually means
    /// the product was requested for a term or credit tier the lender has
    /// not priced.
    #[error(
        "no rate found in sheet for scenario key `{scenario_key}` \
         — the lender may not offer this product at this term/FICO/LTV combination"
    )]
    RateNotFound { scenario_key: String },

    /// The Newton–Raphson APR iteration failed to converge within the
    /// allowed number of steps. `last_residual` is the absolute value of
    /// the final residual (the closer to 0.0, the nearer the solver was
    /// to convergence before giving up).
    #[error(
        "APR iteration did not converge after {iterations} steps \
         (last residual = {last_residual:.6e}); check for degenerate cash flows"
    )]
    AprIterationLimitExceeded { iterations: u32, last_residual: f64 },

    /// Construction of an amortisation schedule failed. `term_months` is
    /// the requested term; `reason` gives the underlying cause.
    #[error("amortisation schedule failed for {term_months}-month term: {reason}")]
    AmortizationFailed { term_months: u32, reason: String },

    /// The scenario passed to the solver was internally inconsistent and
    /// cannot produce a valid result (e.g., a loan amount of $0, or a
    /// rate of 0.000%).
    #[error("invalid scenario: {reason}")]
    InvalidScenario { reason: String },

    /// An intermediate value overflowed the representable range. This
    /// should only occur with pathologically large inputs; `context`
    /// names the computation that overflowed.
    #[error("numerical overflow in {context}")]
    NumericalOverflow { context: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solver_error_rate_not_found_display() {
        let err = SolverError::RateNotFound {
            scenario_key: "CONV30Y_720_9500".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("CONV30Y_720_9500"), "{msg}");
    }

    #[test]
    fn test_solver_error_apr_convergence_display() {
        let err = SolverError::AprIterationLimitExceeded {
            iterations: 100,
            last_residual: 1.23e-4,
        };
        let msg = err.to_string();
        assert!(msg.contains("100"), "{msg}");
        // residual should appear in scientific notation
        assert!(msg.contains("1.23"), "{msg}");
    }

    #[test]
    fn test_solver_error_amortization_failed_display() {
        let err = SolverError::AmortizationFailed {
            term_months: 360,
            reason: "negative remaining balance at month 359".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("360"), "{msg}");
        assert!(msg.contains("negative remaining balance"), "{msg}");
    }

    #[test]
    fn test_solver_error_invalid_scenario_display() {
        let err = SolverError::InvalidScenario {
            reason: "loan amount is zero".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("loan amount is zero"), "{msg}");
    }

    #[test]
    fn test_solver_error_numerical_overflow_display() {
        let err = SolverError::NumericalOverflow {
            context: "monthly_payment * term_months".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("monthly_payment * term_months"), "{msg}");
    }
}
