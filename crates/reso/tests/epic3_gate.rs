//! Epic 3 gate test — full RESO 2.0 pipeline end-to-end.
//!
//! All five canonical fixtures pass through the complete parse chain:
//!   JSON string → PropertyReso → PropertyReso::enrich() → PropertyEnriched
//!
//! Every reference value established during Tasks 3.1–3.9 is verified here.
//!
//! When this file goes green, Epic 3 is declared complete.

use reso::{
    bridge::{
        enriched_to_mismo_address, hoa_for_mismo_expense, property_sub_type_to_mismo,
        select_valuation_price,
    },
    fcc::parse_fcc_response,
    PropertyEnriched, PropertyReso, ResoPropertySubType,
};
use types::{Cents, StateCode};

// ── Fixture loading ───────────────────────────────────────────────────────────

const SFR_JSON: &str = include_str!("fixtures/kyle_tx_sfr.json");
const CONDO_JSON: &str = include_str!("fixtures/kyle_tx_condo.json");
const MOBILE_JSON: &str = include_str!("fixtures/mobile_home.json");
const MFH_JSON: &str = include_str!("fixtures/manufactured_home.json");
const QUAD_JSON: &str = include_str!("fixtures/income_property.json");

fn parse(json: &str) -> PropertyReso {
    serde_json::from_str(json).expect("fixture must parse")
}

fn enrich(json: &str) -> PropertyEnriched {
    parse(json)
        .enrich(None)
        .expect("fixture must enrich without error")
}

// ── Gate 1: all five fixtures parse without error ─────────────────────────────

#[test]
fn gate_all_five_fixtures_deserialize() {
    for (name, json) in [
        ("kyle_tx_sfr", SFR_JSON),
        ("kyle_tx_condo", CONDO_JSON),
        ("mobile_home", MOBILE_JSON),
        ("manufactured_home", MFH_JSON),
        ("income_property", QUAD_JSON),
    ] {
        serde_json::from_str::<PropertyReso>(json)
            .unwrap_or_else(|e| panic!("{name}: JSON parse failed: {e}"));
    }
}

#[test]
fn gate_all_five_fixtures_enrich() {
    for (name, json) in [
        ("kyle_tx_sfr", SFR_JSON),
        ("kyle_tx_condo", CONDO_JSON),
        ("mobile_home", MOBILE_JSON),
        ("manufactured_home", MFH_JSON),
        ("income_property", QUAD_JSON),
    ] {
        parse(json)
            .enrich(None)
            .unwrap_or_else(|e| panic!("{name}: enrich() failed: {e}"));
    }
}

// ── Gate 2: Kyle TX SFR — complete reference value verification ───────────────

#[test]
fn gate_sfr_identity_and_status() {
    let e = enrich(SFR_JSON);
    assert_eq!(e.listing_key.as_str(), "3yd-ABORORTX-12345678");
    assert_eq!(e.listing_id.as_deref(), Some("8765432"));
    assert_eq!(e.standard_status, reso::ResoStandardStatus::Active);
    assert!(e.is_active);
}

#[test]
fn gate_sfr_property_type_and_engine_type() {
    let e = enrich(SFR_JSON);
    assert_eq!(e.property_type, reso::ResoPropertyType::Residential);
    assert_eq!(
        e.property_sub_type,
        Some(ResoPropertySubType::SingleFamilyResidence)
    );
    assert_eq!(e.engine_type, types::PropertyType::SingleFamilyDetached);
    assert!(!e.is_mobile_home);
    assert_eq!(e.unit_count, 1);
}

#[test]
fn gate_sfr_location_fields() {
    let e = enrich(SFR_JSON);
    assert_eq!(e.state, StateCode::TX);
    assert_eq!(e.city.as_deref(), Some("Kyle"));
    assert_eq!(e.county.as_deref(), Some("Hays"));
    assert_eq!(e.postal_code.as_deref(), Some("78640")); // ZIP+4 stripped
    assert_eq!(e.best_address.as_deref(), Some("1234 Mockingbird Ln"));
    assert!((e.latitude.unwrap() - 30.0394).abs() < 1e-3);
    assert!((e.longitude.unwrap() - (-97.8772)).abs() < 1e-3);
}

#[test]
fn gate_sfr_fips_from_tax_tract_field() {
    let e = enrich(SFR_JSON);
    // TaxTract "48209010905" → county FIPS "48209"
    assert_eq!(e.fips_code.as_ref().unwrap().to_string(), "48209");
    assert_eq!(e.tract_geoid, None); // tract only from FCC, not from field path
}

