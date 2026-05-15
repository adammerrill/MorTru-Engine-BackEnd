//! Task 3.3 — Address and location extraction (Categories 5–6).

use reso::PropertyReso;

fn prop(json: &str) -> PropertyReso {
    serde_json::from_str(json).unwrap()
}

// ── State code ────────────────────────────────────────────────────────────────

#[test]
fn test_state_code_tx_parses() {
    let p = prop(r#"{"StateOrProvince":"TX"}"#);
    assert_eq!(p.state_code().unwrap(), types::StateCode::TX);
}

#[test]
fn test_state_code_lowercase_accepted() {
    let p = prop(r#"{"StateOrProvince":"tx"}"#);
    assert_eq!(p.state_code().unwrap(), types::StateCode::TX);
}

#[test]
fn test_state_code_california() {
    let p = prop(r#"{"StateOrProvince":"CA"}"#);
    assert_eq!(p.state_code().unwrap(), types::StateCode::CA);
}

#[test]
fn test_state_code_absent_returns_missing_field() {
    let p = prop(r#"{"ListingKey":"x"}"#);
    assert!(matches!(
        p.state_code(),
        Err(reso::ResoError::MissingField {
            field: "StateOrProvince"
        })
    ));
}

#[test]
fn test_state_code_invalid_returns_invalid_lookup() {
    let p = prop(r#"{"StateOrProvince":"XX"}"#);
    assert!(matches!(
        p.state_code(),
        Err(reso::ResoError::InvalidLookup { .. })
    ));
}

#[test]
fn test_has_valid_state_true() {
    assert!(prop(r#"{"StateOrProvince":"TX"}"#).has_valid_state());
}

#[test]
fn test_has_valid_state_false_for_unknown() {
    assert!(!prop(r#"{"StateOrProvince":"ZZ"}"#).has_valid_state());
}

// ── Postal code ───────────────────────────────────────────────────────────────

#[test]
fn test_postal_code_5digit_plain() {
    let p = prop(r#"{"PostalCode":"78640"}"#);
    assert_eq!(p.postal_code_5digit(), Some("78640"));
}

#[test]
fn test_postal_code_5digit_strips_plus4() {
    let p = prop(r#"{"PostalCode":"78640-1234"}"#);
    assert_eq!(p.postal_code_5digit(), Some("78640"));
}

#[test]
fn test_postal_code_5digit_absent_returns_none() {
    assert_eq!(prop(r#"{"ListingKey":"x"}"#).postal_code_5digit(), None);
}

#[test]
fn test_postal_code_5digit_trims_whitespace() {
    let p = prop(r#"{"PostalCode":" 78640 "}"#);
    assert_eq!(p.postal_code_5digit(), Some("78640"));
}

// ── County ────────────────────────────────────────────────────────────────────

#[test]
fn test_county_name_present() {
    let p = prop(r#"{"CountyOrParish":"Hays"}"#);
    assert_eq!(p.county_name(), Some("Hays"));
}

#[test]
fn test_county_name_trims_whitespace() {
    let p = prop(r#"{"CountyOrParish":" Hays County "}"#);
    assert_eq!(p.county_name(), Some("Hays County"));
}

#[test]
fn test_county_name_absent_returns_none() {
    assert_eq!(prop(r#"{"ListingKey":"x"}"#).county_name(), None);
}

// ── Street address assembly ───────────────────────────────────────────────────

#[test]
fn test_best_address_prefers_unparsed_address() {
    let p = prop(r#"{"UnparsedAddress":"123 Main St","StreetName":"Main","StreetNumber":"456"}"#);
    assert_eq!(p.best_address().unwrap(), "123 Main St");
}

#[test]
fn test_best_address_falls_back_to_components() {
    let p = prop(r#"{"StreetNumber":"123","StreetName":"Main","StreetSuffix":"St"}"#);
    assert_eq!(p.best_address().unwrap(), "123 Main St");
}

#[test]
fn test_composed_street_full_with_dir_and_unit() {
    let p = prop(
        r#"{
        "StreetNumber":"4500",
        "StreetDirPrefix":"N",
        "StreetName":"Lamar",
        "StreetSuffix":"Blvd",
        "UnitNumber":"200",
        "UnitNumberType":"Suite"
    }"#,
    );
    assert_eq!(
        p.composed_street_address().unwrap(),
        "4500 N Lamar Blvd Suite 200"
    );
}

#[test]
fn test_composed_street_name_only() {
    let p = prop(r#"{"StreetName":"Oak"}"#);
    assert_eq!(p.composed_street_address().unwrap(), "Oak");
}

#[test]
fn test_composed_street_no_name_returns_none() {
    let p = prop(r#"{"StreetNumber":"123"}"#);
    assert_eq!(p.composed_street_address(), None);
}

#[test]
fn test_city_state_zip_all_present() {
    let p = prop(r#"{"City":"Kyle","StateOrProvince":"TX","PostalCode":"78640"}"#);
    assert_eq!(p.city_state_zip().unwrap(), "Kyle, TX 78640");
}

#[test]
fn test_city_state_zip_no_zip() {
    let p = prop(r#"{"City":"Kyle","StateOrProvince":"TX"}"#);
    assert_eq!(p.city_state_zip().unwrap(), "Kyle, TX");
}

#[test]
fn test_city_state_zip_state_only() {
    let p = prop(r#"{"StateOrProvince":"TX"}"#);
    assert_eq!(p.city_state_zip().unwrap(), "TX");
}

#[test]
fn test_city_state_zip_all_absent_returns_none() {
    assert_eq!(prop(r#"{"ListingKey":"x"}"#).city_state_zip(), None);
}

// ── Geographic coordinates ────────────────────────────────────────────────────

#[test]
fn test_coordinates_both_present() {
    let p = prop(r#"{"Latitude":30.0394,"Longitude":-97.8772}"#);
    assert_eq!(p.coordinates(), Some((30.0394, -97.8772)));
}

#[test]
fn test_coordinates_one_missing_returns_none() {
    assert_eq!(prop(r#"{"Latitude":30.0}"#).coordinates(), None);
    assert_eq!(prop(r#"{"Longitude":-97.0}"#).coordinates(), None);
}

#[test]
fn test_has_coordinates_true() {
    assert!(prop(r#"{"Latitude":30.0394,"Longitude":-97.8772}"#).has_coordinates());
}

#[test]
fn test_validated_coordinates_valid_us_property() {
    let p = prop(r#"{"Latitude":30.0394,"Longitude":-97.8772}"#);
    let (lat, lon) = p.validated_coordinates().unwrap();
    assert!((lat - 30.0394).abs() < 1e-6);
    assert!((lon - (-97.8772)).abs() < 1e-6);
}

#[test]
fn test_validated_coordinates_null_island_rejected() {
    let p = prop(r#"{"Latitude":0.0,"Longitude":0.0}"#);
    assert!(matches!(
        p.validated_coordinates(),
        Err(reso::ResoError::InvalidCoordinate { .. })
    ));
}

#[test]
fn test_validated_coordinates_out_of_bounds_lat() {
    let p = prop(r#"{"Latitude":91.0,"Longitude":-97.0}"#);
    assert!(matches!(
        p.validated_coordinates(),
        Err(reso::ResoError::InvalidCoordinate { .. })
    ));
}

#[test]
fn test_validated_coordinates_absent_returns_missing_field() {
    let p = prop(r#"{"ListingKey":"x"}"#);
    assert!(matches!(
        p.validated_coordinates(),
        Err(reso::ResoError::MissingField { .. })
    ));
}

#[test]
fn test_has_valid_coordinates_true() {
    assert!(prop(r#"{"Latitude":30.0394,"Longitude":-97.8772}"#).has_valid_coordinates());
}

#[test]
fn test_has_valid_coordinates_false_for_null_island() {
    assert!(!prop(r#"{"Latitude":0.0,"Longitude":0.0}"#).has_valid_coordinates());
}

// ── FIPS derivation ───────────────────────────────────────────────────────────

#[test]
fn test_fips_from_fields_via_tax_tract_geoid() {
    // TaxTract populated with 11-digit GEOID: state(2) + county(3) + tract(6)
    // "48209010905" → county FIPS "48209"
    let p = prop(r#"{"TaxTract":"48209010905"}"#);
    assert_eq!(p.fips_from_fields().unwrap(), "48209");
}

#[test]
fn test_fips_from_fields_non_geoid_returns_none() {
    let p = prop(r#"{"TaxTract":"6629"}"#);
    assert_eq!(p.fips_from_fields(), None);
}

#[test]
fn test_fips_from_fields_absent_returns_none() {
    assert_eq!(prop(r#"{"ListingKey":"x"}"#).fips_from_fields(), None);
}

#[test]
fn test_has_field_derived_fips_true() {
    assert!(prop(r#"{"TaxTract":"48209010905"}"#).has_field_derived_fips());
}

#[test]
fn test_has_field_derived_fips_false_without_geoid() {
    assert!(!prop(r#"{"ListingKey":"x"}"#).has_field_derived_fips());
}
