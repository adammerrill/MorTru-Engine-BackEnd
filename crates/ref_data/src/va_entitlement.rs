//! Task 4.27 — VA loan limits, guaranty, and entitlement analysis.
//!
//! # The post-2020 reality (source of truth)
//!
//! Under the Blue Water Navy Vietnam Veterans Act (effective 2020-01-01), VA
//! imposes **no loan-amount cap** for veterans with **full entitlement** — VA
//! guarantees 25% and the veteran can borrow whatever the lender approves with
//! zero down. The county conforming loan limit (the FHFA/GSE limit) only governs
//! the **partial-entitlement** case. So a "VA county loan limit" is not a VA cap;
//! it is the GSE conforming limit, reused for the partial-entitlement guaranty
//! formula. [`va_county_loan_limit`] therefore delegates to the GSE limit.
//!
//! # Entitlement model (VA Lenders Handbook 26-7; guaranty calc circular)
//!
//! * **Basic (tier-1) entitlement** = $36,000, covering loans ≤ $144,000 at 25%.
//! * **Bonus (tier-2) entitlement** bridges to 25% of the county conforming
//!   limit for loans above $144,000.
//! * **Full entitlement**: max guaranty = 25% of loan; no VA cap (lender +
//!   appraisal govern; a lender max-loan overlay may still apply).
//! * **Partial entitlement**: max guaranty = lesser of (25% of loan) or
//!   (25% × county limit − entitlement already used). Zero-down max =
//!   remaining entitlement × 4. Above that, the lender requires a down payment
//!   of 25% of the *gap* (not 25% of the price).
//! * **Joint VA loan** (veteran + a non-veteran who is NOT the spouse): VA
//!   guarantees only the veteran's pro-rata share; the non-veteran share
//!   typically requires a down payment (commonly 12.5% of that share — a
//!   secondary-market requirement, captured as guidance).
//! * **Disability**: a veteran receiving (or eligible for) service-connected
//!   disability compensation is **funding-fee exempt** (ties to `va_fee.rs`).
//!
//! # Scope boundary (what this module does NOT do)
//!
//! This is the reference-data calculator: it operates on COE-supplied inputs
//! (entitlement status, entitlement already charged) and published formulas.
//! Reconstructing charged entitlement from a borrower's prior-loan history,
//! restoration-eligibility determination, and multi-property entitlement
//! tracking are borrower/underwriting concerns owned by the qualifying engine
//! (Epic 6), which consumes this module's outputs.
//!
//! # Lender overlay
//!
//! The lender's maximum VA loan amount is expressed through the shared
//! [`crate::lender::LenderOverlays`] structure (field `max_va_loan_amount_cents`)
//! so that all lender tightening lives in one place. It can only *reduce* the
//! achievable maximum, never raise it.
//!
//! # Provenance
//!
//! Every public method returns `Derived<T>` carrying the VA citation and the
//! ordered rule trail.

use serde::{Deserialize, Serialize};
use types::{Cents, Derived, Provenance};

/// Entitlement status as reflected on the Certificate of Eligibility (COE).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntitlementStatus {
    /// Never used, or fully restored — no county-limit cap applies.
    Full,
    /// Some entitlement is charged to an active or unrestored prior loan.
    Partial,
}

/// Loan purpose — entitlement treatment differs for IRRRLs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VaLoanPurpose {
    /// Purchase, construction, or condominium (the §3710(a)(1)(2)(3)(5)(6)(8) set).
    Purchase,
    /// Interest Rate Reduction Refinancing Loan — guaranty is always 25% of the
    /// loan regardless of amount or remaining entitlement (38 CFR 36.4302(b)).
    Irrrl,
    /// Cash-out / regular refinance — treated like a purchase for guaranty.
    CashOutRefinance,
}

/// COE entitlement code (identifier), capturing the restoration/eligibility
/// basis stated on the Certificate of Eligibility. These identify *why* the
/// entitlement is full or partial; the guaranty math keys off `EntitlementStatus`
/// but the code is the audit-trail identifier. (Restoration *eligibility* logic
/// is owned by the qualifying engine, Epic 6 — this is the recorded status only.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoeEntitlementCode {
    /// No prior use — full entitlement.
    NeverUsed,
    /// Prior VA loan paid in full and property disposed — entitlement restored.
    RestoredDisposedPaidInFull,
    /// One-time restoration used (loan paid off, property retained). Full, once.
    OneTimeRestoration,
    /// Prior VA loan active/unrestored — partial entitlement, amount charged.
    InUseNotRestored,
    /// Entitlement substituted by another veteran who assumed the loan.
    SubstitutionOfEntitlement,
}

