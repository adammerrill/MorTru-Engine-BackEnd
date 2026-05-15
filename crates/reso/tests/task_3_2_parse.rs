//! Task 3.2 — PropertyReso parsing and validation methods (Categories 1–4).

use reso::{PropertyReso, ResoError, ResoPropertySubType, ResoPropertyType, ResoStandardStatus};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn prop(json: &str) -> PropertyReso {
    serde_json::from_str(json).unwrap()
}

fn minimal() -> PropertyReso {
    prop(
        r#"{"ListingKey":"abc123","StandardStatus":"Active","PropertyType":"Residential","PropertySubType":"Single Family Residence"}"#,
    )
}

// ── Category 1: Identity ──────────────────────────────────────────────────────

#[test]
fn test_listing_key_required_present() {
    let p = minimal();
    assert_eq!(p.listing_key_required().unwrap(), "abc123");
}

#[test]
fn test_listing_key_required_absent_returns_error() {
    let p = prop(r#"{"StandardStatus":"Active"}"#);
    assert!(matches!(
        p.listing_key_required(),
        Err(ResoError::MissingField {
            field: "ListingKey"
        })
    ));
}

#[test]
fn test_best_key_uses_listing_key_first() {
    let p = prop(r#"{"ListingKey":"lk1","OriginatingSystemKey":"osk1","ListingId":"id1"}"#);
    assert_eq!(p.best_key().unwrap(), "lk1");
}

#[test]
fn test_best_key_falls_back_to_originating_system_key() {
    let p = prop(r#"{"OriginatingSystemKey":"osk1","ListingId":"id1"}"#);
    assert_eq!(p.best_key().unwrap(), "osk1");
}

#[test]
fn test_best_key_falls_back_to_listing_id() {
    let p = prop(r#"{"ListingId":"lid1"}"#);
    assert_eq!(p.best_key().unwrap(), "lid1");
}

#[test]
fn test_best_key_all_absent_returns_error() {
    let p = prop(r#"{"StandardStatus":"Active"}"#);
    assert!(p.best_key().is_err());
}

#[test]
fn test_display_listing_id_prefers_listing_id() {
    let p = prop(r#"{"ListingKey":"lk1","ListingId":"lid1"}"#);
    assert_eq!(p.display_listing_id(), Some("lid1"));
}

#[test]
fn test_display_listing_id_falls_back_to_listing_key() {
    let p = prop(r#"{"ListingKey":"lk1"}"#);
    assert_eq!(p.display_listing_id(), Some("lk1"));
}

#[test]
fn test_display_listing_id_absent_returns_none() {
    let p = prop(r#"{"StandardStatus":"Active"}"#);
    assert_eq!(p.display_listing_id(), None);
}

// ── Category 2: Timestamps ────────────────────────────────────────────────────

#[test]
fn test_is_unmodified_since_entry_same_timestamps() {
    let p = prop(
        r#"{"ModificationTimestamp":"2025-01-01T12:00:00Z","OriginalEntryTimestamp":"2025-01-01T12:00:00Z"}"#,
    );
    assert!(p.is_unmodified_since_entry());
}

#[test]
fn test_is_unmodified_since_entry_different_timestamps() {
    let p = prop(
        r#"{"ModificationTimestamp":"2025-01-15T12:00:00Z","OriginalEntryTimestamp":"2025-01-01T12:00:00Z"}"#,
    );
    assert!(!p.is_unmodified_since_entry());
}

#[test]
fn test_is_unmodified_since_entry_absent_returns_false() {
    let p = prop(r#"{"ListingKey":"x"}"#);
    assert!(!p.is_unmodified_since_entry());
}

#[test]
fn test_has_been_modified_different_timestamps() {
    let p = prop(
        r#"{"ModificationTimestamp":"2025-01-15T00:00:00Z","OriginalEntryTimestamp":"2025-01-01T00:00:00Z"}"#,
    );
    assert!(p.has_been_modified());
}

#[test]
fn test_has_been_modified_same_timestamps_returns_false() {
    let p = prop(
        r#"{"ModificationTimestamp":"2025-01-01T00:00:00Z","OriginalEntryTimestamp":"2025-01-01T00:00:00Z"}"#,
    );
    assert!(!p.has_been_modified());
}

#[test]
fn test_has_price_reduction_with_timestamp() {
    let p = prop(r#"{"PriceChangeTimestamp":"2025-01-10T00:00:00Z"}"#);
    assert!(p.has_price_reduction());
}

#[test]
fn test_has_price_reduction_absent_returns_false() {
    let p = prop(r#"{"ListingKey":"x"}"#);
    assert!(!p.has_price_reduction());
}

#[test]
fn test_has_status_changed_with_timestamp() {
    let p = prop(r#"{"StatusChangeTimestamp":"2025-01-05T00:00:00Z"}"#);
    assert!(p.has_status_changed());
}

// ── Category 3: Listing Status ────────────────────────────────────────────────

#[test]
fn test_standard_status_active_parses() {
    let p = prop(r#"{"StandardStatus":"Active"}"#);
    assert_eq!(
        p.standard_status_parsed().unwrap(),
        ResoStandardStatus::Active
    );
}

#[test]
fn test_standard_status_closed_parses() {
    let p = prop(r#"{"StandardStatus":"Closed"}"#);
    assert_eq!(
        p.standard_status_parsed().unwrap(),
        ResoStandardStatus::Closed
    );
}

#[test]
fn test_standard_status_absent_returns_missing_field() {
    let p = prop(r#"{"ListingKey":"x"}"#);
    assert!(matches!(
        p.standard_status_parsed(),
        Err(ResoError::MissingField {
            field: "StandardStatus"
        })
    ));
}

#[test]
fn test_standard_status_invalid_returns_invalid_lookup() {
    let p = prop(r#"{"StandardStatus":"NotARealStatus"}"#);
    assert!(matches!(
        p.standard_status_parsed(),
        Err(ResoError::InvalidLookup { .. })
    ));
}

#[test]
fn test_is_active_true() {
    assert!(prop(r#"{"StandardStatus":"Active"}"#).is_active());
}

#[test]
fn test_is_active_false_for_pending() {
    assert!(!prop(r#"{"StandardStatus":"Pending"}"#).is_active());
}

#[test]
fn test_is_coming_soon() {
    assert!(prop(r#"{"StandardStatus":"Coming Soon"}"#).is_coming_soon());
}

#[test]
fn test_is_pending() {
    assert!(prop(r#"{"StandardStatus":"Pending"}"#).is_pending());
}

#[test]
fn test_is_closed() {
    assert!(prop(r#"{"StandardStatus":"Closed"}"#).is_closed());
}

#[test]
fn test_is_available_for_viewing_active() {
    assert!(prop(r#"{"StandardStatus":"Active"}"#).is_available_for_viewing());
}

#[test]
fn test_is_available_for_viewing_active_under_contract() {
    assert!(prop(r#"{"StandardStatus":"Active Under Contract"}"#).is_available_for_viewing());
}

#[test]
fn test_is_available_for_viewing_false_for_closed() {
    assert!(!prop(r#"{"StandardStatus":"Closed"}"#).is_available_for_viewing());
}

#[test]
fn test_is_off_market_expired() {
    assert!(prop(r#"{"StandardStatus":"Expired"}"#).is_off_market());
}

#[test]
fn test_is_off_market_canceled() {
    assert!(prop(r#"{"StandardStatus":"Canceled"}"#).is_off_market());
}

#[test]
fn test_is_off_market_false_for_active() {
    assert!(!prop(r#"{"StandardStatus":"Active"}"#).is_off_market());
}

// ── Category 4: Property Type ─────────────────────────────────────────────────

#[test]
fn test_property_type_residential_parses() {
    let p = prop(r#"{"PropertyType":"Residential"}"#);
    assert_eq!(
        p.property_type_parsed().unwrap(),
        ResoPropertyType::Residential
    );
}

#[test]
fn test_property_type_absent_returns_error() {
    let p = prop(r#"{"ListingKey":"x"}"#);
    assert!(matches!(
        p.property_type_parsed(),
        Err(ResoError::MissingField {
            field: "PropertyType"
        })
    ));
}

#[test]
fn test_property_sub_type_sfr_parses() {
    let p = prop(r#"{"PropertySubType":"Single Family Residence"}"#);
    assert_eq!(
        p.property_sub_type_parsed().unwrap(),
        Some(ResoPropertySubType::SingleFamilyResidence)
    );
}

#[test]
fn test_property_sub_type_absent_returns_ok_none() {
    let p = prop(r#"{"PropertyType":"Residential"}"#);
    assert_eq!(p.property_sub_type_parsed().unwrap(), None);
}

#[test]
fn test_engine_property_type_sfr() {
    let p = prop(r#"{"PropertyType":"Residential","PropertySubType":"Single Family Residence"}"#);
    assert_eq!(
        p.engine_property_type().unwrap(),
        types::PropertyType::SingleFamilyDetached
    );
}

#[test]
fn test_engine_property_type_mobile_home_ineligible() {
    let p = prop(r#"{"PropertyType":"Residential","PropertySubType":"Mobile Home"}"#);
    assert_eq!(
        p.engine_property_type().unwrap(),
        types::PropertyType::MobileHome
    );
}

#[test]
fn test_engine_property_type_duplex() {
    let p = prop(r#"{"PropertyType":"Residential","PropertySubType":"Duplex"}"#);
    assert_eq!(
        p.engine_property_type().unwrap(),
        types::PropertyType::TwoUnit
    );
}

#[test]
fn test_engine_property_type_no_subtype_residential() {
    // No PropertySubType → fall back to PropertyType mapping → SFR
    let p = prop(r#"{"PropertyType":"Residential"}"#);
    assert_eq!(
        p.engine_property_type().unwrap(),
        types::PropertyType::SingleFamilyDetached
    );
}

#[test]
fn test_is_residential_true() {
    assert!(prop(r#"{"PropertyType":"Residential"}"#).is_residential());
    assert!(prop(r#"{"PropertyType":"Residential Income"}"#).is_residential());
}

#[test]
fn test_is_residential_false_for_land() {
    assert!(!prop(r#"{"PropertyType":"Land"}"#).is_residential());
}

#[test]
fn test_is_mobile_home_ineligible_by_subtype() {
    let p = prop(r#"{"PropertyType":"Residential","PropertySubType":"Mobile Home"}"#);
    assert!(p.is_mobile_home_ineligible());
}

#[test]
fn test_is_mobile_home_ineligible_by_property_type_park() {
    let p = prop(r#"{"PropertyType":"Manufactured In Park"}"#);
    assert!(p.is_mobile_home_ineligible());
}

#[test]
fn test_is_mobile_home_ineligible_false_for_manufactured() {
    let p = prop(r#"{"PropertyType":"Residential","PropertySubType":"Manufactured Home"}"#);
    assert!(!p.is_mobile_home_ineligible());
}

#[test]
fn test_residential_unit_count_from_field() {
    let p = prop(r#"{"NumberOfUnitsTotal":2}"#);
    assert_eq!(p.residential_unit_count().unwrap(), 2);
}

#[test]
fn test_residential_unit_count_zero_treated_as_one() {
    let p = prop(r#"{"NumberOfUnitsTotal":0}"#);
    assert_eq!(p.residential_unit_count().unwrap(), 1);
}

#[test]
fn test_residential_unit_count_five_returns_error() {
    let p = prop(r#"{"NumberOfUnitsTotal":5}"#);
    assert!(p.residential_unit_count().is_err());
}

#[test]
fn test_residential_unit_count_from_duplex_subtype() {
    let p = prop(r#"{"PropertyType":"Residential","PropertySubType":"Duplex"}"#);
    assert_eq!(p.residential_unit_count().unwrap(), 2);
}

#[test]
fn test_residential_unit_count_from_quadruplex_subtype() {
    let p = prop(r#"{"PropertyType":"Residential","PropertySubType":"Quadruplex"}"#);
    assert_eq!(p.residential_unit_count().unwrap(), 4);
}

#[test]
fn test_residential_unit_count_sfr_defaults_to_one() {
    let p = prop(r#"{"PropertyType":"Residential","PropertySubType":"Single Family Residence"}"#);
    assert_eq!(p.residential_unit_count().unwrap(), 1);
}
