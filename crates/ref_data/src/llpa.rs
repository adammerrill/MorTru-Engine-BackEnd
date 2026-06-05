//! Task 4.29 — Loan-Level Price Adjustment (LLPA) / Credit-Fee pricing.
//!
//! Composes the GSE risk-based price grids (Fannie Mae LLPA Matrix, Freddie Mac
//! Single-Family Seller/Servicer Guide **Exhibit 19 — Credit Fees**) with an
//! optional per-lender overlay (additive pricing overrides + tighten-only
//! eligibility) to produce a fully itemized, fully traced price for a
//! conventional loan scenario.
//!
//! # Why a dedicated module (vs. `rate_sheet::LlpaMatrix`)
//!
//! `rate_sheet::LlpaMatrix` is a single flat-row FNMA grid summed by category —
//! adequate for one agency's simple matrix. Exhibit 19 is structurally richer:
//! three purpose-specific base grids (purchase / no-cash-out / cash-out), a
//! special-attribute grid per purpose, fee caps, credits, and a Custom-MI grid.
//! Lender overlays then layer additive overrides and tighten-only eligibility on
//! top. This module models that composition; `LlpaMatrix` is retained unchanged
//! for backward compatibility.
//!
//! # MISMO / RESO alignment
//!
//! Each adjustment is emitted as a MISMO-style [`PriceAdjustment`] line item
//! (`adjustment_type`, `description`, `bps`), mirroring the MISMO `LOAN_PRICING`
//! / `PRICE_ADJUSTMENT` structure where adjustments are additive, individually
//! typed, and sum to a net. Adjustments are quoted in **price basis points**
//! (positive = cost to the borrower, negative = credit), the LLPA convention.
//! Scenario keys map to MISMO/RESO: loan purpose → `LoanPurposeType` /
//! `RefinanceCashOutDeterminationType`; occupancy → `PropertyUsageType`;
//! representative score → `IndicatorScoreValue`; LTV → `LTVRatioPercent`.
//!
//! # Provenance
//!
//! Every public method returns a [`Derived<T>`] so the price and each applied
//! adjustment carry the agency citation, the resolved dataset file, and the
//! ordered rule trail. This satisfies the engine's "100% explainable" contract.
//!
//! # Updating
//!
//! GSE grids change quarterly/ad-hoc; lender overlays change per rate sheet.
//! Add `fannie_llpa_{YYYY}.json`, `freddie_credit_fees_{YYYY}.json`, or
//! `{lender}_overlay_{YYYY}.json`; no Rust changes needed.

use serde::{Deserialize, Serialize};
use types::{BasisPoints, Cents, CreditScore, Derived, LtvBasisPoints, Provenance};

// ── Canonical scenario keys (MISMO/RESO-aligned) ────────────────────────────

/// Which GSE grid governs. Maps to the investor/agency the loan is priced for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GseAgency {
    /// Fannie Mae LLPA Matrix.
    Fannie,
    /// Freddie Mac Exhibit 19 Credit Fees.
    Freddie,
}

impl GseAgency {
    /// Logical dataset name (versioned file stem) for this agency.
    #[must_use]
    pub const fn dataset(self) -> &'static str {
        match self {
            GseAgency::Fannie => "fannie_llpa",
            GseAgency::Freddie => "freddie_credit_fees",
        }
    }
}

/// Loan purpose. Maps to MISMO `LoanPurposeType` +
/// `RefinanceCashOutDeterminationType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LlpaPurpose {
    Purchase,
    /// Rate/term — MISMO Refinance + NoCashOut (Freddie folds Special-Purpose
    /// Cash-out here as well).
    NoCashOutRefi,
    /// MISMO Refinance + CashOut.
    CashOutRefi,
}

impl LlpaPurpose {
    /// Base/special grid key as stored in the dataset JSON.
    #[must_use]
    pub const fn grid_key(self) -> &'static str {
        match self {
            LlpaPurpose::Purchase => "purchase",
            LlpaPurpose::NoCashOutRefi => "no_cash_out_refi",
            LlpaPurpose::CashOutRefi => "cash_out_refi",
        }
    }
}

