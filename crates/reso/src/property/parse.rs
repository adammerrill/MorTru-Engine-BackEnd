//! Parsing and validation methods for [`PropertyReso`] — Categories 1–4.
//!
//! All methods in this file operate on the already-deserialized raw fields.
//! They never touch the wire format — that is handled by `serde` in `mod.rs`.
//!
//! # Categories covered
//!
//! - Category 1: Identity — key extraction, best-available-key logic
//! - Category 2: Timestamps — modification detection, recency helpers
//! - Category 3: Listing Status — typed status parsing, availability predicates
//! - Category 4: Property Type — PropertyType/SubType parsing, engine type mapping

use crate::{
    error::{ResoError, ResoResult},
    lookups::{ResoPropertySubType, ResoPropertyType, ResoStandardStatus},
    property::PropertyReso,
};

impl PropertyReso {
    // ── Category 1: Identity ──────────────────────────────────────────────────

    /// Return `listing_key` or `Err(ResoError::MissingField)`.
    ///
    /// `ListingKey` is the RESO-required system identifier. Every record from
    /// a compliant RESO Web API endpoint must have this field. Treat its
    /// absence as a data pipeline failure, not a graceful missing-data case.
    pub fn listing_key_required(&self) -> ResoResult<&str> {
        self.listing_key.as_deref().ok_or(ResoError::MissingField {
            field: "ListingKey",
        })
    }

    /// Best available unique identifier for this listing.
    ///
    /// Priority: `ListingKey` → `OriginatingSystemKey` → `ListingId`.
    /// Returns `Err` only if all three are absent.
    pub fn best_key(&self) -> ResoResult<&str> {
        self.listing_key
            .as_deref()
            .or(self.originating_system_key.as_deref())
            .or(self.listing_id.as_deref())
            .ok_or(ResoError::MissingField {
                field: "ListingKey/OriginatingSystemKey/ListingId",
            })
    }

    /// Human-readable MLS listing number (e.g. shown on Zillow/Realtor.com).
    ///
    /// Returns `listing_id` if present, otherwise falls back to `listing_key`.
    /// Returns `None` only if both are absent (unusual for published listings).
    pub fn display_listing_id(&self) -> Option<&str> {
        self.listing_id.as_deref().or(self.listing_key.as_deref())
    }

    /// True if this record appears to be a first-time entry with no subsequent
    /// modifications (`modification_timestamp == original_entry_timestamp`).
    ///
    /// Useful for distinguishing truly new listings from refreshed data pulls.
    /// Returns `false` if either timestamp is absent (conservative).
    pub fn is_unmodified_since_entry(&self) -> bool {
        match (&self.modification_timestamp, &self.original_entry_timestamp) {
            (Some(m), Some(o)) => m == o,
            _ => false,
        }
    }

    // ── Category 2: Timestamps ────────────────────────────────────────────────

    /// True if `modification_timestamp` is present and differs from
    /// `original_entry_timestamp`, indicating at least one field update.
    pub fn has_been_modified(&self) -> bool {
        match (&self.modification_timestamp, &self.original_entry_timestamp) {
            (Some(m), Some(o)) => m != o,
            (Some(_), None) => true, // modified but no entry timestamp (unusual)
            _ => false,
        }
    }

    /// True if the listing has a price change recorded.
    /// Uses `price_change_timestamp` as the signal.
    pub fn has_price_reduction(&self) -> bool {
        self.price_change_timestamp.is_some()
    }

    /// True if the listing status has changed since entry.
    /// Uses `status_change_timestamp` as the signal.
    pub fn has_status_changed(&self) -> bool {
        self.status_change_timestamp.is_some()
    }

    // ── Category 3: Listing Status ────────────────────────────────────────────

    /// Parse `StandardStatus` into [`ResoStandardStatus`].
    ///
    /// Returns `Err(MissingField)` if the field is absent, or
    /// `Err(InvalidLookup)` if the value is not a RESO 2.0 standard status.
    pub fn standard_status_parsed(&self) -> ResoResult<ResoStandardStatus> {
        let s = self
            .standard_status
            .as_deref()
            .ok_or(ResoError::MissingField {
                field: "StandardStatus",
            })?;
        ResoStandardStatus::from_reso_str(s)
    }

    /// True if the listing is active (accepting offers).
    ///
    /// Returns `false` if StandardStatus is absent or unparseable.
    pub fn is_active(&self) -> bool {
        self.standard_status_parsed()
            .map(|s| s == ResoStandardStatus::Active)
            .unwrap_or(false)
    }

    /// True if the listing is coming soon (not yet publicly active).
    pub fn is_coming_soon(&self) -> bool {
        self.standard_status_parsed()
            .map(|s| s == ResoStandardStatus::ComingSoon)
            .unwrap_or(false)
    }

    /// True if the listing is pending (under contract, contingencies may exist).
    pub fn is_pending(&self) -> bool {
        self.standard_status_parsed()
            .map(|s| s == ResoStandardStatus::Pending)
            .unwrap_or(false)
    }

    /// True if the transaction has closed (sold or leased).
    pub fn is_closed(&self) -> bool {
        self.standard_status_parsed()
            .map(|s| s == ResoStandardStatus::Closed)
            .unwrap_or(false)
    }

