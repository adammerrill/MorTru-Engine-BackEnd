//! Task 4.25 — Down Payment Assistance (DPA) program catalog for TX, CA, FL.
//!
//! DPA programs are administered by state Housing Finance Agencies (HFAs) and
//! local (county/city) agencies. They reduce the cash a buyer needs at closing
//! by supplying part or all of the down payment and/or closing costs as a
//! grant, deferred ("silent") second, forgivable second, or amortizing second.
//!
//! # MISMO / RESO alignment
//!
//! DPA is **secondary financing** layered beneath the first mortgage. The model
//! maps to MISMO and RESO as follows so it composes with the rest of the engine:
//!
//! * `DpaAssistanceType` → MISMO `DownPaymentType` / subordinate-lien funding.
//!   Grant ≈ MISMO `Grant`; deferred/forgivable/amortizing ≈ a `SubordinateLien`
//!   `LOAN` whose `LoanProgram` is the DPA program and whose payment behavior is
//!   captured by `interest_rate_bps`, `term_months`, and the forgiveness fields.
//! * `DpaFundsSource` → MISMO `FundsSourceType`
//!   (`StateAgency` / `LocalAgency` / `Nonprofit` / `Employer`).
//! * Income and purchase-price gates are checked against the **RESO**-derived
//!   `ListPrice`/`ClosePrice` and the property's location (county FIPS), so the
//!   `DpaEligibilityInput` is populated directly from RESO `Property` data plus
//!   borrower facts. Property-type eligibility uses RESO `PropertyType`.
//! * `HeroCategory` corresponds to borrower employment classification, used by
//!   "hero"/essential-worker programs (teachers, peace officers, fire, EMS,
//!   healthcare, veterans, etc.).
//!
//! # Provenance
//!
//! Every public method returns a `Derived<T>` (see `types::provenance`): the
//! answer plus the exact catalog record and the ordered chain of rules. Call
//! `.explain()` for the human-readable audit trail.
//!
//! # Data accuracy
//!
//! Program *structure* is authoritative. Specific dollar limits are
//! **point-in-time** and carry an `effective_date` + `source_citation`; HFAs
//! revise county income/price limits at least annually. Update by adding
//! `dpa_catalog_{YYYY}.json`; no Rust changes needed. County limits are seeded
//! for fixture counties plus a state default fallback.

use serde::{Deserialize, Serialize};
use types::{Cents, Derived, Provenance};

/// How the assistance is structured (maps to MISMO down-payment / secondary financing).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DpaAssistanceType {
    /// Never repaid (MISMO `Grant`).
    Grant,
    /// "Silent second": 0% (or low), no monthly payment, repaid on sale/refi/payoff.
    DeferredLoan,
    /// Forgiven over a set number of years; repaid only if the buyer exits early.
    ForgivableLoan,
    /// Repaid in monthly installments alongside the first mortgage.
    AmortizingLoan,
}

/// What the assistance amount is a function of.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DpaAmountBasis {
    PercentOfPurchasePrice,
    PercentOfLoanAmount,
    FixedDollar,
}

/// Source of funds (maps to MISMO `FundsSourceType`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DpaFundsSource {
    StateAgency,
    LocalAgency,
    Nonprofit,
    Employer,
}

/// Jurisdiction administering the program.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JurisdictionLevel {
    State,
    County,
    City,
}

/// Essential-worker / "hero" categories for targeted programs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HeroCategory {
    Teacher,
    TeacherAide,
    Librarian,
    SchoolNurse,
    SchoolCounselor,
    PeaceOfficer,
    Firefighter,
    Ems,
    CorrectionsOfficer,
    CountyJailer,
    Veteran,
    ActiveMilitary,
    Healthcare,
    NursingFaculty,
    PublicSecurityOfficer,
}

