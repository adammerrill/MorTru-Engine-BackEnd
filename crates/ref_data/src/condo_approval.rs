//! FHA condominium project approval lookup.
//!
//! FHA financing of condominium units requires that the project be on HUD's
//! approved condominium project list. This module provides a lookup by
//! HUD project ID and by address fragment.
//!
//! When a property is identified as a condo (from RESO data), the analysis
//! engine must check this store before allowing FHA qualification. A condo
//! in a non-approved project is ineligible for FHA even if all other criteria
//! are met.
//!
//! # Updating
//!
//! HUD publishes the approved list at https://entp.hud.gov/idapp/html/condlook.cfm.
//! Update `data/fha_condo_approved.json` with new approvals, expirations, and
//! revocations. The JSON file is versioned by source date.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// Approval status for an FHA condo project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CondoApprovalStatus {
    Approved,
    Expired,
    Withdrawn,
    Rejected,
}

impl CondoApprovalStatus {
    /// True only for currently approved projects.
    #[must_use]
    pub fn is_currently_approved(self) -> bool {
        self == Self::Approved
    }
}

/// One approved (or expired/rejected) condominium project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FhaCondoProject {
    /// HUD-assigned condominium project ID (e.g., "A000123").
    pub fha_project_id: String,
    pub project_name: String,
    pub address: String,
    pub city: String,
    pub state: String,
    pub zip5: String,
    pub county_fips: String,
    pub status: CondoApprovalStatus,
    /// Date approval expires. `None` for projects that were rejected or never expire.
    pub approval_expiry: Option<NaiveDate>,
    pub units_in_project: Option<u16>,
}

/// Top-level shape of `fha_condo_approved.json`.
#[derive(Debug, Deserialize)]
pub struct FhaCondoApprovedFile {
    pub source_date: String,
    pub projects: Vec<FhaCondoProject>,
}
