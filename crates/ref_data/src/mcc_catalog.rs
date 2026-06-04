//! Task 4.24 — Mortgage Credit Certificate (MCC) program catalog.
//!
//! MCC programs are administered by state/local Housing Finance Agencies (HFAs).
//! They convert a percentage of a borrower's annual mortgage interest into a
//! dollar-for-dollar **federal income tax credit**. Under IRS rules, when the
//! credit rate exceeds 20% the annual credit is capped at $2,000.
//!
//! Eligibility gates: first-time-homebuyer status (with veteran and
//! targeted-area exemptions), household income limits (by size and
//! targeted-area status), and purchase-price limits.
//!
//! # Provenance
//!
//! Every public method returns a `Derived<T>` so the caller gets not just the
//! answer but the ordered chain of rules and the exact catalog record that
//! produced it. Call `.explain()` on any result for the human-readable trail.
//!
//! # Updating
//!
//! All program parameters live in `data/mcc_catalog_{year}.json`. HFAs revise
//! income/price limits and occasionally credit rates annually. Add a new
//! `mcc_catalog_{YYYY}.json` to update; no Rust code changes are needed.

use serde::{Deserialize, Serialize};
use types::{Cents, Derived, Provenance};

/// One MCC program in the catalog (one row of `mcc_catalog_{year}.json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MccProgram {
    pub program_id: String,
    /// Two-letter state code the program serves.
    pub state: String,
    pub administering_agency: String,
    pub program_name: String,
    /// Credit rate in basis points (4000 = 40% of annual interest).
    pub credit_rate_bps: u32,
    /// Annual dollar cap on the credit, in cents. IRS caps at $2,000 when the
    /// rate exceeds 20%; programs at ≤20% may carry a `0` (no cap) sentinel.
    pub annual_credit_cap_cents: i64,
    /// Whether first-time homebuyer status is required.
    pub fthb_required: bool,
    /// Veterans are exempt from the FTHB requirement (per 26 U.S.C. §143(d)(2)).
    pub veteran_fthb_exempt: bool,
    /// Purchases in a federally targeted area are exempt from FTHB.
    pub targeted_area_fthb_exempt: bool,
    /// Income limit (cents) for 1–2 person households, non-targeted area.
    pub income_limit_1_2_cents: i64,
    /// Income limit (cents) for 3+ person households, non-targeted area.
    pub income_limit_3plus_cents: i64,
    /// Income limit (cents) for 1–2 person households in a targeted area.
    pub income_limit_targeted_1_2_cents: i64,
    /// Income limit (cents) for 3+ person households in a targeted area.
    pub income_limit_targeted_3plus_cents: i64,
    /// Purchase-price limit (cents), non-targeted area.
    pub price_limit_cents: i64,
    /// Purchase-price limit (cents), targeted area.
    pub price_limit_targeted_cents: i64,
}

impl MccProgram {
    /// Applicable income limit given household size and targeted-area status.
    #[must_use]
    pub fn income_limit(&self, household_size: u8, targeted: bool) -> Cents {
        let small = household_size <= 2;
        Cents(match (targeted, small) {
            (true, true) => self.income_limit_targeted_1_2_cents,
            (true, false) => self.income_limit_targeted_3plus_cents,
            (false, true) => self.income_limit_1_2_cents,
            (false, false) => self.income_limit_3plus_cents,
        })
    }

    /// Applicable purchase-price limit given targeted-area status.
    #[must_use]
    pub fn price_limit(&self, targeted: bool) -> Cents {
        Cents(if targeted {
            self.price_limit_targeted_cents
        } else {
            self.price_limit_cents
        })
    }
}

/// Borrower / transaction facts needed to evaluate MCC eligibility.
#[derive(Debug, Clone)]
pub struct MccEligibilityInput {
    pub state: String,
    pub is_first_time_homebuyer: bool,
    pub is_veteran: bool,
    pub in_targeted_area: bool,
    pub household_size: u8,
    pub annual_household_income: Cents,
    pub purchase_price: Cents,
}

