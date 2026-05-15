//! Tasks 3.6 + 3.7 — HOA/tax/pricing/listing helpers and FCC FIPS client.

use reso::{parse_fcc_response, FccClient, PropertyReso, ResoError};
use types::Cents;

fn prop(json: &str) -> PropertyReso {
    serde_json::from_str(json).unwrap()
}

// ════════════════════════════════════════════════════════════════════════════
// TASK 3.6 — HOA, tax, pricing, listing (Categories 13–21)
// ════════════════════════════════════════════════════════════════════════════

// ── HOA ───────────────────────────────────────────────────────────────────────

#[test]
fn test_hoa_yn_true() {
    assert!(prop(r#"{"AssociationYN":true}"#).hoa_yn());
}

#[test]
fn test_hoa_yn_false_when_absent() {
    assert!(!prop(r#"{"ListingKey":"x"}"#).hoa_yn());
}

#[test]
fn test_hoa_monthly_already_monthly() {
    let p = prop(r#"{"AssociationFee":75.0,"AssociationFeeFrequency":"Monthly"}"#);
    assert_eq!(p.hoa_monthly_cents(), Some(Cents(7500)));
}

#[test]
fn test_hoa_monthly_from_annual() {
    // $900/yr ÷ 12 = $75/mo = Cents(7500)
    let p = prop(r#"{"AssociationFee":900.0,"AssociationFeeFrequency":"Annually"}"#);
    assert_eq!(p.hoa_monthly_cents(), Some(Cents(7500)));
}

#[test]
fn test_hoa_monthly_from_quarterly() {
    // $225/qtr ÷ 3 = $75/mo
    let p = prop(r#"{"AssociationFee":225.0,"AssociationFeeFrequency":"Quarterly"}"#);
    assert_eq!(p.hoa_monthly_cents(), Some(Cents(7500)));
}

#[test]
fn test_hoa_monthly_from_semi_annually() {
    // $450/semi ÷ 6 = $75/mo
    let p = prop(r#"{"AssociationFee":450.0,"AssociationFeeFrequency":"SemiAnnually"}"#);
    assert_eq!(p.hoa_monthly_cents(), Some(Cents(7500)));
}

#[test]
fn test_hoa_monthly_one_time_returns_none() {
    let p = prop(r#"{"AssociationFee":500.0,"AssociationFeeFrequency":"OneTime"}"#);
    assert_eq!(p.hoa_monthly_cents(), None);
}

#[test]
fn test_hoa_monthly_absent_fee_returns_none() {
    assert_eq!(prop(r#"{"ListingKey":"x"}"#).hoa_monthly_cents(), None);
}

#[test]
fn test_hoa_annual_cents() {
    // $75/mo × 12 = $900/yr
    let p = prop(r#"{"AssociationFee":75.0,"AssociationFeeFrequency":"Monthly"}"#);
    assert_eq!(p.hoa_annual_cents(), Some(Cents(90000)));
}

#[test]
fn test_total_monthly_hoa_two_fees() {
    let p = prop(
        r#"{
        "AssociationFee":75.0,"AssociationFeeFrequency":"Monthly",
        "AssociationFee2":50.0,"AssociationFeeFrequency2":"Monthly"
    }"#,
    );
    assert_eq!(p.total_monthly_hoa_cents(), Some(Cents(12500)));
}

#[test]
fn test_total_monthly_hoa_no_fees_returns_none() {
    assert_eq!(
        prop(r#"{"ListingKey":"x"}"#).total_monthly_hoa_cents(),
        None
    );
}

// ── Tax ───────────────────────────────────────────────────────────────────────

#[test]
fn test_tax_annual_cents() {
    let p = prop(r#"{"TaxAnnualAmount":8900.0}"#);
    assert_eq!(p.tax_annual_cents(), Some(Cents(890_000)));
}

#[test]
fn test_tax_annual_cents_absent_returns_none() {
    assert_eq!(prop(r#"{"ListingKey":"x"}"#).tax_annual_cents(), None);
}

#[test]
fn test_tax_year_present() {
    assert_eq!(prop(r#"{"TaxYear":2024}"#).tax_year(), Some(2024));
}

#[test]
fn test_has_tax_exemptions_true() {
    let p = prop(r#"{"TaxExemptions":["Homestead","Senior"]}"#);
    assert!(p.has_tax_exemptions());
}

#[test]
fn test_has_tax_exemptions_empty_returns_false() {
    let p = prop(r#"{"TaxExemptions":[]}"#);
    assert!(!p.has_tax_exemptions());
}

// ── Pricing ───────────────────────────────────────────────────────────────────

#[test]
fn test_list_price_cents() {
    let p = prop(r#"{"ListPrice":459000.0}"#);
    assert_eq!(p.list_price_cents(), Some(Cents(45_900_000)));
}

#[test]
fn test_close_price_cents() {
    let p = prop(r#"{"ClosePrice":455000.0}"#);
    assert_eq!(p.close_price_cents(), Some(Cents(45_500_000)));
}

#[test]
fn test_price_reduction_amount() {
    let p = prop(r#"{"OriginalListPrice":475000.0,"ListPrice":459000.0}"#);
    // $475k - $459k = $16k reduction
    assert_eq!(p.price_reduction_amount(), Some(Cents(1_600_000)));
}

#[test]
fn test_price_reduction_no_reduction_returns_none() {
    // Same price — no reduction
    let p = prop(r#"{"OriginalListPrice":459000.0,"ListPrice":459000.0}"#);
    assert_eq!(p.price_reduction_amount(), None);
}

#[test]
fn test_is_price_reduced_true() {
    let p = prop(r#"{"OriginalListPrice":475000.0,"ListPrice":459000.0}"#);
    assert!(p.is_price_reduced());
}

#[test]
fn test_is_price_reduced_false_when_same() {
    let p = prop(r#"{"OriginalListPrice":459000.0,"ListPrice":459000.0}"#);
    assert!(!p.is_price_reduced());
}

#[test]
fn test_price_per_sqft_cents() {
    // $459,000 ÷ 2,345 sqft ≈ $195.74/sqft = Cents(19574)
    let p = prop(r#"{"ListPrice":459000.0,"LivingArea":2345.0}"#);
    let ppsf = p.price_per_sqft_cents().unwrap();
    // Allow ±1 cent rounding
    assert!(ppsf.0 >= 19573 && ppsf.0 <= 19575, "got {}", ppsf.0);
}

#[test]
fn test_days_on_market_present() {
    assert_eq!(prop(r#"{"DaysOnMarket":14}"#).days_on_market(), Some(14));
}

// ── Buyer financing flags ─────────────────────────────────────────────────────

#[test]
fn test_has_buyer_financing_fha_true() {
    let p = prop(r#"{"BuyerFinancing":["Conventional","FHA"]}"#);
    assert!(p.has_buyer_financing_fha());
}

#[test]
fn test_has_buyer_financing_va_true() {
    let p = prop(r#"{"BuyerFinancing":["VA","Cash"]}"#);
    assert!(p.has_buyer_financing_va());
}

#[test]
fn test_has_buyer_financing_usda_true() {
    let p = prop(r#"{"BuyerFinancing":["USDA","Conventional"]}"#);
    assert!(p.has_buyer_financing_usda());
}

#[test]
fn test_has_buyer_financing_cash_true() {
    assert!(prop(r#"{"BuyerFinancing":["Cash"]}"#).has_buyer_financing_cash());
}

#[test]
fn test_has_buyer_financing_absent_returns_false() {
    assert!(!prop(r#"{"ListingKey":"x"}"#).has_buyer_financing_fha());
}

#[test]
fn test_has_seller_concessions_yes() {
    assert!(prop(r#"{"Concessions":"Yes"}"#).has_seller_concessions());
}

#[test]
fn test_has_seller_concessions_case_insensitive() {
    assert!(prop(r#"{"Concessions":"yes"}"#).has_seller_concessions());
}

#[test]
fn test_has_seller_concessions_no_returns_false() {
    assert!(!prop(r#"{"Concessions":"No"}"#).has_seller_concessions());
}

// ── Flood zone ────────────────────────────────────────────────────────────────

#[test]
fn test_flood_insurance_required_zone_ae() {
    assert!(prop(r#"{"FloodZone":"AE"}"#).is_flood_insurance_required());
}

#[test]
fn test_flood_insurance_required_zone_ve() {
    assert!(prop(r#"{"FloodZone":"VE"}"#).is_flood_insurance_required());
}

#[test]
fn test_flood_insurance_required_zone_a() {
    assert!(prop(r#"{"FloodZone":"A"}"#).is_flood_insurance_required());
}

#[test]
fn test_flood_insurance_not_required_zone_x() {
    assert!(!prop(r#"{"FloodZone":"X"}"#).is_flood_insurance_required());
}

#[test]
fn test_flood_insurance_not_required_when_absent() {
    assert!(!prop(r#"{"ListingKey":"x"}"#).is_flood_insurance_required());
}

// ════════════════════════════════════════════════════════════════════════════
// TASK 3.7 — FCC FIPS geocoding client
// ════════════════════════════════════════════════════════════════════════════

// ── FipsResolution ────────────────────────────────────────────────────────────

#[test]
fn test_parse_fcc_kyle_tx_reference() {
    // Real FCC response for Kyle TX (30.0394, -97.8772)
    let json = r#"{
        "status": "OK",
        "County": { "FIPS": "48209", "name": "Hays" },
        "State":  { "FIPS": "48", "code": "TX", "name": "Texas" },
        "Block":  { "FIPS": "482090109053009" }
    }"#;
    let r = parse_fcc_response(json).unwrap();
    assert_eq!(r.fips_code.to_string(), "48209");
    assert_eq!(r.county_name, "Hays");
    assert_eq!(r.state_code, types::StateCode::TX);
    assert_eq!(r.tract_geoid.as_deref(), Some("48209010905"));
}

#[test]
fn test_parse_fcc_tract_geoid_first_11_digits_of_block() {
    let json = r#"{
        "status": "OK",
        "County": { "FIPS": "06037", "name": "Los Angeles" },
        "State":  { "FIPS": "06", "code": "CA", "name": "California" },
        "Block":  { "FIPS": "060371234567001" }
    }"#;
    let r = parse_fcc_response(json).unwrap();
    assert_eq!(r.tract_geoid.as_deref(), Some("06037123456"));
}

#[test]
fn test_parse_fcc_no_block_returns_none_geoid() {
    let json = r#"{
        "status": "OK",
        "County": { "FIPS": "48209", "name": "Hays" },
        "State":  { "FIPS": "48", "code": "TX", "name": "Texas" }
    }"#;
    let r = parse_fcc_response(json).unwrap();
    assert_eq!(r.tract_geoid, None);
}

#[test]
fn test_parse_fcc_status_not_ok_returns_error() {
    let json = r#"{"status":"FAIL"}"#;
    assert!(matches!(
        parse_fcc_response(json),
        Err(ResoError::FccApiError { .. })
    ));
}

#[test]
fn test_parse_fcc_missing_county_returns_error() {
    let json = r#"{
        "status": "OK",
        "State": { "FIPS": "48", "code": "TX", "name": "Texas" }
    }"#;
    assert!(matches!(
        parse_fcc_response(json),
        Err(ResoError::FccApiError { .. })
    ));
}

#[test]
fn test_parse_fcc_missing_state_returns_error() {
    let json = r#"{
        "status": "OK",
        "County": { "FIPS": "48209", "name": "Hays" }
    }"#;
    assert!(matches!(
        parse_fcc_response(json),
        Err(ResoError::FccApiError { .. })
    ));
}

#[test]
fn test_parse_fcc_invalid_state_code_returns_error() {
    let json = r#"{
        "status": "OK",
        "County": { "FIPS": "48209", "name": "Hays" },
        "State":  { "FIPS": "48", "code": "XX", "name": "Unknown" }
    }"#;
    assert!(matches!(
        parse_fcc_response(json),
        Err(ResoError::FccApiError { .. })
    ));
}

#[test]
fn test_parse_fcc_invalid_json_returns_error() {
    assert!(matches!(
        parse_fcc_response("not json"),
        Err(ResoError::FccApiError { .. })
    ));
}

// ── FccClient ─────────────────────────────────────────────────────────────────

#[test]
fn test_fcc_client_new_uses_production_url() {
    let c = FccClient::new();
    assert!(c.base_url().contains("geo.fcc.gov"));
}

#[test]
fn test_fcc_client_with_base_url() {
    let c = FccClient::with_base_url("http://localhost:8080");
    assert_eq!(c.base_url(), "http://localhost:8080");
}

#[test]
fn test_fcc_client_default_is_production() {
    let c = FccClient::default();
    assert!(c.base_url().contains("geo.fcc.gov"));
}

// ── Coordinate validation ─────────────────────────────────────────────────────

#[test]
fn test_validate_coordinates_valid_us() {
    assert!(FccClient::validate_coordinates(30.0394, -97.8772).is_ok());
}

#[test]
fn test_validate_coordinates_null_island_rejected() {
    assert!(matches!(
        FccClient::validate_coordinates(0.0, 0.0),
        Err(ResoError::InvalidCoordinate { .. })
    ));
}

#[test]
fn test_validate_coordinates_out_of_bounds_lat() {
    assert!(matches!(
        FccClient::validate_coordinates(91.0, -97.0),
        Err(ResoError::InvalidCoordinate { .. })
    ));
}

#[test]
fn test_validate_coordinates_out_of_bounds_lon() {
    assert!(matches!(
        FccClient::validate_coordinates(30.0, -181.0),
        Err(ResoError::InvalidCoordinate { .. })
    ));
}
