//! RESO Data Dictionary 2.0 — Lookup value catalogs.
//!
//! Each enum represents a RESO standard Lookup field with its canonical
//! string values exactly as defined in DD 2.0. The string values are the
//! authoritative RESO names — casing matters.

pub mod property_type;
pub mod standard_status;

pub use property_type::{ResoPropertySubType, ResoPropertyType};
pub use standard_status::ResoStandardStatus;
