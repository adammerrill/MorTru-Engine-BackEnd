//! Produces the scenario universe the funnel counts and the solver scores.
//! The critical task (T11.4) is **month-granular term expansion**: every term
//! from 96–360 within each program's band, not just the band boundaries —
//! because horizon-cost optimization can land on any single month.
//!
//! ## Tasks
//! - **T11.1** `Scenario` — one enumerated unit (program, term, balance, tier, MI).
//! - **T11.2** `EnumerationAxes` — the input ranges to expand over.
//! - **T11.3** `enumerate(axes)` — the cartesian product as a lazy iterator.
//! - **T11.4** month-granular term expansion (every month in band).
//!
//! Term→band routing is program-specific (`band_for_conv`/`govt`/`usda`), and
//! `LoanProduct` variants already encode (program, band), so a (program, term)
//! pair maps to exactly one `LoanProduct`.

pub mod pruning;
pub use pruning::*;
use types::{BalanceType, LoanProduct, ProgramCode, TermBand, TermMonths, Tier};

/// One enumerated scenario: the unit eligibility judges, the solver prices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Scenario {
    pub program: ProgramCode,
    pub product: LoanProduct,
    pub term: TermMonths,
    pub balance_type: BalanceType,
    pub tier: Tier,
    /// MI option index (0 = none/NA; 1–15 plan variants), matching `ScenarioKey`.
    pub mi_option: u8,
}

/// Map a (program, term) pair to the `LoanProduct` variant whose band contains
/// the term. `None` if the term is out of range for the program.
#[must_use]
pub fn product_for(program: ProgramCode, term: TermMonths) -> Option<LoanProduct> {
    if matches!(program, ProgramCode::Usda) {
        return term.band_for_usda().map(|_| LoanProduct::FixedUsda30);
    }
    if program.is_government() {
        return match term.band_for_govt()? {
            TermBand::GovtBand8To15 => Some(match program {
                ProgramCode::Va | ProgramCode::VaJumbo => LoanProduct::FixedVa8To15,
                _ => LoanProduct::FixedFha8To15,
            }),
            TermBand::GovtBand16To30 => Some(match program {
                ProgramCode::Va | ProgramCode::VaJumbo => LoanProduct::FixedVa16To30,
                _ => LoanProduct::FixedFha16To30,
            }),
            _ => None,
        };
    }
    // Conventional + affordable conventional products use the conv bands.
    match term.band_for_conv()? {
        TermBand::Band8To10 => Some(LoanProduct::FixedConv8To10),
        TermBand::Band11To15 => Some(LoanProduct::FixedConv11To15),
        TermBand::Band16To20 => Some(LoanProduct::FixedConv16To20),
        TermBand::Band21To30 => Some(LoanProduct::FixedConv21To30),
        _ => None,
    }
}

/// The axes of the cartesian enumeration. Each is a set the enumerator expands
/// over. Term ranges are expanded month-by-month (T11.4) within `term_bands`.
#[derive(Debug, Clone)]
pub struct EnumerationAxes {
    pub programs: Vec<ProgramCode>,
    /// Bands to expand month-granularly. Defaults to all bands valid for each
    /// program when constructed via `for_programs`.
    pub term_bands: Vec<TermBand>,
    pub balance_types: Vec<BalanceType>,
    pub tiers: Vec<Tier>,
    /// MI options to consider (0 = none). Empty → `[0]`.
    pub mi_options: Vec<u8>,
}

impl EnumerationAxes {
    /// Build axes spanning the natural term bands for each program, with a
    /// single conforming/standard/no-MI baseline. Callers widen as needed.
    #[must_use]
    pub fn for_programs(programs: Vec<ProgramCode>) -> Self {
        // Collect the distinct bands these programs use.
        let mut bands: Vec<TermBand> = Vec::new();
        for &p in &programs {
            let probe = if matches!(p, ProgramCode::Usda) {
                vec![TermBand::Usda30Only]
            } else if p.is_government() {
                vec![TermBand::GovtBand8To15, TermBand::GovtBand16To30]
            } else {
                vec![
                    TermBand::Band8To10,
                    TermBand::Band11To15,
                    TermBand::Band16To20,
                    TermBand::Band21To30,
                ]
            };
            for b in probe {
                if !bands.contains(&b) {
                    bands.push(b);
                }
            }
        }
        EnumerationAxes {
            programs,
            term_bands: bands,
            balance_types: vec![BalanceType::Conforming],
            tiers: vec![Tier::Standard],
            mi_options: vec![0],
        }
    }

    /// Total scenario count without materializing — month-granular.
    /// Each (program, band) contributes `month_count` terms, but only where the
    /// band is valid for that program.
    #[must_use]
    pub fn count(&self) -> u64 {
        self.enumerate().count() as u64
    }

    /// T11.3 + T11.4 — lazy cartesian product with month-granular term expansion.
    /// Each program expands over only the bands valid for it (intersected with
    /// the configured `term_bands`), so unioned multi-program axes don't double
    /// count a shared term under another program's band.
    pub fn enumerate(&self) -> impl Iterator<Item = Scenario> + '_ {
        let mi_opts = if self.mi_options.is_empty() {
            &[0u8][..]
        } else {
            &self.mi_options[..]
        };
        self.programs.iter().flat_map(move |&program| {
            self.term_bands
                .iter()
                .copied()
                .filter(move |&band| Self::band_valid_for(program, band))
                .flat_map(move |band| {
                    band.all_months()
                        .filter_map(move |term| {
                            product_for(program, term).map(|product| (program, term, product))
                        })
                        .flat_map(move |(program, term, product)| {
                            self.balance_types.iter().flat_map(move |&balance_type| {
                                self.tiers.iter().flat_map(move |&tier| {
                                    mi_opts.iter().map(move |&mi_option| Scenario {
                                        program,
                                        product,
                                        term,
                                        balance_type,
                                        tier,
                                        mi_option,
                                    })
                                })
                            })
                        })
                })
        })
    }

    /// Is a term band one this program actually uses? Prevents a program from
    /// expanding over another program's band when axes are unioned.
    fn band_valid_for(program: ProgramCode, band: TermBand) -> bool {
        if matches!(program, ProgramCode::Usda) {
            return matches!(band, TermBand::Usda30Only);
        }
        if program.is_government() {
            return matches!(band, TermBand::GovtBand8To15 | TermBand::GovtBand16To30);
        }
        matches!(
            band,
            TermBand::Band8To10
                | TermBand::Band11To15
                | TermBand::Band16To20
                | TermBand::Band21To30
        )
    }
}

/// Convenience: enumerate every scenario for a single program over its natural
/// bands (conforming/standard/no-MI baseline).
#[must_use]
pub fn enumerate_program(program: ProgramCode) -> Vec<Scenario> {
    EnumerationAxes::for_programs(vec![program])
        .enumerate()
        .collect()
}
