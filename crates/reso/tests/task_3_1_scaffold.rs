//! Task 3.1 gate tests — crate scaffold, error types, and 235-field struct.

use reso::{
    ResoError, ResoPropertySubType, ResoPropertyType, ResoStandardStatus, PropertyReso,
};

// ── Error type tests ──────────────────────────────────────────────────────────

#[test]
fn test_reso_error_missing_field_display() {
    let e = ResoError::MissingField { field: "ListingKey" };
    assert!(e.to_string().contains("ListingKey"));
}

#[test]
fn test_reso_error_invalid_lookup_display() {
    let e = ResoError::InvalidLookup {
        field: "StandardStatus",
        value: "Bogus".into(),
    };
    assert!(e.to_string().contains("Bogus"));
    assert!(e.to_string().contains("StandardStatus"));
}

#[test]
fn test_reso_error_unknown_property_type_display() {
    let e = ResoError::UnknownPropertyType { value: "Yacht".into() };
    assert!(e.to_string().contains("Yacht"));
}

#[test]
fn test_reso_error_unknown_property_subtype_display() {
    let e = ResoError::UnknownPropertySubType { value: "Houseboat".into() };
    assert!(e.to_string().contains("Houseboat"));
}

#[test]
fn test_reso_error_invalid_coordinate_display() {
    let e = ResoError::InvalidCoordinate { lat: 99.0, lon: -200.0 };
    assert!(e.to_string().contains("99"));
}

#[test]
fn test_reso_error_implements_std_error() {
    let e: &dyn std::error::Error =
        &ResoError::MissingField { field: "ListingKey" };
    let _ = e.to_string();
}

#[test]
fn test_reso_error_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<ResoError>();
}

// ── PropertyReso deserialization ──────────────────────────────────────────────

#[test]
fn test_property_reso_default_is_all_none() {
    let p = PropertyReso::default();
    assert!(p.listing_key.is_none());
    assert!(p.list_price.is_none());
    assert!(p.city.is_none());
}

#[test]
fn test_property_reso_from_minimal_json() {
    let json = r#"{"ListingKey":"abc123","City":"Kyle","StateOrProvince":"TX"}"#;
    let p: PropertyReso = serde_json::from_str(json).unwrap();
    assert_eq!(p.listing_key.as_deref(), Some("abc123"));
    assert_eq!(p.city.as_deref(), Some("Kyle"));
    assert_eq!(p.state_or_province.as_deref(), Some("TX"));
}

#[test]
fn test_property_reso_ignores_unknown_fields() {
    // Unknown fields must be silently ignored (future RESO expansion)
    let json = r#"{"ListingKey":"x","FutureResoField2030":"some_value"}"#;
    let p: PropertyReso = serde_json::from_str(json).unwrap();
    assert_eq!(p.listing_key.as_deref(), Some("x"));
}

#[test]
fn test_property_reso_absent_fields_are_none() {
    let json = r#"{"ListingKey":"x"}"#;
    let p: PropertyReso = serde_json::from_str(json).unwrap();
    assert!(p.list_price.is_none());
    assert!(p.latitude.is_none());
    assert!(p.bedrooms_total.is_none());
    assert!(p.association_fee.is_none());
}

#[test]
fn test_property_reso_boolean_field() {
    let json = r#"{"AssociationYN":true,"NewConstructionYN":false}"#;
    let p: PropertyReso = serde_json::from_str(json).unwrap();
    assert_eq!(p.association_yn, Some(true));
    assert_eq!(p.new_construction_yn, Some(false));
}

#[test]
fn test_property_reso_collection_field() {
    let json = r#"{"BuyerFinancing":["Conventional","FHA","VA"]}"#;
    let p: PropertyReso = serde_json::from_str(json).unwrap();
    let bf = p.buyer_financing.unwrap();
    assert_eq!(bf.len(), 3);
    assert!(bf.contains(&"Conventional".to_string()));
    assert!(bf.contains(&"FHA".to_string()));
}

#[test]
fn test_property_reso_decimal_field() {
    let json = r#"{"ListPrice":459000.00,"LivingArea":2345.5}"#;
    let p: PropertyReso = serde_json::from_str(json).unwrap();
    assert!(p.list_price.is_some());
    assert!(p.living_area.is_some());
}

#[test]
fn test_property_reso_roundtrip_json() {
    let json = r#"{"ListingKey":"xyz","ListPrice":459000.0,"City":"Kyle","BedroomsTotal":4,"AssociationYN":true}"#;
    let p: PropertyReso = serde_json::from_str(json).unwrap();
    let out = serde_json::to_string(&p).unwrap();
    let p2: PropertyReso = serde_json::from_str(&out).unwrap();
    assert_eq!(p.listing_key, p2.listing_key);
    assert_eq!(p.city, p2.city);
    assert_eq!(p.bedrooms_total, p2.bedrooms_total);
}

