//! Fee/closing cost MISMO enumeration types.
//!
//! Defines the CFPB Loan Estimate fee section classification, paid-by
//! designation, and calculation type flag used across all Section A–H fees.

// ── Fee Section ───────────────────────────────────────────────────────────────

/// CFPB Loan Estimate fee section.
///
/// Maps to the `IntegratedDisclosureSectionType` MISMO element.
/// Sections D and I are totals (computed, not stored as fee entries).
///
/// | Section | Contents |
/// |---|---|
/// | `A` | Origination charges (lender fees, broker comp) |
/// | `B` | Services borrower did not shop for (appraisal, UFMIP, etc.) |
/// | `C` | Services borrower shopped for (title, escrow) |
/// | `E` | Government recording charges |
/// | `F` | Prepaids (HOI premium, prepaid interest) |
/// | `G` | Initial escrow payment (HOI cushion, tax cushion) |
/// | `H` | Other costs (HOA transfer, home warranty, etc.) |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FeeSection {
    /// Section A — Origination Charges.
    A,
    /// Section B — Services Borrower Did Not Shop For.
    B,
    /// Section C — Services Borrower Did Shop For.
    C,
    /// Section E — Taxes and Other Government Fees.
    E,
    /// Section F — Prepaids.
    F,
    /// Section G — Initial Escrow Payment at Closing.
    G,
    /// Section H — Other.
    H,
}

impl FeeSection {
    /// Parse from a MISMO `IntegratedDisclosureSectionType` string.
    ///
    /// | MISMO value | `FeeSection` |
    /// |---|---|
    /// | `"LoanCosts_OriginationCharges"` | `A` |
    /// | `"LoanCosts_ServicesNotShoppedFor"` | `B` |
    /// | `"LoanCosts_ServicesShoppedFor"` | `C` |
    /// | `"OtherCosts_TaxesAndGovernmentFees"` | `E` |
    /// | `"OtherCosts_Prepaids"` | `F` |
    /// | `"OtherCosts_InitialEscrowPayment"` | `G` |
    /// | `"OtherCosts_Other"` | `H` |
    ///
    /// # Errors
    /// Returns `MismoError::InvalidEnum` for unrecognised values.
    pub fn try_from_str(s: &str) -> crate::Result<Self> {
        match s.trim() {
            "LoanCosts_OriginationCharges" | "SectionA" | "A" => Ok(Self::A),

            "LoanCosts_ServicesNotShoppedFor" | "SectionB" | "B" => Ok(Self::B),

            "LoanCosts_ServicesShoppedFor" | "SectionC" | "C" => Ok(Self::C),

            "OtherCosts_TaxesAndGovernmentFees" | "SectionE" | "E" => Ok(Self::E),

            "OtherCosts_Prepaids" | "SectionF" | "F" => Ok(Self::F),

            "OtherCosts_InitialEscrowPayment" | "SectionG" | "G" => Ok(Self::G),

            "OtherCosts_Other" | "SectionH" | "H" => Ok(Self::H),

            _ => Err(crate::MismoError::InvalidEnum {
                element: "IntegratedDisclosureSectionType",
                value: s.to_owned(),
            }),
        }
    }

    /// Returns the single-letter section label for display.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::A => "A",
            Self::B => "B",
            Self::C => "C",
            Self::E => "E",
            Self::F => "F",
            Self::G => "G",
            Self::H => "H",
        }
    }

    /// Returns true if this section's fees count toward Loan Costs (D).
    #[must_use]
    pub const fn is_loan_cost(self) -> bool {
        matches!(self, Self::A | Self::B | Self::C)
    }

    /// Returns true if this section's fees count toward Other Costs (I).
    #[must_use]
    pub const fn is_other_cost(self) -> bool {
        matches!(self, Self::E | Self::F | Self::G | Self::H)
    }
}

// ── Fee Paid By ───────────────────────────────────────────────────────────────

/// Who pays a closing cost fee.
///
/// Maps to the MISMO `FeePaymentPaidByType` element.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeePaidBy {
    /// Borrower pays at closing or from loan proceeds.
    Borrower,
    /// Seller pays at closing (reduces net sale proceeds).
    Seller,
    /// Lender pays (lender credits, typically tied to rate).
    Lender,
    /// Third party pays (e.g. builder, gift, employer relocation).
    Other,
}

impl FeePaidBy {
    /// Parse from a MISMO `FeePaymentPaidByType` string.
    ///
    /// # Errors
    /// Returns `MismoError::InvalidEnum` for unrecognised values.
    pub fn try_from_str(s: &str) -> crate::Result<Self> {
        match s.trim() {
            "Borrower" | "BorrowerPaid" => Ok(Self::Borrower),
            "Seller" | "SellerPaid" => Ok(Self::Seller),
            "Lender" | "LenderPaid" => Ok(Self::Lender),
            "Other" | "PaidByOther" => Ok(Self::Other),
            _ => Err(crate::MismoError::InvalidEnum {
                element: "FeePaymentPaidByType",
                value: s.to_owned(),
            }),
        }
    }
}

// ── Fee Calculation Type ──────────────────────────────────────────────────────

/// How a closing cost fee amount is determined.
///
/// Used by the closing cost engine (Epic 9) to decide whether to use
/// a formula or a fixed number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeeCalculationType {
    /// Fixed dollar amount from user input or rate card.
    Numerical,
    /// Computed from loan parameters (e.g. comp_bps × loan_amount).
    Formula,
    /// Not applicable for this loan scenario.
    Unavailable,
}

impl FeeCalculationType {
    /// Parse from an engine string.
    ///
    /// # Errors
    /// Returns `MismoError::InvalidEnum` for unrecognised values.
    pub fn try_from_str(s: &str) -> crate::Result<Self> {
        match s.trim() {
            "Numerical" => Ok(Self::Numerical),
            "Formula" => Ok(Self::Formula),
            "Unavailable" => Ok(Self::Unavailable),
            _ => Err(crate::MismoError::InvalidEnum {
                element: "FeeCalculationType",
                value: s.to_owned(),
            }),
        }
    }
}
