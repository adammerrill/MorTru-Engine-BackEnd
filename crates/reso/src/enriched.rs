//! `PropertyEnriched` — validated, typed property record for the engine.
//!
//! `PropertyEnriched` is the output of `PropertyReso::enrich()`. It holds the
//! ~40 fields the mortgage engine consumes, all converted to domain types
//! (`types::Cents`, `types::StateCode`, etc.), plus `raw: Box<PropertyReso>`
//! which preserves all 235 RESO fields for future expansion.
//!
//! # Required fields
//!
//! `enrich()` returns `Err` if any of these are absent or invalid:
//!
//! - `ListingKey` → `types::MlsListingKey`
//! - `StandardStatus` → `ResoStandardStatus`
//! - `PropertyType` → `ResoPropertyType`
//! - `StateOrProvince` → `types::StateCode`
//!
//! All other fields are extracted on a best-effort basis and stored as `Option`.
//!
//! # FIPS resolution
//!
//! Pass a `FipsResolution` from the FCC geocoder (Task 3.7) to populate
//! `fips_code` and `tract_geoid`. Without it, both fields are `None` and
//! all geo-eligibility checks that require a FIPS code are deferred until
//! Epic 5 (geo enrichment pipeline) runs.

use std::str::FromStr;

use rust_decimal::Decimal;

use crate::{
    error::{ResoError, ResoResult},
    fcc::FipsResolution,
    lookups::{ResoPropertySubType, ResoPropertyType, ResoStandardStatus},
    property::PropertyReso,
};
use types::{Cents, MlsListingKey, StateCode};

// ── PropertyEnriched ──────────────────────────────────────────────────────────

/// Validated, typed property record assembled from `PropertyReso`.
///
/// The `raw` field preserves the original `PropertyReso` with all 235
/// RESO fields, so no data is ever discarded.
#[derive(Debug, Clone)]
pub struct PropertyEnriched {
    // ── Identity ──────────────────────────────────────────────────────────────
    pub listing_key: MlsListingKey,
    pub listing_id: Option<String>,
    pub originating_system: Option<String>,

    // ── Status ────────────────────────────────────────────────────────────────
    pub standard_status: ResoStandardStatus,
    pub is_active: bool,

    // ── Property type ─────────────────────────────────────────────────────────
    pub property_type: ResoPropertyType,
    pub property_sub_type: Option<ResoPropertySubType>,
    /// Engine domain type — drives eligibility checks.
    pub engine_type: types::PropertyType,
    /// True when sub-type is `MobileHome` — always ineligible for agency loans.
    pub is_mobile_home: bool,

    // ── Location ──────────────────────────────────────────────────────────────
    pub state: StateCode,
    pub city: Option<String>,
    pub county: Option<String>,
    /// 5-digit ZIP code (ZIP+4 stripped).
    pub postal_code: Option<String>,
    pub best_address: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    /// County FIPS — `None` until FCC geocoder resolves coordinates.
    pub fips_code: Option<types::FipsCode>,
    /// 11-digit census tract GEOID — `None` until FCC geocoder runs.
    pub tract_geoid: Option<String>,

    // ── Physical ──────────────────────────────────────────────────────────────
    /// Residential unit count (1–4). Derived from field or PropertySubType.
    pub unit_count: u8,
    pub living_area_sqft: Option<u32>,
    pub year_built: Option<u16>,
    pub is_new_construction: bool,
    pub is_attached: bool,
    pub lot_size_sqft: Option<u32>,

    // ── Rooms / amenities ─────────────────────────────────────────────────────
    pub bedrooms: Option<u8>,
    pub bathrooms_decimal: Option<Decimal>,
    pub has_garage: bool,
    pub has_pool: bool,
    pub has_basement: bool,

    // ── HOA ───────────────────────────────────────────────────────────────────
    pub hoa_yn: bool,
    pub hoa_monthly_cents: Option<Cents>,
    pub hoa_annual_cents: Option<Cents>,

    // ── Tax ───────────────────────────────────────────────────────────────────
    pub tax_annual_cents: Option<Cents>,
    pub tax_year: Option<u16>,
    pub parcel_number: Option<String>,

    // ── Pricing ───────────────────────────────────────────────────────────────
    pub list_price: Option<Cents>,
    pub close_price: Option<Cents>,
    pub days_on_market: Option<u32>,

    // ── Schools ───────────────────────────────────────────────────────────────
    pub school_district: Option<String>,
    pub elementary_school: Option<String>,
    pub high_school: Option<String>,

    // ── Flood ─────────────────────────────────────────────────────────────────
    pub flood_zone: Option<String>,
    pub flood_insurance_required: bool,

    // ── All 235 raw RESO fields ───────────────────────────────────────────────
    /// Original `PropertyReso` with all 235 fields.
    /// Access any field not promoted above via `enriched.raw.field_name`.
    pub raw: Box<PropertyReso>,
}

