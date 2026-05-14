//! Mortgage insurance MISMO enumeration types.
//!
//! Covers the four MI program types (FHA, VA, USDA, conventional PMI),
//! the premium renewal calculation method, and the first-premium timing.

// ── MI Program Type ───────────────────────────────────────────────────────────

/// Which mortgage insurance program applies.
///
/// Determines the upfront premium formula (if any) and the monthly premium
/// calculation path used by the amortization engine (Epic 8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MismoMiProgramType {
    /// FHA: UFMIP 1.75% upfront + declining monthly MIP.
    FhaMip,
    /// VA: funding fee upfront (0–3.30%), no monthly premium.
    VaFundingFee,
    /// USDA: 1.00% guarantee fee upfront + 0.35% annual.
    UsdaGuaranteeFee,
    /// Conventional PMI: no upfront, monthly premium based on LTV/score/plan.
    ConventionalPmi,
    /// No mortgage insurance required (LTV <= 80% or exempt).
    None,
}

impl MismoMiProgramType {
    /// Parse from a MISMO `MIPremiumSourceType` string.
    ///
    /// | MISMO value | `MismoMiProgramType` |
    /// |---|---|
    /// | `"FHAUpfrontMIP"` | `FhaMip` |
    /// | `"VAFundingFee"` | `VaFundingFee` |
    /// | `"USDAGuaranteeFee"` | `UsdaGuaranteeFee` |
    /// | `"PrivateMI"` | `ConventionalPmi` |
    /// | `"None"` or `""` | `None` |
    ///
    /// # Errors
    /// Returns `MismoError::InvalidEnum` for unrecognised values.
    pub fn try_from_str(s: &str) -> crate::Result<Self> {
        match s.trim() {
            "FHAUpfrontMIP" => Ok(Self::FhaMip),
            "VAFundingFee" => Ok(Self::VaFundingFee),
            "USDAGuaranteeFee" => Ok(Self::UsdaGuaranteeFee),
            "PrivateMI" => Ok(Self::ConventionalPmi),
            "None" | "" => Ok(Self::None),
            _ => Err(crate::MismoError::InvalidEnum {
                element: "MIPremiumSourceType",
                value: s.to_owned(),
            }),
        }
    }

    /// Returns true if this program has an upfront premium component.
    #[must_use]
    pub const fn has_upfront(self) -> bool {
        matches!(
            self,
            Self::FhaMip | Self::VaFundingFee | Self::UsdaGuaranteeFee
        )
    }

    /// Returns true if this program has a recurring monthly premium.
    #[must_use]
    pub const fn has_monthly(self) -> bool {
        matches!(
            self,
            Self::FhaMip | Self::UsdaGuaranteeFee | Self::ConventionalPmi
        )
    }
}

// ── MI Renewal Type ───────────────────────────────────────────────────────────

/// How the monthly MI premium is recalculated after the first year.
///
/// The renewal type determines which balance figure is used as the premium
/// base in the amortization engine.
///
/// The FHA purchase reference scenario uses `Declining` (FHA MIP recalculated on the
/// declining remaining balance each year).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MiRenewalType {
    /// Rate is constant and applied to the original loan balance throughout.
    Level,
    /// Rate is applied to the remaining (declining) balance each period.
    /// Standard for FHA MIP and most PMI plans.
    Declining,
    /// Rate is recalculated annually on the original balance.
    Annual,
    /// Declining for 11 years, then the premium drops (FHA 30yr <= 90% LTV).
    /// When LTV at origination > 90%, FHA MIP collects for the life of the loan
    /// (i.e., no 11-year drop — use `Declining` for those cases instead).
    ElevenYear,
}

impl MiRenewalType {
    /// Parse from a MISMO `MIPremiumRenewalType` string.
    ///
    /// # Errors
    /// Returns `MismoError::InvalidEnum` for unrecognised values.
    pub fn try_from_str(s: &str) -> crate::Result<Self> {
        match s.trim() {
            "Level" => Ok(Self::Level),
            "Declining" => Ok(Self::Declining),
            "Annual" => Ok(Self::Annual),
            "ElevenYear" => Ok(Self::ElevenYear),
            _ => Err(crate::MismoError::InvalidEnum {
                element: "MIPremiumRenewalType",
                value: s.to_owned(),
            }),
        }
    }
}

// ── MI First Premium Type ─────────────────────────────────────────────────────

/// When the first MI premium is collected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MiFirstPremiumType {
    /// Premium is collected at closing (standard for FHA UFMIP).
    AtClosing,
    /// Premium is collected with the first mortgage payment.
    FirstPayment,
    /// Premium collection is deferred (rare — lender-funded MI).
    Deferred,
}

impl MiFirstPremiumType {
    /// Parse from a MISMO `MIPaymentRemittanceType` string.
    ///
    /// # Errors
    /// Returns `MismoError::InvalidEnum` for unrecognised values.
    pub fn try_from_str(s: &str) -> crate::Result<Self> {
        match s.trim() {
            "AtClosing" => Ok(Self::AtClosing),
            "FirstPayment" => Ok(Self::FirstPayment),
            "Deferred" => Ok(Self::Deferred),
            _ => Err(crate::MismoError::InvalidEnum {
                element: "MIPaymentRemittanceType",
                value: s.to_owned(),
            }),
        }
    }
}
