//! Task 4.26 — Seller concession / Interested Party Contribution (IPC) caps.
//!
//! Agencies cap how much an "interested party" (seller, builder, real-estate
//! agent, lender affiliate) may contribute toward a buyer's closing costs and
//! prepaids. Contributions above the cap become **sales concessions**: the
//! sales price is reduced by the excess and LTV/CLTV is recomputed against the
//! lower value. This module supplies the caps and the within-limit evaluation.
//!
//! # Full agency coverage (sources of truth)
//!
//! * **FHA** — 6% of sales price. HUD Handbook 4000.1 (II.A.4).
//! * **USDA** — 6% of sales price. HB-1-3555, Chapter 6.
//! * **VA** — 4% of "reasonable value" for *seller concessions* (a defined
//!   subset: prepaids, funding-fee payment, excess discount points, debt
//!   payoff). Normal buyer closing costs paid by the seller are NOT counted in
//!   the 4% and are effectively unlimited. VA Lenders Handbook (Pamphlet 26-7),
//!   Chapter 8.
//! * **FNMA** — tiered by occupancy and LTV. Selling Guide B3-4.1-02
//!   (terminology updated to "maximum financing concessions" per SEL-2025-03,
//!   tier values unchanged).
//! * **FHLMC** — identical tier structure. Single-Family Seller/Servicer Guide
//!   Section 5501.5.
//! * **GNMA** — sets no IPC limits of its own. Ginnie Mae securitizes FHA, VA,
//!   and USDA loans, so a GNMA-pooled loan inherits the **insuring agency's**
//!   cap. Modeled via [`gnma_inherits_program`], not as a catalog row.
//!
//! # MISMO / RESO alignment
//!
//! Maps to MISMO `INTERESTED_PARTY_CONTRIBUTION` / `FINANCING_CONCESSION` /
//! `SALES_CONCESSION`. The cap percentage is `InterestedPartyContributionPercent`
//! and the dollar limit is `InterestedPartyContributionAmount`. The cap basis is
//! the **RESO** `ListPrice`/`ClosePrice` (lesser of price or appraised value when
//! the guide requires it). Occupancy maps to MISMO `PropertyUsageType`.
//!
//! # Provenance
//!
//! Every public method returns a `Derived<T>` so the cap and the within-limit
//! determination carry the agency citation and the ordered rule trail.
//!
//! # Updating
//!
//! Caps change rarely but do (e.g. periodic GSE bulletins). Update by adding
//! `seller_concession_caps_{YYYY}.json`; no Rust changes needed.

use serde::{Deserialize, Serialize};
use types::{Cents, Derived, Provenance};

/// Property occupancy (maps to MISMO `PropertyUsageType`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Occupancy {
    Primary,
    SecondHome,
    Investment,
}

/// The basis the cap percentage is applied to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapBasis {
    /// Percentage of the contract sales price.
    SalesPrice,
    /// Percentage of the lesser of sales price or appraised value.
    LesserOfPriceOrValue,
    /// Percentage of VA "reasonable value" (the appraised value).
    ReasonableValue,
}

/// One cap row: program × occupancy × LTV tier (one entry of the catalog file).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SellerConcessionCap {
    /// Program/investor: "fha","va","usda","fnma","fhlmc".
    pub program: String,
    pub occupancy: Occupancy,
    /// LTV tier lower bound in bps, exclusive (0 for the lowest tier).
    pub ltv_min_bps: u32,
    /// LTV tier upper bound in bps, inclusive.
    pub ltv_max_bps: u32,
    /// The cap, in basis points of the basis (600 = 6%).
    pub cap_bps: u32,
    pub basis: CapBasis,
    /// Human note (e.g. VA's distinction between concessions and normal costs).
    pub note: String,
}

/// Inputs needed to resolve the applicable cap.
#[derive(Debug, Clone)]
pub struct ConcessionCapInput {
    pub program: String,
    pub occupancy: Occupancy,
    pub ltv_bps: u32,
}

/// Result of checking a proposed contribution against the cap.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConcessionOutcome {
    pub cap_bps: u32,
    pub max_allowed: Cents,
    pub proposed: Cents,
    pub within_limit: bool,
    /// Amount over the cap that must be treated as a sales concession
    /// (deducted from sales price, triggering LTV recalculation). 0 if within.
    pub excess: Cents,
}

