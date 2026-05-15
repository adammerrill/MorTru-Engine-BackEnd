//! Tasks 3.8 + 3.9 — PropertyEnriched + RESO ↔ MISMO bridge.

use reso::{
    bridge::{
        enriched_to_mismo_address, hoa_for_mismo_expense, property_sub_type_to_mismo,
        select_valuation_price, state_code_to_mismo,
    },
    fcc::{parse_fcc_response, FipsResolution},
    PropertyReso, ResoError, ResoPropertySubType,
};
use types::{Cents, StateCode};

fn make_prop(json: &str) -> PropertyReso {
    serde_json::from_str(json).unwrap()
}

/// Minimal valid SFR property for Kyle TX — all required fields present.
fn kyle_tx_sfr() -> PropertyReso {
    make_prop(
        r#"{
        "ListingKey": "3yd-ABORORTX-12345678",
        "StandardStatus": "Active",
        "PropertyType": "Residential",
        "PropertySubType": "Single Family Residence",
        "StateOrProvince": "TX",
        "City": "Kyle",
        "PostalCode": "78640-1234",
        "CountyOrParish": "Hays",
        "ListPrice": 459000.0,
        "LivingArea": 2345.0,
        "BedroomsTotal": 4,
        "BathroomsTotalDecimal": 2.5,
        "YearBuilt": 2018,
        "NewConstructionYN": false,
        "GarageYN": true,
        "AssociationYN": true,
        "AssociationFee": 75.0,
        "AssociationFeeFrequency": "Monthly",
        "TaxAnnualAmount": 8900.0,
        "TaxYear": 2024,
        "ParcelNumber": "1234567",
        "TaxTract": "48209010905",
        "FloodZone": "X",
        "SchoolDistrict": "Hays CISD",
        "ElementarySchool": "Laura B. Negley",
        "HighSchool": "Lehman",
        "Latitude": 30.0394,
        "Longitude": -97.8772
    }"#,
    )
}

fn kyle_fips_resolution() -> FipsResolution {
    parse_fcc_response(
        r#"{
        "status": "OK",
        "County": { "FIPS": "48209", "name": "Hays" },
        "State":  { "FIPS": "48", "code": "TX", "name": "Texas" },
        "Block":  { "FIPS": "482090109053009" }
    }"#,
    )
    .unwrap()
}

// ════════════════════════════════════════════════════════════════════════════
// TASK 3.8 — PropertyEnriched + PropertyReso::enrich()
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn test_enrich_kyle_tx_sfr_succeeds() {
    let e = kyle_tx_sfr().enrich(None).unwrap();
    assert_eq!(e.listing_key.as_str(), "3yd-ABORORTX-12345678");
    assert_eq!(e.state, StateCode::TX);
    assert_eq!(e.engine_type, types::PropertyType::SingleFamilyDetached);
    assert!(!e.is_mobile_home);
    assert!(e.is_active);
}

#[test]
fn test_enrich_missing_listing_key_returns_error() {
    let p = make_prop(
        r#"{"StandardStatus":"Active","PropertyType":"Residential","StateOrProvince":"TX"}"#,
    );
    assert!(matches!(
        p.enrich(None),
        Err(ResoError::MissingField {
            field: "ListingKey"
        })
    ));
}

