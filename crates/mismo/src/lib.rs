//! MISMO 3.4 reference-model schema for the Meridian Mortgage Engine.
//!
//! This crate provides:
//! - **Parsing**: convert MISMO 3.4 XML documents into typed Rust structs
//! - **Serialization**: produce valid MISMO 3.4 XML from typed Rust structs
//! - **Enum mappings**: translate MISMO string enumerations to/from
//!   domain types defined in the [`types`] crate
//!
//! # Document structure
//!
//! A MISMO 3.4 document follows the container hierarchy:
//! ```text
//! MESSAGE
//! └── DEAL_SETS
//!     └── DEAL_SET
//!         └── DEALS
//!             └── DEAL
//!                 ├── LOANS/LOAN          — mortgage terms, MI, fees, AUS
//!                 ├── PARTIES/PARTY       — borrowers, VA/USDA data
//!                 └── COLLATERALS/COLLATERAL — property, address, HOA
//! ```
//!
//! # Entry point
//!
//! The primary public type is [`schema::message::MismoMessage`] (introduced in
//! Task 2.9). Parse a complete loan document with:
//! ```ignore
//! let msg = MismoMessage::from_xml(xml_str)?;
//! let deal = msg.parse_all()?;
//! ```
//!
//! # Crate structure
//!
//! ```text
//! mismo/
//! ├── enums/   — MISMO string enum ↔ domain type mappings   (Task 2.2)
//! ├── schema/  — MISMO element structs                       (Tasks 2.3–2.9)
//! └── xml/     — XML serialization/deserialization helpers   (Task 2.1)
//! ```
//!
//! # Error handling
//!
//! All fallible operations in this crate return [`Result<T>`] which is an
//! alias for `std::result::Result<T, MismoError>`. See [`MismoError`] for
//! the full variant set.

pub mod enums;
pub mod schema;
pub mod xml;

mod error;
pub use error::{MismoError, Result};
