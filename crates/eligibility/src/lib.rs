//! Epic 8 / Task T8.1 — loan-product eligibility (Conventional + FHA).
//!
//! Evaluates whether a pre-PII scenario passes a program's **eligibility
//! guardrails** — NOT underwriting. We apply only the gates derivable from
//! borrower-stated inputs: representative credit score, LTV (tier-aware),
//! loan-amount vs county limit (unit-aware, high-balance-aware), occupancy,
//! and property type. We deliberately SKIP the DTI / reserves gates that
//! `EligibilityError` defines, because those need verified income/assets we
//! never collect (consistent with the pre-credit-pull posture).
//!
//! Evaluation order follows `ref_data::program_rules`:
//!   1. credit score ≥ minimum (FHA alt-tier aware)
//!   2. LTV ≤ maximum for tier (high-balance aware)
//!   3. loan amount ≤ county limit (unit-aware)
//!   4. occupancy permitted
//!   5. property type eligible
//!
//! Each verdict is a `Derived<ProgramVerdict>` carrying the full reasoning
//! trail. Multiple failures are reported together — the funnel needs to know
//! *all* reasons a scenario is excluded, not just the first.
//!
//! T8.1 ships Conventional + FHA (highest volume). VA/USDA/HomeReady/Home
//! Possible/Bond follow; the trait + per-program dispatch are program-agnostic,
//! so adding a program is a new match arm + data, not a new abstraction.

use ref_data::{
    FhaLoanLimits, GseLoanLimits, ProgramEligibilityRules, RefDataResult, RefDataStore, Versioned,
};
use types::{
    Cents, CreditScore, Derived, EligibilityError, LtvBasisPoints, Occupancy, ProgramCode,
    PropertyType,
};

/// The narrow slice of `RefDataStore` that eligibility needs. Blanket-impl'd
/// for any `RefDataStore`, so production passes the real store and tests stub
/// only these three methods.
pub trait EligibilityData {
    fn program_rules(&self, program: ProgramCode) -> RefDataResult<ProgramEligibilityRules>;
    fn gse_loan_limits(&self, fips: &str, year: u16) -> RefDataResult<Versioned<GseLoanLimits>>;
    fn fha_loan_limits(&self, fips: &str, year: u16) -> RefDataResult<Versioned<FhaLoanLimits>>;
}

impl<S: RefDataStore> EligibilityData for S {
    fn program_rules(&self, program: ProgramCode) -> RefDataResult<ProgramEligibilityRules> {
        RefDataStore::program_rules(self, program)
    }
    fn gse_loan_limits(&self, fips: &str, year: u16) -> RefDataResult<Versioned<GseLoanLimits>> {
        RefDataStore::gse_loan_limits(self, fips, year)
    }
    fn fha_loan_limits(&self, fips: &str, year: u16) -> RefDataResult<Versioned<FhaLoanLimits>> {
        RefDataStore::fha_loan_limits(self, fips, year)
    }
}

/// A pre-PII eligibility scenario: only borrower-stated, pre-credit-pull facts.
#[derive(Debug, Clone)]
pub struct EligibilityScenario {
    pub program: ProgramCode,
    /// Representative credit score (engine computes this upstream from the
    /// per-borrower scores; here it is the single value the gates use).
    pub representative_score: CreditScore,
    pub loan_amount: Cents,
    pub property_value: Cents,
    pub ltv: LtvBasisPoints,
    pub occupancy: Occupancy,
    pub property_type: PropertyType,
    /// County FIPS (5 digit) — required for the loan-limit lookup.
    pub county_fips: String,
    pub year: u16,
}

impl EligibilityScenario {
    /// Down payment as LTV-complement basis points (10000 − ltv), clamped at 0.
    #[must_use]
    fn down_payment_bps(&self) -> u32 {
        10_000u32.saturating_sub(self.ltv.0)
    }
}

