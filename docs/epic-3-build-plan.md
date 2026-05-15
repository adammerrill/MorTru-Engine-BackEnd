# Epic 3 — RESO 2.0 Property Data
## Crate: `reso` | Target: ~165 tests | Parallelizable with Epic 4

---

## Purpose

The `reso` crate ingests MLS listing data from the RESO 2.0 Data Dictionary
standard. When a borrower applies using an MLS listing address, the engine
pulls property data from the MLS feed to pre-populate the loan scenario —
property type, assessed value, taxes, HOA, and GPS coordinates for FIPS
geocoding.

The crate consumes **65 of the 235 RESO 2.0 fields**. Everything else is
irrelevant to mortgage underwriting.

## Crate structure

```
crates/reso/
├── src/
│   ├── lib.rs             pub use re-exports + ResoError
│   ├── error.rs           ResoError type
│   ├── property_raw.rs    PropertyReso struct (65 fields, all Option<String>)
│   ├── property_types.rs  RESO → PropertyType enum mapping
│   ├── property_enriched.rs PropertyEnriched typed output struct
│   ├── parse.rs           PropertyReso → PropertyEnriched conversion
│   ├── geocode.rs         FCC FIPS resolution async client
│   ├── repository.rs      ResoRepository trait + InMemoryResoRepo
│   └── reconcile.rs       RESO ↔ MISMO address reconciliation
└── tests/
    ├── property_type_tests.rs
    ├── parse_tests.rs
    ├── geocode_tests.rs
    ├── repository_tests.rs
    ├── reconcile_tests.rs
    └── epic3_gate.rs
```

---

## Task 3.1 — PropertyReso raw struct + ResoError

**Target:** ~25 tests

### ResoError

```rust
#[derive(Debug, thiserror::Error)]
pub enum ResoError {
    #[error("RESO field '{field}' has invalid value '{value}': {detail}")]
    InvalidField { field: &'static str, value: String, detail: String },

    #[error("RESO field '{field}' is required but absent")]
    MissingField { field: &'static str },

    #[error("RESO property type '{0}' is not recognized")]
    UnknownPropertyType(String),

    #[error("FCC geocoding failed: {0}")]
    GeocodeFailed(String),

    #[error("FCC geocoding returned no result for ({lat}, {lon})")]
    GeocodeNoResult { lat: f64, lon: f64 },

    #[error("Repository error: {0}")]
    Repository(String),
}
```

### PropertyReso — 65 consumed fields

All fields are `Option<String>` or `Option<f64>` (matching RESO's nullable
design). Deserialized directly from MLS JSON feed via serde.

| Category | RESO field name | Our field name |
|---|---|---|
| ID | `ListingKey` | `listing_key` |
| ID | `ListingId` | `listing_id` |
| ID | `OriginatingSystemName` | `mls_name` |
| Address | `UnparsedAddress` | `unparsed_address` |
| Address | `StreetNumber` | `street_number` |
| Address | `StreetDirPrefix` | `street_dir_prefix` |
| Address | `StreetName` | `street_name` |
| Address | `StreetSuffix` | `street_suffix` |
| Address | `StreetDirSuffix` | `street_dir_suffix` |
| Address | `UnitNumber` | `unit_number` |
| Address | `City` | `city` |
| Address | `StateOrProvince` | `state` |
| Address | `PostalCode` | `postal_code` |
| Address | `CountyOrParish` | `county` |
| Geo | `Latitude` | `latitude` (`Option<f64>`) |
| Geo | `Longitude` | `longitude` (`Option<f64>`) |
| Geo | `MLSAreaMajor` | `mls_area` |
| Property | `PropertyType` | `property_type` |
| Property | `PropertySubType` | `property_sub_type` |
| Property | `YearBuilt` | `year_built` |
| Property | `AboveGradeFinishedArea` | `above_grade_sqft` |
| Property | `LivingArea` | `living_area_sqft` |
| Property | `BedroomsTotal` | `bedrooms_total` |
| Property | `BathroomsTotalInteger` | `bathrooms_total` |
| Property | `StoriesTotal` | `stories_total` |
| Property | `GarageSpaces` | `garage_spaces` |
| Property | `PoolPrivateYN` | `pool_yn` |
| Property | `NewConstructionYN` | `new_construction_yn` |
| Property | `AttachedGarageYN` | `attached_garage_yn` |
| Property | `FireplaceYN` | `fireplace_yn` |
| HOA | `AssociationYN` | `hoa_yn` |
| HOA | `AssociationFee` | `hoa_fee` |
| HOA | `AssociationFeeFrequency` | `hoa_fee_frequency` |
| HOA | `AssociationFee2` | `hoa_fee_2` |
| HOA | `AssociationFeeFrequency2` | `hoa_fee_frequency_2` |
| HOA | `AssociationName` | `hoa_name` |
| Tax | `TaxAnnualAmount` | `tax_annual_amount` |
| Tax | `TaxYear` | `tax_year` |
| Tax | `TaxLegalDescription` | `tax_legal_description` |
| Listing | `StandardStatus` | `standard_status` |
| Listing | `ListPrice` | `list_price` |
| Listing | `OriginalListPrice` | `original_list_price` |
| Listing | `ClosePrice` | `close_price` |
| Listing | `CloseDate` | `close_date` |
| Listing | `DaysOnMarket` | `days_on_market` |
| Listing | `PurchaseContractDate` | `purchase_contract_date` |
| Listing | `ListingContractDate` | `listing_contract_date` |
| MH/Modular | `MobileHomeLength` | `mh_length` |
| MH/Modular | `MobileHomeWidth` | `mh_width` |
| MH/Modular | `MobileHomeTitleNumber` | `mh_title_number` |
| Zoning | `Zoning` | `zoning` |
| Zoning | `ZoningDescription` | `zoning_description` |
| Build | `StructureType` | `structure_type` |
| Build | `ArchitecturalStyle` | `architectural_style` |
| Build | `ConstructionMaterials` | `construction_materials` |
| Build | `FoundationDetails` | `foundation_details` |
| HOI | `InsuranceExpense` | `annual_hoi` |
| Condo | `AssociationFeeIncludes` | `hoa_fee_includes` |
| Condo | `StoriesInBuilding` | `stories_in_building` |
| System | `SourceSystemModificationTimestamp` | `modified_at` |
| System | `SourceSystemKey` | `source_key` |
| System | `PhotosCount` | `photos_count` |
| System | `PublicRemarks` | `public_remarks` (for agent notes) |

