//! RESO Data Dictionary 2.0 — Property resource for the MorTru Engine.
//!
//! # Standard
//!
//! RESO Data Dictionary 2.0 (DD 2.0), finalized 2021.  
//! Reference: <https://ddwiki.reso.org/display/DDW20>  
//! Delivered via RESO Web API 2.0 (OData v4 JSON).
//!
//! # Scope
//!
//! This crate captures all 235 RESO 2.0 Property resource fields identified
//! as relevant to residential mortgage analysis, organized across 27 categories.
//! All fields are `Option<T>` — the RESO Web API omits absent fields.
//! Unknown fields are silently ignored (future-proof against new RESO fields).
//!
//! All 235 fields remain accessible via [`property::PropertyReso`] even if
//! the engine currently uses only a subset — the full payload is preserved
//! for future platform expansion without schema migration.
//!
//! # Field categories
//!
//! | Category | Fields |
//! |---|---|
//! | Identity (10), Timestamps (8), Status (8), Type (4) | 30 |
//! | Address (22), Coordinates (5) | 27 |
//! | Physical (20), Construction (14), Lot (15) | 49 |
//! | Rooms (13), Parking (10), Systems (18) | 41 |
//! | Interior (8), Exterior (8), Pool/Spa (6), View (8) | 30 |
//! | HOA (14), Tax/Legal (14), Schools (8), Green (10) | 46 |
//! | Pricing (15), Dates (10), Remarks (6), Showing (10) | 41 |
//! | Agent/Office (20), Flood (4), Senior/Special (3) | 27 |
//!
//! # Tasks
//!
//! - Task 3.1: Crate scaffold + `ResoError`
//! - Task 3.2: Identity, Timestamps, Status, Type fields
//! - Task 3.3: Address + Location fields
//! - Task 3.4: Physical, Construction, Lot fields
//! - Task 3.5: Rooms, Parking, Systems fields
//! - Task 3.6: Interior, Exterior, Pool, View fields
//! - Task 3.7: HOA, Tax, Schools, Green fields
//! - Task 3.8: Pricing, Dates, Remarks, Showing, Agent, Flood fields
//! - Task 3.9: RESO lookup catalog (PropertyType, SubType, Status)
//! - Task 3.10: FCC FIPS geocoding client
//! - Task 3.11: `PropertyEnriched` + `PropertyReso::enrich()`
//! - Task 3.12: RESO ↔ MISMO bridge
//! - Task 3.13: Epic 3 gate test

pub mod bridge;
pub mod enriched;
pub mod error;
pub mod fcc;
pub mod lookups;
pub mod property;

pub use bridge::{
    enriched_to_mismo_address, hoa_for_mismo_expense, property_sub_type_to_mismo,
    select_valuation_price, state_code_to_mismo, MismoAddressFields,
};
pub use enriched::PropertyEnriched;
pub use error::{ResoError, ResoResult};
pub use fcc::{parse_fcc_response, FccClient, FipsResolution};
pub use lookups::{ResoPropertySubType, ResoPropertyType, ResoStandardStatus};
pub use property::PropertyReso;
