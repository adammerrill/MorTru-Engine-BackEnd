//! MISMO 3.4 string enumeration types and their mappings to domain types.
//!
//! Each submodule covers a logical group of related enumerations:
//!
//! | Module | Contents |
//! |---|---|
//! | `loan_type` | `LoanPurpose`, `AmortizationType`, `LienPriority`, `ProgramCode` parse/serialize |
//! | `property`  | `PropertyType`, `Occupancy` parse/serialize |
//! | `party`     | `VaFundingFeeTier`, `AffordableLendingProgram` |
//! | `mi`        | `MismoMiProgramType`, `MiRenewalType`, `MiFirstPremiumType` |
//! | `aus`       | `AusType` parse, `AusRecommendation` |
//! | `fee`       | `FeeSection`, `FeePaidBy`, `FeeCalculationType` |
//! | `comp`      | `CompType`, `CompDisclosure` |
//!
//! # Usage pattern
//!
//! XML schema structs (Tasks 2.3–2.9) hold raw `String` fields. The enum
//! modules convert those strings to typed domain values:
//!
//! ```ignore
//! use mismo::enums::loan_type;
//!
//! let purpose = loan_type::try_loan_purpose(&mortgage_terms.loan_purpose_type)?;
//! ```

pub mod aus;
pub mod comp;
pub mod fee;
pub mod loan_type;
pub mod mi;
pub mod party;
pub mod property;