/// The outcome of evaluating one program against one scenario.
#[derive(Debug)]
pub enum ProgramVerdict {
    Eligible,
    Ineligible(Vec<EligibilityError>),
}

impl ProgramVerdict {
    #[must_use]
    pub fn is_eligible(&self) -> bool {
        matches!(self, ProgramVerdict::Eligible)
    }
    #[must_use]
    pub fn reasons(&self) -> &[EligibilityError] {
        match self {
            ProgramVerdict::Eligible => &[],
            ProgramVerdict::Ineligible(v) => v,
        }
    }
}

/// The eligibility computation seam. Implemented over a `RefDataStore` so the
/// rules/limits come from the versioned catalogs, never hard-coded.
pub trait EligibilityEngine {
    /// Evaluate one program against the scenario, returning a traced verdict.
    fn evaluate_program(
        &self,
        scenario: &EligibilityScenario,
        program: ProgramCode,
    ) -> RefDataResult<Derived<ProgramVerdict>>;
}

/// Default engine backed by any `RefDataStore`.
pub struct StoreEligibilityEngine<'a, S: EligibilityData> {
    store: &'a S,
}

impl<S: EligibilityData> std::fmt::Debug for StoreEligibilityEngine<'_, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StoreEligibilityEngine")
            .finish_non_exhaustive()
    }
}

impl<'a, S: EligibilityData> StoreEligibilityEngine<'a, S> {
    #[must_use]
    pub fn new(store: &'a S) -> Self {
        Self { store }
    }

    /// Occupancy permitted for a program. Conv allows all; FHA primary only.
    fn occupancy_ok(program: ProgramCode, requires_primary: bool, occ: Occupancy) -> bool {
        if requires_primary {
            return matches!(occ, Occupancy::PrimaryResidence);
        }
        // Program-specific carve-outs beyond the primary-residence flag.
        match program {
            ProgramCode::Fha => matches!(occ, Occupancy::PrimaryResidence),
            _ => true,
        }
    }

    /// Property type eligibility. Personal-property/mobile homes are ineligible
    /// for both Conv and FHA as real property.
    fn property_type_ok(pt: PropertyType) -> bool {
        !pt.is_ineligible_personal_property()
    }
}