#[test]
fn test_enrich_missing_status_returns_error() {
    let p = make_prop(r#"{"ListingKey":"k1","PropertyType":"Residential","StateOrProvince":"TX"}"#);
    assert!(matches!(
        p.enrich(None),
        Err(ResoError::MissingField {
            field: "StandardStatus"
        })
    ));
}

#[test]
fn test_enrich_missing_property_type_returns_error() {
    let p = make_prop(r#"{"ListingKey":"k1","StandardStatus":"Active","StateOrProvince":"TX"}"#);
    assert!(matches!(
        p.enrich(None),
        Err(ResoError::MissingField {
            field: "PropertyType"
        })
    ));
}

#[test]
fn test_enrich_missing_state_returns_error() {
    let p =
        make_prop(r#"{"ListingKey":"k1","StandardStatus":"Active","PropertyType":"Residential"}"#);
    assert!(matches!(
        p.enrich(None),
        Err(ResoError::MissingField {
            field: "StateOrProvince"
        })
    ));
}

#[test]
fn test_enrich_mobile_home_sets_ineligible_flag() {
    let p = make_prop(
        r#"{
        "ListingKey":"k1","StandardStatus":"Active",
        "PropertyType":"Residential","PropertySubType":"Mobile Home",
        "StateOrProvince":"TX"
    }"#,
    );
    let e = p.enrich(None).unwrap();
    assert!(e.is_mobile_home);
    assert_eq!(e.engine_type, types::PropertyType::MobileHome);
}

#[test]
fn test_enrich_manufactured_home_not_ineligible() {
    let p = make_prop(
        r#"{
        "ListingKey":"k1","StandardStatus":"Active",
        "PropertyType":"Residential","PropertySubType":"Manufactured Home",
        "StateOrProvince":"TX"
    }"#,
    );
    let e = p.enrich(None).unwrap();
    assert!(!e.is_mobile_home);
    assert_eq!(e.engine_type, types::PropertyType::ManufacturedHome);
}

#[test]
fn test_enrich_duplex_unit_count_is_two() {
    let p = make_prop(
        r#"{
        "ListingKey":"k1","StandardStatus":"Active",
        "PropertyType":"Residential","PropertySubType":"Duplex",
        "StateOrProvince":"TX"
    }"#,
    );
    let e = p.enrich(None).unwrap();
    assert_eq!(e.unit_count, 2);
    assert_eq!(e.engine_type, types::PropertyType::TwoUnit);
}

#[test]
fn test_enrich_fips_from_fcc_resolution() {
    let e = kyle_tx_sfr().enrich(Some(kyle_fips_resolution())).unwrap();
    assert_eq!(e.fips_code.unwrap().to_string(), "48209");
    assert_eq!(e.tract_geoid.as_deref(), Some("48209010905"));
}

#[test]
fn test_enrich_fips_from_field_when_no_resolution() {
    // TaxTract "48209010905" → county FIPS "48209"
    let e = kyle_tx_sfr().enrich(None).unwrap();
    assert_eq!(e.fips_code.unwrap().to_string(), "48209");
    assert_eq!(e.tract_geoid, None); // tract only comes from FCC
}

#[test]
fn test_enrich_fips_none_without_fields_or_resolution() {
    let p = make_prop(
        r#"{
        "ListingKey":"k1","StandardStatus":"Active",
        "PropertyType":"Residential","StateOrProvince":"TX"
    }"#,
    );
    let e = p.enrich(None).unwrap();
    assert!(e.fips_code.is_none());
}

#[test]
fn test_enrich_postal_code_stripped() {
    let e = kyle_tx_sfr().enrich(None).unwrap();
    assert_eq!(e.postal_code.as_deref(), Some("78640"));
}

#[test]
fn test_enrich_hoa_normalized_to_monthly() {
    let e = kyle_tx_sfr().enrich(None).unwrap();
    assert!(e.hoa_yn);
    assert_eq!(e.hoa_monthly_cents, Some(Cents(7500))); // $75/mo
    assert_eq!(e.hoa_annual_cents, Some(Cents(90000)));
}

#[test]
fn test_enrich_tax_annual() {
    let e = kyle_tx_sfr().enrich(None).unwrap();
    assert_eq!(e.tax_annual_cents, Some(Cents(890_000)));
    assert_eq!(e.tax_year, Some(2024));
    assert_eq!(e.parcel_number.as_deref(), Some("1234567"));
}

#[test]
fn test_enrich_list_price() {
    let e = kyle_tx_sfr().enrich(None).unwrap();
    assert_eq!(e.list_price, Some(Cents(45_900_000)));
}

