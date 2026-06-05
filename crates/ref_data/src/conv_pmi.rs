//! Epic 4.5.2 — Conventional Private Mortgage Insurance (PMI) rate quotes.
//!
//! Produces an **estimated MI rate quote** from a published MI-company rate
//! card, modeled on the MISMO V3.5 **MI Estimated Rate Quote** transaction
//! (the rate obtained early in origination "for disclosure", prior to ordering
//! actual MI). This is exactly MorTru's pre-application use: an estimate, never
//! an MI order, framed estimate-not-offer.
//!
//! # What this is and is NOT
//! - IS: published-rate-card lookup (base grid + additive adjustments + min-rate
//!   floor) for the six conventional MI companies, each card versioned and
//!   citation-stamped, every quote a [`Derived<T>`].
//! - IS NOT: a live MI rate-engine integration (risk-based "black-box" pricing).
//!   Those are vendor APIs (MGIC MiQ, Radian, Essent RateFinder, etc.) and are
//!   out of scope as shipped data — flagged for Epic 7+ vendor integration.
//!
//! # Rate encoding
//! MI premium rates are **annual** percentages of the loan amount. The cards
//! carry two-decimal precision (e.g. 0.58%). To avoid float drift, rates and
//! adjustments are stored as integer **thousandths of a percent** (`milli_pct`):
//! 0.58% → `580`, 1.25% → `1250`, −0.13% → `-130`. Monthly premium dollars =
//! `loan × rate% / 100 / 12`; single/upfront = `loan × rate% / 100`.
//!
//! # DTI adjustments (decision: store, default-qualified)
//! Cards include DTI>45% / DTI>50% adjustment rows. MorTru does not underwrite
//! (no DTI), so a quote assumes a **qualified DTI tier** ([`DtiTier::Qualified`],
//! ≤45%) by default → the DTI adjustment is `0`. The rows are stored and the
//! selection logic is wired so a future DTI feature activates with no schema or
//! engine change.
//!
//! # Per-insurer structural differences (faithfully modeled)
//! - **Non-fixed rates:** some insurers ship explicit non-fixed grids (Enact),
//!   others derive them as `fixed × 1.25, round to nearest bp` (Essent). Modeled
//!   via [`NonFixedMethod`].
//! - **Min rate floor:** per-insurer, per-plan (e.g. Essent single 0.55%,
//!   monthly 0.14%, split 0.05%). Applied after adjustments.
//! - **Credit floor:** Enact extends to 600–619; Essent stops at 620–639;
//!   Radian adds a `<620` column. Handled by each card's own score buckets.
//! - **State variants / effective windows:** NY/WA carve-outs ship as separate
//!   versioned cards keyed by state applicability (time-windowed, like overlays).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use types::{Cents, CreditScore, Derived, LtvBasisPoints, Provenance};

// ── Canonical quote keys (MISMO MI Rate Quote aligned) ──────────────────────

/// The six approved conventional MI companies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MiCompany {
    Mgic,
    Radian,
    Essent,
    NationalMi,
    Enact,
    Arch,
}

impl MiCompany {
    /// Versioned-file stem for this company's card (e.g. `enact_pmi`).
    #[must_use]
    pub const fn dataset(self) -> &'static str {
        match self {
            MiCompany::Mgic => "mgic_pmi",
            MiCompany::Radian => "radian_pmi",
            MiCompany::Essent => "essent_pmi",
            MiCompany::NationalMi => "national_mi_pmi",
            MiCompany::Enact => "enact_pmi",
            MiCompany::Arch => "arch_pmi",
        }
    }
}

/// Premium plan. Maps to MISMO `MIPremiumSourceType` / `MIPremiumRatePlanType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MiPlan {
    /// Borrower-paid, premium in the monthly payment.
    MonthlyBpmi,
    /// Borrower-paid, one-time premium at closing.
    SingleBpmi,
    /// Lender-paid, one-time (priced into the rate; borrower not charged
    /// separately). Stored for completeness / finance-charge attribution.
    SingleLpmi,
    /// Lender-paid monthly.
    MonthlyLpmi,
    /// Upfront + monthly combination.
    SplitPremium,
}