// ── StandardStatus lookup ─────────────────────────────────────────────────────

#[test]
fn test_standard_status_active_parses() {
    assert_eq!(
        ResoStandardStatus::from_reso_str("Active").unwrap(),
        ResoStandardStatus::Active
    );
}

#[test]
fn test_standard_status_closed_parses() {
    assert_eq!(
        ResoStandardStatus::from_reso_str("Closed").unwrap(),
        ResoStandardStatus::Closed
    );
}

#[test]
fn test_standard_status_all_variants_roundtrip() {
    use ResoStandardStatus::*;
    for status in [
        Active, ActiveUnderContract, Canceled, Closed, ComingSoon,
        Delete, Expired, Incomplete, Pending, Withdrawn,
    ] {
        let s = status.to_reso_str();
        let parsed = ResoStandardStatus::from_reso_str(s).unwrap();
        assert_eq!(parsed, status, "roundtrip failed for {s}");
    }
}

#[test]
fn test_standard_status_unknown_returns_error() {
    assert!(ResoStandardStatus::from_reso_str("MadeUp").is_err());
}

#[test]
fn test_standard_status_is_active_or_coming_soon() {
    assert!(ResoStandardStatus::Active.is_active_or_coming_soon());
    assert!(ResoStandardStatus::ComingSoon.is_active_or_coming_soon());
    assert!(!ResoStandardStatus::Closed.is_active_or_coming_soon());
    assert!(!ResoStandardStatus::Expired.is_active_or_coming_soon());
}

// ── PropertyType lookup ───────────────────────────────────────────────────────

#[test]
fn test_property_type_residential_parses() {
    assert_eq!(
        ResoPropertyType::from_reso_str("Residential").unwrap(),
        ResoPropertyType::Residential
    );
}

#[test]
fn test_property_type_all_variants_roundtrip() {
    use ResoPropertyType::*;
    for pt in [
        Residential, ResidentialLease, ResidentialIncome, Commercial,
        CommercialLease, CommercialSale, BusinessOpportunity, Farm, Land,
        ManufacturedInPark,
    ] {
        let s = pt.to_reso_str();
        let parsed = ResoPropertyType::from_reso_str(s).unwrap();
        assert_eq!(parsed, pt, "roundtrip failed for {s}");
    }
}

#[test]
fn test_property_type_unknown_returns_error() {
    assert!(ResoPropertyType::from_reso_str("Submarine").is_err());
}

// ── PropertySubType lookup ────────────────────────────────────────────────────

#[test]
fn test_property_subtype_sfr_canonical_parses() {
    assert_eq!(
        ResoPropertySubType::from_reso_str("Single Family Residence").unwrap(),
        ResoPropertySubType::SingleFamilyResidence
    );
}

#[test]
fn test_property_subtype_sfr_abor_variant_parses() {
    // Austin Board of Realtors uses "Single Family Resi"
    assert_eq!(
        ResoPropertySubType::from_reso_str("Single Family Resi").unwrap(),
        ResoPropertySubType::SingleFamilyResidence
    );
}

#[test]
fn test_property_subtype_mobile_home_is_ineligible() {
    let sub = ResoPropertySubType::from_reso_str("Mobile Home").unwrap();
    assert!(sub.is_ineligible_personal_property());
}

#[test]
fn test_property_subtype_manufactured_home_is_eligible() {
    let sub = ResoPropertySubType::from_reso_str("Manufactured Home").unwrap();
    assert!(!sub.is_ineligible_personal_property());
}

#[test]
fn test_property_subtype_to_engine_type_sfr() {
    let sub = ResoPropertySubType::SingleFamilyResidence;
    assert_eq!(sub.to_engine_type(), types::PropertyType::SingleFamilyDetached);
}

#[test]
fn test_property_subtype_to_engine_type_mobile_home() {
    let sub = ResoPropertySubType::MobileHome;
    assert_eq!(sub.to_engine_type(), types::PropertyType::MobileHome);
}

#[test]
fn test_property_subtype_to_engine_type_duplex() {
    assert_eq!(
        ResoPropertySubType::Duplex.to_engine_type(),
        types::PropertyType::TwoUnit
    );
}

#[test]
fn test_property_subtype_all_variants_roundtrip() {
    use ResoPropertySubType::*;
    for sub in [
        SingleFamilyResidence, Condominium, Townhouse, Apartment,
        Cooperative, OwnYourOwn, Duplex, Triplex, Quadruplex,
        MobileHome, ManufacturedHome, Modular, StockCooperative,
        Timeshare, Cabin,
    ] {
        let s = sub.to_reso_str();
        let parsed = ResoPropertySubType::from_reso_str(s).unwrap();
        assert_eq!(parsed, sub, "roundtrip failed for {s}");
    }
}

#[test]
fn test_property_subtype_unknown_returns_error() {
    assert!(ResoPropertySubType::from_reso_str("Houseboat").is_err());
}