/// A county-level income and purchase-price limit entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DpaCountyLimit {
    pub county_fips: String,
    pub county_name: String,
    /// Income limit (cents), 1–2 person household, non-targeted area.
    pub income_1_2_cents: i64,
    /// Income limit (cents), 3+ person household, non-targeted area.
    pub income_3plus_cents: i64,
    /// Income limit (cents), 1–2 person household, targeted area.
    pub income_targeted_1_2_cents: i64,
    /// Income limit (cents), 3+ person household, targeted area.
    pub income_targeted_3plus_cents: i64,
    pub price_limit_cents: i64,
    pub price_limit_targeted_cents: i64,
}

/// One DPA program (one entry of `dpa_catalog_{year}.json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DpaProgram {
    pub program_id: String,
    pub state: String,
    pub administering_agency: String,
    pub program_name: String,
    pub jurisdiction_level: JurisdictionLevel,
    /// Set only for county/city programs; `None` for statewide.
    pub jurisdiction_fips: Option<String>,
    pub funds_source: DpaFundsSource,

    // ── Assistance structure ────────────────────────────────────────────────
    pub assistance_type: DpaAssistanceType,
    pub amount_basis: DpaAmountBasis,
    /// For percent bases: basis points (500 = 5%). For FixedDollar: ignored.
    pub amount_value_bps: u32,
    /// Hard dollar cap in cents. `0` = no cap. For FixedDollar this is the amount.
    pub max_amount_cents: i64,
    /// Interest rate on the assistance in bps (0 for grant / deferred-0%).
    pub interest_rate_bps: u32,
    /// Repayment term in months for amortizing loans; `0` otherwise.
    pub term_months: u16,
    /// Years over which a forgivable loan is forgiven; `0` if not forgivable.
    pub forgivable_years: u8,

    // ── Eligibility ─────────────────────────────────────────────────────────
    pub fthb_required: bool,
    pub veteran_fthb_exempt: bool,
    pub targeted_area_fthb_exempt: bool,
    pub min_credit_score: u16,
    pub max_dti_bps: u32,
    /// True if the buyer must use this agency's first-mortgage program.
    pub requires_agency_first_mortgage: bool,
    pub homebuyer_education_required: bool,
    /// Minimum borrower's-own-funds contribution in bps of price (0 if none).
    pub min_borrower_contribution_bps: u32,
    /// Eligible first-mortgage loan types: any of "fha","va","usda","conventional".
    pub eligible_loan_types: Vec<String>,
    /// If non-empty, the program is restricted to these hero categories.
    pub hero_categories: Vec<HeroCategory>,

    // ── Limits ──────────────────────────────────────────────────────────────
    pub county_limits: Vec<DpaCountyLimit>,
    /// Fallback limits for counties not explicitly listed.
    pub default_limit: DpaCountyLimit,
}

impl DpaProgram {
    /// Resolve the applicable county limit (explicit entry or state default).
    #[must_use]
    pub fn limit_for(&self, county_fips: &str) -> &DpaCountyLimit {
        self.county_limits
            .iter()
            .find(|c| c.county_fips == county_fips)
            .unwrap_or(&self.default_limit)
    }

    /// Income ceiling for a household size + targeted-area status, in cents.
    #[must_use]
    pub fn income_limit(&self, county_fips: &str, household_size: u8, targeted: bool) -> Cents {
        let l = self.limit_for(county_fips);
        let small = household_size <= 2;
        Cents(match (targeted, small) {
            (true, true) => l.income_targeted_1_2_cents,
            (true, false) => l.income_targeted_3plus_cents,
            (false, true) => l.income_1_2_cents,
            (false, false) => l.income_3plus_cents,
        })
    }

    /// Purchase-price ceiling for a county + targeted-area status, in cents.
    #[must_use]
    pub fn price_limit(&self, county_fips: &str, targeted: bool) -> Cents {
        let l = self.limit_for(county_fips);
        Cents(if targeted {
            l.price_limit_targeted_cents
        } else {
            l.price_limit_cents
        })
    }

    fn serves_loan_type(&self, loan_type: &str) -> bool {
        self.eligible_loan_types
            .iter()
            .any(|t| t.eq_ignore_ascii_case(loan_type))
    }
}