/// The result of evaluating MCC eligibility for a borrower against a program.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MccOutcome {
    pub eligible: bool,
    pub program_id: String,
    pub credit_rate_bps: u32,
    /// Reasons the borrower failed (empty when eligible).
    pub disqualifiers: Vec<String>,
}

/// Top-level shape of `mcc_catalog_{year}.json`.
#[derive(Debug, Deserialize)]
pub struct MccCatalogFile {
    pub effective_date: String,
    pub source_citation: String,
    pub programs: Vec<MccProgram>,
}

impl MccCatalogFile {
    fn provenance_for(
        &self,
        program: &MccProgram,
        source_file: &str,
        requested: u16,
        resolved: u16,
    ) -> Provenance {
        Provenance {
            dataset: "mcc_catalog".to_owned(),
            source_file: source_file.to_owned(),
            source_citation: self.source_citation.clone(),
            effective_date: self.effective_date.clone(),
            record_id: program.program_id.clone(),
            requested_version: requested,
            resolved_version: resolved,
        }
    }

    /// Find the MCC program serving a state, wrapped with provenance.
    ///
    /// Returns `None` (no `Derived`) when the state has no program in the catalog.
    pub fn lookup(
        &self,
        state: &str,
        source_file: &str,
        requested: u16,
        resolved: u16,
    ) -> Option<Derived<MccProgram>> {
        let program = self
            .programs
            .iter()
            .find(|p| p.state.eq_ignore_ascii_case(state))?;
        let prov = self.provenance_for(program, source_file, requested, resolved);
        Some(Derived::new(program.clone(), prov).with_step(
            "lookup_program_by_state",
            format!("state={state}"),
            format!(
                "matched '{}' ({})",
                program.program_name, program.administering_agency
            ),
        ))
    }