/// Occupancy. Maps to MISMO `PropertyUsageType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LlpaOccupancy {
    Primary,
    SecondHome,
    Investment,
}

/// Property type relevant to LLPA special attributes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LlpaPropertyType {
    Detached,
    AttachedCondo,
    /// Detached condos are exempt from the condominium fee.
    DetachedCondo,
    Manufactured,
    Units2,
    Units3,
    Units4,
}

/// A single priced scenario. All keys are canonical so the GSE grid and the
/// lender overlay resolve against the same inputs.
#[derive(Debug, Clone)]
pub struct LlpaScenario {
    pub purpose: LlpaPurpose,
    pub occupancy: LlpaOccupancy,
    pub property_type: LlpaPropertyType,
    pub indicator_score: CreditScore,
    /// Gross LTV (the LLPA-grid basis).
    pub ltv: LtvBasisPoints,
    pub loan_amount: Cents,
    pub is_arm: bool,
    pub is_high_balance: bool,
    pub is_super_conforming: bool,
    pub has_subordinate_financing: bool,
    /// HELOC balance at closing; secondary-financing fee is skipped when zero.
    pub heloc_balance_at_closing: Cents,
    pub has_affordable_second: bool,
    /// Two-letter state, used by lender state adjusters (e.g. TX, NY, FL).
    pub state: String,
    /// AMI percent for cap eligibility (None = not an affordable scenario).
    pub ami_percent: Option<u16>,
    pub is_first_time_homebuyer: bool,
    pub is_high_cost_area: bool,
    pub is_duty_to_serve: bool,
    pub is_home_ready_or_possible: bool,
}

impl LlpaScenario {
    /// LTV as whole-percent for grid bucketing (`LtvBasisPoints` is 0.01%/unit).
    #[must_use]
    fn ltv_percent(&self) -> f64 {
        f64::from(self.ltv.0) / 100.0
    }
}

// ── MISMO-style price adjustment line item ──────────────────────────────────

/// One additive price adjustment, modeled on the MISMO `PRICE_ADJUSTMENT`
/// structure. `bps` is in price basis points (1 unit = 0.001% via
/// [`BasisPoints`] is *not* used here; price points use 0.01% = a true bp).
/// Positive = cost to borrower; negative = credit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PriceAdjustment {
    /// MISMO-style adjustment type, e.g. "BaseGrid", "Occupancy",
    /// "LenderOverride", "AffordableCap".
    pub adjustment_type: String,
    /// Human-readable description for the audit trail / disclosure.
    pub description: String,
    /// Price adjustment in true basis points (0.01%), signed.
    pub bps: i32,
}

/// Itemized, composed LLPA pricing result. This is the value inside the
/// `Derived<LlpaPricing>` returned by the engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LlpaPricing {
    pub agency: GseAgency,
    /// Every applied adjustment, in application order.
    pub adjustments: Vec<PriceAdjustment>,
    /// Sum of GSE (base + special-attribute) adjustments, in bps.
    pub gse_subtotal_bps: i32,
    /// Sum of lender overlay adjustments (overrides + incentives), in bps.
    pub lender_subtotal_bps: i32,
    /// True when an affordable/mission cap replaced the positive priced fees.
    pub capped: bool,
    /// Net total price adjustment in bps (after cap + max-net floor).
    pub total_bps: i32,
}

impl LlpaPricing {
    /// Cost in dollars for a given loan amount (positive = borrower cost).
    /// `bps` here are 0.01% units, so cost = loan × bps / 10_000.
    #[must_use]
    pub fn cost_for(&self, loan_amount: Cents) -> Cents {
        let amt = i128::from(loan_amount.0) * i128::from(self.total_bps) / 10_000;
        Cents(amt as i64)
    }
}

/// Eligibility rejection (tighten-only overlay or grid "Not Eligible" cell).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ineligible {
    pub reason: String,
}

// ── GSE dataset file ────────────────────────────────────────────────────────

/// LTV bucket: [min, max] in whole percent, inclusive of max.
#[derive(Debug, Clone, Deserialize)]
pub struct LtvBucket {
    pub id: String,
    pub min: f64,
    pub max: f64,
}