impl<S: EligibilityData> EligibilityEngine for StoreEligibilityEngine<'_, S> {
    fn evaluate_program(
        &self,
        scenario: &EligibilityScenario,
        program: ProgramCode,
    ) -> RefDataResult<Derived<ProgramVerdict>> {
        let program_name = format!("{program:?}");
        let rules = self.store.program_rules(program)?;
        let mut errors: Vec<EligibilityError> = Vec::new();
        let prov = types::Provenance {
            dataset: "program_eligibility".to_owned(),
            source_file: "program_rules".to_owned(),
            source_citation: format!("{program:?} agency guidelines (program_rules)"),
            effective_date: scenario.year.to_string(),
            record_id: program_name.clone(),
            requested_version: scenario.year,
            resolved_version: scenario.year,
        };
        let mut steps: Vec<(String, String, String)> = Vec::new();

        // 1. Credit score (FHA alt-tier aware via down payment).
        let dp_bps = scenario.down_payment_bps();
        let min_score = rules.min_credit_score_for_down_payment(dp_bps);
        let score = scenario.representative_score.0;
        steps.push((
            "credit_score".into(),
            format!("score={score} dp_bps={dp_bps} min={min_score}"),
            if score >= min_score {
                "pass".into()
            } else {
                "FAIL".into()
            },
        ));
        if score < min_score {
            errors.push(EligibilityError::CreditScoreBelowMinimum {
                score,
                minimum: min_score,
                program: program_name.clone(),
            });
        }

        // 2. LTV (high-balance aware). Determine high-balance for Conv via GSE limit.
        let is_high_balance = match program {
            ProgramCode::Conventional => {
                let lim = self
                    .store
                    .gse_loan_limits(&scenario.county_fips, scenario.year)?;
                lim.data
                    .is_high_balance_amount(scenario.loan_amount, scenario.year)
            }
            _ => false,
        };
        let max_ltv = rules.max_ltv_for(scenario.representative_score, is_high_balance);
        steps.push((
            "ltv".into(),
            format!(
                "ltv={} max={} high_balance={is_high_balance}",
                scenario.ltv.0, max_ltv.0
            ),
            if scenario.ltv.0 <= max_ltv.0 {
                "pass".into()
            } else {
                "FAIL".into()
            },
        ));
        if scenario.ltv.0 > max_ltv.0 {
            errors.push(EligibilityError::LtvExceedsLimit {
                ltv_bps: scenario.ltv.0,
                ltv_display: f64::from(scenario.ltv.0) / 100.0,
                limit_bps: max_ltv.0,
                limit_display: f64::from(max_ltv.0) / 100.0,
                program: program_name.clone(),
            });
        }

        // 3. Loan amount ≤ county limit (unit-aware).
        let units = match scenario.property_type {
            PropertyType::TwoUnit => 2,
            PropertyType::ThreeUnit => 3,
            PropertyType::FourUnit => 4,
            _ => 1,
        };
        let limit: Cents = match program {
            ProgramCode::Conventional => {
                let l = self
                    .store
                    .gse_loan_limits(&scenario.county_fips, scenario.year)?;
                l.data.limit_for(units)
            }
            ProgramCode::Fha => {
                let l = self
                    .store
                    .fha_loan_limits(&scenario.county_fips, scenario.year)?;
                l.data.limit_for(units)
            }
            // Other programs (VA/USDA/affordable) land in later tasks.
            _ => {
                return Err(ref_data::RefDataError::Storage(format!(
                    "eligibility not yet implemented for {program:?}"
                )))
            }
        };
        steps.push((
            "loan_limit".into(),
            format!(
                "loan={} limit={} units={units}",
                scenario.loan_amount.0, limit.0
            ),
            if scenario.loan_amount.0 <= limit.0 {
                "pass".into()
            } else {
                "FAIL".into()
            },
        ));
        if scenario.loan_amount.0 > limit.0 {
            errors.push(EligibilityError::LoanAmountOutOfRange {
                amount_dollars: scenario.loan_amount.as_f64_dollars(),
                min_dollars: 0.0,
                max_dollars: limit.as_f64_dollars(),
                program: program_name.clone(),
            });
        }

        // 4. Occupancy.
        let occ_ok = Self::occupancy_ok(
            program,
            rules.requires_primary_residence,
            scenario.occupancy,
        );
        steps.push((
            "occupancy".into(),
            format!(
                "occ={:?} requires_primary={}",
                scenario.occupancy, rules.requires_primary_residence
            ),
            if occ_ok { "pass".into() } else { "FAIL".into() },
        ));
        if !occ_ok {
            errors.push(EligibilityError::IneligibleOccupancy {
                occupancy: format!("{:?}", scenario.occupancy),
                program: program_name.clone(),
            });
        }

        // 5. Property type.
        let pt_ok = Self::property_type_ok(scenario.property_type);
        steps.push((
            "property_type".into(),
            format!("type={:?}", scenario.property_type),
            if pt_ok { "pass".into() } else { "FAIL".into() },
        ));
        if !pt_ok {
            errors.push(EligibilityError::IneligiblePropertyType {
                property_type: format!("{:?}", scenario.property_type),
                program: program_name.clone(),
            });
        }

        let verdict = if errors.is_empty() {
            ProgramVerdict::Eligible
        } else {
            ProgramVerdict::Ineligible(errors)
        };
        let mut d = Derived::new(verdict, prov);
        for (rule, inputs, outcome) in steps {
            d.push_step(rule, inputs, outcome);
        }
        Ok(d)
    }
}