---

## Task 3.2 — RESO property type/sub-type enum mapping

**Target:** ~30 tests

Maps RESO `PropertyType` + `PropertySubType` string combinations to our
`types::PropertyType` enum. Must handle:
- Standard RESO strings
- Known MLS feed variations (ABOR "Single Family Resi", etc.)
- Condo vs. co-op vs. townhouse vs. PUD distinctions
- Manufactured vs. modular vs. mobile home (critical — affects eligibility)
- Multi-family unit count (2–4 unit)

### Mapping table

| RESO PropertyType | RESO PropertySubType | → types::PropertyType |
|---|---|---|
| Residential | Single Family Residence | SingleFamilyDetached |
| Residential | Single Family Resi *(ABOR)* | SingleFamilyDetached |
| Residential | Single Family | SingleFamilyDetached |
| Residential | Condominium | Condominium |
| Residential | Condo | Condominium |
| Residential | Townhouse | Townhouse |
| Residential | Town House | Townhouse |
| Residential | Cooperative | Cooperative |
| Residential | Co-Op | Cooperative |
| Residential | Manufactured Home | ManufacturedHome |
| Residential | Modular | Modular |
| Residential | Modular Home | Modular |
| Residential | Mobile Home | MobileHome |
| Residential | 2 Family | TwoUnit |
| Residential | Duplex | TwoUnit |
| Residential | Triplex | ThreeUnit |
| Residential | 3 Family | ThreeUnit |
| Residential | Fourplex | FourUnit |
| Residential | 4 Family | FourUnit |
| Residential | Attached *(no sub-type)* | SingleFamilyAttached |
| Residential | Detached *(no sub-type)* | SingleFamilyDetached |
| Residential | Row/Townhouse | Townhouse |

### Priority rule

`PropertySubType` is checked first (more specific). If absent, fallback
to `PropertyType` alone. If still ambiguous, return
`ResoError::UnknownPropertyType`.

**MobileHome is always ResoError::IneligiblePropertyType** — callers must
handle this before constructing a loan scenario.

---

## Task 3.3 — PropertyReso → PropertyEnriched

**Target:** ~40 tests

`PropertyEnriched` is the typed output of parsing a `PropertyReso`. All
domain types from the `types` crate are used.