/// Score bucket: [min, max] inclusive.
#[derive(Debug, Clone, Deserialize)]
pub struct ScoreBucket {
    pub id: String,
    pub min: u16,
    pub max: u16,
}

/// A grid: score rows aligned to an ltv column order. `null` cells (None)
/// mean "Not Eligible".
#[derive(Debug, Clone, Deserialize)]
pub struct Grid {
    pub ltv_order: Vec<String>,
    pub rows: std::collections::HashMap<String, Vec<Option<f64>>>,
    #[serde(default)]
    pub not_eligible_above_ltv: Option<f64>,
}

/// A special-attribute grid: attribute name → values aligned to ltv_order.
#[derive(Debug, Clone, Deserialize)]
pub struct SpecialGrid {
    pub ltv_order: Vec<String>,
    pub attributes: std::collections::HashMap<String, Vec<Option<f64>>>,
    #[serde(default)]
    pub not_eligible_above_ltv: Option<f64>,
}

/// Top-level shape of `fannie_llpa_{year}.json` / `freddie_credit_fees_{year}.json`.
#[derive(Debug, Clone, Deserialize)]
pub struct LlpaDatasetFile {
    pub agency: GseAgency,
    pub source_citation: String,
    pub effective_date: String,
    pub ltv_buckets: Vec<LtvBucket>,
    pub score_buckets: Vec<ScoreBucket>,
    /// keyed by purpose grid_key ("purchase", "no_cash_out_refi", "cash_out_refi").
    pub base_grids: std::collections::HashMap<String, Grid>,
    /// keyed by purpose grid_key.
    pub special_attribute_grids: std::collections::HashMap<String, SpecialGrid>,
}

impl LlpaDatasetFile {
    fn percent_to_bps(p: f64) -> i32 {
        // grid values are percent (e.g. 0.375 → 37.5bps → 38? No: 0.375% = 37.5 true bps).
        // Use 0.01% = 1 bp, so 0.375% = 37.5 bps; round half-up to nearest bp.
        (p * 100.0).round() as i32
    }

    fn ltv_col(&self, order: &[String], ltv_pct: f64) -> Option<usize> {
        order.iter().position(|id| {
            self.ltv_buckets
                .iter()
                .find(|b| &b.id == id)
                .is_some_and(|b| ltv_pct >= b.min - 1e-9 && ltv_pct <= b.max + 1e-9)
        })
    }

    fn score_id(&self, score: u16) -> Option<&str> {
        self.score_buckets
            .iter()
            .find(|b| score >= b.min && score <= b.max)
            .map(|b| b.id.as_str())
    }

    fn provenance(&self, file: &str, record_id: String, req: u16, res: u16) -> Provenance {
        Provenance {
            dataset: self.agency.dataset().to_owned(),
            source_file: file.to_owned(),
            source_citation: self.source_citation.clone(),
            effective_date: self.effective_date.clone(),
            record_id,
            requested_version: req,
            resolved_version: res,
        }
    }

    /// Base-grid cell for this scenario, or `Err(Ineligible)` for a Not-Eligible cell.
    fn base_cell(&self, s: &LlpaScenario) -> Result<i32, Ineligible> {
        let key = s.purpose.grid_key();
        let grid = self.base_grids.get(key).ok_or_else(|| Ineligible {
            reason: format!("no base grid '{key}'"),
        })?;
        let ltv_pct = s.ltv_percent();
        if let Some(cut) = grid.not_eligible_above_ltv {
            if ltv_pct > cut + 1e-9 {
                return Err(Ineligible {
                    reason: format!("LTV {ltv_pct}% exceeds {key} cutoff {cut}%"),
                });
            }
        }
        let sid = self
            .score_id(s.indicator_score.0)
            .ok_or_else(|| Ineligible {
                reason: format!("no score bucket for {}", s.indicator_score.0),
            })?;
        let col = self
            .ltv_col(&grid.ltv_order, ltv_pct)
            .ok_or_else(|| Ineligible {
                reason: format!("no LTV bucket for {ltv_pct}%"),
            })?;
        let row = grid.rows.get(sid).ok_or_else(|| Ineligible {
            reason: format!("no score row '{sid}'"),
        })?;
        match row.get(col).copied().flatten() {
            Some(p) => Ok(Self::percent_to_bps(p)),
            None => Err(Ineligible {
                reason: format!("base cell [{sid}, {key}] is Not Eligible"),
            }),
        }
    }

