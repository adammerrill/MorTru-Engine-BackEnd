//! Tasks 3.4 + 3.5 — Physical/construction/lot helpers and
//! room/parking/systems helpers (Categories 7–12).

use reso::PropertyReso;
use rust_decimal::Decimal;
use std::str::FromStr;

fn prop(json: &str) -> PropertyReso {
    serde_json::from_str(json).unwrap()
}

// ════════════════════════════════════════════════════════════════════════════
// TASK 3.4 — Physical dimensions, construction, lot (Categories 7–9)
// ════════════════════════════════════════════════════════════════════════════

// ── Living area ───────────────────────────────────────────────────────────────

#[test]
fn test_living_area_sqft_integer() {
    let p = prop(r#"{"LivingArea":2345.0}"#);
    assert_eq!(p.living_area_sqft(), Some(2345));
}

#[test]
fn test_living_area_sqft_truncates_fractional() {
    let p = prop(r#"{"LivingArea":2345.9}"#);
    assert_eq!(p.living_area_sqft(), Some(2345));
}

#[test]
fn test_living_area_sqft_absent_returns_none() {
    assert_eq!(prop(r#"{"ListingKey":"x"}"#).living_area_sqft(), None);
}

#[test]
fn test_total_area_sqft_with_basement() {
    let p = prop(r#"{"LivingArea":2000.0,"BelowGradeFinishedArea":500.0}"#);
    assert_eq!(p.total_area_sqft(), Some(2500));
}

#[test]
fn test_total_area_sqft_no_basement() {
    let p = prop(r#"{"LivingArea":2000.0}"#);
    assert_eq!(p.total_area_sqft(), Some(2000));
}

#[test]
fn test_total_area_sqft_no_living_area_returns_none() {
    let p = prop(r#"{"BelowGradeFinishedArea":500.0}"#);
    assert_eq!(p.total_area_sqft(), None);
}

#[test]
fn test_stories_rounds_split_level() {
    // 1.5-story split-level → rounds to 2
    let p = prop(r#"{"StoriesTotal":1.5}"#);
    assert_eq!(p.stories(), Some(2));
}

#[test]
fn test_stories_single() {
    assert_eq!(prop(r#"{"StoriesTotal":1.0}"#).stories(), Some(1));
}

#[test]
fn test_stories_absent_returns_none() {
    assert_eq!(prop(r#"{"ListingKey":"x"}"#).stories(), None);
}

// ── Year built ────────────────────────────────────────────────────────────────

#[test]
fn test_year_built_valid_2018() {
    let p = prop(r#"{"YearBuilt":2018}"#);
    assert_eq!(p.year_built().unwrap(), Some(2018));
}

#[test]
fn test_year_built_absent_returns_ok_none() {
    let p = prop(r#"{"ListingKey":"x"}"#);
    assert_eq!(p.year_built().unwrap(), None);
}

#[test]
fn test_year_built_too_old_returns_error() {
    let p = prop(r#"{"YearBuilt":1799}"#);
    assert!(p.year_built().is_err());
}

#[test]
fn test_year_built_future_out_of_range_returns_error() {
    let p = prop(r#"{"YearBuilt":2099}"#);
    assert!(p.year_built().is_err());
}

#[test]
fn test_effective_year_built_prefers_effective() {
    let p = prop(r#"{"YearBuilt":1965,"YearBuiltEffective":2019}"#);
    assert_eq!(p.effective_year_built(), Some(2019));
}

#[test]
fn test_effective_year_built_falls_back_to_year_built() {
    let p = prop(r#"{"YearBuilt":1965}"#);
    assert_eq!(p.effective_year_built(), Some(1965));
}

#[test]
fn test_effective_year_built_absent_returns_none() {
    assert_eq!(prop(r#"{"ListingKey":"x"}"#).effective_year_built(), None);
}

// ── Construction flags ────────────────────────────────────────────────────────

#[test]
fn test_is_new_construction_true() {
    assert!(prop(r#"{"NewConstructionYN":true}"#).is_new_construction());
}

#[test]
fn test_is_new_construction_false_when_absent() {
    assert!(!prop(r#"{"ListingKey":"x"}"#).is_new_construction());
}

#[test]
fn test_is_attached_true() {
    assert!(prop(r#"{"PropertyAttachedYN":true}"#).is_attached());
}

#[test]
fn test_is_attached_false_when_absent() {
    assert!(!prop(r#"{"ListingKey":"x"}"#).is_attached());
}

#[test]
fn test_is_on_slab_true() {
    let p = prop(r#"{"FoundationDetails":["Slab"]}"#);
    assert!(p.is_on_slab());
}

#[test]
fn test_is_on_slab_case_insensitive() {
    let p = prop(r#"{"FoundationDetails":["slab","ConcretePerimeter"]}"#);
    assert!(p.is_on_slab());
}

#[test]
fn test_is_on_slab_false_for_crawlspace() {
    let p = prop(r#"{"FoundationDetails":["CrawlSpace"]}"#);
    assert!(!p.is_on_slab());
}

// ── Lot size ──────────────────────────────────────────────────────────────────

#[test]
fn test_lot_size_sqft_from_sqft_field() {
    let p = prop(r#"{"LotSizeSquareFeet":8712.0}"#);
    assert_eq!(p.lot_size_sqft(), Some(8712));
}

#[test]
fn test_lot_size_sqft_converts_from_acres() {
    // 0.2 acres = 0.2 * 43560 = 8712 sq ft
    let p = prop(r#"{"LotSizeAcres":0.2}"#);
    assert_eq!(p.lot_size_sqft(), Some(8712));
}

#[test]
fn test_lot_size_sqft_prefers_sqft_over_acres() {
    let p = prop(r#"{"LotSizeSquareFeet":9000.0,"LotSizeAcres":0.2}"#);
    assert_eq!(p.lot_size_sqft(), Some(9000));
}

#[test]
fn test_lot_size_sqft_absent_returns_none() {
    assert_eq!(prop(r#"{"ListingKey":"x"}"#).lot_size_sqft(), None);
}

#[test]
fn test_lot_size_acres_from_acres_field() {
    let p = prop(r#"{"LotSizeAcres":0.25}"#);
    let acres = p.lot_size_acres().unwrap();
    assert_eq!(acres, Decimal::from_str("0.25").unwrap());
}

#[test]
fn test_lot_size_acres_converts_from_sqft() {
    // 43560 sq ft = exactly 1 acre
    let p = prop(r#"{"LotSizeSquareFeet":43560.0}"#);
    let acres = p.lot_size_acres().unwrap();
    assert_eq!(acres.round_dp(4), Decimal::from_str("1.0000").unwrap());
}

#[test]
fn test_has_land_lease_true() {
    assert!(prop(r#"{"LandLeaseYN":true}"#).has_land_lease());
}

#[test]
fn test_has_land_lease_false_when_absent() {
    assert!(!prop(r#"{"ListingKey":"x"}"#).has_land_lease());
}

// ════════════════════════════════════════════════════════════════════════════
// TASK 3.5 — Rooms, parking, systems (Categories 10–12)
// ════════════════════════════════════════════════════════════════════════════

// ── Rooms ─────────────────────────────────────────────────────────────────────

#[test]
fn test_bedrooms_present() {
    assert_eq!(prop(r#"{"BedroomsTotal":4}"#).bedrooms(), Some(4));
}

#[test]
fn test_bedrooms_absent_returns_none() {
    assert_eq!(prop(r#"{"ListingKey":"x"}"#).bedrooms(), None);
}

#[test]
fn test_bathrooms_decimal_two_and_half() {
    let p = prop(r#"{"BathroomsTotalDecimal":2.5}"#);
    assert_eq!(
        p.bathrooms_decimal().unwrap(),
        Decimal::from_str("2.5").unwrap()
    );
}

#[test]
fn test_bathrooms_full_count() {
    assert_eq!(prop(r#"{"BathroomsFull":2}"#).bathrooms_full(), Some(2));
}

#[test]
fn test_bathrooms_half_count() {
    assert_eq!(prop(r#"{"BathroomsHalf":1}"#).bathrooms_half(), Some(1));
}

// ── Basement ──────────────────────────────────────────────────────────────────

#[test]
fn test_has_basement_from_yn_true() {
    assert!(prop(r#"{"BasementYN":true}"#).has_basement());
}

#[test]
fn test_has_basement_from_yn_false() {
    assert!(!prop(r#"{"BasementYN":false}"#).has_basement());
}

#[test]
fn test_has_basement_from_collection_finished() {
    let p = prop(r#"{"Basement":["Finished","WalkOut"]}"#);
    assert!(p.has_basement());
}

#[test]
fn test_has_basement_none_collection_is_false() {
    let p = prop(r#"{"Basement":["None"]}"#);
    assert!(!p.has_basement());
}

#[test]
fn test_is_basement_finished_true() {
    let p = prop(r#"{"Basement":["Finished","Interior Entry"]}"#);
    assert!(p.is_basement_finished());
}

#[test]
fn test_is_basement_finished_false_for_unfinished() {
    let p = prop(r#"{"Basement":["Unfinished"]}"#);
    assert!(!p.is_basement_finished());
}

// ── Parking ───────────────────────────────────────────────────────────────────

#[test]
fn test_garage_spaces_present() {
    assert_eq!(prop(r#"{"GarageSpaces":2.0}"#).garage_spaces(), Some(2));
}

#[test]
fn test_has_garage_from_yn() {
    assert!(prop(r#"{"GarageYN":true}"#).has_garage());
}

#[test]
fn test_has_garage_from_spaces() {
    assert!(prop(r#"{"GarageSpaces":1.0}"#).has_garage());
}

#[test]
fn test_has_garage_false_when_absent() {
    assert!(!prop(r#"{"ListingKey":"x"}"#).has_garage());
}

#[test]
fn test_has_attached_garage_true() {
    assert!(prop(r#"{"AttachedGarageYN":true}"#).has_attached_garage());
}

// ── Systems ───────────────────────────────────────────────────────────────────

#[test]
fn test_has_central_ac_exact_match() {
    let p = prop(r#"{"Cooling":["CentralAir"]}"#);
    assert!(p.has_central_ac());
}

#[test]
fn test_has_central_ac_spaced_variant() {
    let p = prop(r#"{"Cooling":["Central Air","AtticFan"]}"#);
    assert!(p.has_central_ac());
}

#[test]
fn test_has_central_ac_false_for_evaporative() {
    let p = prop(r#"{"Cooling":["EvaporativeCooler"]}"#);
    assert!(!p.has_central_ac());
}

#[test]
fn test_has_forced_air_heat() {
    assert!(prop(r#"{"Heating":["ForcedAir","Electric"]}"#).has_forced_air_heat());
}

#[test]
fn test_is_on_public_sewer_true() {
    assert!(prop(r#"{"Sewer":["PublicSewer"]}"#).is_on_public_sewer());
}

#[test]
fn test_is_on_public_sewer_septic_is_false() {
    assert!(!prop(r#"{"Sewer":["SepticTank"]}"#).is_on_public_sewer());
}

#[test]
fn test_is_on_public_water_true() {
    assert!(prop(r#"{"WaterSource":["PublicWater"]}"#).is_on_public_water());
}

#[test]
fn test_is_on_public_water_well_is_false() {
    assert!(!prop(r#"{"WaterSource":["Well"]}"#).is_on_public_water());
}

#[test]
fn test_has_pool_true() {
    assert!(prop(r#"{"PoolPrivateYN":true}"#).has_pool());
}

#[test]
fn test_has_pool_false_when_absent() {
    assert!(!prop(r#"{"ListingKey":"x"}"#).has_pool());
}

#[test]
fn test_has_solar_from_boolean() {
    assert!(prop(r#"{"SolarPanels":true}"#).has_solar());
}

#[test]
fn test_has_solar_from_green_generation() {
    let p = prop(r#"{"GreenEnergyGeneration":["Solar","GridTied"]}"#);
    assert!(p.has_solar());
}

#[test]
fn test_has_solar_false_when_absent() {
    assert!(!prop(r#"{"ListingKey":"x"}"#).has_solar());
}

#[test]
fn test_has_fireplace_from_yn() {
    assert!(prop(r#"{"FireplaceYN":true}"#).has_fireplace());
}

#[test]
fn test_has_fireplace_from_count() {
    assert!(prop(r#"{"FireplacesTotal":2}"#).has_fireplace());
}

#[test]
fn test_has_fireplace_false_when_absent() {
    assert!(!prop(r#"{"ListingKey":"x"}"#).has_fireplace());
}

#[test]
fn test_fireplaces_count() {
    assert_eq!(prop(r#"{"FireplacesTotal":2}"#).fireplaces(), 2);
}

#[test]
fn test_fireplaces_absent_returns_zero() {
    assert_eq!(prop(r#"{"ListingKey":"x"}"#).fireplaces(), 0);
}