```rust
pub struct PropertyEnriched {
    // Identification
    pub listing_key:      Option<MlsListingKey>,
    pub listing_id:       Option<String>,
    pub mls_name:         Option<String>,

    // Address
    pub unparsed_address: Option<String>,
    pub street_number:    Option<String>,
    pub street_dir_prefix:Option<String>,
    pub street_name:      Option<String>,
    pub street_suffix:    Option<String>,
    pub street_dir_suffix:Option<String>,
    pub unit_number:      Option<String>,
    pub city:             String,
    pub state:            StateCode,
    pub postal_code:      String,
    pub county:           Option<String>,

    // Geolocation (set by FCC geocoder in Task 3.4)
    pub latitude:         Option<f64>,
    pub longitude:        Option<f64>,
    pub fips_code:        Option<FipsCode>,
    pub cbsa_code:        Option<CbsaCode>, // set by Epic 5 enrich

    // Property classification
    pub property_type:    PropertyType,
    pub year_built:       Option<u16>,
    pub gross_living_area:Option<u32>,   // sq ft
    pub bedrooms:         Option<u8>,
    pub bathrooms:        Option<u8>,
    pub stories:          Option<u8>,
    pub new_construction: bool,

    // Valuation
    pub list_price:       Option<Cents>,
    pub close_price:      Option<Cents>,
    pub standard_status:  Option<ListingStatus>,

    // Taxes
    pub annual_tax:       Option<Cents>,
    pub tax_year:         Option<u16>,

    // HOI (if available from feed)
    pub annual_hoi:       Option<Cents>,

    // HOA
    pub hoa_yn:           bool,
    pub hoa_monthly:      Option<Cents>,   // normalized to monthly
    pub hoa_annual:       Option<Cents>,   // normalized to annual
    pub hoa_monthly_2:    Option<Cents>,   // second HOA (condos)

    // Metadata
    pub modified_at:      Option<String>,
}
```

### HOA frequency normalization

RESO reports HOA fees with a separate frequency field:
```
Monthly:   hoa_monthly = fee; hoa_annual = fee × 12
Quarterly: hoa_monthly = fee / 3; hoa_annual = fee × 4
Annual:    hoa_monthly = fee / 12; hoa_annual = fee
```

**Rounding:** half-up to nearest cent for monthly when dividing annual/quarterly.

### GLA field precedence

RESO has two area fields:
1. `AboveGradeFinishedArea` — preferred (excludes basement)
2. `LivingArea` — fallback (may include basement in some markets)

Use `AboveGradeFinishedArea` if present, else `LivingArea`.

---

## Task 3.4 — FCC FIPS geocoding client

**Target:** ~25 tests

### API specification

```
GET https://geo.fcc.gov/api/census/block/find?lat={lat}&lon={lon}&format=json

Response:
{
  "status": "OK",
  "Block": { "FIPS": "484092704001067", ... },
  "County": { "FIPS": "48409", "name": "Hays" },
  "State": { "FIPS": "48", "code": "TX", "name": "Texas" }
}
```

The county FIPS is 5 digits: `"48409"` — this is what we need.

### Trait design (testable)

```rust
#[async_trait::async_trait]
pub trait FipsGeocoderClient: Send + Sync {
    async fn resolve(&self, lat: f64, lon: f64) -> Result<FipsCode, ResoError>;
}

/// Production implementation using reqwest
pub struct FccGeocoderClient {
    client: reqwest::Client,
    base_url: String, // allows test override
}

/// In-memory mock for unit tests
pub struct MockGeocoderClient {
    responses: HashMap<(OrderedF64, OrderedF64), FipsCode>,
}
```

### Error handling

- HTTP non-200: `ResoError::GeocodeFailed`
- `status != "OK"`: `ResoError::GeocodeNoResult`
- Missing `County.FIPS`: `ResoError::GeocodeNoResult`
- Network timeout: `ResoError::GeocodeFailed` with timeout message
- FipsCode parse failure: `ResoError::InvalidField`

### Rate limiting

The FCC API has no formal rate limit but recommends polite usage:
- Request timeout: 5 seconds
- Retry once on 5xx
- No retry on 4xx or invalid coordinate

---

## Task 3.5 — ResoRepository trait

**Target:** ~20 tests

```rust
#[async_trait::async_trait]
pub trait ResoRepository: Send + Sync {
    /// Fetch a property by its MLS listing key.
    async fn get_by_listing_key(
        &self,
        key: &MlsListingKey,
    ) -> Result<Option<PropertyReso>, ResoError>;

    /// Fetch by address — fuzzy match within a postal code.
    async fn get_by_address(
        &self,
        street: &str,
        city: &str,
        state: StateCode,
        postal_code: &str,
    ) -> Result<Option<PropertyReso>, ResoError>;
}

/// In-memory implementation for tests
pub struct InMemoryResoRepo {
    by_key:     HashMap<MlsListingKey, PropertyReso>,
    by_address: HashMap<String, MlsListingKey>, // normalized → key
}

/// Postgres implementation (stubbed, wired in Epic 14)
pub struct PostgresResoRepo {
    pool: sqlx::PgPool,
}
```

---

## Task 3.6 — RESO ↔ MISMO address reconciliation

**Target:** ~20 tests