#[test]
fn test_enrich_flood_zone_not_required() {
    let e = kyle_tx_sfr().enrich(None).unwrap();
    assert_eq!(e.flood_zone.as_deref(), Some("X"));
    assert!(!e.flood_insurance_required);
}

#[test]
fn test_enrich_flood_zone_required_for_ae() {
    let p = make_prop(
        r#"{
        "ListingKey":"k1","StandardStatus":"Active",
        "PropertyType":"Residential","StateOrProvince":"TX",
        "FloodZone":"AE"
    }"#,
    );
    let e = p.enrich(None).unwrap();
    assert!(e.flood_insurance_required);
}

#[test]
fn test_enrich_school_fields() {
    let e = kyle_tx_sfr().enrich(None).unwrap();
    assert_eq!(e.school_district.as_deref(), Some("Hays CISD"));
    assert_eq!(e.elementary_school.as_deref(), Some("Laura B. Negley"));
    assert_eq!(e.high_school.as_deref(), Some("Lehman"));
}

#[test]
fn test_enrich_raw_preserves_all_fields() {
    let e = kyle_tx_sfr().enrich(None).unwrap();
    // All 235 fields accessible via raw
    assert_eq!(e.raw.listing_key.as_deref(), Some("3yd-ABORORTX-12345678"));
    assert_eq!(e.raw.list_price.unwrap().to_string(), "459000");
    assert_eq!(e.raw.bedrooms_total, Some(4));
}

#[test]
fn test_enrich_year_built_invalid_gracefully_none() {
    // 1799 is out of range — enrich() should return None not Err
    let p = make_prop(
        r#"{
        "ListingKey":"k1","StandardStatus":"Active",
        "PropertyType":"Residential","StateOrProvince":"TX","YearBuilt":1799
    }"#,
    );
    let e = p.enrich(None).unwrap();
    assert_eq!(e.year_built, None);
}

#[test]
fn test_enrich_coordinates_preserved() {
    let e = kyle_tx_sfr().enrich(None).unwrap();
    assert!((e.latitude.unwrap() - 30.0394).abs() < 1e-4);
    assert!((e.longitude.unwrap() - (-97.8772)).abs() < 1e-4);
}

#[test]
fn test_enrich_living_area_sqft() {
    let e = kyle_tx_sfr().enrich(None).unwrap();
    assert_eq!(e.living_area_sqft, Some(2345));
}

#[test]
fn test_enrich_garage_and_rooms() {
    let e = kyle_tx_sfr().enrich(None).unwrap();
    assert!(e.has_garage);
    assert_eq!(e.bedrooms, Some(4));
}

// ════════════════════════════════════════════════════════════════════════════
// TASK 3.9 — RESO ↔ MISMO bridge
// ════════════════════════════════════════════════════════════════════════════

// ── Property type mapping ─────────────────────────────────────────────────────

#[test]
fn test_bridge_sfr_to_detached() {
    assert_eq!(
        property_sub_type_to_mismo(ResoPropertySubType::SingleFamilyResidence),
        "Detached"
    );
}

#[test]
fn test_bridge_condo_to_condominium() {
    assert_eq!(
        property_sub_type_to_mismo(ResoPropertySubType::Condominium),
        "Condominium"
    );
}

#[test]
fn test_bridge_townhouse_to_attached() {
    assert_eq!(
        property_sub_type_to_mismo(ResoPropertySubType::Townhouse),
        "Attached"
    );
}

#[test]
fn test_bridge_mobile_home_to_mismo() {
    assert_eq!(
        property_sub_type_to_mismo(ResoPropertySubType::MobileHome),
        "MobileHome"
    );
}

#[test]
fn test_bridge_manufactured_home_to_mismo() {
    assert_eq!(
        property_sub_type_to_mismo(ResoPropertySubType::ManufacturedHome),
        "ManufacturedHousing"
    );
}