impl MiPlan {
    /// Grid key as stored in the card JSON.
    #[must_use]
    pub const fn grid_key(self) -> &'static str {
        match self {
            MiPlan::MonthlyBpmi => "monthly_bpmi",
            MiPlan::SingleBpmi => "single_bpmi",
            MiPlan::SingleLpmi => "single_lpmi",
            MiPlan::MonthlyLpmi => "monthly_lpmi",
            MiPlan::SplitPremium => "split_premium",
        }
    }

    /// Whether this plan's primary premium is paid monthly (vs upfront single).
    #[must_use]
    pub const fn is_monthly(self) -> bool {
        matches!(self, MiPlan::MonthlyBpmi | MiPlan::MonthlyLpmi)
    }
}

/// Refundability of a single/split premium. Affects which card applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Refundability {
    Refundable,
    NonRefundable,
    /// Not applicable (monthly plans).
    NotApplicable,
}

/// DTI tier. Default `Qualified` (≤45%) since MorTru does not underwrite; the
/// higher tiers exist so a future DTI feature activates without a schema change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DtiTier {
    /// ≤45% — assumed for all MorTru quotes today; DTI adjustment = 0.
    #[default]
    Qualified,
    /// 45.01%–50%.
    Elevated,
    /// >50%.
    High,
}

impl DtiTier {
    /// Adjustment key suffix, or `None` for the qualified (no-adjustment) tier.
    fn adjustment_key(self) -> Option<&'static str> {
        match self {
            DtiTier::Qualified => None,
            DtiTier::Elevated => Some("dti_45_50"),
            DtiTier::High => Some("dti_gt_50"),
        }
    }
}

/// Property/occupancy/loan attributes that drive additive MI adjustments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MiPropertyType {
    SingleFamilyDetached,
    Condo,
    ManufacturedHousing,
    Units2,
    Units3to4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MiOccupancy {
    Primary,
    SecondHome,
    Investment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MiPurpose {
    Purchase,
    RateTermRefi,
    CashOutRefi,
}

/// A single PMI rate-quote scenario.
#[derive(Debug, Clone)]
pub struct MiScenario {
    pub plan: MiPlan,
    pub refundability: Refundability,
    /// Fixed (or ARM ≥5yr) vs non-fixed (ARM <5yr).
    pub is_fixed: bool,
    /// Amortization term in months (term bucket: >300, >240, >180, ≤180, etc.).
    pub amortization_term_months: u16,
    pub indicator_score: CreditScore,
    /// Gross LTV (MI base LTV; net-of-financed-premium where the card specifies).
    pub ltv: LtvBasisPoints,
    /// Coverage percent required (investor/Fannie-Freddie coverage table).
    pub coverage_percent: u8,
    pub loan_amount: Cents,
    pub property_type: MiPropertyType,
    pub occupancy: MiOccupancy,
    pub purpose: MiPurpose,
    pub borrower_count: u8,
    pub is_relocation: bool,
    /// Defaults to `Qualified`; not collected pre-application.
    pub dti_tier: DtiTier,
}

impl MiScenario {
    #[must_use]
    fn ltv_percent(&self) -> f64 {
        f64::from(self.ltv.0) / 100.0
    }

    /// `term > 20yr` vs `≤ 20yr` — the split most cards use.
    #[must_use]
    fn term_gt_20yr(&self) -> bool {
        self.amortization_term_months > 240
    }
}

// ── Card file shapes ────────────────────────────────────────────────────────

/// How a card produces non-fixed (ARM <5yr) rates from its fixed grid.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NonFixedMethod {
    /// Card ships explicit non-fixed grids (look up `..._non_fixed`).
    Grid,
    /// Multiply fixed base rate by `factor` (e.g. 1.25), round to nearest bp,
    /// THEN apply adjustments. Stored as thousandths (1.25 → 1250).
    Multiplier { factor_milli: u32 },
}