impl CoeEntitlementCode {
    /// The entitlement status this code implies for the guaranty calculation.
    #[must_use]
    pub fn implies_status(self) -> EntitlementStatus {
        match self {
            CoeEntitlementCode::InUseNotRestored => EntitlementStatus::Partial,
            _ => EntitlementStatus::Full,
        }
    }
}

/// One statutory guaranty band (38 U.S.C. 3703(a)(1)(A) / 38 CFR 36.4302).
#[derive(Debug, Clone, Deserialize)]
pub struct GuarantyBand {
    pub min_loan_cents: i64,
    pub max_loan_cents: i64,
    /// Percentage of the loan in bps; `None` if the band is a flat dollar amount.
    pub pct_bps: Option<u32>,
    /// Flat guaranty in cents (e.g. $22,500 band); `None` if percentage-based.
    pub flat_cents: Option<i64>,
    /// Dollar cap on the percentage result (e.g. $36,000 or $60,000); `None` if uncapped.
    pub max_cents: Option<i64>,
    pub note: String,
}

/// Statutory VA entitlement parameters (one `va_entitlement_{year}.json`).
#[derive(Debug, Clone, Deserialize)]
pub struct VaEntitlementParams {
    pub effective_date: String,
    pub source_citation: String,
    /// Basic (tier-1) entitlement in cents (statutory $36,000).
    pub basic_entitlement_cents: i64,
    /// Loan amount up to which tier-1 alone gives a 25% guaranty ($144,000).
    pub tier1_ceiling_cents: i64,
    /// VA guaranty percentage in basis points (2500 = 25%) — the >$144k tier.
    pub guaranty_pct_bps: u32,
    /// Lender/secondary down-payment guidance for the non-veteran share of a
    /// joint loan, in basis points (1250 = 12.5%).
    pub joint_nonveteran_dp_pct_bps: u32,
    /// Statutory guaranty schedule (the four bands).
    pub guaranty_bands: Vec<GuarantyBand>,
    /// IRRRL guaranty percentage in bps (always 25% of the loan).
    pub irrrl_guaranty_pct_bps: u32,
    pub irrrl_note: String,
}

impl VaEntitlementParams {
    /// The statutory maximum guaranty for a loan amount under full entitlement,
    /// per the band schedule (38 CFR 36.4302). Returns (guaranty_cents, band_note).
    #[must_use]
    pub fn statutory_guaranty(&self, loan_cents: i64) -> (i64, String) {
        for b in &self.guaranty_bands {
            if loan_cents > b.min_loan_cents && loan_cents <= b.max_loan_cents {
                let g = if let Some(flat) = b.flat_cents {
                    flat
                } else if let Some(pct) = b.pct_bps {
                    let raw = loan_cents * i64::from(pct) / 10_000;
                    b.max_cents.map_or(raw, |cap| raw.min(cap))
                } else {
                    0
                };
                return (g, b.note.clone());
            }
        }
        // Above all bands (shouldn't happen — last band is open-ended).
        (
            loan_cents * i64::from(self.guaranty_pct_bps) / 10_000,
            "default 25%".to_owned(),
        )
    }
}

/// Inputs for a guaranty / entitlement computation. All borrower-specific
/// facts are supplied by the caller (from the COE and the application).
#[derive(Debug, Clone)]
pub struct VaGuarantyInput {
    /// County conforming limit (from the GSE limit; see `va_county_loan_limit`).
    pub county_conforming_limit: Cents,
    pub entitlement_status: EntitlementStatus,
    /// Entitlement already charged and not restored, in cents (0 for full).
    pub entitlement_used_cents: i64,
    pub proposed_loan_amount: Cents,
    pub down_payment: Cents,
    /// Loan purpose (IRRRLs are guaranteed at a flat 25%).
    pub loan_purpose: VaLoanPurpose,
    /// Total borrowers on the loan.
    pub total_borrowers: u8,
    /// Borrowers contributing VA entitlement (≥1 for a VA loan).
    pub veteran_borrowers: u8,
    /// True when every non-veteran co-borrower is the veteran's spouse
    /// (the spouse exception — no joint-loan down payment is triggered).
    pub nonveteran_coborrowers_all_spouses: bool,
    /// Service-connected disability → funding-fee exempt.
    pub disability_exempt: bool,
    /// Lender's max VA loan overlay, if any (from `LenderOverlays`).
    pub lender_max_va_loan_cents: Option<i64>,
}