/// Borrower / transaction facts (populated from RESO property data + borrower file).
#[derive(Debug, Clone)]
pub struct DpaEligibilityInput {
    pub state: String,
    pub county_fips: String,
    pub is_first_time_homebuyer: bool,
    pub is_veteran: bool,
    pub in_targeted_area: bool,
    pub household_size: u8,
    pub annual_household_income: Cents,
    pub purchase_price: Cents,
    pub loan_type: String,
    pub credit_score: u16,
    pub dti_bps: u32,
    pub borrower_contribution_bps: u32,
    pub using_agency_first_mortgage: bool,
    pub completed_homebuyer_education: bool,
    /// Borrower's hero category, if any.
    pub hero_category: Option<HeroCategory>,
}

/// Eligibility outcome for one program.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DpaOutcome {
    pub eligible: bool,
    pub program_id: String,
    pub disqualifiers: Vec<String>,
}

/// Top-level shape of `dpa_catalog_{year}.json`.
#[derive(Debug, Deserialize)]
pub struct DpaCatalogFile {
    pub effective_date: String,
    pub source_citation: String,
    pub programs: Vec<DpaProgram>,
}

impl DpaCatalogFile {
    fn provenance_for(&self, p: &DpaProgram, file: &str, req: u16, res: u16) -> Provenance {
        Provenance {
            dataset: "dpa_catalog".to_owned(),
            source_file: file.to_owned(),
            source_citation: self.source_citation.clone(),
            effective_date: self.effective_date.clone(),
            record_id: p.program_id.clone(),
            requested_version: req,
            resolved_version: res,
        }
    }

    /// All programs serving a state (statewide + local), each as `Derived`.
    pub fn programs_for_state(
        &self,
        state: &str,
        file: &str,
        req: u16,
        res: u16,
    ) -> Vec<Derived<DpaProgram>> {
        self.programs
            .iter()
            .filter(|p| p.state.eq_ignore_ascii_case(state))
            .map(|p| {
                Derived::new(p.clone(), self.provenance_for(p, file, req, res)).with_step(
                    "lookup_program",
                    format!("state={state}"),
                    format!("matched '{}' ({:?})", p.program_name, p.jurisdiction_level),
                )
            })
            .collect()
    }

    /// One program by id, as `Derived`.
    pub fn program_by_id(
        &self,
        program_id: &str,
        file: &str,
        req: u16,
        res: u16,
    ) -> Option<Derived<DpaProgram>> {
        let p = self.programs.iter().find(|p| p.program_id == program_id)?;
        Some(
            Derived::new(p.clone(), self.provenance_for(p, file, req, res)).with_step(
                "lookup_program_by_id",
                format!("program_id={program_id}"),
                format!("matched '{}'", p.program_name),
            ),
        )
    }