/// One LTV row of a base grid: an inclusive LTV band + coverage% + the rate
/// values aligned to the card's `score_order`.
#[derive(Debug, Clone, Deserialize)]
pub struct MiRateRow {
    pub ltv_min: f64,
    pub ltv_max: f64,
    pub coverage_percent: u8,
    /// Rates in milli-percent, aligned to `score_order`. `null` = not offered.
    pub rates: Vec<Option<i32>>,
}

/// A base grid for one (plan, fixed?, term-bucket) combination.
#[derive(Debug, Clone, Deserialize)]
pub struct MiBaseGrid {
    pub score_order: Vec<String>,
    pub rows: Vec<MiRateRow>,
}

/// Top-level card file: `{company}_pmi_{YYYY}.json`.
#[derive(Debug, Clone, Deserialize)]
pub struct MiCardFile {
    pub company: MiCompany,
    pub source_citation: String,
    pub effective_date: String,
    /// States this card applies to; empty = all states not otherwise carved out.
    #[serde(default)]
    pub applicable_states: Vec<String>,
    /// States explicitly excluded (e.g. NY/WA use a different card).
    #[serde(default)]
    pub excluded_states: Vec<String>,
    pub non_fixed_method: NonFixedMethod,
    /// Per-plan minimum rate floor, milli-percent, keyed by `MiPlan::grid_key`.
    #[serde(default)]
    pub min_rate_milli_pct: HashMap<String, i32>,
    /// Score buckets: id → [min,max] inclusive.
    pub score_buckets: HashMap<String, [u16; 2]>,
    /// Base grids keyed `"{plan}|{fixed|non_fixed}|{gt20|le20}"`.
    pub base_grids: HashMap<String, MiBaseGrid>,
    /// Additive adjustments keyed by name → score_order-aligned milli-pct.
    /// Adjustment grids share each card's `score_order` via `adjustment_order`.
    pub adjustment_order: Vec<String>,
    #[serde(default)]
    pub adjustments: HashMap<String, Vec<Option<i32>>>,
}

impl MiCardFile {
    fn score_id(&self, score: u16) -> Option<&str> {
        self.score_buckets
            .iter()
            .find(|(_, b)| score >= b[0] && score <= b[1])
            .map(|(id, _)| id.as_str())
    }

    fn score_col(order: &[String], id: &str) -> Option<usize> {
        order.iter().position(|s| s == id)
    }

    fn grid_key(plan: MiPlan, fixed: bool, gt20: bool) -> String {
        let f = if fixed { "fixed" } else { "non_fixed" };
        let t = if gt20 { "gt20" } else { "le20" };
        format!("{}|{f}|{t}", plan.grid_key())
    }

    fn provenance(&self, file: &str, record_id: String, req: u16, res: u16) -> Provenance {
        Provenance {
            dataset: self.company.dataset().to_owned(),
            source_file: file.to_owned(),
            source_citation: self.source_citation.clone(),
            effective_date: self.effective_date.clone(),
            record_id,
            requested_version: req,
            resolved_version: res,
        }
    }

