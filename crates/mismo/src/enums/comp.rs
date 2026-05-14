//! Lender compensation MISMO enumeration types.
//!
//! Under TRID (RESPA/REG Z), broker compensation must be disclosed on the
//! Loan Estimate. Borrower-paid compensation appears in Section A; lender-paid
//! compensation is disclosed on Page 3 (not in Section A).

// ── Compensation Type ─────────────────────────────────────────────────────────

/// Who funds the broker/originator compensation.
///
/// Maps to the MISMO `CompensationType` extension element.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompType {
    /// Borrower pays — deducted from borrower's funds; disclosed in Section A.
    BorrowerPaid,
    /// Lender pays — paid from rate premium; disclosed on Page 3 of the LE.
    LenderPaid,
    /// Split — part borrower, part lender (uncommon; usually structured as
    /// one of the above in practice).
    Split,
}

impl CompType {
    /// Parse from a MISMO `CompensationType` string.
    ///
    /// # Errors
    /// Returns `MismoError::InvalidEnum` for unrecognised values.
    pub fn try_from_str(s: &str) -> crate::Result<Self> {
        match s.trim() {
            "BorrowerPaid" | "Borrower" => Ok(Self::BorrowerPaid),
            "LenderPaid" | "Lender" => Ok(Self::LenderPaid),
            "Split" => Ok(Self::Split),
            _ => Err(crate::MismoError::InvalidEnum {
                element: "CompensationType",
                value: s.to_owned(),
            }),
        }
    }

    /// Returns true if this compensation type must appear in Section A
    /// of the Loan Estimate.
    #[must_use]
    pub const fn disclosed_in_section_a(self) -> bool {
        matches!(self, Self::BorrowerPaid)
    }
}

// ── Compensation Disclosure Location ─────────────────────────────────────────

/// Where the compensation is disclosed on the Loan Estimate.
///
/// TRID requires all compensation to be disclosed, but the location
/// differs based on who pays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompDisclosure {
    /// Appears as a line item in Section A of the Loan Estimate.
    /// Required for borrower-paid compensation.
    InSectionA,
    /// Appears in the "Loan Costs" table on Page 3 of the Loan Estimate.
    /// Required for lender-paid compensation.
    OnPage3,
}

impl CompDisclosure {
    /// Derive the disclosure location from the compensation type.
    #[must_use]
    pub const fn from_comp_type(comp: CompType) -> Self {
        match comp {
            CompType::BorrowerPaid => Self::InSectionA,
            CompType::LenderPaid | CompType::Split => Self::OnPage3,
        }
    }
}
