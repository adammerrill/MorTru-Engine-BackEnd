//! RESO 2.0 StandardStatus lookup — canonical listing status values.

use crate::error::ResoError;

/// RESO Data Dictionary 2.0 `StandardStatus` lookup values.
///
/// These are the ONLY valid values per the RESO standard. MLS-specific
/// status values belong in `MlsStatus` (a free-text field), not here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ResoStandardStatus {
    /// Actively listed and available for showings.
    Active,
    /// Active listing with an accepted offer pending close.
    ActiveUnderContract,
    /// Listing cancelled by the seller or listing agent.
    Canceled,
    /// Transaction completed. Sale or lease has closed.
    Closed,
    /// Not yet on the market but marketed in advance.
    ComingSoon,
    /// Record deleted from the system.
    Delete,
    /// Listing period expired without a sale.
    Expired,
    /// Listing entered but not yet complete/approved.
    Incomplete,
    /// Under contract, typically with contingencies.
    Pending,
    /// Temporarily withdrawn from the market.
    Withdrawn,
}

impl ResoStandardStatus {
    /// Parse from the RESO 2.0 canonical string (case-sensitive per spec).
    pub fn from_reso_str(s: &str) -> Result<Self, ResoError> {
        match s {
            "Active" => Ok(Self::Active),
            "Active Under Contract" | "ActiveUnderContract" => Ok(Self::ActiveUnderContract),
            "Canceled" => Ok(Self::Canceled),
            "Closed" => Ok(Self::Closed),
            "Coming Soon" | "ComingSoon" => Ok(Self::ComingSoon),
            "Delete" => Ok(Self::Delete),
            "Expired" => Ok(Self::Expired),
            "Incomplete" => Ok(Self::Incomplete),
            "Pending" => Ok(Self::Pending),
            "Withdrawn" => Ok(Self::Withdrawn),
            other => Err(ResoError::InvalidLookup {
                field: "StandardStatus",
                value: other.to_owned(),
            }),
        }
    }

    /// The RESO 2.0 canonical string value.
    #[must_use]
    pub const fn to_reso_str(self) -> &'static str {
        match self {
            Self::Active => "Active",
            Self::ActiveUnderContract => "Active Under Contract",
            Self::Canceled => "Canceled",
            Self::Closed => "Closed",
            Self::ComingSoon => "Coming Soon",
            Self::Delete => "Delete",
            Self::Expired => "Expired",
            Self::Incomplete => "Incomplete",
            Self::Pending => "Pending",
            Self::Withdrawn => "Withdrawn",
        }
    }

    /// True if listing is in a status that indicates it may still be available.
    #[must_use]
    pub const fn is_active_or_coming_soon(self) -> bool {
        matches!(self, Self::Active | Self::ComingSoon | Self::ActiveUnderContract)
    }
}