/// The fully-analyzed guaranty result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VaGuarantyResult {
    /// VA's dollar guaranty for this specific loan.
    pub guaranty_cents: i64,
    /// Effective guaranty as a percentage of the loan, in bps.
    pub guaranty_pct_bps: u32,
    /// Maximum loan achievable with $0 down. `i64::MAX` denotes "no VA cap"
    /// (full entitlement, no lender overlay).
    pub zero_down_max_cents: i64,
    /// Down payment required for `proposed_loan_amount` (0 if none).
    pub required_down_payment_cents: i64,
    /// Whether the loan achieves the 25% guaranty lenders require.
    pub meets_25pct_guaranty: bool,
    pub funding_fee_exempt: bool,
    /// True if a lender max-loan overlay reduced the achievable maximum.
    pub lender_capped: bool,
    /// True if this is a joint loan triggering a non-veteran down payment.
    pub is_joint_nonveteran_loan: bool,
    /// The statutory guaranty band applied (audit identifier).
    pub guaranty_band_note: String,
    /// True if treated as an IRRRL (flat 25% guaranty).
    pub is_irrrl: bool,
}

/// Top-level file shape.
#[derive(Debug, Deserialize)]
pub struct VaEntitlementFile {
    #[serde(flatten)]
    pub params: VaEntitlementParams,
}

fn provenance(p: &VaEntitlementParams, file: &str, req: u16, res: u16, record: &str) -> Provenance {
    Provenance {
        dataset: "va_entitlement".to_owned(),
        source_file: file.to_owned(),
        source_citation: p.source_citation.clone(),
        effective_date: p.effective_date.clone(),
        record_id: record.to_owned(),
        requested_version: req,
        resolved_version: res,
    }
}

/// The VA "county loan limit" is the GSE conforming limit. This helper makes the
/// delegation explicit and records it in the trail. Callers pass the GSE limit
/// they already resolved (from `geo.rs`); this wraps it with VA provenance.
#[must_use]
pub fn va_county_loan_limit(
    gse_limit: Cents,
    params: &VaEntitlementParams,
    file: &str,
    req: u16,
    res: u16,
) -> Derived<Cents> {
    Derived::new(
        gse_limit,
        provenance(params, file, req, res, "va_county_limit_delegation"),
    )
    .with_step(
        "delegate_to_gse_conforming_limit",
        format!("gse_limit=${}", gse_limit.0 / 100),
        "VA imposes no county cap post-2020; partial-entitlement math uses the GSE limit"
            .to_owned(),
    )
}