#[test]
fn gate_sfr_physical_dimensions() {
    let e = enrich(SFR_JSON);
    assert_eq!(e.living_area_sqft, Some(2345));
    assert_eq!(e.year_built, Some(2018));
    assert!(!e.is_new_construction);
    assert!(!e.is_attached);
    assert_eq!(e.lot_size_sqft, Some(8712));
}

#[test]
fn gate_sfr_rooms_and_amenities() {
    let e = enrich(SFR_JSON);
    assert_eq!(e.bedrooms, Some(4));
    assert!(e.has_garage);
    assert!(!e.has_pool);
    assert!(!e.has_basement);
}

#[test]
fn gate_sfr_hoa_normalized_to_monthly() {
    let e = enrich(SFR_JSON);
    assert!(e.hoa_yn);
    assert_eq!(e.hoa_monthly_cents, Some(Cents(7_500))); // $75/mo
    assert_eq!(e.hoa_annual_cents, Some(Cents(90_000))); // $900/yr
}

#[test]
fn gate_sfr_tax_fields() {
    let e = enrich(SFR_JSON);
    assert_eq!(e.tax_annual_cents, Some(Cents(890_000))); // $8,900
    assert_eq!(e.tax_year, Some(2024));
    assert_eq!(e.parcel_number.as_deref(), Some("1234-5678-90"));
}

#[test]
fn gate_sfr_pricing_and_reduction() {
    let e = enrich(SFR_JSON);
    assert_eq!(e.list_price, Some(Cents(45_900_000))); // $459,000
    assert_eq!(e.close_price, None); // still active — not sold
    assert_eq!(e.days_on_market, Some(22));
    // $475k original → $459k current = $16k reduction
    assert!(e.raw.is_price_reduced());
}

#[test]
fn gate_sfr_flood_zone_x_no_insurance_required() {
    let e = enrich(SFR_JSON);
    assert_eq!(e.flood_zone.as_deref(), Some("X"));
    assert!(!e.flood_insurance_required);
}

#[test]
fn gate_sfr_school_fields() {
    let e = enrich(SFR_JSON);
    assert_eq!(e.school_district.as_deref(), Some("Hays CISD"));
    assert_eq!(e.elementary_school.as_deref(), Some("Laura B. Negley"));
    assert_eq!(e.high_school.as_deref(), Some("Lehman"));
}

#[test]
fn gate_sfr_raw_fields_all_preserved() {
    let e = enrich(SFR_JSON);
    assert_eq!(e.raw.listing_key.as_deref(), Some("3yd-ABORORTX-12345678"));
    assert_eq!(e.raw.bedrooms_total, Some(4));
    assert_eq!(
        e.raw.garage_spaces.map(|d| d.to_string()).as_deref(),
        Some("2")
    );
    assert_eq!(e.raw.association_fee_frequency.as_deref(), Some("Monthly"));
}

// ── Gate 3: Condo — HOA annual fee normalized to monthly ─────────────────────

#[test]
fn gate_condo_hoa_annual_to_monthly() {
    let e = enrich(CONDO_JSON);
    // $2,400/year ÷ 12 = $200/month
    assert_eq!(e.hoa_monthly_cents, Some(Cents(20_000)));
    assert_eq!(e.hoa_annual_cents, Some(Cents(240_000)));
}

#[test]
fn gate_condo_is_attached() {
    let e = enrich(CONDO_JSON);
    assert!(e.is_attached);
    assert_eq!(e.engine_type, types::PropertyType::Condominium);
    assert_eq!(e.unit_count, 1);
}

// ── Gate 4: Mobile Home — always ineligible ───────────────────────────────────

#[test]
fn gate_mobile_home_ineligible_flag() {
    let e = enrich(MOBILE_JSON);
    assert!(
        e.is_mobile_home,
        "MobileHome must set is_mobile_home = true"
    );
    assert_eq!(e.engine_type, types::PropertyType::MobileHome);
    assert_eq!(e.unit_count, 1);
}

#[test]
fn gate_mobile_home_has_land_lease() {
    let e = enrich(MOBILE_JSON);
    assert!(e.raw.has_land_lease());
}

// ── Gate 5: Manufactured Home — eligible ─────────────────────────────────────

#[test]
fn gate_manufactured_home_not_ineligible() {
    let e = enrich(MFH_JSON);
    assert!(
        !e.is_mobile_home,
        "ManufacturedHome must NOT set is_mobile_home"
    );
    assert_eq!(e.engine_type, types::PropertyType::ManufacturedHome);
    assert!(!e.raw.has_land_lease());
}

