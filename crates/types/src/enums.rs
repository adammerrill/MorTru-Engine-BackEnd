//! Common enumerations used throughout the engine.
//!
//! Every enum derives `Copy, Clone, Debug, PartialEq, Eq, Hash,
//! Serialize, Deserialize` and provides `to_mismo()` / `from_mismo()`
//! where a MISMO 3.4 enumeration applies, plus `to_reso_lookup()` /
//! `from_reso_lookup()` where a RESO 2.0 lookup applies.

pub mod amortization_type;
pub mod loan_product;
pub mod loan_purpose;
pub mod misc;
pub mod occupancy;
pub mod program_code;
pub mod property_type;