    /// Base rate (milli-pct) for the scenario, before adjustments and floor.
    /// Handles the non-fixed multiplier method by reading the fixed grid.
    fn base_rate(&self, s: &MiScenario) -> Result<i32, MiUnavailable> {
        let gt20 = s.term_gt_20yr();
        // For Multiplier cards, non-fixed reads the FIXED grid then scales.
        let (lookup_fixed, scale): (bool, Option<u32>) = match (&self.non_fixed_method, s.is_fixed)
        {
            (_, true) => (true, None),
            (NonFixedMethod::Grid, false) => (false, None),
            (NonFixedMethod::Multiplier { factor_milli }, false) => (true, Some(*factor_milli)),
        };
        let key = Self::grid_key(s.plan, lookup_fixed, gt20);
        let grid = self.base_grids.get(&key).ok_or_else(|| MiUnavailable {
            reason: format!("no base grid '{key}'"),
        })?;
        let sid = self
            .score_id(s.indicator_score.0)
            .ok_or_else(|| MiUnavailable {
                reason: format!("no score bucket for {}", s.indicator_score.0),
            })?;
        let col = Self::score_col(&grid.score_order, sid).ok_or_else(|| MiUnavailable {
            reason: format!("score '{sid}' not in grid '{key}'"),
        })?;
        let ltv_pct = s.ltv_percent();
        let row = grid
            .rows
            .iter()
            .find(|r| {
                r.coverage_percent == s.coverage_percent
                    && ltv_pct >= r.ltv_min - 1e-9
                    && ltv_pct <= r.ltv_max + 1e-9
            })
            .ok_or_else(|| MiUnavailable {
                reason: format!(
                    "no row for LTV {ltv_pct}% coverage {}% in '{key}'",
                    s.coverage_percent
                ),
            })?;
        let base = row
            .rates
            .get(col)
            .copied()
            .flatten()
            .ok_or_else(|| MiUnavailable {
                reason: format!("rate not offered: score {sid}, LTV {ltv_pct}%, '{key}'"),
            })?;
        match scale {
            None => Ok(base),
            Some(factor_milli) => {
                // fixed × factor, rounded to nearest basis point (0.01% = 10 milli)
                let scaled = i64::from(base) * i64::from(factor_milli) / 1000;
                let bp_rounded = ((scaled + 5) / 10) * 10;
                Ok(bp_rounded as i32)
            }
        }
    }

    /// LTV-band suffix used by some adjustment keys (e.g. `second_home@95`).
    fn ltv_band_label(ltv_pct: f64) -> &'static str {
        if ltv_pct > 95.0 {
            "97"
        } else if ltv_pct > 90.0 {
            "95"
        } else if ltv_pct > 85.0 {
            "90"
        } else {
            "85"
        }
    }

    /// Adjustment names applicable to this scenario, in deterministic order.
    fn applicable_adjustments(s: &MiScenario) -> Vec<String> {
        let mut v = Vec::new();
        if s.borrower_count >= 2 {
            v.push(format!(
                "ge2_borrowers@{}",
                Self::ltv_band_label(s.ltv_percent())
            ));
        }
        match s.occupancy {
            MiOccupancy::SecondHome => v.push("second_home".into()),
            MiOccupancy::Investment => v.push("investment_property".into()),
            MiOccupancy::Primary => {}
        }
        match s.property_type {
            MiPropertyType::ManufacturedHousing => v.push("manufactured_housing".into()),
            MiPropertyType::Units3to4 => v.push("units_3_4".into()),
            _ => {}
        }
        match s.purpose {
            MiPurpose::CashOutRefi => v.push("cash_out_refi".into()),
            MiPurpose::RateTermRefi => v.push("rate_term_refi".into()),
            MiPurpose::Purchase => {}
        }
        if s.is_relocation {
            v.push("relocation".into());
        }
        // DTI tier (defaults to qualified → no key).
        if let Some(base) = s.dti_tier.adjustment_key() {
            v.push(format!("{base}@{}", Self::ltv_band_label(s.ltv_percent())));
        }
        v
    }

    fn adjustment_value(&self, s: &MiScenario, name: &str) -> Option<i32> {
        let vals = self.adjustments.get(name)?;
        let sid = self.score_id(s.indicator_score.0)?;
        let col = Self::score_col(&self.adjustment_order, sid)?;
        vals.get(col).copied().flatten()
    }
}

// ── Result types ────────────────────────────────────────────────────────────

/// One additive MI rate adjustment line (MISMO-style).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MiAdjustment {
    pub adjustment_type: String,
    pub description: String,
    /// Milli-percent, signed.
    pub milli_pct: i32,
}

/// A fully itemized, fully traced PMI rate quote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MiRateQuote {
    pub company: MiCompany,
    pub plan: MiPlan,
    pub base_milli_pct: i32,
    pub adjustments: Vec<MiAdjustment>,
    /// Net annual rate after adjustments + min-rate floor, in milli-percent.
    pub net_milli_pct: i32,
    /// True if the min-rate floor raised the net rate.
    pub floored: bool,
}

