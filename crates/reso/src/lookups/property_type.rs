//! RESO 2.0 PropertyType and PropertySubType lookups.
//!
//! These are the canonical RESO string values. The mapping to the engine's
//! `types::PropertyType` is defined in `bridge.rs` (Task 3.12).

use crate::error::ResoError;

/// RESO 2.0 `PropertyType` lookup — the top-level property classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ResoPropertyType {
    /// 1–4 unit residential properties (the primary engine use case).
    Residential,
    /// Residential rental/lease listings.
    ResidentialLease,
    /// 5+ unit apartment buildings and income properties.
    ResidentialIncome,
    /// Commercial sale listings.
    Commercial,
    /// Commercial lease listings.
    CommercialLease,
    /// Commercial sale (alias used by some boards).
    CommercialSale,
    /// Business opportunity (goodwill/going concern sale).
    BusinessOpportunity,
    /// Agricultural / farm listings.
    Farm,
    /// Vacant land.
    Land,
    /// Manufactured housing in a park (land-lease situation).
    ManufacturedInPark,
}

impl ResoPropertyType {
    /// Parse from RESO 2.0 canonical string.
    pub fn from_reso_str(s: &str) -> Result<Self, ResoError> {
        match s {
            "Residential" => Ok(Self::Residential),
            "Residential Lease" | "ResidentialLease" => Ok(Self::ResidentialLease),
            "Residential Income" | "ResidentialIncome" => Ok(Self::ResidentialIncome),
            "Commercial" => Ok(Self::Commercial),
            "Commercial Lease" | "CommercialLease" => Ok(Self::CommercialLease),
            "Commercial Sale" | "CommercialSale" => Ok(Self::CommercialSale),
            "Business Opportunity" | "BusinessOpportunity" => Ok(Self::BusinessOpportunity),
            "Farm" => Ok(Self::Farm),
            "Land" => Ok(Self::Land),
            "Manufactured In Park" | "ManufacturedInPark" => Ok(Self::ManufacturedInPark),
            other => Err(ResoError::UnknownPropertyType {
                value: other.to_owned(),
            }),
        }
    }

    /// The RESO 2.0 canonical string value.
    #[must_use]
    pub const fn to_reso_str(self) -> &'static str {
        match self {
            Self::Residential => "Residential",
            Self::ResidentialLease => "Residential Lease",
            Self::ResidentialIncome => "Residential Income",
            Self::Commercial => "Commercial",
            Self::CommercialLease => "Commercial Lease",
            Self::CommercialSale => "Commercial Sale",
            Self::BusinessOpportunity => "Business Opportunity",
            Self::Farm => "Farm",
            Self::Land => "Land",
            Self::ManufacturedInPark => "Manufactured In Park",
        }
    }
}

/// RESO 2.0 `PropertySubType` lookup — the granular property classification.
///
/// These string values map to engine `types::PropertyType` in `bridge.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ResoPropertySubType {
    /// Detached single-family home on its own lot.
    /// RESO string: "Single Family Residence"
    SingleFamilyResidence,
    /// Condominium (individually owned unit in a multi-unit building).
    Condominium,
    /// Townhouse (typically attached, multi-story, individually owned).
    Townhouse,
    /// Apartment (typically rented, included for lease listings).
    Apartment,
    /// Cooperative (shares in a corporation that owns the building).
    Cooperative,
    /// Own Your Own (California co-op variant).
    OwnYourOwn,
    /// Two-unit property (duplex).
    Duplex,
    /// Three-unit property (triplex).
    Triplex,
    /// Four-unit property (quadruplex).
    Quadruplex,
    /// Pre-HUD mobile home or personal property title unit.
    /// → `types::PropertyType::MobileHome` — ALWAYS INELIGIBLE for agency financing.
    MobileHome,
    /// HUD-code manufactured home (post-1976) on permanent foundation.
    /// → `types::PropertyType::ManufacturedHome` — eligible for FHA/VA/USDA/Conv.
    ManufacturedHome,
    /// Factory-built modular home, treated as site-built for underwriting.
    Modular,
    /// Stock cooperative.
    StockCooperative,
    /// Timeshare property.
    Timeshare,
    /// Cabin or recreational property.
    Cabin,
}

