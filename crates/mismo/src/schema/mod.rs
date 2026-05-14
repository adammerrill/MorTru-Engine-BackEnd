//! MISMO 3.4 document element structs.
//!
//! Each submodule covers a section of the MISMO MESSAGE container:
//!
//! | Module | MISMO Element | Task |
//! |---|---|---|
//! | `loan_terms` | `MORTGAGE_TERMS`, `AMORTIZATION` | 2.3 ✅ |
//! | `collateral` | `COLLATERAL/SUBJECT_PROPERTY`, `ADDRESS`, `HOA_DETAIL` | 2.4 |
//! | `party` | `PARTIES/PARTY`, `BORROWER`, `ClosingContext` | 2.5 |
//! | `mi` | `MI_DATA_DETAIL` — all four MI programs | 2.6 |
//! | `lender_comp` | Origination compensation (Section A) | 2.7 |
//! | `closing_cost` | `CLOSING_COST` — Sections A–H | 2.8 |
//! | `aus` | `AUTOMATED_UNDERWRITING_SYSTEM` | 2.8 |
//! | `message` | `MESSAGE/DEAL_SETS/DEAL_SET/DEALS/DEAL` root | 2.9 |

pub mod collateral;
pub mod loan_terms;
pub mod mi;
pub mod party;
// Tasks 2.7–2.9 land below as they are delivered.