    /// Names of GSE special attributes that apply to this scenario.
    fn applicable_attributes(s: &LlpaScenario) -> Vec<&'static str> {
        let mut v = Vec::new();
        match s.occupancy {
            LlpaOccupancy::Investment => v.push("investment_property"),
            LlpaOccupancy::SecondHome => v.push("second_home"),
            LlpaOccupancy::Primary => {}
        }
        match s.property_type {
            LlpaPropertyType::Manufactured => v.push("manufactured_homes"),
            LlpaPropertyType::AttachedCondo => v.push("condominium_unit"),
            LlpaPropertyType::DetachedCondo => {} // exempt
            LlpaPropertyType::Units2 | LlpaPropertyType::Units3 | LlpaPropertyType::Units4 => {
                v.push("number_of_units_gt1");
            }
            LlpaPropertyType::Detached => {}
        }
        if s.has_subordinate_financing
            && s.heloc_balance_at_closing.is_positive()
            && !s.has_affordable_second
        {
            v.push("secondary_financing");
        }
        if s.is_arm {
            v.push("adjustable_rate_mortgage");
        }
        if s.is_super_conforming {
            if s.is_arm {
                v.push("super_conforming_arm");
            } else {
                v.push("super_conforming_frm");
            }
        }
        v
    }

    fn special_cell(&self, s: &LlpaScenario, attr: &str) -> Result<Option<i32>, Ineligible> {
        let key = s.purpose.grid_key();
        let Some(grid) = self.special_attribute_grids.get(key) else {
            return Ok(None);
        };
        let Some(vals) = grid.attributes.get(attr) else {
            return Ok(None);
        };
        let ltv_pct = s.ltv_percent();
        let col = self
            .ltv_col(&grid.ltv_order, ltv_pct)
            .ok_or_else(|| Ineligible {
                reason: format!("no LTV bucket for {ltv_pct}% (attr {attr})"),
            })?;
        match vals.get(col).copied().flatten() {
            Some(p) => Ok(Some(Self::percent_to_bps(p))),
            None => Err(Ineligible {
                reason: format!("attribute '{attr}' is Not Eligible at LTV {ltv_pct}%"),
            }),
        }
    }
}

// ── Lender overlay file ─────────────────────────────────────────────────────

/// A flat band: an inclusive `[min, max]` loan-amount window with a bps adjust.
#[derive(Debug, Clone, Deserialize)]
pub struct LoanAmountBand {
    #[serde(default)]
    pub min: Option<i64>,
    #[serde(default)]
    pub max: Option<i64>,
    /// Adjustment in true basis points (0.01%), signed.
    pub bps: i32,
}

impl LoanAmountBand {
    fn contains(&self, cents: i64) -> bool {
        let lo = self.min.unwrap_or(i64::MIN);
        let hi = self.max.unwrap_or(i64::MAX);
        cents >= lo && cents <= hi
    }
}

/// Per-attribute tighten-only max LTV (whole percent).
#[derive(Debug, Clone, Deserialize)]
pub struct MaxLtvRule {
    pub attribute: String,
    pub max_ltv: f64,
}

/// Top-level shape of `{lender}_overlay_{year}.json`.
#[derive(Debug, Clone, Deserialize)]
pub struct LenderOverlayFile {
    pub lender_id: String,
    pub source_citation: String,
    pub effective_date: String,
    /// Best (most negative) net price the lender will honor, in bps.
    pub max_net_pricing_bps: i32,
    /// Same for high-balance loans.
    pub high_balance_max_net_pricing_bps: i32,
    /// Refi75-style flat incentive in bps applied to no-cash-out refis.
    #[serde(default)]
    pub refi_incentive_bps: Option<i32>,
    /// TRAC-style loan-amount credits.
    #[serde(default)]
    pub loan_amount_bands: Vec<LoanAmountBand>,
    /// State adjusters for conventional fixed 30yr, keyed by 2-letter state.
    #[serde(default)]
    pub state_adjustments_bps: std::collections::HashMap<String, i32>,
    /// Affordable/mission cap (bps the priced positive fees collapse to).
    #[serde(default)]
    pub affordable_cap_bps: Option<i32>,
    /// Tighten-only eligibility.
    #[serde(default)]
    pub max_ltv_rules: Vec<MaxLtvRule>,
}