#[test]
fn gate_manufactured_home_lot_size_from_acres() {
    let e = enrich(MFH_JSON);
    // 1.5 acres × 43,560 = 65,340 sq ft
    assert_eq!(e.lot_size_sqft, Some(65_340));
}

// ── Gate 6: Income property (Quadruplex) ─────────────────────────────────────

#[test]
fn gate_income_property_unit_count_four() {
    let e = enrich(QUAD_JSON);
    assert_eq!(e.unit_count, 4);
    assert_eq!(e.engine_type, types::PropertyType::FourUnit);
}

#[test]
fn gate_income_property_flood_zone_ae_requires_insurance() {
    let e = enrich(QUAD_JSON);
    assert_eq!(e.flood_zone.as_deref(), Some("AE"));
    assert!(e.flood_insurance_required);
}

#[test]
fn gate_income_property_travis_county_fips() {
    let e = enrich(QUAD_JSON);
    // TaxTract "48453001801" → county FIPS "48453" (Travis County)
    assert_eq!(e.fips_code.as_ref().unwrap().to_string(), "48453");
    assert_eq!(e.state, StateCode::TX);
}

// ── Gate 7: FCC FIPS client — reference parse ─────────────────────────────────

#[test]
fn gate_fcc_kyle_tx_reference_coordinates() {
    let json = r#"{
        "status": "OK",
        "County": { "FIPS": "48209", "name": "Hays" },
        "State":  { "FIPS": "48", "code": "TX", "name": "Texas" },
        "Block":  { "FIPS": "482090109053009" }
    }"#;
    let r = parse_fcc_response(json).unwrap();
    assert_eq!(r.fips_code.to_string(), "48209");
    assert_eq!(r.county_name, "Hays");
    assert_eq!(r.state_code, StateCode::TX);
    assert_eq!(r.tract_geoid.as_deref(), Some("48209010905"));
}

#[test]
fn gate_fcc_enrich_with_resolution_populates_tract() {
    let fcc_json = r#"{
        "status": "OK",
        "County": { "FIPS": "48209", "name": "Hays" },
        "State":  { "FIPS": "48", "code": "TX", "name": "Texas" },
        "Block":  { "FIPS": "482090109053009" }
    }"#;
    let resolution = parse_fcc_response(fcc_json).unwrap();
    let e = parse(SFR_JSON).enrich(Some(resolution)).unwrap();
    assert_eq!(e.fips_code.as_ref().unwrap().to_string(), "48209");
    assert_eq!(e.tract_geoid.as_deref(), Some("48209010905"));
}

// ── Gate 8: RESO ↔ MISMO bridge ──────────────────────────────────────────────

#[test]
fn gate_bridge_all_five_property_types() {
    let cases = [
        (SFR_JSON, "Detached"),
        (CONDO_JSON, "Condominium"),
        (MOBILE_JSON, "MobileHome"),
        (MFH_JSON, "ManufacturedHousing"),
        (QUAD_JSON, "Detached"), // Quadruplex → "Detached" + unit count
    ];
    for (json, expected_mismo) in cases {
        let e = enrich(json);
        let sub = e
            .property_sub_type
            .expect("fixture must have PropertySubType");
        let mismo = property_sub_type_to_mismo(sub);
        assert_eq!(mismo, expected_mismo, "fixture sub-type {sub:?} mismatch");
    }
}

#[test]
fn gate_bridge_address_sfr() {
    let e = enrich(SFR_JSON);
    let addr = enriched_to_mismo_address(&e);
    assert_eq!(addr.state, "TX");
    assert_eq!(addr.city.as_deref(), Some("Kyle"));
    assert_eq!(addr.postal_code.as_deref(), Some("78640"));
    assert_eq!(addr.county.as_deref(), Some("Hays"));
    assert_eq!(addr.street_address.as_deref(), Some("1234 Mockingbird Ln"));
}

#[test]
fn gate_bridge_valuation_price_priority() {
    // SFR: only list price (still active)
    let sfr = enrich(SFR_JSON);
    assert_eq!(select_valuation_price(&sfr), Some(Cents(45_900_000)));

    // Quad: only list price
    let quad = enrich(QUAD_JSON);
    assert_eq!(select_valuation_price(&quad), Some(Cents(71_000_000)));
}

#[test]
fn gate_bridge_hoa_expense_sfr_vs_no_hoa() {
    let sfr = enrich(SFR_JSON);
    assert_eq!(hoa_for_mismo_expense(&sfr), Some(Cents(7_500)));

    let mfh = enrich(MFH_JSON);
    assert_eq!(hoa_for_mismo_expense(&mfh), None);
}