#[test]
fn test_bridge_cooperative_variants_all_map() {
    for sub in [
        ResoPropertySubType::Cooperative,
        ResoPropertySubType::OwnYourOwn,
        ResoPropertySubType::StockCooperative,
    ] {
        assert_eq!(property_sub_type_to_mismo(sub), "Cooperative");
    }
}

#[test]
fn test_bridge_modular_to_detached() {
    assert_eq!(
        property_sub_type_to_mismo(ResoPropertySubType::Modular),
        "Detached"
    );
}

#[test]
fn test_bridge_all_15_subtypes_produce_nonempty_string() {
    use ResoPropertySubType::*;
    for sub in [
        SingleFamilyResidence,
        Condominium,
        Townhouse,
        Apartment,
        Cooperative,
        OwnYourOwn,
        Duplex,
        Triplex,
        Quadruplex,
        MobileHome,
        ManufacturedHome,
        Modular,
        StockCooperative,
        Timeshare,
        Cabin,
    ] {
        let s = property_sub_type_to_mismo(sub);
        assert!(!s.is_empty(), "{sub:?} produced empty string");
    }
}

#[test]
fn test_bridge_state_code_tx() {
    assert_eq!(state_code_to_mismo(StateCode::TX), "TX");
}

#[test]
fn test_bridge_state_code_ca() {
    assert_eq!(state_code_to_mismo(StateCode::CA), "CA");
}

// ── Address bridge ────────────────────────────────────────────────────────────

#[test]
fn test_bridge_address_full() {
    let e = kyle_tx_sfr().enrich(None).unwrap();
    let addr = enriched_to_mismo_address(&e);
    assert_eq!(addr.state, "TX");
    assert_eq!(addr.city.as_deref(), Some("Kyle"));
    assert_eq!(addr.postal_code.as_deref(), Some("78640"));
    assert_eq!(addr.county.as_deref(), Some("Hays"));
}

#[test]
fn test_bridge_address_state_always_present() {
    let p = make_prop(
        r#"{
        "ListingKey":"k1","StandardStatus":"Active",
        "PropertyType":"Residential","StateOrProvince":"CA"
    }"#,
    );
    let e = p.enrich(None).unwrap();
    let addr = enriched_to_mismo_address(&e);
    assert_eq!(addr.state, "CA");
}

// ── Valuation selection ───────────────────────────────────────────────────────

#[test]
fn test_bridge_valuation_prefers_close_price() {
    let p = make_prop(
        r#"{
        "ListingKey":"k1","StandardStatus":"Closed",
        "PropertyType":"Residential","StateOrProvince":"TX",
        "ListPrice":459000.0,"ClosePrice":455000.0
    }"#,
    );
    let e = p.enrich(None).unwrap();
    assert_eq!(select_valuation_price(&e), Some(Cents(45_500_000)));
}

#[test]
fn test_bridge_valuation_falls_back_to_list_price() {
    let e = kyle_tx_sfr().enrich(None).unwrap();
    assert_eq!(select_valuation_price(&e), Some(Cents(45_900_000)));
}

#[test]
fn test_bridge_valuation_none_when_no_prices() {
    let p = make_prop(
        r#"{
        "ListingKey":"k1","StandardStatus":"Active",
        "PropertyType":"Residential","StateOrProvince":"TX"
    }"#,
    );
    let e = p.enrich(None).unwrap();
    assert_eq!(select_valuation_price(&e), None);
}

// ── HOA for MISMO expense ─────────────────────────────────────────────────────

#[test]
fn test_bridge_hoa_expense_monthly_value() {
    let e = kyle_tx_sfr().enrich(None).unwrap();
    assert_eq!(hoa_for_mismo_expense(&e), Some(Cents(7500)));
}

#[test]
fn test_bridge_hoa_expense_none_when_no_hoa() {
    let p = make_prop(
        r#"{
        "ListingKey":"k1","StandardStatus":"Active",
        "PropertyType":"Residential","StateOrProvince":"TX"
    }"#,
    );
    let e = p.enrich(None).unwrap();
    assert_eq!(hoa_for_mismo_expense(&e), None);
}