    /// Evaluate eligibility for a specific program, fully traced.
    pub fn evaluate(
        &self,
        program_id: &str,
        input: &DpaEligibilityInput,
        file: &str,
        req: u16,
        res: u16,
    ) -> Option<Derived<DpaOutcome>> {
        let p = self.programs.iter().find(|x| x.program_id == program_id)?;
        let mut dq: Vec<String> = Vec::new();
        let mut d = Derived::new(
            DpaOutcome {
                eligible: false,
                program_id: p.program_id.clone(),
                disqualifiers: Vec::new(),
            },
            self.provenance_for(p, file, req, res),
        );
        d.push_step(
            "lookup_program_by_id",
            format!("program_id={program_id}"),
            format!("matched '{}'", p.program_name),
        );

        // Loan type
        if !p.serves_loan_type(&input.loan_type) {
            dq.push(format!("loan type '{}' not eligible", input.loan_type));
            d.push_step(
                "eligible_loan_type",
                format!("loan_type={}", input.loan_type),
                format!("FAILED — program serves {:?}", p.eligible_loan_types),
            );
        } else {
            d.push_step(
                "eligible_loan_type",
                format!("loan_type={}", input.loan_type),
                "satisfied".to_owned(),
            );
        }

        // Hero category (only if program restricts)
        if !p.hero_categories.is_empty() {
            let ok = input
                .hero_category
                .is_some_and(|h| p.hero_categories.contains(&h));
            if ok {
                d.push_step(
                    "hero_category",
                    format!("borrower={:?}", input.hero_category),
                    "satisfied".to_owned(),
                );
            } else {
                dq.push("borrower is not in an eligible hero/essential-worker category".to_owned());
                d.push_step(
                    "hero_category",
                    format!("borrower={:?}", input.hero_category),
                    format!("FAILED — requires one of {:?}", p.hero_categories),
                );
            }
        }

        // FTHB with exemptions
        if p.fthb_required && !input.is_first_time_homebuyer {
            let vet = p.veteran_fthb_exempt && input.is_veteran;
            let tgt = p.targeted_area_fthb_exempt && input.in_targeted_area;
            if vet {
                d.push_step(
                    "first_time_homebuyer",
                    "is_fthb=false, veteran=true",
                    "satisfied via veteran exemption".to_owned(),
                );
            } else if tgt {
                d.push_step(
                    "first_time_homebuyer",
                    "is_fthb=false, targeted=true",
                    "satisfied via targeted-area exemption".to_owned(),
                );
            } else {
                dq.push("not a first-time homebuyer; no exemption applies".to_owned());
                d.push_step(
                    "first_time_homebuyer",
                    format!(
                        "is_fthb=false, veteran={}, targeted={}",
                        input.is_veteran, input.in_targeted_area
                    ),
                    "FAILED — no exemption".to_owned(),
                );
            }
        } else {
            d.push_step(
                "first_time_homebuyer",
                format!(
                    "fthb_required={}, is_fthb={}",
                    p.fthb_required, input.is_first_time_homebuyer
                ),
                "satisfied".to_owned(),
            );
        }

        // Income
        let inc_limit = p.income_limit(
            &input.county_fips,
            input.household_size,
            input.in_targeted_area,
        );
        if input.annual_household_income > inc_limit {
            dq.push(format!(
                "income ${} exceeds limit ${}",
                input.annual_household_income.0 / 100,
                inc_limit.0 / 100
            ));
            d.push_step(
                "income_limit",
                format!(
                    "income=${}, size={}, targeted={}",
                    input.annual_household_income.0 / 100,
                    input.household_size,
                    input.in_targeted_area
                ),
                format!("FAILED — limit ${}", inc_limit.0 / 100),
            );
        } else {
            d.push_step(
                "income_limit",
                format!(
                    "income=${}, size={}, targeted={}",
                    input.annual_household_income.0 / 100,
                    input.household_size,
                    input.in_targeted_area
                ),
                format!("satisfied (limit ${})", inc_limit.0 / 100),
            );
        }

        // Price (a limit of $0 means "no purchase-price cap at this layer")
        let price_limit = p.price_limit(&input.county_fips, input.in_targeted_area);
        if price_limit.0 > 0 && input.purchase_price > price_limit {
            dq.push(format!(
                "price ${} exceeds limit ${}",
                input.purchase_price.0 / 100,
                price_limit.0 / 100
            ));
            d.push_step(
                "purchase_price_limit",
                format!(
                    "price=${}, targeted={}",
                    input.purchase_price.0 / 100,
                    input.in_targeted_area
                ),
                format!("FAILED — limit ${}", price_limit.0 / 100),
            );
        } else {
            let msg = if price_limit.0 == 0 {
                "satisfied (no price limit for this program)".to_owned()
            } else {
                format!("satisfied (limit ${})", price_limit.0 / 100)
            };
            d.push_step(
                "purchase_price_limit",
                format!(
                    "price=${}, targeted={}",
                    input.purchase_price.0 / 100,
                    input.in_targeted_area
                ),
                msg,
            );
        }

        // Credit
        if input.credit_score < p.min_credit_score {
            dq.push(format!(
                "credit {} below minimum {}",
                input.credit_score, p.min_credit_score
            ));
            d.push_step(
                "min_credit_score",
                format!("score={}", input.credit_score),
                format!("FAILED — minimum {}", p.min_credit_score),
            );
        } else {
            d.push_step(
                "min_credit_score",
                format!("score={}", input.credit_score),
                format!("satisfied (min {})", p.min_credit_score),
            );
        }

        // DTI
        if p.max_dti_bps > 0 && input.dti_bps > p.max_dti_bps {
            dq.push(format!(
                "DTI {}% exceeds max {}%",
                input.dti_bps / 100,
                p.max_dti_bps / 100
            ));
            d.push_step(
                "max_dti",
                format!("dti={}%", input.dti_bps / 100),
                format!("FAILED — max {}%", p.max_dti_bps / 100),
            );
        } else {
            d.push_step(
                "max_dti",
                format!("dti={}%", input.dti_bps / 100),
                "satisfied".to_owned(),
            );
        }

        // Agency first mortgage
        if p.requires_agency_first_mortgage && !input.using_agency_first_mortgage {
            dq.push("program requires the agency's first-mortgage product".to_owned());
            d.push_step(
                "requires_agency_first_mortgage",
                "using_agency_first=false",
                "FAILED".to_owned(),
            );
        }

        // Homebuyer education
        if p.homebuyer_education_required && !input.completed_homebuyer_education {
            dq.push("homebuyer education not completed".to_owned());
            d.push_step(
                "homebuyer_education",
                "completed=false",
                "FAILED".to_owned(),
            );
        }

        // Borrower contribution
        if p.min_borrower_contribution_bps > 0
            && input.borrower_contribution_bps < p.min_borrower_contribution_bps
        {
            dq.push(format!(
                "borrower contribution {}bps below required {}bps",
                input.borrower_contribution_bps, p.min_borrower_contribution_bps
            ));
            d.push_step(
                "min_borrower_contribution",
                format!("contribution={}bps", input.borrower_contribution_bps),
                format!("FAILED — min {}bps", p.min_borrower_contribution_bps),
            );
        }

        let eligible = dq.is_empty();
        d.push_step(
            "final_determination",
            format!("{} gate(s) failed", dq.len()),
            if eligible {
                "ELIGIBLE".to_owned()
            } else {
                "INELIGIBLE".to_owned()
            },
        );
        d.value.eligible = eligible;
        d.value.disqualifiers = dq;
        Some(d)
    }
}