impl ResoPropertySubType {
    /// Parse from RESO 2.0 canonical string.
    ///
    /// Handles both canonical RESO strings and common MLS feed variations
    /// (e.g. ABOR Austin uses "Single Family Resi" as shorthand).
    pub fn from_reso_str(s: &str) -> Result<Self, ResoError> {
        match s {
            "Single Family Residence" => Ok(Self::SingleFamilyResidence),
            // ABOR (Austin Board of Realtors) uses this non-standard shorthand
            "Single Family Resi" | "Single Family" | "SFR" => Ok(Self::SingleFamilyResidence),
            "Condominium" | "Condo" => Ok(Self::Condominium),
            "Townhouse" | "Town House" | "Townhome" => Ok(Self::Townhouse),
            "Apartment" => Ok(Self::Apartment),
            "Cooperative" | "Co-op" => Ok(Self::Cooperative),
            "Own Your Own" | "OwnYourOwn" => Ok(Self::OwnYourOwn),
            "Duplex" | "2 Units" => Ok(Self::Duplex),
            "Triplex" | "3 Units" => Ok(Self::Triplex),
            "Quadruplex" | "Quadplex" | "4 Units" => Ok(Self::Quadruplex),
            "Mobile Home" | "MobileHome" => Ok(Self::MobileHome),
            "Manufactured Home" | "ManufacturedHome" => Ok(Self::ManufacturedHome),
            "Modular" | "Modular Home" => Ok(Self::Modular),
            "Stock Cooperative" | "StockCooperative" => Ok(Self::StockCooperative),
            "Timeshare" => Ok(Self::Timeshare),
            "Cabin" => Ok(Self::Cabin),
            other => Err(ResoError::UnknownPropertySubType {
                value: other.to_owned(),
            }),
        }
    }

    /// The RESO 2.0 canonical string value.
    #[must_use]
    pub const fn to_reso_str(self) -> &'static str {
        match self {
            Self::SingleFamilyResidence => "Single Family Residence",
            Self::Condominium => "Condominium",
            Self::Townhouse => "Townhouse",
            Self::Apartment => "Apartment",
            Self::Cooperative => "Cooperative",
            Self::OwnYourOwn => "Own Your Own",
            Self::Duplex => "Duplex",
            Self::Triplex => "Triplex",
            Self::Quadruplex => "Quadruplex",
            Self::MobileHome => "Mobile Home",
            Self::ManufacturedHome => "Manufactured Home",
            Self::Modular => "Modular",
            Self::StockCooperative => "Stock Cooperative",
            Self::Timeshare => "Timeshare",
            Self::Cabin => "Cabin",
        }
    }

    /// True if this subtype is a mobile home (personal property, always ineligible).
    #[must_use]
    pub const fn is_ineligible_personal_property(self) -> bool {
        matches!(self, Self::MobileHome)
    }

    /// Map to the engine's `types::PropertyType`.
    #[must_use]
    pub fn to_engine_type(self) -> types::PropertyType {
        match self {
            Self::SingleFamilyResidence => types::PropertyType::SingleFamilyDetached,
            Self::Condominium => types::PropertyType::Condominium,
            Self::Townhouse => types::PropertyType::Townhouse,
            Self::Apartment => types::PropertyType::Condominium,
            Self::Cooperative | Self::OwnYourOwn | Self::StockCooperative => {
                types::PropertyType::Cooperative
            }
            Self::Duplex => types::PropertyType::TwoUnit,
            Self::Triplex => types::PropertyType::ThreeUnit,
            Self::Quadruplex => types::PropertyType::FourUnit,
            Self::MobileHome => types::PropertyType::MobileHome,
            Self::ManufacturedHome => types::PropertyType::ManufacturedHome,
            Self::Modular => types::PropertyType::Modular,
            Self::Timeshare | Self::Cabin => types::PropertyType::SingleFamilyDetached,
        }
    }
}