    /// Evaluate full MCC eligibility for a borrower, returning a fully-traced outcome.
    pub fn evaluate(
        &self,
        input: &MccEligibilityInput,
        source_file: &str,
        requested: u16,
        resolved: u16,
    ) -> Option<Derived<MccOutcome>> {
        let program = self
            .programs
            .iter()
            .find(|p| p.state.eq_ignore_ascii_case(&input.state))?;
        let prov = self.provenance_for(program, source_file, requested, resolved);
        let mut disqualifiers: Vec<String> = Vec::new();

        let mut derived = Derived::new(
            MccOutcome {
                eligible: false, // set at the end
                program_id: program.program_id.clone(),
                credit_rate_bps: program.credit_rate_bps,
                disqualifiers: Vec::new(),
            },
            prov,
        );
        derived.push_step(
            "lookup_program_by_state",
            format!("state={}", input.state),
            format!("matched '{}'", program.program_name),
        );

        // ── Gate 1: First-time homebuyer (with exemptions) ──────────────────
        if program.fthb_required && !input.is_first_time_homebuyer {
            let vet_ok = program.veteran_fthb_exempt && input.is_veteran;
            let targeted_ok = program.targeted_area_fthb_exempt && input.in_targeted_area;
            if vet_ok {
                derived.push_step(
                    "first_time_homebuyer_requirement",
                    format!("is_fthb=false, is_veteran={}", input.is_veteran),
                    "satisfied via veteran exemption".to_owned(),
                );
            } else if targeted_ok {
                derived.push_step(
                    "first_time_homebuyer_requirement",
                    format!("is_fthb=false, targeted_area={}", input.in_targeted_area),
                    "satisfied via targeted-area exemption".to_owned(),
                );
            } else {
                disqualifiers.push("not a first-time homebuyer; no exemption applies".to_owned());
                derived.push_step(
                    "first_time_homebuyer_requirement",
                    format!(
                        "is_fthb=false, is_veteran={}, targeted_area={}",
                        input.is_veteran, input.in_targeted_area
                    ),
                    "FAILED — no exemption".to_owned(),
                );
            }
        } else {
            derived.push_step(
                "first_time_homebuyer_requirement",
                format!(
                    "fthb_required={}, is_fthb={}",
                    program.fthb_required, input.is_first_time_homebuyer
                ),
                "satisfied".to_owned(),
            );
        }

        // ── Gate 2: Income limit ────────────────────────────────────────────
        let income_limit = program.income_limit(input.household_size, input.in_targeted_area);
        if input.annual_household_income > income_limit {
            disqualifiers.push(format!(
                "household income ${} exceeds limit ${}",
                input.annual_household_income.0 / 100,
                income_limit.0 / 100
            ));
            derived.push_step(
                "income_limit",
                format!(
                    "income=${}, size={}, targeted={}",
                    input.annual_household_income.0 / 100,
                    input.household_size,
                    input.in_targeted_area
                ),
                format!("FAILED — exceeds limit ${}", income_limit.0 / 100),
            );
        } else {
            derived.push_step(
                "income_limit",
                format!(
                    "income=${}, size={}, targeted={}",
                    input.annual_household_income.0 / 100,
                    input.household_size,
                    input.in_targeted_area
                ),
                format!("satisfied (limit ${})", income_limit.0 / 100),
            );
        }

        // ── Gate 3: Purchase-price limit ────────────────────────────────────
        let price_limit = program.price_limit(input.in_targeted_area);
        if input.purchase_price > price_limit {
            disqualifiers.push(format!(
                "purchase price ${} exceeds limit ${}",
                input.purchase_price.0 / 100,
                price_limit.0 / 100
            ));
            derived.push_step(
                "purchase_price_limit",
                format!(
                    "price=${}, targeted={}",
                    input.purchase_price.0 / 100,
                    input.in_targeted_area
                ),
                format!("FAILED — exceeds limit ${}", price_limit.0 / 100),
            );
        } else {
            derived.push_step(
                "purchase_price_limit",
                format!(
                    "price=${}, targeted={}",
                    input.purchase_price.0 / 100,
                    input.in_targeted_area
                ),
                format!("satisfied (limit ${})", price_limit.0 / 100),
            );
        }

        let eligible = disqualifiers.is_empty();
        derived.push_step(
            "final_determination",
            format!("{} gate(s) failed", disqualifiers.len()),
            if eligible {
                "ELIGIBLE".to_owned()
            } else {
                "INELIGIBLE".to_owned()
            },
        );
        derived.value.eligible = eligible;
        derived.value.disqualifiers = disqualifiers;
        Some(derived)
    }
}

/// Estimate the annual MCC tax credit given the program and the borrower's
/// first-year mortgage interest. Returns a fully-traced `Derived<Cents>`.
///
/// credit = min(cap, credit_rate × annual_interest); a `0` cap means uncapped.
#[must_use]
pub fn estimate_annual_credit(
    program: Derived<MccProgram>,
    annual_interest: Cents,
) -> Derived<Cents> {
    let rate_bps = program.value.credit_rate_bps;
    let cap = program.value.annual_credit_cap_cents;
    let uncapped = annual_interest.0 as i128 * i128::from(rate_bps) / 10_000;
    let (credit, capped) = if cap > 0 && uncapped > i128::from(cap) {
        (cap, true)
    } else {
        (uncapped as i64, false)
    };
    let interest_dollars = annual_interest.0 / 100;
    let uncapped_dollars = (uncapped / 100) as i64;
    program
        .map(|_| Cents(credit))
        .with_step(
            "apply_credit_rate",
            format!(
                "annual_interest=${interest_dollars}, rate={}%",
                rate_bps / 100
            ),
            format!("uncapped credit=${uncapped_dollars}"),
        )
        .with_step(
            "apply_annual_cap",
            format!(
                "cap=${}",
                if cap > 0 {
                    (cap / 100).to_string()
                } else {
                    "none".to_owned()
                }
            ),
            if capped {
                format!("capped to ${}", credit / 100)
            } else {
                format!("under cap, credit=${}", credit / 100)
            },
        )
}
