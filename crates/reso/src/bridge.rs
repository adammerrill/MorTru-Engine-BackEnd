//! RESO ↔ MISMO field bridge — used by Epic 6 (ingest) to reconcile property
//! data from a RESO MLS feed with a MISMO 3.4 loan file.
//!
//! # Property type mapping
//!
//! RESO `PropertySubType` maps to MISMO `PropertyType` string:
//!
//! | RESO SubType | MISMO PropertyType |
//! |---|---|
//! | Single Family Residence | "Detached" |
//! | Condominium | "Condominium" |
//! | Townhouse | "Attached" |
//! | Cooperative / Own Your Own / Stock Cooperative | "Cooperative" |
//! | Mobile Home | "MobileHome" |
//! | Manufactured Home | "ManufacturedHousing" |
//! | Modular | "Detached" |
//! | Duplex / Triplex / Quadruplex | "Detached" + unit count from RESO |
//! | Apartment | "Condominium" |
//! | Cabin / Timeshare | "Detached" |
//!
//! # Valuation priority
//!
//! For purchase transactions:
//!   `ClosePrice` (recorded sale) > `ListPrice` (current ask)
//!
//! In practice the MISMO loan file already contains the appraised value and
//! sales contract amount; `select_valuation_price()` is used as a fallback
//! or cross-check when RESO data is the only source.

use crate::{enriched::PropertyEnriched, lookups::ResoPropertySubType};

// ── Property type mapping ─────────────────────────────────────────────────────

/// Map a RESO `PropertySubType` to the MISMO 3.4 `PropertyType` string.
///
/// These strings align with the MISMO GSEProjectClassificationType lookup
/// and the `PropertyType` element in the MISMO `SubjectProperty` container.
#[must_use]
pub const fn property_sub_type_to_mismo(sub: ResoPropertySubType) -> &'static str {
    match sub {
        ResoPropertySubType::SingleFamilyResidence => "Detached",
        ResoPropertySubType::Condominium => "Condominium",
        ResoPropertySubType::Townhouse => "Attached",
        ResoPropertySubType::Apartment => "Condominium",
        ResoPropertySubType::Cooperative
        | ResoPropertySubType::OwnYourOwn
        | ResoPropertySubType::StockCooperative => "Cooperative",
        ResoPropertySubType::Duplex => "Detached",
        ResoPropertySubType::Triplex => "Detached",
        ResoPropertySubType::Quadruplex => "Detached",
        ResoPropertySubType::MobileHome => "MobileHome",
        ResoPropertySubType::ManufacturedHome => "ManufacturedHousing",
        ResoPropertySubType::Modular => "Detached",
        ResoPropertySubType::Cabin => "Detached",
        ResoPropertySubType::Timeshare => "Detached",
    }
}

/// Map a `types::StateCode` to its 2-letter MISMO / RESO string.
///
/// MISMO uses the same 2-letter USPS state codes as RESO — this is a
/// passthrough for documentation clarity at the Epic 6 call sites.
#[must_use]
pub fn state_code_to_mismo(state: types::StateCode) -> &'static str {
    state.as_str()
}

// ── Address bridge ────────────────────────────────────────────────────────────

/// MISMO `SubjectProperty` address fields extracted from `PropertyEnriched`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MismoAddressFields {
    /// Full street address (best available — unparsed or composed).
    pub street_address: Option<String>,
    pub city: Option<String>,
    /// 2-letter state abbreviation.
    pub state: String,
    /// 5-digit ZIP code.
    pub postal_code: Option<String>,
    pub county: Option<String>,
}

/// Extract MISMO-compatible address fields from `PropertyEnriched`.
#[must_use]
pub fn enriched_to_mismo_address(p: &PropertyEnriched) -> MismoAddressFields {
    MismoAddressFields {
        street_address: p.best_address.clone(),
        city: p.city.clone(),
        state: p.state.as_str().to_owned(),
        postal_code: p.postal_code.clone(),
        county: p.county.clone(),
    }
}

// ── Valuation selection ───────────────────────────────────────────────────────

/// Select the best available valuation price from `PropertyEnriched`.
///
/// Priority: `ClosePrice` (recorded sale) → `ListPrice` (current ask).
///
/// In the ingest bridge this is used as a cross-check against the MISMO
/// appraised value and sales contract amount, not as a substitute for them.
#[must_use]
pub fn select_valuation_price(p: &PropertyEnriched) -> Option<types::Cents> {
    p.close_price.or(p.list_price)
}

/// Monthly HOA amount for inclusion in MISMO `HousingExpenseType`.
///
/// Returns `total_monthly_hoa_cents()` (primary + secondary combined).
/// The MISMO housing expense element expects a monthly dollar amount.
#[must_use]
pub fn hoa_for_mismo_expense(p: &PropertyEnriched) -> Option<types::Cents> {
    p.hoa_monthly_cents
}