impl MiRateQuote {
    /// Net annual rate as a percentage (e.g. 0.58).
    #[must_use]
    pub fn annual_percent(&self) -> f64 {
        f64::from(self.net_milli_pct) / 1000.0
    }

    /// Annual premium dollars = loan × rate% / 100.
    #[must_use]
    pub fn annual_premium(&self, loan: Cents) -> Cents {
        let amt = i128::from(loan.0) * i128::from(self.net_milli_pct) / 100_000;
        Cents(amt as i64)
    }

    /// Monthly premium dollars for monthly plans (annual / 12); for single/split
    /// plans this is the upfront single premium spread is not implied — callers
    /// use [`Self::annual_premium`] as the one-time amount instead.
    #[must_use]
    pub fn monthly_premium(&self, loan: Cents) -> Cents {
        Cents(self.annual_premium(loan).0 / 12)
    }
}

/// MI not offered for this scenario (a valid scenario the card doesn't cover).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MiUnavailable {
    pub reason: String,
}

// ── Engine entry point ──────────────────────────────────────────────────────

/// Produce an estimated PMI rate quote from `card` for `scenario`.
///
/// `req`/`res` are requested/resolved card versions (from the store), carried
/// into provenance so an effective-date fallback is audited.
pub fn quote(
    card: &MiCardFile,
    card_file: &str,
    scenario: &MiScenario,
    req: u16,
    res: u16,
) -> Result<Derived<MiRateQuote>, MiUnavailable> {
    let base = card.base_rate(scenario)?;
    let mut adjustments = Vec::new();
    let mut net = base;

    for name in MiCardFile::applicable_adjustments(scenario) {
        if let Some(v) = card.adjustment_value(scenario, &name) {
            if v != 0 {
                adjustments.push(MiAdjustment {
                    adjustment_type: "MiAdjustment".to_owned(),
                    description: name.clone(),
                    milli_pct: v,
                });
                net += v;
            }
        }
    }

    // Min-rate floor (per plan).
    let floor = card
        .min_rate_milli_pct
        .get(scenario.plan.grid_key())
        .copied()
        .unwrap_or(0);
    let floored = net < floor;
    if floored {
        net = floor;
    }

    let quote = MiRateQuote {
        company: card.company,
        plan: scenario.plan,
        base_milli_pct: base,
        adjustments,
        net_milli_pct: net,
        floored,
    };

    let record_id = format!(
        "{}_{}_{}_{}",
        scenario.plan.grid_key(),
        if scenario.is_fixed {
            "fixed"
        } else {
            "nonfixed"
        },
        scenario.indicator_score.0,
        scenario.ltv.0
    );
    let prov = card.provenance(card_file, record_id, req, res);
    let mut d = Derived::new(quote.clone(), prov);

    d.push_step(
        "base_rate",
        format!(
            "{:?} {} {} score={} ltv={:.2}% cov={}%",
            card.company,
            scenario.plan.grid_key(),
            if scenario.is_fixed {
                "fixed"
            } else {
                "non-fixed"
            },
            scenario.indicator_score.0,
            scenario.ltv_percent(),
            scenario.coverage_percent,
        ),
        format!("base {base} milli-pct"),
    );
    if !quote.adjustments.is_empty() {
        d.push_step(
            "adjustments",
            format!("{} applied", quote.adjustments.len()),
            format!(
                "sum {} milli-pct",
                quote.adjustments.iter().map(|a| a.milli_pct).sum::<i32>()
            ),
        );
    }
    if floored {
        d.push_step(
            "min_rate_floor",
            format!("floor {floor} milli-pct"),
            "net raised to floor".to_owned(),
        );
    }
    d.push_step(
        "net_rate",
        format!(
            "dti_tier={:?} (assumed qualified pre-app)",
            scenario.dti_tier
        ),
        format!(
            "net {} milli-pct = {:.3}% annual ({} / yr on {})",
            quote.net_milli_pct,
            quote.annual_percent(),
            quote.annual_premium(scenario.loan_amount),
            scenario.loan_amount
        ),
    );

    Ok(d)
}