/// Full entitlement + guaranty analysis. Pure function of the supplied inputs.
#[must_use]
pub fn compute_va_guaranty(
    input: &VaGuarantyInput,
    params: &VaEntitlementParams,
    file: &str,
    req: u16,
    res: u16,
) -> Derived<VaGuarantyResult> {
    let pct = i64::from(params.guaranty_pct_bps); // bps (the >$144k tier)
    let loan = input.proposed_loan_amount.0;
    let is_irrrl = input.loan_purpose == VaLoanPurpose::Irrrl;

    // Joint-loan determination (veteran + non-veteran non-spouse).
    let is_joint = input.veteran_borrowers < input.total_borrowers
        && !input.nonveteran_coborrowers_all_spouses;

    // Veteran pro-rata share of the loan (for joint loans).
    let vet_share_num = i64::from(input.veteran_borrowers.max(1));
    let vet_share_den = i64::from(input.total_borrowers.max(1));
    let guaranteed_base = if is_joint {
        loan * vet_share_num / vet_share_den
    } else {
        loan
    };

    let mut steps: Vec<(&str, String, String)> = Vec::new();
    let band_note: String;

    if is_irrrl {
        steps.push((
            "irrrl_guaranty",
            format!("IRRRL loan ${}", loan / 100),
            "flat 25% guaranty regardless of amount/entitlement (38 CFR 36.4302(b))".to_owned(),
        ));
    }
    steps.push((
        "classify_entitlement",
        format!(
            "status={:?}, used=${}",
            input.entitlement_status,
            input.entitlement_used_cents / 100
        ),
        format!("{:?} entitlement", input.entitlement_status),
    ));

    let (zero_down_max, guaranty): (i64, i64) = match input.entitlement_status {
        EntitlementStatus::Full => {
            // Statutory guaranty schedule (38 CFR 36.4302); IRRRL is flat 25%.
            let g;
            if is_irrrl {
                g = guaranteed_base * i64::from(params.irrrl_guaranty_pct_bps) / 10_000;
                band_note = "IRRRL flat 25%".to_owned();
            } else {
                let (sg, note) = params.statutory_guaranty(guaranteed_base);
                g = sg;
                band_note = note;
            }
            steps.push((
                "full_entitlement_guaranty",
                format!("statutory band on ${}", guaranteed_base / 100),
                format!("guaranty=${} [{}], no VA loan cap", g / 100, band_note),
            ));
            (i64::MAX, g)
        }
        EntitlementStatus::Partial => {
            // Remaining entitlement = 25% of county limit − used.
            let max_avail = input.county_conforming_limit.0 * pct / 10_000;
            let remaining = (max_avail - input.entitlement_used_cents).max(0);
            let zdm = remaining * 4;
            // Guaranty for the actual loan: lesser of 25% of loan vs remaining.
            let g = (guaranteed_base * pct / 10_000).min(remaining);
            band_note = "partial: lesser of 25% loan or (25% county − used)".to_owned();
            steps.push((
                "partial_entitlement_remaining",
                format!(
                    "25% of county ${} − used ${}",
                    input.county_conforming_limit.0 / 100,
                    input.entitlement_used_cents / 100
                ),
                format!(
                    "remaining entitlement=${}, zero-down max=${}",
                    remaining / 100,
                    zdm / 100
                ),
            ));
            (zdm, g)
        }
    };

    // Apply lender overlay to the zero-down ceiling.
    let mut lender_capped = false;
    let mut effective_zero_down = zero_down_max;
    if let Some(cap) = input.lender_max_va_loan_cents {
        if cap < effective_zero_down {
            effective_zero_down = cap;
            steps.push((
                "apply_lender_overlay",
                format!("lender max=${}", cap / 100),
                format!("ceiling set to ${} (overlay tightens)", cap / 100),
            ));
        }
        // The cap constrains THIS loan only when the loan exceeds it.
        if loan > cap {
            lender_capped = true;
        }
    }

    // Down payment required if the loan exceeds the zero-down ceiling.
    // For partial entitlement the shortfall DP is 25% of the gap; for a
    // full-entitlement loan over a lender cap, the gap above the cap.
    let mut required_dp = 0i64;
    if effective_zero_down != i64::MAX && loan > effective_zero_down {
        let gap = loan - effective_zero_down;
        required_dp = match input.entitlement_status {
            EntitlementStatus::Partial => gap * pct / 10_000, // 25% of gap
            EntitlementStatus::Full => gap,                   // lender cap: fund the excess
        };
        steps.push((
            "shortfall_down_payment",
            format!(
                "loan ${} > zero-down max ${}",
                loan / 100,
                effective_zero_down / 100
            ),
            format!("required down payment ≈ ${}", required_dp / 100),
        ));
    }

    // Joint-loan non-veteran down payment guidance.
    if is_joint {
        let nonvet_share = loan - guaranteed_base;
        let joint_dp = nonvet_share * i64::from(params.joint_nonveteran_dp_pct_bps) / 10_000;
        required_dp = required_dp.max(joint_dp);
        steps.push((
            "joint_loan_nonveteran_dp",
            format!(
                "non-veteran share ${} × {}%",
                nonvet_share / 100,
                params.joint_nonveteran_dp_pct_bps / 100
            ),
            format!(
                "joint-loan down payment guidance ≈ ${} (secondary-market practice)",
                joint_dp / 100
            ),
        ));
    }

    // 25% guaranty check (guaranty + down payment ≥ 25% of loan).
    let coverage = guaranty + input.down_payment.0.max(required_dp);
    let meets_25 = loan == 0 || coverage * 10_000 / loan >= pct;
    steps.push((
        "guaranty_coverage_check",
        format!(
            "guaranty ${} + DP ${} vs 25% of ${}",
            guaranty / 100,
            input.down_payment.0.max(required_dp) / 100,
            loan / 100
        ),
        if meets_25 {
            "meets 25% coverage".to_owned()
        } else {
            "below 25% — more down payment needed".to_owned()
        },
    ));

    if input.disability_exempt {
        steps.push((
            "funding_fee_exemption",
            "service-connected disability".to_owned(),
            "VA funding fee EXEMPT".to_owned(),
        ));
    }

    let guaranty_pct_bps = if loan > 0 {
        (guaranty * 10_000 / loan) as u32
    } else {
        0
    };
    let result = VaGuarantyResult {
        guaranty_cents: guaranty,
        guaranty_pct_bps,
        zero_down_max_cents: effective_zero_down,
        required_down_payment_cents: required_dp,
        meets_25pct_guaranty: meets_25,
        funding_fee_exempt: input.disability_exempt,
        lender_capped,
        is_joint_nonveteran_loan: is_joint,
        guaranty_band_note: band_note,
        is_irrrl,
    };

    let mut d = Derived::new(
        result,
        provenance(params, file, req, res, "va_guaranty_calc"),
    );
    for (r, i, o) in steps {
        d.push_step(r, i, o);
    }
    d
}