impl LenderOverlayFile {
    /// Reject the scenario if a tighten-only overlay disallows it.
    fn check_eligibility(&self, s: &LlpaScenario) -> Result<(), Ineligible> {
        let ltv_pct = f64::from(s.ltv.0) / 100.0;
        for r in &self.max_ltv_rules {
            let hit = match r.attribute.as_str() {
                "cash_out_refi" => s.purpose == LlpaPurpose::CashOutRefi,
                "investment_property" => s.occupancy == LlpaOccupancy::Investment,
                "second_home" => s.occupancy == LlpaOccupancy::SecondHome,
                "high_balance_cash_out" => {
                    s.is_high_balance && s.purpose == LlpaPurpose::CashOutRefi
                }
                _ => false,
            };
            if hit && ltv_pct > r.max_ltv + 1e-9 {
                return Err(Ineligible {
                    reason: format!(
                        "{} LTV {ltv_pct}% exceeds lender max {}%",
                        r.attribute, r.max_ltv
                    ),
                });
            }
        }
        Ok(())
    }
}

// ── Cap eligibility (agency affordable/mission programs) ────────────────────

fn affordable_capped(s: &LlpaScenario) -> bool {
    if s.is_home_ready_or_possible {
        return true;
    }
    if let Some(ami) = s.ami_percent {
        if s.is_first_time_homebuyer && !s.is_high_cost_area && ami <= 100 {
            return true;
        }
        if s.is_first_time_homebuyer && s.is_high_cost_area && ami <= 120 {
            return true;
        }
        if s.is_duty_to_serve && ami <= 100 {
            return true;
        }
    }
    false
}

// ── The composition engine (free function, store calls it) ──────────────────

