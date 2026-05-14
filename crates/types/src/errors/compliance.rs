//! `ComplianceError` — errors raised when a loan scenario violates a
//! regulatory requirement.
//!
//! Compliance rejections are hard stops: the scenario cannot be delivered to
//! a borrower in its current form. Each variant references the specific
//! regulation so the compliance officer can trace the decision to its
//! authoritative source.
//!
//! ## Regulation references
//!
//! | Variant | Authority |
//! |---|---|
//! | [`Self::HoepaAprThresholdExceeded`] | 15 U.S.C. § 1602(bb); Reg Z § 1026.32 |
//! | [`Self::HoepaPointsAndFeesExceeded`] | 15 U.S.C. § 1602(bb); Reg Z § 1026.32(a)(1)(ii) |
//! | [`Self::QmSafeHarborFailed`] | Reg Z § 1026.43(e) |
//! | [`Self::AtrFailed`] | Reg Z § 1026.43(c) |
//! | [`Self::StateLicensingRequirementNotMet`] | State-specific NMLS regulations |
//! | [`Self::FloodZoneRequirementNotMet`] | 42 U.S.C. § 4012a; 12 CFR § 22 |

use thiserror::Error;

/// A loan scenario violates a consumer-protection or licensing regulation.
#[derive(Debug, Error)]
pub enum ComplianceError {
    /// The APR exceeds the HOEPA Section 32 threshold (Reg Z § 1026.32).
    /// For first-lien loans the trigger is APOR + 6.5 percentage points.
    /// Both values are stored as basis-points at 0.001% resolution
    /// (matching [`crate::BasisPoints`] storage).
    #[error(
        "HOEPA APR trigger: APR of {apr_bps} bps ({apr_display:.3}%) exceeds \
         the threshold of {threshold_bps} bps ({threshold_display:.3}%) \
         (Reg Z § 1026.32)"
    )]
    HoepaAprThresholdExceeded {
        apr_bps: u32,
        apr_display: f64,
        threshold_bps: u32,
        threshold_display: f64,
    },

    /// The total points and fees exceed the HOEPA Section 32 threshold
    /// (Reg Z § 1026.32(a)(1)(ii)). All cent values are in the engine's
    /// canonical [`crate::Cents`] units.
    #[error(
        "HOEPA points-and-fees trigger: ${fee_dollars:.2} fees exceed the \
         ${threshold_dollars:.2} threshold (Reg Z § 1026.32(a)(1)(ii))"
    )]
    HoepaPointsAndFeesExceeded {
        fee_dollars: f64,
        threshold_dollars: f64,
    },

    /// The loan does not qualify for QM safe-harbor or rebuttable-presumption
    /// status under Reg Z § 1026.43(e). `reason` identifies which prong of
    /// the QM test failed (e.g., `"DTI exceeds 43% without AUS approval"`).
    #[error("QM safe-harbor failed: {reason} (Reg Z § 1026.43(e))")]
    QmSafeHarborFailed { reason: String },

    /// The loan fails the Ability-to-Repay rule under Reg Z § 1026.43(c).
    /// `reason` identifies the specific ATR factor that was not satisfied.
    #[error("ATR evaluation failed: {reason} (Reg Z § 1026.43(c))")]
    AtrFailed { reason: String },

    /// The lender is not licensed to originate in the property's state, or a
    /// state-specific product restriction prohibits this loan. `state` is the
    /// 2-letter postal code; `requirement` describes the unmet condition.
    #[error(
        "state licensing requirement not met in {state}: {requirement}"
    )]
    StateLicensingRequirementNotMet { state: String, requirement: String },

    /// The property is in a Special Flood Hazard Area and the required flood
    /// insurance or disclosure has not been satisfied (42 U.S.C. § 4012a).
    /// `fips` is the 5-digit county FIPS code; `requirement` describes
    /// what is missing.
    #[error(
        "flood zone requirement not met for FIPS {fips}: {requirement} \
         (42 U.S.C. § 4012a)"
    )]
    FloodZoneRequirementNotMet { fips: String, requirement: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compliance_error_hoepa_apr_display() {
        let err = ComplianceError::HoepaAprThresholdExceeded {
            apr_bps: 13_500,
            apr_display: 13.500,
            threshold_bps: 12_875,
            threshold_display: 12.875,
        };
        let msg = err.to_string();
        assert!(msg.contains("13500"), "{msg}");
        assert!(msg.contains("12875"), "{msg}");
        assert!(msg.contains("1026.32"), "{msg}");
    }

    #[test]
    fn test_compliance_error_hoepa_fees_display() {
        let err = ComplianceError::HoepaPointsAndFeesExceeded {
            fee_dollars: 14_300.00,
            threshold_dollars: 12_456.78,
        };
        let msg = err.to_string();
        assert!(msg.contains("14300"), "{msg}");
        assert!(msg.contains("12456"), "{msg}");
        assert!(msg.contains("1026.32"), "{msg}");
    }

    #[test]
    fn test_compliance_error_qm_display() {
        let err = ComplianceError::QmSafeHarborFailed {
            reason: "DTI of 47.25% exceeds 43% without AUS approval".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("47.25%"), "{msg}");
        assert!(msg.contains("1026.43"), "{msg}");
    }

    #[test]
    fn test_compliance_error_atr_display() {
        let err = ComplianceError::AtrFailed {
            reason: "no verified income documentation provided".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("no verified income"), "{msg}");
        assert!(msg.contains("1026.43"), "{msg}");
    }

    #[test]
    fn test_compliance_error_licensing_display() {
        let err = ComplianceError::StateLicensingRequirementNotMet {
            state: "NY".to_string(),
            requirement: "lender NMLS license not active in New York".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("NY"), "{msg}");
        assert!(msg.contains("NMLS"), "{msg}");
    }

    #[test]
    fn test_compliance_error_flood_zone_display() {
        let err = ComplianceError::FloodZoneRequirementNotMet {
            fips: "06037".to_string(),
            requirement: "flood insurance evidence not present".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("06037"), "{msg}");
        assert!(msg.contains("flood insurance"), "{msg}");
        assert!(msg.contains("4012a"), "{msg}");
    }
}