When both a RESO `PropertyEnriched` and a MISMO `CollateralParsed` are
available for the same property, their address data must be reconciled:

### Reconciliation priority (FIPS)

```
Priority 1: FCC geocoded FIPS from RESO lat/lng (most authoritative)
Priority 2: MISMO FIPSCode XML field (explicit in document)
Priority 3: Derived from RESO state + county FIPS components
Priority 4: Deferred to Epic 5 (FCC API call at enrichment time)
```

### Field precedence

| Field | Source priority |
|---|---|
| `fips_code` | RESO geocoded > MISMO FIPSCode > derived |
| `state` | MISMO (legally authoritative for the loan) |
| `city` | MISMO (from loan application) |
| `postal_code` | RESO (from MLS, often more current) |
| `county` | RESO (from MLS, authoritative) |
| `list_price / sales_price` | MISMO (from purchase contract) |
| `appraised_value` | MISMO (from appraisal, always) |
| `annual_tax` | RESO (from tax records) > MISMO estimate |
| `annual_hoi` | RESO (from listing) > MISMO estimate |
| `hoa_yn / hoa_monthly` | RESO (from listing) > MISMO |

```rust
pub struct ReconciliationResult {
    pub fips_code:       Option<FipsCode>,
    pub state:           StateCode,
    pub city:            String,
    pub postal_code:     String,
    pub county:          Option<String>,
    pub annual_tax:      Option<Cents>,
    pub annual_hoi:      Option<Cents>,
    pub hoa_yn:          bool,
    pub hoa_monthly:     Option<Cents>,
    pub fips_source:     FipsSource, // audit trail
    pub conflicts:       Vec<ReconciliationConflict>,
}

pub enum FipsSource {
    ResoGeocoded,
    MismoXmlField,
    DerivedFromComponents,
    Deferred,
}
```

---

## Task 3.7 — Epic 3 gate

**Target:** ~15 tests

Gate tests verify the complete parse pipeline across all property types
represented in the RESO standard. All tests use fixture JSON files at
`crates/reso/tests/fixtures/`.

### Fixture files

| File | Property type | Key test |
|---|---|---|
| `sfr_kyle_tx.json` | SingleFamilyDetached | Full field parse, HOA normalization |
| `condo_austin_tx.json` | Condominium | Dual HOA, stories-in-building |
| `mh_hays_county_tx.json` | ManufacturedHome | MH dimensions, title number |
| `mobile_home_reject.json` | MobileHome | Returns IneligiblePropertyType |
| `duplex_san_marcos_tx.json` | TwoUnit | Multi-unit count parse |

### Gate assertions

1. All 5 fixture types parse without error (except mobile home)
2. Mobile home returns the correct error type
3. HOA monthly/annual normalization is correct for quarterly fee
4. FCC mock client returns the expected FIPS for Kyle TX coordinates
5. Address reconciliation: RESO FIPS overrides MISMO when both present
6. `PropertyEnriched` from SFR fixture matches all reference values

---

## Reference fixture — SFR Kyle TX

Reference values for `crates/reso/tests/fixtures/sfr_kyle_tx.json`:

| RESO field | Raw value | Parsed value |
|---|---|---|
| `PropertyType` | "Residential" | — |
| `PropertySubType` | "Single Family Residence" | `SingleFamilyDetached` |
| `City` | "Kyle" | "Kyle" |
| `StateOrProvince` | "TX" | `StateCode::TX` |
| `PostalCode` | "78640" | "78640" |
| `CountyOrParish` | "Hays" | Some("Hays") |
| `Latitude` | 30.0152 | 30.0152f64 |
| `Longitude` | -97.8798 | -97.8798f64 |
| `ListPrice` | "459000" | `Cents(45_900_000)` |
| `TaxAnnualAmount` | "10523" | `Cents(1_052_300)` |
| `TaxYear` | "2024" | Some(2024) |
| `AssociationYN` | "1" | true |
| `AssociationFee` | "250" | — |
| `AssociationFeeFrequency` | "Monthly" | `Cents(25_000)/mo` |
| `YearBuilt` | "2018" | Some(2018) |
| `AboveGradeFinishedArea` | "2450" | Some(2450) |

FCC geocode result for (30.0152, -97.8798): `FipsCode("48209")` (Hays County TX)

---

## Dependencies to add

The `reso` crate adds:
- `serde_json = { workspace = true }` (already in workspace)
- `async-trait = "0.1"` (for trait objects)
- `reqwest = { version = "0.12", features = ["json"] }` (FCC HTTP client)
- `tokio = { workspace = true, features = ["rt-multi-thread"] }` (for async tests)

Dev dependencies:
- `mockito = "1"` (for mocking FCC HTTP responses in tests)