// ── PropertyReso::enrich() ────────────────────────────────────────────────────

impl PropertyReso {
    /// Validate and extract typed fields into [`PropertyEnriched`].
    ///
    /// # Required fields
    ///
    /// Returns `Err` if `ListingKey`, `StandardStatus`, `PropertyType`, or
    /// `StateOrProvince` are missing or contain invalid values.
    ///
    /// # FIPS resolution
    ///
    /// Pass the result of `FccClient::resolve()` to populate `fips_code` and
    /// `tract_geoid`. Pass `None` if coordinates are unavailable — both fields
    /// will be `None` until Epic 5 (geo enrichment) runs.
    ///
    /// # Year built
    ///
    /// If `YearBuilt` is present but outside the valid range (1800–2040),
    /// `year_built` is set to `None` rather than returning an error. An
    /// out-of-range year is a data quality issue, not a fatal parsing failure.
    pub fn enrich(self, fips_resolution: Option<FipsResolution>) -> ResoResult<PropertyEnriched> {
        // ── Required fields ───────────────────────────────────────────────────

        let listing_key_str = self.listing_key_required()?.to_owned();
        let listing_key =
            MlsListingKey::from_str(&listing_key_str).map_err(|_| ResoError::ParseError {
                field: "ListingKey",
                detail: format!("'{listing_key_str}' is not a valid MlsListingKey"),
            })?;

        let standard_status = self.standard_status_parsed()?;
        let property_type = self.property_type_parsed()?;
        let state = self.state_code()?;
        let engine_type = self.engine_property_type()?;

        // ── Optional fields ───────────────────────────────────────────────────

        let property_sub_type = self.property_sub_type_parsed().unwrap_or(None);
        let is_mobile_home = self.is_mobile_home_ineligible();
        let is_active = self.is_active();

        let city = self.city.clone();
        let county = self.county_name().map(str::to_owned);
        let postal_code = self.postal_code_5digit().map(str::to_owned);
        let best_address = self.best_address();
        let latitude = self.latitude;
        let longitude = self.longitude;

        // FIPS: FccResolution overrides field-derived
        let (fips_code, tract_geoid) = match fips_resolution {
            Some(r) => (Some(r.fips_code), r.tract_geoid),
            None => {
                let field_fips = self
                    .fips_from_fields()
                    .and_then(|s| types::FipsCode::from_str(&s).ok());
                (field_fips, None)
            }
        };

        let unit_count = self.residential_unit_count().unwrap_or(1);
        let living_area_sqft = self.living_area_sqft();
        // Absorb year_built range errors gracefully
        let year_built = self.year_built().unwrap_or(None);
        let is_new_construction = self.is_new_construction();
        let is_attached = self.is_attached();
        let lot_size_sqft = self.lot_size_sqft();

        let bedrooms = self.bedrooms();
        let bathrooms_decimal = self.bathrooms_decimal();
        let has_garage = self.has_garage();
        let has_pool = self.has_pool();
        let has_basement = self.has_basement();

        let hoa_yn = self.hoa_yn();
        let hoa_monthly_cents = self.hoa_monthly_cents();
        let hoa_annual_cents = self.hoa_annual_cents();

        let tax_annual_cents = self.tax_annual_cents();
        let tax_year = self.tax_year();
        let parcel_number = self.parcel_number.clone();

        let list_price = self.list_price_cents();
        let close_price = self.close_price_cents();
        let days_on_market = self.days_on_market();

        let school_district = self.school_district.clone();
        let elementary_school = self.elementary_school.clone();
        let high_school = self.high_school.clone();

        let flood_zone = self.flood_zone.clone();
        let flood_insurance_required = self.is_flood_insurance_required();

        let listing_id = self.listing_id.clone();
        let originating_system = self.originating_system_name.clone();

        Ok(PropertyEnriched {
            listing_key,
            listing_id,
            originating_system,
            standard_status,
            is_active,
            property_type,
            property_sub_type,
            engine_type,
            is_mobile_home,
            state,
            city,
            county,
            postal_code,
            best_address,
            latitude,
            longitude,
            fips_code,
            tract_geoid,
            unit_count,
            living_area_sqft,
            year_built,
            is_new_construction,
            is_attached,
            lot_size_sqft,
            bedrooms,
            bathrooms_decimal,
            has_garage,
            has_pool,
            has_basement,
            hoa_yn,
            hoa_monthly_cents,
            hoa_annual_cents,
            tax_annual_cents,
            tax_year,
            parcel_number,
            list_price,
            close_price,
            days_on_market,
            school_district,
            elementary_school,
            high_school,
            flood_zone,
            flood_insurance_required,
            raw: Box::new(self),
        })
    }
}
