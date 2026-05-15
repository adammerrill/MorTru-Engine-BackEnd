//! RESO Data Dictionary 2.0 — `PropertyReso` struct.
//!
//! All 235 Property resource fields are captured here, organized across
//! sub-modules by category. Every field is `Option<T>` — the RESO Web
//! API omits fields that have no value; absence is not an error.
//!
//! # JSON deserialization
//!
//! ```no_run
//! use reso::property::PropertyReso;
//!
//! let json = r#"{"ListingKey":"abc","ListPrice":459000.0,"City":"Kyle"}"#;
//! let p: PropertyReso = serde_json::from_str(json).unwrap();
//! assert_eq!(p.listing_key.as_deref(), Some("abc"));
//! ```
//!
//! # Field names
//!
//! All fields use `#[serde(rename = "PascalCase")]` matching the RESO
//! Data Dictionary 2.0 standard exactly. Unknown fields are silently
//! ignored — `#[serde(deny_unknown_fields)]` is intentionally NOT used
//! to future-proof against new RESO fields without breaking changes.

pub mod address;
pub mod listing;
pub mod parse;
pub mod physical;
pub mod rooms;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

// ── Category 1: Identity (10 fields) ─────────────────────────────────────────

/// Primary system identifier for this listing.
/// Globally unique across the RESO Data Source. Immutable for life of listing.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct PropertyReso {
    // ── Category 1: Identity ──────────────────────────────────────────────────
    /// System-unique, immutable identifier. Primary key. (DD 2.0 required)
    #[serde(rename = "ListingKey", skip_serializing_if = "Option::is_none")]
    pub listing_key: Option<String>,

    /// Numeric form of ListingKey for integer-keyed systems.
    #[serde(rename = "ListingKeyNumeric", skip_serializing_if = "Option::is_none")]
    pub listing_key_numeric: Option<i64>,

    /// Human-readable MLS listing number shown on public sites.
    #[serde(rename = "ListingId", skip_serializing_if = "Option::is_none")]
    pub listing_id: Option<String>,

    /// System locale of the MLS (e.g. "en-US").
    #[serde(rename = "SystemLocale", skip_serializing_if = "Option::is_none")]
    pub system_locale: Option<String>,

    /// Primary key in the originating MLS/system.
    #[serde(
        rename = "OriginatingSystemKey",
        skip_serializing_if = "Option::is_none"
    )]
    pub originating_system_key: Option<String>,

    /// Name of the originating MLS or system.
    #[serde(
        rename = "OriginatingSystemName",
        skip_serializing_if = "Option::is_none"
    )]
    pub originating_system_name: Option<String>,

    /// Key from the RESO Data Source.
    #[serde(rename = "SourceSystemKey", skip_serializing_if = "Option::is_none")]
    pub source_system_key: Option<String>,

    /// Name of the RESO Data Source.
    #[serde(rename = "SourceSystemName", skip_serializing_if = "Option::is_none")]
    pub source_system_name: Option<String>,

    /// Timestamp when data passed through a RESO bridge.
    #[serde(
        rename = "BridgeModificationTimestamp",
        skip_serializing_if = "Option::is_none"
    )]
    pub bridge_modification_timestamp: Option<String>,

    /// "Sale", "Lease", or "Rent".
    #[serde(rename = "TransactionType", skip_serializing_if = "Option::is_none")]
    pub transaction_type: Option<String>,

    // ── Category 2: Timestamps (8 fields) ─────────────────────────────────────
    /// Most recent modification timestamp for any field. (DD 2.0 required)
    #[serde(
        rename = "ModificationTimestamp",
        skip_serializing_if = "Option::is_none"
    )]
    pub modification_timestamp: Option<String>,

    /// When listing first entered the MLS system.
    #[serde(
        rename = "OriginalEntryTimestamp",
        skip_serializing_if = "Option::is_none"
    )]
    pub original_entry_timestamp: Option<String>,

    /// When StandardStatus last changed.
    #[serde(
        rename = "StatusChangeTimestamp",
        skip_serializing_if = "Option::is_none"
    )]
    pub status_change_timestamp: Option<String>,

    /// When ListPrice last changed.
    #[serde(
        rename = "PriceChangeTimestamp",
        skip_serializing_if = "Option::is_none"
    )]
    pub price_change_timestamp: Option<String>,

    /// Timestamp of last major field change.
    #[serde(
        rename = "MajorChangeTimestamp",
        skip_serializing_if = "Option::is_none"
    )]
    pub major_change_timestamp: Option<String>,

    /// When media/photos were last updated.
    #[serde(
        rename = "PhotosChangeTimestamp",
        skip_serializing_if = "Option::is_none"
    )]
    pub photos_change_timestamp: Option<String>,

    /// Date of last contract status change.
    #[serde(
        rename = "ContractStatusChangeDate",
        skip_serializing_if = "Option::is_none"
    )]
    pub contract_status_change_date: Option<String>,

    /// Date listing expires per the listing agreement.
    #[serde(rename = "ExpirationDate", skip_serializing_if = "Option::is_none")]
    pub expiration_date: Option<String>,

    // ── Category 3: Listing Status (8 fields) ─────────────────────────────────
    /// RESO standard status. One of: Active, ActiveUnderContract, Canceled,
    /// Closed, ComingSoon, Delete, Expired, Incomplete, Pending, Withdrawn.
    #[serde(rename = "StandardStatus", skip_serializing_if = "Option::is_none")]
    pub standard_status: Option<String>,

    /// MLS-specific status string (varies by board, non-standard).
    #[serde(rename = "MlsStatus", skip_serializing_if = "Option::is_none")]
    pub mls_status: Option<String>,

    /// Listing agreement type. ExclusiveRightToSell, ExclusiveAgency, Open, Net.
    #[serde(rename = "ListingAgreement", skip_serializing_if = "Option::is_none")]
    pub listing_agreement: Option<String>,

    /// Free-text contingency description (inspection, financing, etc.).
    #[serde(rename = "Contingency", skip_serializing_if = "Option::is_none")]
    pub contingency: Option<String>,

    /// Date contingency expires.
    #[serde(rename = "ContingencyDate", skip_serializing_if = "Option::is_none")]
    pub contingency_date: Option<String>,

    /// Special conditions: None, Auction, BankOwned, HUD, ShortSale, Probate.
    #[serde(
        rename = "SpecialListingConditions",
        skip_serializing_if = "Option::is_none"
    )]
    pub special_listing_conditions: Option<Vec<String>>,

    /// Disclosure types: Lead, Mold, AsIs, NoDisclosures, etc.
    #[serde(rename = "Disclosures", skip_serializing_if = "Option::is_none")]
    pub disclosures: Option<Vec<String>>,

    /// Possession terms: NegotiableDate, SubjectToTenantRights, Seller, Vacant.
    #[serde(rename = "Possession", skip_serializing_if = "Option::is_none")]
    pub possession: Option<Vec<String>>,

    // ── Category 4: Property Type / Subtype (4 fields) ────────────────────────
    /// RESO PropertyType lookup. Residential, ResidentialIncome, Land, etc.
    #[serde(rename = "PropertyType", skip_serializing_if = "Option::is_none")]
    pub property_type: Option<String>,

    /// RESO PropertySubType lookup. "Single Family Residence", "Condominium", etc.
    #[serde(rename = "PropertySubType", skip_serializing_if = "Option::is_none")]
    pub property_sub_type: Option<String>,

    /// Structural condition: Excellent, Good, Average, Fair, Poor, TearDown.
    #[serde(
        rename = "StructuralCondition",
        skip_serializing_if = "Option::is_none"
    )]
    pub structural_condition: Option<String>,

    /// Property condition: NewConstruction, UpdatedRemodeled, Fixer, AsIs.
    #[serde(rename = "PropertyCondition", skip_serializing_if = "Option::is_none")]
    pub property_condition: Option<Vec<String>>,

    // ── Category 5: Address (22 fields) ──────────────────────────────────────
    /// Full address as a single unparsed string.
    #[serde(rename = "UnparsedAddress", skip_serializing_if = "Option::is_none")]
    pub unparsed_address: Option<String>,

    /// Street number (string — may include letters).
    #[serde(rename = "StreetNumber", skip_serializing_if = "Option::is_none")]
    pub street_number: Option<String>,

    /// Numeric portion of street number for sorting.
    #[serde(
        rename = "StreetNumberNumeric",
        skip_serializing_if = "Option::is_none"
    )]
    pub street_number_numeric: Option<i32>,

    /// Street directional prefix: N, S, E, W, NE, NW, SE, SW.
    #[serde(rename = "StreetDirPrefix", skip_serializing_if = "Option::is_none")]
    pub street_dir_prefix: Option<String>,

    /// Street name without number, suffix, or directional.
    #[serde(rename = "StreetName", skip_serializing_if = "Option::is_none")]
    pub street_name: Option<String>,

    /// Street suffix: St, Ave, Blvd, Dr, Ln, Rd, Ct, Way, Pl, etc.
    #[serde(rename = "StreetSuffix", skip_serializing_if = "Option::is_none")]
    pub street_suffix: Option<String>,

    /// Additional suffix modifier (e.g. "Extension").
    #[serde(
        rename = "StreetSuffixModifier",
        skip_serializing_if = "Option::is_none"
    )]
    pub street_suffix_modifier: Option<String>,

    /// Trailing directional: N, S, E, W, NE, NW, SE, SW.
    #[serde(rename = "StreetDirSuffix", skip_serializing_if = "Option::is_none")]
    pub street_dir_suffix: Option<String>,

    /// Unit, apartment, suite, or building number.
    #[serde(rename = "UnitNumber", skip_serializing_if = "Option::is_none")]
    pub unit_number: Option<String>,

    /// Unit designator type: Apt, Suite, Unit, Building, Floor.
    #[serde(rename = "UnitNumberType", skip_serializing_if = "Option::is_none")]
    pub unit_number_type: Option<String>,

    /// Sub-city region or neighborhood name.
    #[serde(rename = "CityRegion", skip_serializing_if = "Option::is_none")]
    pub city_region: Option<String>,

    /// City name.
    #[serde(rename = "City", skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,

    /// US state abbreviation or Canadian province code.
    #[serde(rename = "StateOrProvince", skip_serializing_if = "Option::is_none")]
    pub state_or_province: Option<String>,

    /// 5-digit ZIP code or ZIP+4.
    #[serde(rename = "PostalCode", skip_serializing_if = "Option::is_none")]
    pub postal_code: Option<String>,

    /// ZIP+4 extension when available.
    #[serde(rename = "PostalCodePlus4", skip_serializing_if = "Option::is_none")]
    pub postal_code_plus4: Option<String>,

    /// County or parish name (e.g. "Hays").
    #[serde(rename = "CountyOrParish", skip_serializing_if = "Option::is_none")]
    pub county_or_parish: Option<String>,

    /// Country code. "US" for United States.
    #[serde(rename = "Country", skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,

    /// Legal subdivision or planned community name.
    #[serde(rename = "SubdivisionName", skip_serializing_if = "Option::is_none")]
    pub subdivision_name: Option<String>,

    /// MLS-defined major geographic area.
    #[serde(rename = "MLSAreaMajor", skip_serializing_if = "Option::is_none")]
    pub mls_area_major: Option<String>,

    /// MLS-defined minor geographic area.
    #[serde(rename = "MLSAreaMinor", skip_serializing_if = "Option::is_none")]
    pub mls_area_minor: Option<String>,

    /// Driving directions narrative.
    #[serde(rename = "Directions", skip_serializing_if = "Option::is_none")]
    pub directions: Option<String>,

    /// Legal township/range description.
    #[serde(rename = "TownshipRange", skip_serializing_if = "Option::is_none")]
    pub township_range: Option<String>,

    // ── Category 6: Geographic Coordinates (5 fields) ─────────────────────────
    /// WGS 84 decimal degrees latitude. Primary FIPS derivation source.
    #[serde(rename = "Latitude", skip_serializing_if = "Option::is_none")]
    pub latitude: Option<f64>,

    /// WGS 84 decimal degrees longitude.
    #[serde(rename = "Longitude", skip_serializing_if = "Option::is_none")]
    pub longitude: Option<f64>,

    /// Elevation above sea level.
    #[serde(rename = "Elevation", skip_serializing_if = "Option::is_none")]
    pub elevation: Option<Decimal>,

    /// Elevation units: Feet, Meters.
    #[serde(rename = "ElevationUnits", skip_serializing_if = "Option::is_none")]
    pub elevation_units: Option<String>,

    /// Supplementary map reference (Thomas Brothers grid, etc.).
    #[serde(rename = "MapCoordinate", skip_serializing_if = "Option::is_none")]
    pub map_coordinate: Option<String>,

    // ── Category 7: Physical Dimensions (20 fields) ───────────────────────────
    /// Finished livable square footage (GLA). Primary area field used by engine.
    #[serde(rename = "LivingArea", skip_serializing_if = "Option::is_none")]
    pub living_area: Option<Decimal>,

    /// Units for LivingArea: SquareFeet, SquareMeters.
    #[serde(rename = "LivingAreaUnits", skip_serializing_if = "Option::is_none")]
    pub living_area_units: Option<String>,

    /// Source for LivingArea: Appraiser, Assessor, Builder, Estimated, etc.
    #[serde(rename = "LivingAreaSource", skip_serializing_if = "Option::is_none")]
    pub living_area_source: Option<String>,

    /// Finished area above grade.
    #[serde(
        rename = "AboveGradeFinishedArea",
        skip_serializing_if = "Option::is_none"
    )]
    pub above_grade_finished_area: Option<Decimal>,

    /// Units for AboveGradeFinishedArea.
    #[serde(
        rename = "AboveGradeFinishedAreaUnits",
        skip_serializing_if = "Option::is_none"
    )]
    pub above_grade_finished_area_units: Option<String>,

    /// Source for AboveGradeFinishedArea.
    #[serde(
        rename = "AboveGradeFinishedAreaSource",
        skip_serializing_if = "Option::is_none"
    )]
    pub above_grade_finished_area_source: Option<String>,

    /// Finished below-grade (basement) area.
    #[serde(
        rename = "BelowGradeFinishedArea",
        skip_serializing_if = "Option::is_none"
    )]
    pub below_grade_finished_area: Option<Decimal>,

    /// Units for BelowGradeFinishedArea.
    #[serde(
        rename = "BelowGradeFinishedAreaUnits",
        skip_serializing_if = "Option::is_none"
    )]
    pub below_grade_finished_area_units: Option<String>,

    /// Source for BelowGradeFinishedArea.
    #[serde(
        rename = "BelowGradeFinishedAreaSource",
        skip_serializing_if = "Option::is_none"
    )]
    pub below_grade_finished_area_source: Option<String>,

    /// Unfinished below-grade area.
    #[serde(
        rename = "BelowGradeUnfinishedArea",
        skip_serializing_if = "Option::is_none"
    )]
    pub below_grade_unfinished_area: Option<Decimal>,

    /// Total building footprint including all floors.
    #[serde(rename = "BuildingAreaTotal", skip_serializing_if = "Option::is_none")]
    pub building_area_total: Option<Decimal>,

    /// Units for BuildingAreaTotal.
    #[serde(rename = "BuildingAreaUnits", skip_serializing_if = "Option::is_none")]
    pub building_area_units: Option<String>,

    /// Source for BuildingAreaTotal.
    #[serde(rename = "BuildingAreaSource", skip_serializing_if = "Option::is_none")]
    pub building_area_source: Option<String>,

    /// Number of stories (may be fractional e.g. 1.5 for split-level).
    #[serde(rename = "StoriesTotal", skip_serializing_if = "Option::is_none")]
    pub stories_total: Option<Decimal>,

    /// Floor levels: One, Two, Three, MultiSplit, ThreeOrMore.
    #[serde(rename = "Levels", skip_serializing_if = "Option::is_none")]
    pub levels: Option<Vec<String>>,

    /// Total dwelling units (1 = SFR, 2-4 = small multi-unit, etc.).
    #[serde(rename = "NumberOfUnitsTotal", skip_serializing_if = "Option::is_none")]
    pub number_of_units_total: Option<i32>,

    /// Total units in the larger community/project (for condos/co-ops).
    #[serde(
        rename = "NumberOfUnitsInCommunity",
        skip_serializing_if = "Option::is_none"
    )]
    pub number_of_units_in_community: Option<i32>,

    /// Floor level of main entry (important for condos/co-ops).
    #[serde(rename = "EntryLevel", skip_serializing_if = "Option::is_none")]
    pub entry_level: Option<i32>,

    /// Entry access type: GroundLevel, Stairs, ElevatorRequired.
    #[serde(rename = "EntryLocation", skip_serializing_if = "Option::is_none")]
    pub entry_location: Option<String>,

    /// Shared wall configuration: NoCommonWalls, 1CommonWall, 2CommonWalls, EndUnit.
    #[serde(rename = "CommonWalls", skip_serializing_if = "Option::is_none")]
    pub common_walls: Option<Vec<String>>,

    // ── Category 8: Age and Construction (14 fields) ──────────────────────────
    /// Year original structure was completed.
    #[serde(rename = "YearBuilt", skip_serializing_if = "Option::is_none")]
    pub year_built: Option<i32>,

    /// Source for YearBuilt: Appraiser, Assessor, Builder, Owner, PublicRecords.
    #[serde(rename = "YearBuiltSource", skip_serializing_if = "Option::is_none")]
    pub year_built_source: Option<String>,

    /// Year of last major renovation affecting effective age.
    #[serde(rename = "YearBuiltEffective", skip_serializing_if = "Option::is_none")]
    pub year_built_effective: Option<i32>,

    /// True if property has never been occupied (new construction).
    #[serde(rename = "NewConstructionYN", skip_serializing_if = "Option::is_none")]
    pub new_construction_yn: Option<bool>,

    /// Builder name for new construction.
    #[serde(rename = "BuilderName", skip_serializing_if = "Option::is_none")]
    pub builder_name: Option<String>,

    /// Builder model name for new construction.
    #[serde(rename = "BuilderModel", skip_serializing_if = "Option::is_none")]
    pub builder_model: Option<String>,

    /// Structure type: House, Townhouse, Duplex, Manufactured, Mobile, etc.
    #[serde(rename = "StructureType", skip_serializing_if = "Option::is_none")]
    pub structure_type: Option<Vec<String>>,

    /// Architectural style: Contemporary, Traditional, Ranch, Colonial, etc.
    #[serde(rename = "ArchitecturalStyle", skip_serializing_if = "Option::is_none")]
    pub architectural_style: Option<Vec<String>>,

    /// Construction materials: Frame, Brick, Stone, Stucco, Wood, Concrete, etc.
    #[serde(
        rename = "ConstructionMaterials",
        skip_serializing_if = "Option::is_none"
    )]
    pub construction_materials: Option<Vec<String>>,

    /// Foundation type: Slab, CrawlSpace, ConcretePerimeter, Block, etc.
    #[serde(rename = "FoundationDetails", skip_serializing_if = "Option::is_none")]
    pub foundation_details: Option<Vec<String>>,

    /// Foundation area in square feet.
    #[serde(rename = "FoundationArea", skip_serializing_if = "Option::is_none")]
    pub foundation_area: Option<Decimal>,

    /// Roof type: Composition, Shingle, Tile, Metal, Flat, Membrane, etc.
    #[serde(rename = "Roof", skip_serializing_if = "Option::is_none")]
    pub roof: Option<Vec<String>>,

    /// True if property shares a wall with another dwelling unit.
    #[serde(rename = "PropertyAttachedYN", skip_serializing_if = "Option::is_none")]
    pub property_attached_yn: Option<bool>,

    // Note: PropertyCondition is defined in Category 4 (property_condition field).
    // It is not duplicated here — see category 4 above.

    // ── Category 9: Lot / Land (15 fields) ────────────────────────────────────
    /// Lot size in acres.
    #[serde(rename = "LotSizeAcres", skip_serializing_if = "Option::is_none")]
    pub lot_size_acres: Option<Decimal>,

    /// Lot size in square feet.
    #[serde(rename = "LotSizeSquareFeet", skip_serializing_if = "Option::is_none")]
    pub lot_size_square_feet: Option<Decimal>,

    /// Lot size in LotSizeUnits.
    #[serde(rename = "LotSizeArea", skip_serializing_if = "Option::is_none")]
    pub lot_size_area: Option<Decimal>,

    /// Units for LotSizeArea: Acres, SquareFeet, SquareMeters, Hectares.
    #[serde(rename = "LotSizeUnits", skip_serializing_if = "Option::is_none")]
    pub lot_size_units: Option<String>,

    /// Free-text lot dimensions (e.g. "100x150").
    #[serde(rename = "LotSizeDimensions", skip_serializing_if = "Option::is_none")]
    pub lot_size_dimensions: Option<String>,

    /// Source for LotSizeArea.
    #[serde(rename = "LotSizeSource", skip_serializing_if = "Option::is_none")]
    pub lot_size_source: Option<String>,

    /// Lot features: BackYard, FrontYard, CornerLot, CulDeSac, Level, etc.
    #[serde(rename = "LotFeatures", skip_serializing_if = "Option::is_none")]
    pub lot_features: Option<Vec<String>>,

    /// Linear feet of road frontage.
    #[serde(rename = "FrontageLength", skip_serializing_if = "Option::is_none")]
    pub frontage_length: Option<Decimal>,

    /// Frontage type: None, Lakefront, RiverFront, Waterfront.
    #[serde(rename = "FrontageType", skip_serializing_if = "Option::is_none")]
    pub frontage_type: Option<String>,

    /// Road frontage type: CityStreet, CountyRoad, Highway, PrivateRoad, etc.
    #[serde(rename = "RoadFrontageType", skip_serializing_if = "Option::is_none")]
    pub road_frontage_type: Option<Vec<String>>,

    /// Road surface type: Asphalt, Concrete, Dirt, Gravel, Paved, etc.
    #[serde(rename = "RoadSurfaceType", skip_serializing_if = "Option::is_none")]
    pub road_surface_type: Option<Vec<String>>,

    /// True if land is leased (not owned) — affects underwriting.
    #[serde(rename = "LandLeaseYN", skip_serializing_if = "Option::is_none")]
    pub land_lease_yn: Option<bool>,

    /// Monthly land lease payment.
    #[serde(rename = "LandLeaseAmount", skip_serializing_if = "Option::is_none")]
    pub land_lease_amount: Option<Decimal>,

    /// Frequency of land lease payment.
    #[serde(
        rename = "LandLeaseAmountFrequency",
        skip_serializing_if = "Option::is_none"
    )]
    pub land_lease_amount_frequency: Option<String>,

    /// Land lease expiration date.
    #[serde(
        rename = "LandLeaseExpirationDate",
        skip_serializing_if = "Option::is_none"
    )]
    pub land_lease_expiration_date: Option<String>,

    // ── Category 10: Rooms (13 fields) ────────────────────────────────────────
    /// Total bedrooms including all levels.
    #[serde(rename = "BedroomsTotal", skip_serializing_if = "Option::is_none")]
    pub bedrooms_total: Option<i32>,

    /// Total bathrooms as decimal: full + half×0.5 + quarter×0.25.
    #[serde(
        rename = "BathroomsTotalDecimal",
        skip_serializing_if = "Option::is_none"
    )]
    pub bathrooms_total_decimal: Option<Decimal>,

    /// Total bathroom count as integer.
    #[serde(
        rename = "BathroomsTotalInteger",
        skip_serializing_if = "Option::is_none"
    )]
    pub bathrooms_total_integer: Option<i32>,

    /// Full bathrooms (toilet + sink + tub or shower).
    #[serde(rename = "BathroomsFull", skip_serializing_if = "Option::is_none")]
    pub bathrooms_full: Option<i32>,

    /// Half bathrooms (toilet + sink only).
    #[serde(rename = "BathroomsHalf", skip_serializing_if = "Option::is_none")]
    pub bathrooms_half: Option<i32>,

    /// Quarter bathrooms (single fixture).
    #[serde(
        rename = "BathroomsOneQuarter",
        skip_serializing_if = "Option::is_none"
    )]
    pub bathrooms_one_quarter: Option<i32>,

    /// Three-quarter bathrooms (toilet + sink + shower, no tub).
    #[serde(
        rename = "BathroomsThreeQuarter",
        skip_serializing_if = "Option::is_none"
    )]
    pub bathrooms_three_quarter: Option<i32>,

    /// Total rooms (MLS counting method varies).
    #[serde(rename = "RoomsTotal", skip_serializing_if = "Option::is_none")]
    pub rooms_total: Option<i32>,

    /// Rooms that could be used as bedrooms.
    #[serde(rename = "BedroomsPossible", skip_serializing_if = "Option::is_none")]
    pub bedrooms_possible: Option<i32>,

    /// Bedrooms on the primary entry level.
    #[serde(rename = "MainLevelBedrooms", skip_serializing_if = "Option::is_none")]
    pub main_level_bedrooms: Option<i32>,

    /// Bathrooms on the primary entry level.
    #[serde(rename = "MainLevelBathrooms", skip_serializing_if = "Option::is_none")]
    pub main_level_bathrooms: Option<i32>,

    /// Basement type: Finished, Unfinished, PartiallyFinished, WalkOut, etc.
    #[serde(rename = "Basement", skip_serializing_if = "Option::is_none")]
    pub basement: Option<Vec<String>>,

    /// True if any basement/below-grade area exists.
    #[serde(rename = "BasementYN", skip_serializing_if = "Option::is_none")]
    pub basement_yn: Option<bool>,

    // ── Category 11: Parking (10 fields) ──────────────────────────────────────
    /// Total parking spaces of all types combined.
    #[serde(rename = "ParkingTotal", skip_serializing_if = "Option::is_none")]
    pub parking_total: Option<i32>,

    /// Enclosed garage spaces.
    #[serde(rename = "GarageSpaces", skip_serializing_if = "Option::is_none")]
    pub garage_spaces: Option<Decimal>,

    /// Covered carport spaces.
    #[serde(rename = "CarportSpaces", skip_serializing_if = "Option::is_none")]
    pub carport_spaces: Option<Decimal>,

    /// All covered spaces (garage + carport).
    #[serde(rename = "CoveredSpaces", skip_serializing_if = "Option::is_none")]
    pub covered_spaces: Option<Decimal>,

    /// Uncovered/open parking spaces.
    #[serde(rename = "OpenParkingSpaces", skip_serializing_if = "Option::is_none")]
    pub open_parking_spaces: Option<Decimal>,

    /// Parking features: Garage, Carport, Driveway, StreetParking, etc.
    #[serde(rename = "ParkingFeatures", skip_serializing_if = "Option::is_none")]
    pub parking_features: Option<Vec<String>>,

    /// True if garage is physically attached to dwelling.
    #[serde(rename = "AttachedGarageYN", skip_serializing_if = "Option::is_none")]
    pub attached_garage_yn: Option<bool>,

    /// True if any garage exists.
    #[serde(rename = "GarageYN", skip_serializing_if = "Option::is_none")]
    pub garage_yn: Option<bool>,

    /// True if any carport exists.
    #[serde(rename = "CarportYN", skip_serializing_if = "Option::is_none")]
    pub carport_yn: Option<bool>,

    /// True if open/uncovered parking exists.
    #[serde(rename = "OpenParkingYN", skip_serializing_if = "Option::is_none")]
    pub open_parking_yn: Option<bool>,

    // ── Category 12: Systems and Utilities (18 fields) ────────────────────────
    /// Heating system type: CentralAir, ForcedAir, HeatPump, Radiant, etc.
    #[serde(rename = "Heating", skip_serializing_if = "Option::is_none")]
    pub heating: Option<Vec<String>>,

    /// True if any heating system is present.
    #[serde(rename = "HeatingYN", skip_serializing_if = "Option::is_none")]
    pub heating_yn: Option<bool>,

    /// Cooling system type: CentralAir, EvaporativeCooler, AtticFan, etc.
    #[serde(rename = "Cooling", skip_serializing_if = "Option::is_none")]
    pub cooling: Option<Vec<String>>,

    /// True if any cooling system is present.
    #[serde(rename = "CoolingYN", skip_serializing_if = "Option::is_none")]
    pub cooling_yn: Option<bool>,

    /// Electrical service type: CircuitBreakers, FuseBox, 110V, 220V, etc.
    #[serde(rename = "Electric", skip_serializing_if = "Option::is_none")]
    pub electric: Option<Vec<String>>,

    /// Gas type: NaturalGas, Propane, None.
    #[serde(rename = "Gas", skip_serializing_if = "Option::is_none")]
    pub gas: Option<Vec<String>>,

    /// Sewer type: PublicSewer, SepticTank, SharedSeptic, Cesspool.
    #[serde(rename = "Sewer", skip_serializing_if = "Option::is_none")]
    pub sewer: Option<Vec<String>>,

    /// Water source: PublicWater, Well, CisternCapture, etc.
    #[serde(rename = "WaterSource", skip_serializing_if = "Option::is_none")]
    pub water_source: Option<Vec<String>>,

    /// Available utilities: ElectricityConnected, NaturalGasAvailable, etc.
    #[serde(rename = "Utilities", skip_serializing_if = "Option::is_none")]
    pub utilities: Option<Vec<String>>,

    /// Appliances included: Dishwasher, Dryer, Refrigerator, Washer, etc.
    #[serde(rename = "Appliances", skip_serializing_if = "Option::is_none")]
    pub appliances: Option<Vec<String>>,

    /// Laundry location: InUnit, LaundryRoom, CommonArea, None.
    #[serde(rename = "LaundryFeatures", skip_serializing_if = "Option::is_none")]
    pub laundry_features: Option<Vec<String>>,

    /// Other equipment: SatelliteDish, SecuritySystem, SolarPanels, Generator.
    #[serde(rename = "OtherEquipment", skip_serializing_if = "Option::is_none")]
    pub other_equipment: Option<Vec<String>>,

    /// Total number of fireplaces.
    #[serde(rename = "FireplacesTotal", skip_serializing_if = "Option::is_none")]
    pub fireplaces_total: Option<i32>,

    /// Fireplace features: Gas, WoodBurning, Electric, etc.
    #[serde(rename = "FireplaceFeatures", skip_serializing_if = "Option::is_none")]
    pub fireplace_features: Option<Vec<String>>,

    /// True if any fireplace exists.
    #[serde(rename = "FireplaceYN", skip_serializing_if = "Option::is_none")]
    pub fireplace_yn: Option<bool>,

    /// True if solar panels are installed.
    #[serde(rename = "SolarPanels", skip_serializing_if = "Option::is_none")]
    pub solar_panels: Option<bool>,

    /// Window features: DoublePaneWindows, Skylights, StormWindows, etc.
    #[serde(rename = "WindowFeatures", skip_serializing_if = "Option::is_none")]
    pub window_features: Option<Vec<String>>,

    /// Door features: FrenchDoors, SlidingGlassDoor, StormDoor, etc.
    #[serde(rename = "DoorFeatures", skip_serializing_if = "Option::is_none")]
    pub door_features: Option<Vec<String>>,

    // ── Category 13: Interior (8 fields) ──────────────────────────────────────
    /// Flooring types: Carpet, CeramicTile, Hardwood, Laminate, Stone, etc.
    #[serde(rename = "Flooring", skip_serializing_if = "Option::is_none")]
    pub flooring: Option<Vec<String>>,

    /// Interior features: BuiltInFeatures, CathedralCeilings, WalkInClosets, etc.
    #[serde(rename = "InteriorFeatures", skip_serializing_if = "Option::is_none")]
    pub interior_features: Option<Vec<String>>,

    /// Security features: CarbonMonoxide, FireAlarm, GatedCommunity, etc.
    #[serde(rename = "SecurityFeatures", skip_serializing_if = "Option::is_none")]
    pub security_features: Option<Vec<String>>,

    /// Accessibility features: GrabBars, HandicapParking, Ramp, etc.
    #[serde(
        rename = "AccessibilityFeatures",
        skip_serializing_if = "Option::is_none"
    )]
    pub accessibility_features: Option<Vec<String>>,

    /// Kitchen features: Granite, IslandKitchen, QuartzCounters, etc.
    #[serde(rename = "KitchenFeatures", skip_serializing_if = "Option::is_none")]
    pub kitchen_features: Option<Vec<String>>,

    /// Master bathroom features: DoubleSinks, SoakingTub, SeparateShower, etc.
    #[serde(
        rename = "MasterBathroomFeatures",
        skip_serializing_if = "Option::is_none"
    )]
    pub master_bathroom_features: Option<Vec<String>>,

    /// Other rooms: BonusRoom, Den, FamilyRoom, Library, OfficeStudy, etc.
    #[serde(rename = "OtherRooms", skip_serializing_if = "Option::is_none")]
    pub other_rooms: Option<Vec<String>>,

    /// Free-text basement description.
    #[serde(
        rename = "BasementDescription",
        skip_serializing_if = "Option::is_none"
    )]
    pub basement_description: Option<String>,

    // ── Category 14: Exterior (8 fields) ──────────────────────────────────────
    /// Exterior features: Balcony, Barbecue, Lighting, PrivateYard, etc.
    #[serde(rename = "ExteriorFeatures", skip_serializing_if = "Option::is_none")]
    pub exterior_features: Option<Vec<String>>,

    /// Patio and porch features: Covered, Deck, Enclosed, FrontPorch, etc.
    #[serde(
        rename = "PatioAndPorchFeatures",
        skip_serializing_if = "Option::is_none"
    )]
    pub patio_and_porch_features: Option<Vec<String>>,

    /// Other structures: Barn, Gazebo, GuestHouse, Shed, Workshop, None.
    #[serde(rename = "OtherStructures", skip_serializing_if = "Option::is_none")]
    pub other_structures: Option<Vec<String>>,

    /// Fencing type: BackYard, Block, ChainLink, Privacy, Wood, None.
    #[serde(rename = "Fencing", skip_serializing_if = "Option::is_none")]
    pub fencing: Option<Vec<String>>,

    /// Vegetation: Brush, Crops, Grassed, PartiallyTreed, Wooded.
    #[serde(rename = "Vegetation", skip_serializing_if = "Option::is_none")]
    pub vegetation: Option<Vec<String>>,

    /// Free-text topography description.
    #[serde(rename = "Topography", skip_serializing_if = "Option::is_none")]
    pub topography: Option<String>,

    /// Municipal zoning code (e.g. "R-1", "PUD", "SF-3").
    #[serde(rename = "Zoning", skip_serializing_if = "Option::is_none")]
    pub zoning: Option<String>,

    /// Free-text zoning description.
    #[serde(rename = "ZoningDescription", skip_serializing_if = "Option::is_none")]
    pub zoning_description: Option<String>,

    // ── Category 15: Pool / Spa (6 fields) ────────────────────────────────────
    /// True if private pool on property.
    #[serde(rename = "PoolPrivateYN", skip_serializing_if = "Option::is_none")]
    pub pool_private_yn: Option<bool>,

    /// Pool features: Gunite, Heated, InGround, SaltWater, Fenced, None, etc.
    #[serde(rename = "PoolFeatures", skip_serializing_if = "Option::is_none")]
    pub pool_features: Option<Vec<String>>,

    /// True if spa or hot tub on property.
    #[serde(rename = "SpaYN", skip_serializing_if = "Option::is_none")]
    pub spa_yn: Option<bool>,

    /// Spa features: Gunite, Heated, InGround, Private, Fiberglass, etc.
    #[serde(rename = "SpaFeatures", skip_serializing_if = "Option::is_none")]
    pub spa_features: Option<Vec<String>>,

    /// True if pool is personal property excluded from the sale.
    #[serde(
        rename = "PoolPersonalProperty",
        skip_serializing_if = "Option::is_none"
    )]
    pub pool_personal_property: Option<bool>,

    /// True if spa is personal property excluded from the sale.
    #[serde(
        rename = "SpaPersonalProperty",
        skip_serializing_if = "Option::is_none"
    )]
    pub spa_personal_property: Option<bool>,

    // ── Category 16: View / Waterfront (8 fields) ─────────────────────────────
    /// True if property has a notable view.
    #[serde(rename = "ViewYN", skip_serializing_if = "Option::is_none")]
    pub view_yn: Option<bool>,

    /// View types: CityLights, GolfCourse, Hills, Lake, Mountain, Ocean, etc.
    #[serde(rename = "View", skip_serializing_if = "Option::is_none")]
    pub view: Option<Vec<String>>,

    /// Name of lake, river, or ocean if waterfront.
    #[serde(rename = "WaterBodyName", skip_serializing_if = "Option::is_none")]
    pub water_body_name: Option<String>,

    /// True if property directly fronts a body of water.
    #[serde(rename = "WaterfrontYN", skip_serializing_if = "Option::is_none")]
    pub waterfront_yn: Option<bool>,

    /// Waterfront features: Lake, River, Ocean, Creek, BeachAccess, None.
    #[serde(rename = "WaterfrontFeatures", skip_serializing_if = "Option::is_none")]
    pub waterfront_features: Option<Vec<String>>,

    /// Linear feet of water frontage.
    #[serde(
        rename = "WaterFrontageLength",
        skip_serializing_if = "Option::is_none"
    )]
    pub water_frontage_length: Option<Decimal>,

    /// True if property has water access (not necessarily waterfront).
    #[serde(rename = "WaterAccess", skip_serializing_if = "Option::is_none")]
    pub water_access: Option<bool>,

    /// Boat facilities: BoatDock, BoatSlip, BoatLift, None.
    #[serde(rename = "BoatFacilities", skip_serializing_if = "Option::is_none")]
    pub boat_facilities: Option<Vec<String>>,

    // ── Category 17: HOA / Community (14 fields) ──────────────────────────────
    /// True if property is subject to an HOA.
    #[serde(rename = "AssociationYN", skip_serializing_if = "Option::is_none")]
    pub association_yn: Option<bool>,

    /// Primary HOA fee amount. Convert to monthly using AssociationFeeFrequency.
    #[serde(rename = "AssociationFee", skip_serializing_if = "Option::is_none")]
    pub association_fee: Option<Decimal>,

    /// HOA fee frequency: Monthly, Annually, Quarterly, SemiAnnually, OneTime.
    #[serde(
        rename = "AssociationFeeFrequency",
        skip_serializing_if = "Option::is_none"
    )]
    pub association_fee_frequency: Option<String>,

    /// What HOA fee includes: CommonArea, Insurance, Pool, Trash, Water, etc.
    #[serde(
        rename = "AssociationFeeIncludes",
        skip_serializing_if = "Option::is_none"
    )]
    pub association_fee_includes: Option<Vec<String>>,

    /// Primary HOA name.
    #[serde(rename = "AssociationName", skip_serializing_if = "Option::is_none")]
    pub association_name: Option<String>,

    /// Primary HOA phone number.
    #[serde(rename = "AssociationPhone", skip_serializing_if = "Option::is_none")]
    pub association_phone: Option<String>,

    /// Secondary HOA fee (master + sub HOA model).
    #[serde(rename = "AssociationFee2", skip_serializing_if = "Option::is_none")]
    pub association_fee2: Option<Decimal>,

    /// Secondary HOA fee frequency.
    #[serde(
        rename = "AssociationFeeFrequency2",
        skip_serializing_if = "Option::is_none"
    )]
    pub association_fee_frequency2: Option<String>,

    /// What secondary HOA fee includes.
    #[serde(
        rename = "AssociationFeeIncludes2",
        skip_serializing_if = "Option::is_none"
    )]
    pub association_fee_includes2: Option<Vec<String>>,

    /// Secondary HOA name.
    #[serde(rename = "AssociationName2", skip_serializing_if = "Option::is_none")]
    pub association_name2: Option<String>,

    /// Secondary HOA phone number.
    #[serde(rename = "AssociationPhone2", skip_serializing_if = "Option::is_none")]
    pub association_phone2: Option<String>,

    /// Pets policy: Yes, No, BreedRestrictions, CatsOK, DogsOK, NumberLimit, etc.
    #[serde(rename = "PetsAllowed", skip_serializing_if = "Option::is_none")]
    pub pets_allowed: Option<String>,

    /// HOA amenities: Pool, Spa, TennisCourt, ClubHouse, GolfCourse, Gym, etc.
    #[serde(
        rename = "AssociationAmenities",
        skip_serializing_if = "Option::is_none"
    )]
    pub association_amenities: Option<Vec<String>>,

    /// Community features: Golf, Gated, Lake, Pool, BikingTrails, Park, etc.
    #[serde(rename = "CommunityFeatures", skip_serializing_if = "Option::is_none")]
    pub community_features: Option<Vec<String>>,

    // ── Category 18: Tax / Legal (14 fields) ──────────────────────────────────
    /// Annual property tax amount. Engine uses this for escrow calculation.
    #[serde(rename = "TaxAnnualAmount", skip_serializing_if = "Option::is_none")]
    pub tax_annual_amount: Option<Decimal>,

    /// Assessed value for tax purposes.
    #[serde(rename = "TaxAssessedValue", skip_serializing_if = "Option::is_none")]
    pub tax_assessed_value: Option<Decimal>,

    /// Tax year to which TaxAnnualAmount applies.
    #[serde(rename = "TaxYear", skip_serializing_if = "Option::is_none")]
    pub tax_year: Option<i32>,

    /// Full legal description of the parcel.
    #[serde(
        rename = "TaxLegalDescription",
        skip_serializing_if = "Option::is_none"
    )]
    pub tax_legal_description: Option<String>,

    /// County assessor parcel number (APN).
    #[serde(rename = "ParcelNumber", skip_serializing_if = "Option::is_none")]
    pub parcel_number: Option<String>,

    /// Legal block number.
    #[serde(rename = "TaxBlock", skip_serializing_if = "Option::is_none")]
    pub tax_block: Option<String>,

    /// Legal lot number.
    #[serde(rename = "TaxLot", skip_serializing_if = "Option::is_none")]
    pub tax_lot: Option<String>,

    /// Tax map reference number.
    #[serde(rename = "TaxMapNumber", skip_serializing_if = "Option::is_none")]
    pub tax_map_number: Option<String>,

    /// Census tract or tax tract number.
    #[serde(rename = "TaxTract", skip_serializing_if = "Option::is_none")]
    pub tax_tract: Option<String>,

    /// Tax record book number.
    #[serde(rename = "TaxBookNumber", skip_serializing_if = "Option::is_none")]
    pub tax_book_number: Option<String>,

    /// Tax status: Taxable, Exempt, ExemptSenior, ExemptVeteran.
    #[serde(rename = "TaxStatusCurrent", skip_serializing_if = "Option::is_none")]
    pub tax_status_current: Option<String>,

    /// Tax exemptions: Agriculture, Homestead, Senior, Veteran, Widow.
    #[serde(rename = "TaxExemptions", skip_serializing_if = "Option::is_none")]
    pub tax_exemptions: Option<Vec<String>>,

    /// Other annual tax assessments: Mello-Roos, special assessments, CID fees.
    #[serde(
        rename = "TaxOtherAnnualAssessmentAmount",
        skip_serializing_if = "Option::is_none"
    )]
    pub tax_other_annual_assessment_amount: Option<Decimal>,

    /// Current land use: Agricultural, Commercial, Industrial, Residential.
    #[serde(rename = "CurrentUse", skip_serializing_if = "Option::is_none")]
    pub current_use: Option<Vec<String>>,

    // ── Category 19: Schools (8 fields) ───────────────────────────────────────
    /// Elementary school name.
    #[serde(rename = "ElementarySchool", skip_serializing_if = "Option::is_none")]
    pub elementary_school: Option<String>,

    /// Elementary school district.
    #[serde(
        rename = "ElementarySchoolDistrict",
        skip_serializing_if = "Option::is_none"
    )]
    pub elementary_school_district: Option<String>,

    /// Middle or junior high school name.
    #[serde(
        rename = "MiddleOrJuniorSchool",
        skip_serializing_if = "Option::is_none"
    )]
    pub middle_or_junior_school: Option<String>,

    /// Middle school district.
    #[serde(
        rename = "MiddleOrJuniorSchoolDistrict",
        skip_serializing_if = "Option::is_none"
    )]
    pub middle_or_junior_school_district: Option<String>,

    /// High school name.
    #[serde(rename = "HighSchool", skip_serializing_if = "Option::is_none")]
    pub high_school: Option<String>,

    /// High school district.
    #[serde(rename = "HighSchoolDistrict", skip_serializing_if = "Option::is_none")]
    pub high_school_district: Option<String>,

    /// Unified school district (overrides above if single district).
    #[serde(rename = "SchoolDistrict", skip_serializing_if = "Option::is_none")]
    pub school_district: Option<String>,

    /// Charter, magnet, or other special school.
    #[serde(rename = "OtherSchool", skip_serializing_if = "Option::is_none")]
    pub other_school: Option<String>,

    // ── Category 20: Green / Sustainability (10 fields) ───────────────────────
    /// Green building certifications: LEED, EnergyStar, GreenPoint, etc.
    #[serde(
        rename = "GreenBuildingVerificationType",
        skip_serializing_if = "Option::is_none"
    )]
    pub green_building_verification_type: Option<Vec<String>>,

    /// Energy-efficient features: Appliances, Doors, HVAC, Insulation, etc.
    #[serde(
        rename = "GreenEnergyEfficient",
        skip_serializing_if = "Option::is_none"
    )]
    pub green_energy_efficient: Option<Vec<String>>,

    /// Renewable energy generation: Solar, Wind, GridTied.
    #[serde(
        rename = "GreenEnergyGeneration",
        skip_serializing_if = "Option::is_none"
    )]
    pub green_energy_generation: Option<Vec<String>>,

    /// Indoor air quality features: ContaminantControl, LowVOC, Ventilation.
    #[serde(
        rename = "GreenIndoorAirQuality",
        skip_serializing_if = "Option::is_none"
    )]
    pub green_indoor_air_quality: Option<Vec<String>>,

    /// Green landscaping: NativeXeriscaping, GrayWater, DroughtTolerantPlants.
    #[serde(rename = "GreenLandscaping", skip_serializing_if = "Option::is_none")]
    pub green_landscaping: Option<Vec<String>>,

    /// Water conservation: LowFlowFixtures, GrayWaterSystem, EfficientHotWater.
    #[serde(
        rename = "GreenWaterConservation",
        skip_serializing_if = "Option::is_none"
    )]
    pub green_water_conservation: Option<Vec<String>>,

    /// Sustainability certifications.
    #[serde(
        rename = "GreenSustainability",
        skip_serializing_if = "Option::is_none"
    )]
    pub green_sustainability: Option<Vec<String>>,

    /// Year of green certification.
    #[serde(
        rename = "GreenVerificationYear",
        skip_serializing_if = "Option::is_none"
    )]
    pub green_verification_year: Option<i32>,

    /// Certifying organization name.
    #[serde(
        rename = "GreenVerificationBody",
        skip_serializing_if = "Option::is_none"
    )]
    pub green_verification_body: Option<String>,

    /// Certification level (e.g. "Platinum", "Gold", "85 points").
    #[serde(
        rename = "GreenVerificationRating",
        skip_serializing_if = "Option::is_none"
    )]
    pub green_verification_rating: Option<String>,

    // ── Category 21: Pricing (15 fields) ──────────────────────────────────────
    /// Current list price. Engine uses this as appraised value proxy if needed.
    #[serde(rename = "ListPrice", skip_serializing_if = "Option::is_none")]
    pub list_price: Option<Decimal>,

    /// Final contracted/closed price.
    #[serde(rename = "ClosePrice", skip_serializing_if = "Option::is_none")]
    pub close_price: Option<Decimal>,

    /// First list price when listing was created.
    #[serde(rename = "OriginalListPrice", skip_serializing_if = "Option::is_none")]
    pub original_list_price: Option<Decimal>,

    /// Last list price before current price change.
    #[serde(rename = "PreviousListPrice", skip_serializing_if = "Option::is_none")]
    pub previous_list_price: Option<Decimal>,

    /// ListPrice ÷ LivingArea.
    #[serde(
        rename = "ListPricePerSquareFoot",
        skip_serializing_if = "Option::is_none"
    )]
    pub list_price_per_square_foot: Option<Decimal>,

    /// ClosePrice ÷ LivingArea.
    #[serde(
        rename = "ClosePricePerSquareFoot",
        skip_serializing_if = "Option::is_none"
    )]
    pub close_price_per_square_foot: Option<Decimal>,

    /// Seller concessions: Yes, No.
    #[serde(rename = "Concessions", skip_serializing_if = "Option::is_none")]
    pub concessions: Option<String>,

    /// Dollar amount of seller concessions.
    #[serde(rename = "ConcessionsAmount", skip_serializing_if = "Option::is_none")]
    pub concessions_amount: Option<Decimal>,

    /// Description of concession terms.
    #[serde(
        rename = "ConcessionsComments",
        skip_serializing_if = "Option::is_none"
    )]
    pub concessions_comments: Option<String>,

    /// Buyer financing types: Conventional, FHA, VA, Cash, USDA, etc.
    #[serde(rename = "BuyerFinancing", skip_serializing_if = "Option::is_none")]
    pub buyer_financing: Option<Vec<String>>,

    /// Net operating income for income properties.
    #[serde(rename = "NetOperatingIncome", skip_serializing_if = "Option::is_none")]
    pub net_operating_income: Option<Decimal>,

    /// Gross income for income properties.
    #[serde(rename = "GrossIncome", skip_serializing_if = "Option::is_none")]
    pub gross_income: Option<Decimal>,

    /// Gross scheduled income for income properties.
    #[serde(
        rename = "GrossScheduledIncome",
        skip_serializing_if = "Option::is_none"
    )]
    pub gross_scheduled_income: Option<Decimal>,

    /// Currently leased units in income property.
    #[serde(
        rename = "NumberOfUnitsLeased",
        skip_serializing_if = "Option::is_none"
    )]
    pub number_of_units_leased: Option<i32>,

    /// Cap rate for income properties.
    #[serde(rename = "CapRate", skip_serializing_if = "Option::is_none")]
    pub cap_rate: Option<Decimal>,

    // ── Category 22: Key Dates (10 fields) ────────────────────────────────────
    /// Listing agreement execution date.
    #[serde(
        rename = "ListingContractDate",
        skip_serializing_if = "Option::is_none"
    )]
    pub listing_contract_date: Option<String>,

    /// Closing/settlement date.
    #[serde(rename = "CloseDate", skip_serializing_if = "Option::is_none")]
    pub close_date: Option<String>,

    /// Offer acceptance date (contract ratification).
    #[serde(
        rename = "PurchaseContractDate",
        skip_serializing_if = "Option::is_none"
    )]
    pub purchase_contract_date: Option<String>,

    /// Date listing became publicly visible on MLS.
    #[serde(rename = "OnMarketDate", skip_serializing_if = "Option::is_none")]
    pub on_market_date: Option<String>,

    /// Date listing was removed from active status.
    #[serde(rename = "OffMarketDate", skip_serializing_if = "Option::is_none")]
    pub off_market_date: Option<String>,

    /// Date listing was withdrawn.
    #[serde(rename = "WithdrawalDate", skip_serializing_if = "Option::is_none")]
    pub withdrawal_date: Option<String>,

    /// Date listing was reactivated after off-market period.
    #[serde(rename = "BackOnMarketDate", skip_serializing_if = "Option::is_none")]
    pub back_on_market_date: Option<String>,

    /// Earliest available possession date.
    #[serde(rename = "AvailabilityDate", skip_serializing_if = "Option::is_none")]
    pub availability_date: Option<String>,

    /// Cumulative days on market (current listing period).
    #[serde(rename = "DaysOnMarket", skip_serializing_if = "Option::is_none")]
    pub days_on_market: Option<i32>,

    /// Total days on market across all listing periods.
    #[serde(
        rename = "CumulativeDaysOnMarket",
        skip_serializing_if = "Option::is_none"
    )]
    pub cumulative_days_on_market: Option<i32>,

    // ── Category 23: Listing Remarks (6 fields) ───────────────────────────────
    /// Public-facing property description.
    #[serde(rename = "PublicRemarks", skip_serializing_if = "Option::is_none")]
    pub public_remarks: Option<String>,

    /// Agent-only remarks (not published to public).
    #[serde(rename = "PrivateRemarks", skip_serializing_if = "Option::is_none")]
    pub private_remarks: Option<String>,

    /// Remarks for third-party syndication portals.
    #[serde(rename = "SyndicationRemarks", skip_serializing_if = "Option::is_none")]
    pub syndication_remarks: Option<String>,

    /// Remarks for buyer's agent eyes only.
    #[serde(rename = "BuyerAgentRemarks", skip_serializing_if = "Option::is_none")]
    pub buyer_agent_remarks: Option<String>,

    /// Disclaimer text for the listing.
    #[serde(rename = "Disclaimer", skip_serializing_if = "Option::is_none")]
    pub disclaimer: Option<String>,

    /// Sites to syndicate listing to.
    #[serde(rename = "SyndicateTo", skip_serializing_if = "Option::is_none")]
    pub syndicate_to: Option<Vec<String>>,

    // ── Category 24: Showing / Media (10 fields) ──────────────────────────────
    /// Instructions for scheduling a showing.
    #[serde(
        rename = "ShowingInstructions",
        skip_serializing_if = "Option::is_none"
    )]
    pub showing_instructions: Option<String>,

    /// Lockbox placement instructions.
    #[serde(rename = "LockBoxLocation", skip_serializing_if = "Option::is_none")]
    pub lock_box_location: Option<String>,

    /// Lockbox type: Combo, Electronic, KeyBox, None.
    #[serde(rename = "LockBoxType", skip_serializing_if = "Option::is_none")]
    pub lock_box_type: Option<String>,

    /// Showing contact type: Agent, GoAndShow, ListingAgentMustAccompany.
    #[serde(rename = "ShowingContactType", skip_serializing_if = "Option::is_none")]
    pub showing_contact_type: Option<Vec<String>>,

    /// Showing contact name.
    #[serde(rename = "ShowingContactName", skip_serializing_if = "Option::is_none")]
    pub showing_contact_name: Option<String>,

    /// Showing contact phone.
    #[serde(
        rename = "ShowingContactPhone",
        skip_serializing_if = "Option::is_none"
    )]
    pub showing_contact_phone: Option<String>,

    /// Count of photos in the Media resource.
    #[serde(rename = "PhotosCount", skip_serializing_if = "Option::is_none")]
    pub photos_count: Option<i32>,

    /// Count of videos in the Media resource.
    #[serde(rename = "VideosCount", skip_serializing_if = "Option::is_none")]
    pub videos_count: Option<i32>,

    /// Count of documents in the Media resource.
    #[serde(rename = "DocumentsCount", skip_serializing_if = "Option::is_none")]
    pub documents_count: Option<i32>,

    /// URL for unbranded virtual tour.
    #[serde(
        rename = "VirtualTourURLUnbranded",
        skip_serializing_if = "Option::is_none"
    )]
    pub virtual_tour_url_unbranded: Option<String>,

    // ── Category 25: Agent / Office (20 fields) ───────────────────────────────
    /// System key for listing agent (Member resource FK).
    #[serde(rename = "ListAgentKey", skip_serializing_if = "Option::is_none")]
    pub list_agent_key: Option<String>,

    /// Numeric form of ListAgentKey.
    #[serde(
        rename = "ListAgentKeyNumeric",
        skip_serializing_if = "Option::is_none"
    )]
    pub list_agent_key_numeric: Option<i64>,

    /// MLS ID of the listing agent.
    #[serde(rename = "ListAgentMlsId", skip_serializing_if = "Option::is_none")]
    pub list_agent_mls_id: Option<String>,

    /// Full name of listing agent.
    #[serde(rename = "ListAgentFullName", skip_serializing_if = "Option::is_none")]
    pub list_agent_full_name: Option<String>,

    /// Listing agent email address.
    #[serde(rename = "ListAgentEmail", skip_serializing_if = "Option::is_none")]
    pub list_agent_email: Option<String>,

    /// Listing agent direct phone.
    #[serde(
        rename = "ListAgentDirectPhone",
        skip_serializing_if = "Option::is_none"
    )]
    pub list_agent_direct_phone: Option<String>,

    /// Listing agent state license number.
    #[serde(
        rename = "ListAgentStateLicense",
        skip_serializing_if = "Option::is_none"
    )]
    pub list_agent_state_license: Option<String>,

    /// System key for listing office (Office resource FK).
    #[serde(rename = "ListOfficeKey", skip_serializing_if = "Option::is_none")]
    pub list_office_key: Option<String>,

    /// MLS ID of listing office.
    #[serde(rename = "ListOfficeMlsId", skip_serializing_if = "Option::is_none")]
    pub list_office_mls_id: Option<String>,

    /// Name of listing office/brokerage.
    #[serde(rename = "ListOfficeName", skip_serializing_if = "Option::is_none")]
    pub list_office_name: Option<String>,

    /// Listing office phone number.
    #[serde(rename = "ListOfficePhone", skip_serializing_if = "Option::is_none")]
    pub list_office_phone: Option<String>,

    /// System key for buyer's agent.
    #[serde(rename = "BuyerAgentKey", skip_serializing_if = "Option::is_none")]
    pub buyer_agent_key: Option<String>,

    /// MLS ID of buyer's agent.
    #[serde(rename = "BuyerAgentMlsId", skip_serializing_if = "Option::is_none")]
    pub buyer_agent_mls_id: Option<String>,

    /// Full name of buyer's agent.
    #[serde(rename = "BuyerAgentFullName", skip_serializing_if = "Option::is_none")]
    pub buyer_agent_full_name: Option<String>,

    /// System key for buyer's office.
    #[serde(rename = "BuyerOfficeKey", skip_serializing_if = "Option::is_none")]
    pub buyer_office_key: Option<String>,

    /// Name of buyer's brokerage.
    #[serde(rename = "BuyerOfficeName", skip_serializing_if = "Option::is_none")]
    pub buyer_office_name: Option<String>,

    /// System key for co-listing agent.
    #[serde(rename = "CoListAgentKey", skip_serializing_if = "Option::is_none")]
    pub co_list_agent_key: Option<String>,

    /// MLS ID of co-listing agent.
    #[serde(rename = "CoListAgentMlsId", skip_serializing_if = "Option::is_none")]
    pub co_list_agent_mls_id: Option<String>,

    /// Full name of co-listing agent.
    #[serde(
        rename = "CoListAgentFullName",
        skip_serializing_if = "Option::is_none"
    )]
    pub co_list_agent_full_name: Option<String>,

    /// System key for co-buyer's agent.
    #[serde(rename = "CoBuyerAgentKey", skip_serializing_if = "Option::is_none")]
    pub co_buyer_agent_key: Option<String>,

    // ── Category 26: Flood / Insurance (4 fields) ─────────────────────────────
    /// FEMA flood zone designation: AE, X, VE, AO, etc.
    /// AE = high-risk annual flood zone (SFHA — requires flood insurance).
    /// X = minimal flood hazard (no flood insurance required).
    #[serde(rename = "FloodZone", skip_serializing_if = "Option::is_none")]
    pub flood_zone: Option<String>,

    /// FIRM flood zone code (more granular than FloodZone).
    #[serde(rename = "FloodZoneCode", skip_serializing_if = "Option::is_none")]
    pub flood_zone_code: Option<String>,

    /// FEMA FIRM panel number for the flood map.
    #[serde(
        rename = "FloodMapPanelNumber",
        skip_serializing_if = "Option::is_none"
    )]
    pub flood_map_panel_number: Option<String>,

    /// Effective date of the FIRM flood map panel.
    #[serde(
        rename = "FloodMapPanelEffectiveDate",
        skip_serializing_if = "Option::is_none"
    )]
    pub flood_map_panel_effective_date: Option<String>,

    // ── Category 27: Senior / Special (3 fields) ──────────────────────────────
    /// True if property is in a 55+ senior restricted community.
    #[serde(rename = "SeniorCommunityYN", skip_serializing_if = "Option::is_none")]
    pub senior_community_yn: Option<bool>,

    /// True if horses are permitted on the property.
    #[serde(rename = "HorseYN", skip_serializing_if = "Option::is_none")]
    pub horse_yn: Option<bool>,

    /// Horse amenities: Corral, Pasture, RidingTrail, Stable, etc.
    #[serde(rename = "HorseAmenities", skip_serializing_if = "Option::is_none")]
    pub horse_amenities: Option<Vec<String>>,
}