/// Estimate the DPA dollar amount for a program given price and loan amount,
/// returning a fully-traced `Derived<Cents>`.
#[must_use]
pub fn estimate_dpa_amount(
    program: Derived<DpaProgram>,
    purchase_price: Cents,
    loan_amount: Cents,
) -> Derived<Cents> {
    let p = &program.value;
    let (raw, basis_desc): (i128, String) = match p.amount_basis {
        DpaAmountBasis::FixedDollar => (i128::from(p.max_amount_cents), "fixed dollar".to_owned()),
        DpaAmountBasis::PercentOfPurchasePrice => (
            i128::from(purchase_price.0) * i128::from(p.amount_value_bps) / 10_000,
            format!(
                "{}% of price ${}",
                p.amount_value_bps / 100,
                purchase_price.0 / 100
            ),
        ),
        DpaAmountBasis::PercentOfLoanAmount => (
            i128::from(loan_amount.0) * i128::from(p.amount_value_bps) / 10_000,
            format!(
                "{}% of loan ${}",
                p.amount_value_bps / 100,
                loan_amount.0 / 100
            ),
        ),
    };
    let cap = p.max_amount_cents;
    let (amount, capped) =
        if p.amount_basis != DpaAmountBasis::FixedDollar && cap > 0 && raw > i128::from(cap) {
            (cap, true)
        } else {
            (raw as i64, false)
        };
    program
        .map(|_| Cents(amount))
        .with_step(
            "compute_base_amount",
            basis_desc,
            format!("raw=${}", (raw / 100) as i64),
        )
        .with_step(
            "apply_cap",
            format!(
                "cap=${}",
                if cap > 0 {
                    (cap / 100).to_string()
                } else {
                    "none".to_owned()
                }
            ),
            if capped {
                format!("capped to ${}", amount / 100)
            } else {
                format!("amount=${}", amount / 100)
            },
        )
}
