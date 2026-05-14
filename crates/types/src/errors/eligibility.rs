//! `EligibilityError` — errors raised when a loan scenario fails
//! a program eligibility requirement.
//!
//! These are **policy rejections**, not data errors. The scenario's input data
//! was valid, but the combination of borrower attributes and program guidelines
//! does not permit origination. Each variant carries enough context for the
//! pricing engine to surface a human-readable explanation to the LO.

use thiserror::Error;

/// A loan scenario failed one or more program eligibility requirements.
#[derive(Debug, Error)]
pub enum EligibilityError {
    /// The representative credit score is below the program's minimum.
    /// `program` names the guideline document (e.g. `"FNMA Selling Guide
    /// B3-5.1"`) so the LO can look up compensating factors.
    #[error(
        "credit score {score} is below the minimum {minimum} required \
         by program `{program}`"
    )]
    CreditScoreBelowMinimum {
        score: u16,
        minimum: u16,
        program: String,
    },

    /// The loan-to-value ratio exceeds the program ceiling.
    /// Both values are stored as basis points (0.01% per unit) so they
    /// can be formatted without a separate conversion.
    #[error(
        "LTV of {ltv_bps} bps ({ltv_display:.2}%) exceeds the program \
         limit of {limit_bps} bps ({limit_display:.2}%) for `{program}`"
    )]
    LtvExceedsLimit {
        ltv_bps: u32,
        ltv_display: f64,
        limit_bps: u32,
        limit_display: f64,
        program: String,
    },

    /// The debt-to-income ratio exceeds the program ceiling.
    #[error(
        "DTI of {dti_bps} bps ({dti_display:.2}%) exceeds the program \
         limit of {limit_bps} bps ({limit_display:.2}%) for `{program}`"
    )]
    DtiExceedsLimit {
        dti_bps: u32,
        dti_display: f64,
        limit_bps: u32,
        limit_display: f64,
        program: String,
    },

    /// The requested loan amount falls outside the program's floor/ceiling.
    /// All amounts are in cents (1 cent = $0.01).
    #[error(
        "loan amount ${amount_dollars:.2} is outside the allowed range \
         ${min_dollars:.2}–${max_dollars:.2} for `{program}`"
    )]
    LoanAmountOutOfRange {
        amount_dollars: f64,
        min_dollars: f64,
        max_dollars: f64,
        program: String,
    },

    /// The property type is not on the approved list for this program
    /// (e.g., condotels and co-ops are ineligible for standard conventional).
    #[error(
        "property type `{property_type}` is ineligible under program `{program}`"
    )]
    IneligiblePropertyType {
        property_type: String,
        program: String,
    },

    /// The occupancy type is not permitted by the program
    /// (e.g., investment property in a first-time-homebuyer program).
    #[error(
        "occupancy type `{occupancy}` is ineligible under program `{program}`"
    )]
    IneligibleOccupancy { occupancy: String, program: String },

    /// The borrower's post-closing liquid reserves are below the minimum
    /// required months of PITI payment. `required_months` and
    /// `available_months` are the integer month counts.
    #[error(
        "insufficient reserves: {available_months} months available, \
         {required_months} months required by `{program}`"
    )]
    InsufficientReserves {
        required_months: u32,
        available_months: u32,
        program: String,
    },

    /// A required input field was absent from the scenario. This surfaces
    /// when the caller omitted a mandatory attribute (e.g., forgot to set
    /// the county FIPS code for a program that uses county-level LLPAs).
    #[error("required field `{field}` is missing from the scenario")]
    MissingRequiredField { field: &'static str },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eligibility_error_credit_score_display() {
        let err = EligibilityError::CreditScoreBelowMinimum {
            score: 619,
            minimum: 620,
            program: "FNMA HomeReady".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("619"), "{msg}");
        assert!(msg.contains("620"), "{msg}");
        assert!(msg.contains("FNMA HomeReady"), "{msg}");
    }

    #[test]
    fn test_eligibility_error_ltv_display() {
        let err = EligibilityError::LtvExceedsLimit {
            ltv_bps: 9701,
            ltv_display: 97.01,
            limit_bps: 9700,
            limit_display: 97.00,
            program: "FHLMC Super Conforming".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("9701"), "{msg}");
        assert!(msg.contains("9700"), "{msg}");
        assert!(msg.contains("FHLMC Super Conforming"), "{msg}");
    }

    #[test]
    fn test_eligibility_error_dti_display() {
        let err = EligibilityError::DtiExceedsLimit {
            dti_bps: 4501,
            dti_display: 45.01,
            limit_bps: 4500,
            limit_display: 45.00,
            program: "VA IRRRL".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("4501"), "{msg}");
        assert!(msg.contains("VA IRRRL"), "{msg}");
    }

    #[test]
    fn test_eligibility_error_loan_amount_display() {
        let err = EligibilityError::LoanAmountOutOfRange {
            amount_dollars: 726_201.0,
            min_dollars: 50_000.0,
            max_dollars: 726_200.0,
            program: "Conforming Conventional".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("726201"), "{msg}");
        assert!(msg.contains("Conforming Conventional"), "{msg}");
    }

    #[test]
    fn test_eligibility_error_property_type_display() {
        let err = EligibilityError::IneligiblePropertyType {
            property_type: "Condotel".to_string(),
            program: "FNMA DU Refi Plus".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("Condotel"), "{msg}");
        assert!(msg.contains("FNMA DU Refi Plus"), "{msg}");
    }

    #[test]
    fn test_eligibility_error_occupancy_display() {
        let err = EligibilityError::IneligibleOccupancy {
            occupancy: "Investment".to_string(),
            program: "USDA Rural Development".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("Investment"), "{msg}");
        assert!(msg.contains("USDA Rural Development"), "{msg}");
    }

    #[test]
    fn test_eligibility_error_reserves_display() {
        let err = EligibilityError::InsufficientReserves {
            required_months: 6,
            available_months: 3,
            program: "Jumbo Prime".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains('3'), "{msg}");
        assert!(msg.contains('6'), "{msg}");
        assert!(msg.contains("Jumbo Prime"), "{msg}");
    }

    #[test]
    fn test_eligibility_error_missing_field_display() {
        let err = EligibilityError::MissingRequiredField { field: "county_fips" };
        let msg = err.to_string();
        assert!(msg.contains("county_fips"), "{msg}");
    }
}