    /// True if the listing is in a status where showings are appropriate.
    ///
    /// Covers: Active, Active Under Contract, Coming Soon.
    pub fn is_available_for_viewing(&self) -> bool {
        self.standard_status_parsed()
            .map(|s| s.is_active_or_coming_soon())
            .unwrap_or(false)
    }

    /// True if the listing is no longer available (expired, canceled, withdrawn).
    pub fn is_off_market(&self) -> bool {
        matches!(
            self.standard_status_parsed(),
            Ok(ResoStandardStatus::Expired
                | ResoStandardStatus::Canceled
                | ResoStandardStatus::Withdrawn
                | ResoStandardStatus::Delete)
        )
    }

    // ── Category 4: Property Type ─────────────────────────────────────────────

    /// Parse `PropertyType` into [`ResoPropertyType`].
    pub fn property_type_parsed(&self) -> ResoResult<ResoPropertyType> {
        let s = self
            .property_type
            .as_deref()
            .ok_or(ResoError::MissingField {
                field: "PropertyType",
            })?;
        ResoPropertyType::from_reso_str(s)
    }

    /// Parse `PropertySubType` into [`ResoPropertySubType`] if present.
    ///
    /// Returns `Ok(None)` if the field is absent (not an error — many feeds
    /// omit PropertySubType for non-residential listings).
    pub fn property_sub_type_parsed(&self) -> ResoResult<Option<ResoPropertySubType>> {
        match self.property_sub_type.as_deref() {
            None => Ok(None),
            Some(s) => ResoPropertySubType::from_reso_str(s).map(Some),
        }
    }

    /// Map `PropertySubType` to the engine's `types::PropertyType`.
    ///
    /// The engine type is what drives eligibility checks. If `PropertySubType`
    /// is absent, falls back to `PropertyType` mapping:
    /// - Residential → `SingleFamilyDetached`
    /// - ResidentialIncome → `TwoUnit` (conservative — unit count refines this)
    /// - All others → `SingleFamilyDetached` (conservative)
    pub fn engine_property_type(&self) -> ResoResult<types::PropertyType> {
        if let Some(sub) = self.property_sub_type_parsed()? {
            return Ok(sub.to_engine_type());
        }

        // No sub-type — derive from top-level PropertyType
        let pt = self.property_type_parsed()?;
        let engine_type = match pt {
            ResoPropertyType::ManufacturedInPark => types::PropertyType::MobileHome,
            ResoPropertyType::ResidentialIncome => types::PropertyType::TwoUnit,
            _ => types::PropertyType::SingleFamilyDetached,
        };
        Ok(engine_type)
    }

    /// True if this is a residential listing (SFR, condo, townhouse, etc.).
    pub fn is_residential(&self) -> bool {
        matches!(
            self.property_type_parsed(),
            Ok(ResoPropertyType::Residential
                | ResoPropertyType::ResidentialLease
                | ResoPropertyType::ResidentialIncome)
        )
    }

    /// True if this is a Mobile Home — always ineligible for agency financing.
    ///
    /// Checks both the PropertySubType (most reliable) and falls back to
    /// PropertyType `ManufacturedInPark` (land-lease mobile home situation).
    pub fn is_mobile_home_ineligible(&self) -> bool {
        let sub_is_mobile = self
            .property_sub_type_parsed()
            .map(|s| {
                s.map(|p| p.is_ineligible_personal_property())
                    .unwrap_or(false)
            })
            .unwrap_or(false);

        let type_is_park = matches!(
            self.property_type_parsed(),
            Ok(ResoPropertyType::ManufacturedInPark)
        );

        sub_is_mobile || type_is_park
    }

    /// Residential unit count for this property (1–4).
    ///
    /// Uses `NumberOfUnitsTotal` if available, then derives from `PropertySubType`.
    /// Returns `Err` if the unit count exceeds 4 (not a residential property
    /// in the agency program sense) or if type information is missing.
    pub fn residential_unit_count(&self) -> ResoResult<u8> {
        // Prefer the explicit field
        if let Some(n) = self.number_of_units_total {
            return match n {
                1..=4 => Ok(n as u8),
                0 => Ok(1), // 0 is sometimes used for SFR in non-conforming feeds
                _ => Err(ResoError::ParseError {
                    field: "NumberOfUnitsTotal",
                    detail: format!("{n} exceeds the 1-4 residential unit limit"),
                }),
            };
        }

        // Derive from PropertySubType
        let unit_count = match self.property_sub_type_parsed()? {
            Some(ResoPropertySubType::Duplex) => 2,
            Some(ResoPropertySubType::Triplex) => 3,
            Some(ResoPropertySubType::Quadruplex) => 4,
            Some(ResoPropertySubType::Apartment) => {
                return Err(ResoError::ParseError {
                    field: "PropertySubType",
                    detail: "Apartment has indeterminate unit count — use NumberOfUnitsTotal"
                        .into(),
                });
            }
            _ => 1, // SFR, condo, townhouse, manufactured, modular = 1 unit
        };
        Ok(unit_count)
    }
}