/// Price `scenario` against the GSE `gse` dataset and an optional lender
/// `overlay`, returning a fully-itemized, fully-traced `Derived<LlpaPricing>`.
///
/// `req`/`res` are the requested/resolved dataset versions (from the store's
/// `read_versioned_json`), carried into provenance so year-fallback is audited.
#[allow(clippy::too_many_arguments)]
pub fn price(
    gse: &LlpaDatasetFile,
    gse_file: &str,
    overlay: Option<(&str, &LenderOverlayFile)>,
    scenario: &LlpaScenario,
    req: u16,
    res: u16,
) -> Result<Derived<LlpaPricing>, Ineligible> {
    // Tighten-only eligibility first.
    if let Some((_, ov)) = overlay {
        ov.check_eligibility(scenario)?;
    }

    let mut adjustments: Vec<PriceAdjustment> = Vec::new();

    // 1. GSE base grid.
    let base = gse.base_cell(scenario)?;
    adjustments.push(PriceAdjustment {
        adjustment_type: "BaseGrid".to_owned(),
        description: format!(
            "{:?} base grid: score {}, LTV {:.2}%",
            gse.agency,
            scenario.indicator_score.0,
            scenario.ltv_percent()
        ),
        bps: base,
    });

    // 2. GSE special attributes (additive).
    for attr in LlpaDatasetFile::applicable_attributes(scenario) {
        if let Some(bps) = gse.special_cell(scenario, attr)? {
            if bps != 0 {
                adjustments.push(PriceAdjustment {
                    adjustment_type: "SpecialAttribute".to_owned(),
                    description: attr.to_owned(),
                    bps,
                });
            }
        }
    }
    let gse_subtotal_bps: i32 = adjustments.iter().map(|a| a.bps).sum();

    // 3 + 4. Lender overrides + incentives.
    let mut lender_subtotal_bps = 0;
    if let Some((_, ov)) = overlay {
        if scenario.purpose == LlpaPurpose::NoCashOutRefi {
            if let Some(b) = ov.refi_incentive_bps {
                adjustments.push(PriceAdjustment {
                    adjustment_type: "LenderIncentive".to_owned(),
                    description: "refi incentive".to_owned(),
                    bps: b,
                });
                lender_subtotal_bps += b;
            }
        }
        for band in &ov.loan_amount_bands {
            if band.contains(scenario.loan_amount.0) && band.bps != 0 {
                adjustments.push(PriceAdjustment {
                    adjustment_type: "LenderOverride".to_owned(),
                    description: "loan-amount band credit".to_owned(),
                    bps: band.bps,
                });
                lender_subtotal_bps += band.bps;
                break;
            }
        }
        if !scenario.is_arm {
            if let Some(&b) = ov.state_adjustments_bps.get(&scenario.state) {
                if b != 0 {
                    adjustments.push(PriceAdjustment {
                        adjustment_type: "LenderOverride".to_owned(),
                        description: format!("state adjuster {}", scenario.state),
                        bps: b,
                    });
                    lender_subtotal_bps += b;
                }
            }
        }
    }

    // 5. Affordable/mission cap: collapse positive fees, keep credits.
    let mut capped = false;
    let mut priced = gse_subtotal_bps + lender_subtotal_bps;
    if affordable_capped(scenario) {
        let cap = overlay
            .and_then(|(_, ov)| ov.affordable_cap_bps)
            .unwrap_or(0);
        let credits: i32 = adjustments.iter().map(|a| a.bps).filter(|b| *b < 0).sum();
        priced = cap + credits;
        capped = true;
        adjustments.push(PriceAdjustment {
            adjustment_type: "AffordableCap".to_owned(),
            description: "affordable/mission program cap applied".to_owned(),
            bps: cap,
        });
    }

    // 6. Max-net-price floor.
    let total_bps = if let Some((_, ov)) = overlay {
        let floor = if scenario.is_high_balance {
            ov.high_balance_max_net_pricing_bps
        } else {
            ov.max_net_pricing_bps
        };
        priced.max(floor)
    } else {
        priced
    };

    let pricing = LlpaPricing {
        agency: gse.agency,
        adjustments,
        gse_subtotal_bps,
        lender_subtotal_bps,
        capped,
        total_bps,
    };

    // Provenance: GSE dataset is the governing source; record id names the cell.
    let record_id = format!(
        "{}_{}_{}",
        scenario.purpose.grid_key(),
        scenario.indicator_score.0,
        scenario.ltv.0
    );
    let prov = gse.provenance(gse_file, record_id, req, res);
    let mut d = Derived::new(pricing.clone(), prov);

    d.push_step(
        "gse_base_and_attributes",
        format!(
            "{:?} {} score={} ltv={:.2}%",
            gse.agency,
            scenario.purpose.grid_key(),
            scenario.indicator_score.0,
            scenario.ltv_percent()
        ),
        format!("GSE subtotal {gse_subtotal_bps} bps"),
    );
    if let Some((ofile, _)) = overlay {
        d.push_step(
            "lender_overlay",
            format!("overlay file {ofile}"),
            format!("lender subtotal {lender_subtotal_bps} bps"),
        );
    }
    if capped {
        d.push_step(
            "affordable_cap",
            "home_ready_or_possible / FTHB / duty-to-serve".to_owned(),
            "positive fees collapsed to cap; credits preserved".to_owned(),
        );
    }
    d.push_step(
        "net_total",
        format!("gse {gse_subtotal_bps} + lender {lender_subtotal_bps}"),
        format!(
            "total {} bps = {} on {}",
            pricing.total_bps,
            pricing.cost_for(scenario.loan_amount),
            scenario.loan_amount
        ),
    );

    Ok(d)
}

/// Convert a net price-bps result to a [`BasisPoints`] value at the engine's
/// canonical 0.001% precision (1 true bp = 10 stored units). Useful when the
/// caller wants the price expressed in the same encoding as a note rate.
#[must_use]
pub fn price_bps_to_basis_points(price_bps: i32) -> Option<BasisPoints> {
    if price_bps < 0 {
        return None;
    }
    u32::try_from(price_bps * 10).ok().map(BasisPoints)
}