/// Maps a GNMA-securitized loan to the insuring agency whose cap governs.
/// Returns the program key to look up, or `None` if the loan type isn't
/// GNMA-eligible (GNMA pools only FHA/VA/USDA).
#[must_use]
pub fn gnma_inherits_program(loan_type: &str) -> Option<&'static str> {
    match loan_type.to_ascii_lowercase().as_str() {
        "fha" => Some("fha"),
        "va" => Some("va"),
        "usda" | "rd" | "rural_development" => Some("usda"),
        _ => None,
    }
}

/// Top-level shape of `seller_concession_caps_{year}.json`.
#[derive(Debug, Deserialize)]
pub struct SellerConcessionCapFile {
    pub effective_date: String,
    pub source_citation: String,
    pub caps: Vec<SellerConcessionCap>,
}

impl SellerConcessionCapFile {
    fn provenance_for(
        &self,
        cap: &SellerConcessionCap,
        file: &str,
        req: u16,
        res: u16,
    ) -> Provenance {
        Provenance {
            dataset: "seller_concession_caps".to_owned(),
            source_file: file.to_owned(),
            source_citation: self.source_citation.clone(),
            effective_date: self.effective_date.clone(),
            record_id: format!(
                "{}_{:?}_{}-{}",
                cap.program, cap.occupancy, cap.ltv_min_bps, cap.ltv_max_bps
            ),
            requested_version: req,
            resolved_version: res,
        }
    }

    fn find(&self, input: &ConcessionCapInput) -> Option<&SellerConcessionCap> {
        self.caps.iter().find(|c| {
            c.program.eq_ignore_ascii_case(&input.program)
                && c.occupancy == input.occupancy
                && input.ltv_bps > c.ltv_min_bps
                && input.ltv_bps <= c.ltv_max_bps
        })
    }

    /// Resolve the applicable cap, wrapped with provenance.
    pub fn cap(
        &self,
        input: &ConcessionCapInput,
        file: &str,
        req: u16,
        res: u16,
    ) -> Option<Derived<SellerConcessionCap>> {
        let cap = self.find(input)?;
        let prov = self.provenance_for(cap, file, req, res);
        Some(Derived::new(cap.clone(), prov).with_step(
            "resolve_concession_cap",
            format!(
                "program={}, occ={:?}, ltv={}bps",
                input.program, input.occupancy, input.ltv_bps
            ),
            format!("cap {}% ({:?})", cap.cap_bps / 100, cap.basis),
        ))
    }

    /// Evaluate a proposed contribution against the cap, fully traced.
    ///
    /// `basis_amount` is the dollar figure the percentage applies to (sales
    /// price, lesser-of, or reasonable value per the cap's `basis`).
    pub fn evaluate(
        &self,
        input: &ConcessionCapInput,
        basis_amount: Cents,
        proposed: Cents,
        file: &str,
        req: u16,
        res: u16,
    ) -> Option<Derived<ConcessionOutcome>> {
        let cap = self.find(input)?;
        let max_allowed = Cents(basis_amount.0 * i64::from(cap.cap_bps) / 10_000);
        let within = proposed <= max_allowed;
        let excess = if within {
            Cents(0)
        } else {
            Cents(proposed.0 - max_allowed.0)
        };
        let outcome = ConcessionOutcome {
            cap_bps: cap.cap_bps,
            max_allowed,
            proposed,
            within_limit: within,
            excess,
        };
        let prov = self.provenance_for(cap, file, req, res);
        let mut d = Derived::new(outcome, prov);
        d.push_step(
            "resolve_concession_cap",
            format!(
                "program={}, occ={:?}, ltv={}bps",
                input.program, input.occupancy, input.ltv_bps
            ),
            format!("cap {}% ({:?})", cap.cap_bps / 100, cap.basis),
        );
        d.push_step(
            "compute_max_allowed",
            format!("{}% of ${}", cap.cap_bps / 100, basis_amount.0 / 100),
            format!("max ${}", max_allowed.0 / 100),
        );
        d.push_step(
            "compare_proposed",
            format!("proposed ${}", proposed.0 / 100),
            if within {
                format!(
                    "WITHIN limit (${} <= ${})",
                    proposed.0 / 100,
                    max_allowed.0 / 100
                )
            } else {
                format!(
                    "EXCEEDS by ${} — excess is a sales concession; reduce price and recompute LTV",
                    excess.0 / 100
                )
            },
        );
        Some(d)
    }
}
