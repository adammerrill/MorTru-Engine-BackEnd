//! `GoalMask` — bitflags encoding optimization objectives for an analysis run.
//!
//! # Concept
//!
//! An analysis produces hundreds of thousands of scored loan scenarios. The
//! engine does not collapse them to a single "best" loan — instead it ranks
//! every scenario against every **enabled goal** independently and returns
//! the Pareto-optimal frontier: the set of scenarios where no other scenario
//! is better on every dimension simultaneously.
//!
//! A `GoalMask` is a `u64` bitmask where each bit enables one objective.
//! The operator configures which goals are active at request time; the
//! borrower gets a ranked result for each active goal.
//!
//! # Quick start
//!
//! ```rust
//! use types::GoalMask;
//!
//! // Owner-occupant purchase — use the built-in consumer default
//! let consumer = GoalMask::DEFAULT_CONSUMER;
//! assert_eq!(consumer.active_count(), 5);
//!
//! // Real-estate investor — use the built-in investor default
//! let investor = GoalMask::DEFAULT_INVESTOR;
//! assert!(investor.is_investor_mode());
//!
//! // Custom: consumer who also wants assumability for resale leverage
//! let custom = GoalMask::DEFAULT_CONSUMER.enable(GoalMask::MAXIMUM_ASSUMABILITY);
//! assert_eq!(custom.active_count(), 6);
//!
//! // Disable a goal at runtime
//! let without_apr = consumer.disable(GoalMask::LOWEST_APR);
//! assert!(!without_apr.contains(GoalMask::LOWEST_APR));
//! ```
//!
//! # Goal categories
//!
//! Goals are organised into seven categories; each bit position is permanent
//! and must never be re-used after assignment.
//!
//! | Bits  | Category                        | Personas         |
//! |-------|---------------------------------|------------------|
//! | 0–2   | Cost optimisation               | Consumer         |
//! | 3–6   | Payment optimisation            | Consumer/Investor|
//! | 7–10  | Cash & debt optimisation        | Consumer         |
//! | 11–12 | Rate / APR optimisation         | Consumer         |
//! | 13–15 | Compound goals                  | Consumer         |
//! | 16–22 | Equity, tax & MI optimisation   | Consumer         |
//! | 23–29 | Yield, leverage & velocity      | Investor         |
//! | 30–33 | Liability & exit                | Shared           |
//!
//! # Adding a new goal
//!
//! 1. Pick the next unassigned bit (currently 34 through 63 are free).
//! 2. Add the `const` inside `bitflags! { }` with a clear doc comment.
//! 3. Add an entry to `GOAL_TABLE` with name, description, and persona.
//! 4. If it belongs in a default mask, update `DEFAULT_CONSUMER` or
//!    `DEFAULT_INVESTOR` accordingly.
//! 5. Implement the scoring function in the `solver` crate's `GoalScorer`.
//! 6. Add a test in `tests/goal_mask.rs`.
//!
//! Never re-use or renumber a bit that has already been released — the
//! integer representation is persisted in database records and API payloads.
//!
//! # Pareto frontier integration
//!
//! The Pareto frontier algorithm (Task 14.9) receives a `(SolvedScenario,
//! GoalScores)` slice and an `enabled_goals: GoalMask`. For each enabled
//! goal it sorts scenarios by that goal's score and eliminates dominated
//! ones. The final frontier contains all scenarios that are non-dominated
//! across the full active goal set. A scenario must be better on AT LEAST
//! ONE goal (and worse-or-equal on none) to appear on the frontier.
//!
//! # Serialisation contract
//!
//! `GoalMask` serialises as its raw `u64` integer (via `#[serde(transparent)]`).
//! Persisted values must remain stable — never change the bit assignment of
//! any goal that has been serialised to a database or transmitted to a client.

use bitflags::bitflags;
use serde::{Deserialize, Serialize};

// ─────────────────────────────────────────────────────────────────────────────
// GoalPersona
// ─────────────────────────────────────────────────────────────────────────────

/// Which borrower persona a goal primarily serves.
///
/// Goals marked [`GoalPersona::Shared`] are valid for both consumers and
/// investors, but their ranking weight differs: an investor who enables
/// [`GoalMask::ZERO_PREPAYMENT_PENALTY`] cares about exit flexibility;
/// a consumer enabling the same goal cares about refinancing optionality.
/// The bit is the same; the downstream scoring context differs.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum GoalPersona {
    /// Goals primarily relevant to owner-occupant consumers.
    Consumer,
    /// Goals primarily relevant to real-estate investors.
    Investor,
    /// Goals meaningful for both personas (liability / exit goals).
    Shared,
}

// ─────────────────────────────────────────────────────────────────────────────
// Metadata table
// ─────────────────────────────────────────────────────────────────────────────

struct GoalInfo {
    bits: u64,
    /// Short display name for UI labels and log output.
    short_name: &'static str,
    /// One-sentence description for developer docs and tooltips.
    description: &'static str,
    persona: GoalPersona,
}

/// Static metadata for every assigned goal bit.
/// The array order mirrors the bit order; do not re-sort.
static GOAL_TABLE: &[GoalInfo] = &[
    // ── Cost (Consumer, bits 0–2) ─────────────────────────────────────────
    GoalInfo {
        bits: 1 << 0,
        short_name: "Lowest Horizon Cost",
        description: "Minimise total interest + points paid within the borrower's stated hold horizon.",
        persona: GoalPersona::Consumer,
    },
    GoalInfo {
        bits: 1 << 1,
        short_name: "Lowest Lifetime Cost",
        description: "Minimise total interest paid over the full amortisation schedule.",
        persona: GoalPersona::Consumer,
    },
    GoalInfo {
        bits: 1 << 2,
        short_name: "Fastest Break-Even",
        description: "Minimise months until the savings from a lower rate recover the cost of buying it down.",
        persona: GoalPersona::Consumer,
    },
    // ── Payment (Consumer/Investor, bits 3–6) ─────────────────────────────
    GoalInfo {
        bits: 1 << 3,
        short_name: "Lowest Payment",
        description: "Minimise the monthly principal-and-interest payment.",
        persona: GoalPersona::Consumer,
    },
    GoalInfo {
        bits: 1 << 4,
        short_name: "Lowest Payment at Max Term",
        description: "Minimise payment by selecting the longest term the program allows.",
        persona: GoalPersona::Consumer,
    },
    GoalInfo {
        bits: 1 << 5,
        short_name: "Highest Payment Stability",
        description: "Prefer fixed-rate over ARM; score ARMs lower based on initial/periodic/lifetime caps.",
        persona: GoalPersona::Consumer,
    },
    GoalInfo {
        bits: 1 << 6,
        short_name: "Minimise ARM Payment Shock",
        description: "For ARMs: minimise the worst-case initial adjustment cap, periodic cap, and lifetime ceiling.",
        persona: GoalPersona::Consumer,
    },
    // ── Cash & Debt (Consumer, bits 7–10) ────────────────────────────────
    GoalInfo {
        bits: 1 << 7,
        short_name: "Lowest Cash to Close",
        description: "Minimise total upfront cash required at closing (down payment + all fees).",
        persona: GoalPersona::Consumer,
    },
    GoalInfo {
        bits: 1 << 8,
        short_name: "Lowest Lender Fees",
        description: "Minimise origination and underwriting fees, excluding rate buy-down points.",
        persona: GoalPersona::Consumer,
    },
    GoalInfo {
        bits: 1 << 9,
        short_name: "Maximum Purchasing Power",
        description: "Maximise the purchase price achievable at the borrower's stated monthly payment ceiling.",
        persona: GoalPersona::Consumer,
    },
    GoalInfo {
        bits: 1 << 10,
        short_name: "Lowest Blended Debt Rate",
        description: "Minimise total monthly outlay across mortgage and high-yield revolving debt retired via cash-out proceeds.",
        persona: GoalPersona::Consumer,
    },
    // ── Rate / APR (Consumer, bits 11–12) ────────────────────────────────
    GoalInfo {
        bits: 1 << 11,
        short_name: "Lowest Rate",
        description: "Minimise the stated note rate.",
        persona: GoalPersona::Consumer,
    },
    GoalInfo {
        bits: 1 << 12,
        short_name: "Lowest APR",
        description: "Minimise the Annual Percentage Rate as defined by Reg Z § 1026.22.",
        persona: GoalPersona::Consumer,
    },
    // ── Compound (Consumer, bits 13–15) ──────────────────────────────────
    GoalInfo {
        bits: 1 << 13,
        short_name: "Lowest Horizon at Target Payment",
        description: "Achieve the borrower's target monthly payment at the shortest time horizon.",
        persona: GoalPersona::Consumer,
    },
    GoalInfo {
        bits: 1 << 14,
        short_name: "Lowest CTC at Target Payment",
        description: "Achieve the borrower's target payment at the lowest cash-to-close.",
        persona: GoalPersona::Consumer,
    },
    GoalInfo {
        bits: 1 << 15,
        short_name: "Lowest Horizon at Target CTC",
        description: "Achieve the borrower's target cash-to-close at the shortest time horizon.",
        persona: GoalPersona::Consumer,
    },
    // ── Equity / Tax / MI (Consumer, bits 16–22) ─────────────────────────
    GoalInfo {
        bits: 1 << 16,
        short_name: "Max Equity at Horizon",
        description: "Maximise remaining principal paid down at the borrower's hold horizon.",
        persona: GoalPersona::Consumer,
    },
    GoalInfo {
        bits: 1 << 17,
        short_name: "Max Principal at Horizon",
        description: "Maximise total cumulative principal reduction at the hold horizon.",
        persona: GoalPersona::Consumer,
    },
    GoalInfo {
        bits: 1 << 18,
        short_name: "Fastest 80% LTV",
        description: "Minimise months to reach 80% LTV and trigger automatic PMI cancellation.",
        persona: GoalPersona::Consumer,
    },
    GoalInfo {
        bits: 1 << 19,
        short_name: "Fastest MI Cancel",
        description: "Minimise months until MI can be cancelled by statute (HPA) or contract.",
        persona: GoalPersona::Consumer,
    },
    GoalInfo {
        bits: 1 << 20,
        short_name: "Max Mortgage Interest Deduction",
        description: "Maximise deductible mortgage interest (Schedule A) relative to the standard deduction threshold for itemising borrowers.",
        persona: GoalPersona::Consumer,
    },
    GoalInfo {
        bits: 1 << 21,
        short_name: "Lowest MI Cost",
        description: "Minimise total MI premiums paid over the hold horizon.",
        persona: GoalPersona::Consumer,
    },
    GoalInfo {
        bits: 1 << 22,
        short_name: "Lowest Upfront MI",
        description: "Minimise the upfront MI premium or VA/USDA funding fee at closing.",
        persona: GoalPersona::Consumer,
    },
    // ── Investor: Yield & Leverage (bits 23–27) ──────────────────────────
    GoalInfo {
        bits: 1 << 23,
        short_name: "Highest Cash-on-Cash Return",
        description: "Maximise annual cash-on-cash return (NOI ÷ equity deployed).",
        persona: GoalPersona::Investor,
    },
    GoalInfo {
        bits: 1 << 24,
        short_name: "Highest Monthly Cash Flow",
        description: "Maximise gross monthly cash flow (rental income minus full PITIA).",
        persona: GoalPersona::Investor,
    },
    GoalInfo {
        bits: 1 << 25,
        short_name: "Maximum Leverage at Target DSCR",
        description: "Maximise loan proceeds subject to a minimum DSCR constraint, preserving debt-service coverage while extracting maximum capital.",
        persona: GoalPersona::Investor,
    },
    GoalInfo {
        bits: 1 << 26,
        short_name: "Highest IRR at Horizon",
        description: "Maximise Internal Rate of Return on the equity position at the stated hold horizon, incorporating appreciation and amortisation.",
        persona: GoalPersona::Investor,
    },
    GoalInfo {
        bits: 1 << 27,
        short_name: "Lowest Holding Expense",
        description: "Minimise total holding expense (debt service + reserves + MI) over the hold period.",
        persona: GoalPersona::Investor,
    },
    // ── Investor: Velocity & Liquidity (bits 28–29) ──────────────────────
    GoalInfo {
        bits: 1 << 28,
        short_name: "Minimise Title Seasoning",
        description: "Minimise title seasoning requirements to enable a cash-out refinance immediately after forced appreciation (BRRRR strategy).",
        persona: GoalPersona::Investor,
    },
    GoalInfo {
        bits: 1 << 29,
        short_name: "Lowest Reserve Requirement",
        description: "Minimise mandatory post-closing liquid reserve overlays to maximise deployable capital.",
        persona: GoalPersona::Investor,
    },
    // ── Shared: Liability & Exit (bits 30–33) ────────────────────────────
    GoalInfo {
        bits: 1 << 30,
        short_name: "Zero Prepayment Penalty",
        description: "Require zero prepayment penalty — preserve full refinancing and early-payoff optionality.",
        persona: GoalPersona::Shared,
    },
    GoalInfo {
        bits: 1 << 31,
        short_name: "Minimise Prepayment Penalty",
        description: "Minimise prepayment penalty structure (yield maintenance, defeasance, or step-down) to allow unpenalised exit.",
        persona: GoalPersona::Shared,
    },
    GoalInfo {
        bits: 1 << 32,
        short_name: "Maximum Assumability",
        description: "Prefer assumable FHA/VA debt to transfer a below-market rate to a future buyer, commanding a price premium on exit.",
        persona: GoalPersona::Shared,
    },
    GoalInfo {
        bits: 1 << 33,
        short_name: "Non-Recourse Liability",
        description: "Prefer non-recourse DSCR lending that isolates liability to the asset and bypasses personal DTI analysis.",
        persona: GoalPersona::Shared,
    },
];

// ─────────────────────────────────────────────────────────────────────────────
// Bitmask constants for consumer persona goals
// ─────────────────────────────────────────────────────────────────────────────

const CONSUMER_BITS: u64 = (1 << 0)  // LOWEST_HORIZON_COST
    | (1 << 1)   // LOWEST_LIFETIME_COST
    | (1 << 2)   // FASTEST_BREAK_EVEN
    | (1 << 3)   // LOWEST_PAYMENT
    | (1 << 4)   // LOWEST_PAYMENT_AT_MAX_TERM
    | (1 << 5)   // HIGHEST_PAYMENT_STABILITY
    | (1 << 6)   // MINIMIZE_ARM_PAYMENT_SHOCK
    | (1 << 7)   // LOWEST_CASH_TO_CLOSE
    | (1 << 8)   // LOWEST_LENDER_FEES
    | (1 << 9)   // MAXIMUM_PURCHASING_POWER
    | (1 << 10)  // LOWEST_BLENDED_TOTAL_DEBT_RATE
    | (1 << 11)  // LOWEST_RATE
    | (1 << 12)  // LOWEST_APR
    | (1 << 13)  // LOWEST_HORIZON_AT_TARGET_PAYMENT
    | (1 << 14)  // LOWEST_CTC_AT_TARGET_PAYMENT
    | (1 << 15)  // LOWEST_HORIZON_AT_TARGET_CTC
    | (1 << 16)  // MAX_EQUITY_AT_HORIZON
    | (1 << 17)  // MAX_PRINCIPAL_AT_HORIZON
    | (1 << 18)  // FASTEST_EIGHTY_LTV
    | (1 << 19)  // FASTEST_MI_CANCEL
    | (1 << 20)  // MAX_MORTGAGE_INTEREST_DEDUCTION
    | (1 << 21)  // LOWEST_MI_COST
    | (1 << 22); // LOWEST_UPFRONT_MI

const INVESTOR_BITS: u64 = (1 << 23)  // HIGHEST_CASH_ON_CASH_RETURN
    | (1 << 24)  // HIGHEST_MONTHLY_CASH_FLOW
    | (1 << 25)  // MAXIMUM_LEVERAGE_AT_TARGET_DSCR
    | (1 << 26)  // HIGHEST_IRR_AT_HORIZON
    | (1 << 27)  // LOWEST_HOLDING_EXPENSE
    | (1 << 28)  // MINIMIZE_TITLE_SEASONING
    | (1 << 29); // LOWEST_RESERVE_REQUIREMENT

const SHARED_BITS: u64 = (1 << 30)  // ZERO_PREPAYMENT_PENALTY
    | (1 << 31)  // MINIMIZE_PREPAYMENT_PENALTY
    | (1 << 32)  // MAXIMUM_ASSUMABILITY
    | (1 << 33); // NON_RECOURSE_LIABILITY

// ─────────────────────────────────────────────────────────────────────────────
// GoalMask bitflags definition
// ─────────────────────────────────────────────────────────────────────────────

bitflags! {
    /// Optimization goal bitfield for a loan analysis request.
    ///
    /// OR together the goals you want active. Use the composite constants
    /// [`Self::DEFAULT_CONSUMER`] or [`Self::DEFAULT_INVESTOR`] as starting
    /// points, then enable or disable individual goals with
    /// [`Self::enable`] / [`Self::disable`].
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(transparent)]
    pub struct GoalMask: u64 {
        // ── Cost optimisation (Consumer, bits 0–2) ────────────────────────
        /// Minimise total cost (interest + points) within the borrower's hold
        /// horizon. The engine computes the present value of all cash outflows
        /// over `horizon_months` for each scenario and ranks ascending.
        const LOWEST_HORIZON_COST             = 1 << 0;
        /// Minimise total interest over the full amortisation schedule.
        /// Useful when the borrower intends to hold to payoff or for
        /// comparing fixed-rate terms (15-year vs 30-year trade-off).
        const LOWEST_LIFETIME_COST            = 1 << 1;
        /// Minimise the months until the cost of buying the rate down (discount
        /// points) is recovered through the lower payment. Favoured by
        /// borrowers who know they will hold past the break-even.
        const FASTEST_BREAK_EVEN              = 1 << 2;

        // ── Payment optimisation (Consumer/Investor, bits 3–6) ────────────
        /// Minimise the monthly principal-and-interest payment. The most
        /// commonly requested consumer goal; enables PITI qualification for
        /// debt-to-income purposes.
        const LOWEST_PAYMENT                  = 1 << 3;
        /// Minimise payment by selecting the longest eligible term in the
        /// program matrix. This may sacrifice equity build-up for cash-flow
        /// flexibility.
        const LOWEST_PAYMENT_AT_MAX_TERM      = 1 << 4;
        /// Maximise payment certainty. Fixed-rate products score perfectly;
        /// ARMs are penalised by cap severity. Useful for risk-averse
        /// borrowers who want to compare fixed vs adjustable on stability
        /// grounds.
        const HIGHEST_PAYMENT_STABILITY       = 1 << 5;
        /// For ARM products only: minimise the worst-case payment shock
        /// from initial, periodic, and lifetime caps. Relevant when a
        /// borrower is comparing `5/6 SOFR` vs `7/6 SOFR` and needs to see
        /// the worst-case payment at each cap tier.
        const MINIMIZE_ARM_PAYMENT_SHOCK      = 1 << 6;

        // ── Cash & Debt optimisation (Consumer, bits 7–10) ────────────────
        /// Minimise total upfront cash at closing (down payment + origination
        /// fee + prepaid items + escrow setup + discount points). The
        /// canonical goal for cash-constrained first-time buyers.
        const LOWEST_CASH_TO_CLOSE            = 1 << 7;
        /// Minimise origination fees and underwriting charges, excluding
        /// discount points. Useful when comparing lenders on fee structures
        /// rather than rate.
        const LOWEST_LENDER_FEES              = 1 << 8;
        /// Given a target monthly payment, maximise the purchase price the
        /// borrower can afford. Used in "reverse" affordability mode.
        const MAXIMUM_PURCHASING_POWER        = 1 << 9;
        /// In cash-out scenarios, minimise the borrower's total blended
        /// monthly debt outlay across the mortgage AND all revolving
        /// accounts retired with the cash-out proceeds.
        const LOWEST_BLENDED_TOTAL_DEBT_RATE  = 1 << 10;

        // ── Rate / APR optimisation (Consumer, bits 11–12) ────────────────
        /// Minimise the note rate. Simple and widely understood; appropriate
        /// when the borrower will compare with competitor quotes.
        const LOWEST_RATE                     = 1 << 11;
        /// Minimise the APR (Reg Z § 1026.22). Better than rate alone because
        /// it captures fees. Essential for TRID compliance comparisons.
        const LOWEST_APR                      = 1 << 12;

        // ── Compound goals (Consumer, bits 13–15) ─────────────────────────
        /// With a payment ceiling constraint, find the scenario that reaches
        /// break-even soonest. Requires `target_payment_max` in the request.
        const LOWEST_HORIZON_AT_TARGET_PAYMENT = 1 << 13;
        /// With a payment ceiling, minimise the upfront cash cost.
        /// Requires `target_payment_max` in the request.
        const LOWEST_CTC_AT_TARGET_PAYMENT    = 1 << 14;
        /// With a cash-to-close ceiling, find the shortest horizon.
        /// Requires `cash_available` in the request.
        const LOWEST_HORIZON_AT_TARGET_CTC    = 1 << 15;

        // ── Equity / Tax / MI (Consumer, bits 16–22) ──────────────────────
        /// Maximise the principal paid down by the hold horizon. Relevant
        /// for borrowers who plan to use equity for a future purchase.
        const MAX_EQUITY_AT_HORIZON           = 1 << 16;
        /// Maximise cumulative principal reduction. Similar to
        /// `MAX_EQUITY_AT_HORIZON` but counts total principal paid rather
        /// than remaining balance.
        const MAX_PRINCIPAL_AT_HORIZON        = 1 << 17;
        /// Minimise months to 80% LTV. Triggers Homeowners Protection Act
        /// (HPA) automatic PMI cancellation. Valuable when the borrower's
        /// down payment is between 5-19%.
        const FASTEST_EIGHTY_LTV              = 1 << 18;
        /// Minimise months to PMI/MIP cancellation by any means — either
        /// by reaching 80% LTV (conventional) or the mandatory 11-year FHA
        /// MIP termination.
        const FASTEST_MI_CANCEL               = 1 << 19;
        /// For itemising borrowers: maximise the mortgage interest deduction
        /// (MID) relative to the applicable standard deduction threshold.
        /// Only relevant when `Schedule A > standard deduction`.
        const MAX_MORTGAGE_INTEREST_DEDUCTION = 1 << 20;
        /// Minimise total MI premium over the hold horizon (upfront MIP +
        /// monthly MIP, or PMI, or VA/USDA fees).
        const LOWEST_MI_COST                  = 1 << 21;
        /// Minimise the upfront MI component only (FHA UFMIP, VA funding
        /// fee, USDA guarantee fee). Relevant for cash-constrained borrowers
        /// who can handle the monthly MI but not the upfront hit.
        const LOWEST_UPFRONT_MI               = 1 << 22;

        // ── Investor: Yield & Leverage (bits 23–27) ───────────────────────
        /// Maximise annual cash-on-cash return: `NOI / equity_deployed`.
        /// The primary metric for buy-and-hold investors evaluating whether
        /// the deal pencils at a given rate and leverage.
        const HIGHEST_CASH_ON_CASH_RETURN     = 1 << 23;
        /// Maximise gross monthly cash flow: `rental_income - PITIA`.
        /// Favoured when the investor needs immediate positive carry.
        const HIGHEST_MONTHLY_CASH_FLOW       = 1 << 24;
        /// Maximise loan proceeds subject to a minimum DSCR constraint.
        /// Used in DSCR-product scenarios where the investor wants maximum
        /// leverage without violating the lender's coverage ratio floor.
        /// Requires `target_dscr_floor` in the analysis request.
        const MAXIMUM_LEVERAGE_AT_TARGET_DSCR = 1 << 25;
        /// Maximise IRR on the equity stake at the hold horizon.
        /// Incorporates net rental income, appreciation, principal
        /// pay-down, and sale proceeds. Requires a projected exit cap rate.
        const HIGHEST_IRR_AT_HORIZON          = 1 << 26;
        /// Minimise total holding expense (debt service + mandatory reserves
        /// + MI) over the hold period. Useful for fix-and-flip or bridge
        /// scenarios where the asset will be sold quickly.
        const LOWEST_HOLDING_EXPENSE          = 1 << 27;

        // ── Investor: Velocity & Liquidity (bits 28–29) ───────────────────
        /// Minimise title seasoning requirements. BRRRR investors need to
        /// execute a cash-out refi immediately after forced appreciation;
        /// many lenders require 6–12 months on title before allowing this.
        const MINIMIZE_TITLE_SEASONING        = 1 << 28;
        /// Minimise post-closing reserve requirements. Reserve overlays
        /// (often 6–12 months PITIA) lock capital that investors need to
        /// deploy into the next acquisition.
        const LOWEST_RESERVE_REQUIREMENT      = 1 << 29;

        // ── Shared: Liability & Exit (bits 30–33) ─────────────────────────
        /// Require zero prepayment penalty. Consumers want this to protect
        /// refinancing optionality; investors want it to allow unpenalised
        /// asset liquidation.
        const ZERO_PREPAYMENT_PENALTY         = 1 << 30;
        /// Minimise prepayment penalty severity. When zero-penalty products
        /// are not available or not optimal, find the product with the
        /// cheapest exit cost (step-down, defeasance, or yield maintenance).
        const MINIMIZE_PREPAYMENT_PENALTY     = 1 << 31;
        /// Prefer assumable debt (FHA/VA). A below-market assumable loan
        /// transfers to the buyer at closing, effectively reducing the
        /// buyer's cost and allowing the seller to command a higher price.
        const MAXIMUM_ASSUMABILITY            = 1 << 32;
        /// Prefer non-recourse DSCR lending that isolates liability to the
        /// asset and bypasses personal Global Cash Flow (GCF) or DTI
        /// analysis. Critical for investors holding multiple properties.
        const NON_RECOURSE_LIABILITY          = 1 << 33;

        // ── Composite defaults ────────────────────────────────────────────

        /// Default goal set for an owner-occupant consumer purchase.
        /// Activates: `LOWEST_HORIZON_COST`, `LOWEST_PAYMENT`,
        /// `LOWEST_CASH_TO_CLOSE`, `LOWEST_RATE`, `LOWEST_APR`.
        const DEFAULT_CONSUMER = Self::LOWEST_HORIZON_COST.bits()
            | Self::LOWEST_PAYMENT.bits()
            | Self::LOWEST_CASH_TO_CLOSE.bits()
            | Self::LOWEST_RATE.bits()
            | Self::LOWEST_APR.bits();

        /// Default goal set for a real-estate investor.
        /// Activates: `HIGHEST_CASH_ON_CASH_RETURN`, `HIGHEST_MONTHLY_CASH_FLOW`,
        /// `MAXIMUM_LEVERAGE_AT_TARGET_DSCR`, `LOWEST_CASH_TO_CLOSE`.
        const DEFAULT_INVESTOR = Self::HIGHEST_CASH_ON_CASH_RETURN.bits()
            | Self::HIGHEST_MONTHLY_CASH_FLOW.bits()
            | Self::MAXIMUM_LEVERAGE_AT_TARGET_DSCR.bits()
            | Self::LOWEST_CASH_TO_CLOSE.bits();
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// GoalMask methods
// ─────────────────────────────────────────────────────────────────────────────

impl GoalMask {
    // ── Counting and filtering ────────────────────────────────────────────

    /// Count of individually-set goal bits.
    #[must_use]
    pub fn active_count(self) -> u32 {
        self.bits().count_ones()
    }

    /// True if any goal is enabled.
    #[must_use]
    pub fn is_any_active(self) -> bool {
        !self.is_empty()
    }

    /// Return a new mask containing only the consumer-persona goals from
    /// this mask, discarding investor and shared goals.
    #[must_use]
    pub fn consumer_goals(self) -> GoalMask {
        GoalMask::from_bits_truncate(self.bits() & CONSUMER_BITS)
    }

    /// Return a new mask containing only the investor-persona goals.
    #[must_use]
    pub fn investor_goals(self) -> GoalMask {
        GoalMask::from_bits_truncate(self.bits() & INVESTOR_BITS)
    }

    /// Return a new mask containing only the shared (liability/exit) goals.
    #[must_use]
    pub fn shared_goals(self) -> GoalMask {
        GoalMask::from_bits_truncate(self.bits() & SHARED_BITS)
    }

    // ── Persona detection ────────────────────────────────────────────────

    /// True if any investor-specific goal is active. Does NOT imply that
    /// consumer goals are absent — a mixed mask is valid.
    #[must_use]
    pub fn is_investor_mode(self) -> bool {
        (self.bits() & INVESTOR_BITS) != 0
    }

    /// True if only consumer and/or shared goals are active (no
    /// investor-specific goals). This is the expected state for
    /// owner-occupant requests.
    #[must_use]
    pub fn is_consumer_mode(self) -> bool {
        (self.bits() & INVESTOR_BITS) == 0 && self.is_any_active()
    }

    // ── Goal modification ─────────────────────────────────────────────────

    /// Enable a specific goal, returning the modified mask.
    #[must_use]
    pub fn enable(self, goal: GoalMask) -> GoalMask {
        self | goal
    }

    /// Disable a specific goal, returning the modified mask.
    #[must_use]
    pub fn disable(self, goal: GoalMask) -> GoalMask {
        self & !goal
    }

    /// Toggle a goal (enable if disabled, disable if enabled).
    #[must_use]
    pub fn toggle_goal(self, goal: GoalMask) -> GoalMask {
        self ^ goal
    }

    // ── Per-goal metadata ─────────────────────────────────────────────────

    /// Short display name for a single-bit goal mask.
    ///
    /// Returns `None` if `goal` has zero bits set or more than one bit set.
    /// Use [`Self::iter_goals`] + `name_of` to label a composite mask.
    #[must_use]
    pub fn name_of(goal: GoalMask) -> Option<&'static str> {
        if goal.bits().count_ones() != 1 {
            return None;
        }
        let b = goal.bits();
        GOAL_TABLE.iter().find(|g| g.bits == b).map(|g| g.short_name)
    }

    /// One-sentence description for a single-bit goal mask.
    /// Returns `None` for composite or empty masks.
    #[must_use]
    pub fn description_of(goal: GoalMask) -> Option<&'static str> {
        if goal.bits().count_ones() != 1 {
            return None;
        }
        let b = goal.bits();
        GOAL_TABLE.iter().find(|g| g.bits == b).map(|g| g.description)
    }

    /// Persona for a single-bit goal mask.
    /// Returns `None` for composite or empty masks.
    #[must_use]
    pub fn persona_of(goal: GoalMask) -> Option<GoalPersona> {
        if goal.bits().count_ones() != 1 {
            return None;
        }
        let b = goal.bits();
        GOAL_TABLE.iter().find(|g| g.bits == b).map(|g| g.persona)
    }

    // ── Iteration ─────────────────────────────────────────────────────────

    /// Iterate over each individually-set goal bit as its own single-bit
    /// `GoalMask`. The iterator yields goals in ascending bit order.
    ///
    /// ```rust
    /// use types::GoalMask;
    ///
    /// let mask = GoalMask::LOWEST_RATE | GoalMask::LOWEST_APR;
    /// for goal in mask.iter_goals() {
    ///     if let Some(name) = GoalMask::name_of(goal) {
    ///         println!("Active goal: {name}");
    ///     }
    /// }
    /// ```
    pub fn iter_goals(self) -> impl Iterator<Item = GoalMask> {
        let bits = self.bits();
        (0u32..64).filter_map(move |i| {
            let bit = 1u64 << i;
            if bits & bit != 0 {
                GoalMask::from_bits(bit)
            } else {
                None
            }
        })
    }

    /// Returns a `Vec` of `(name, description, persona)` for every active
    /// goal in this mask. Useful for UI rendering and log output.
    #[must_use]
    pub fn describe_active(self) -> Vec<(&'static str, &'static str, GoalPersona)> {
        self.iter_goals()
            .filter_map(|g| {
                let b = g.bits();
                GOAL_TABLE
                    .iter()
                    .find(|info| info.bits == b)
                    .map(|info| (info.short_name, info.description, info.persona))
            })
            .collect()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_consumer_mask_contains_five_goals() {
        let mask = GoalMask::DEFAULT_CONSUMER;
        assert_eq!(mask.active_count(), 5);
        assert!(mask.contains(GoalMask::LOWEST_HORIZON_COST));
        assert!(mask.contains(GoalMask::LOWEST_PAYMENT));
        assert!(mask.contains(GoalMask::LOWEST_CASH_TO_CLOSE));
        assert!(mask.contains(GoalMask::LOWEST_RATE));
        assert!(mask.contains(GoalMask::LOWEST_APR));
    }

    #[test]
    fn test_default_investor_mask_contains_four_goals() {
        let mask = GoalMask::DEFAULT_INVESTOR;
        assert_eq!(mask.active_count(), 4);
        assert!(mask.contains(GoalMask::HIGHEST_CASH_ON_CASH_RETURN));
        assert!(mask.contains(GoalMask::HIGHEST_MONTHLY_CASH_FLOW));
        assert!(mask.contains(GoalMask::MAXIMUM_LEVERAGE_AT_TARGET_DSCR));
        assert!(mask.contains(GoalMask::LOWEST_CASH_TO_CLOSE));
    }

    #[test]
    fn test_goal_mask_serde_roundtrip() {
        let mask = GoalMask::DEFAULT_CONSUMER;
        let json = serde_json::to_string(&mask).unwrap();
        let back: GoalMask = serde_json::from_str(&json).unwrap();
        assert_eq!(back, mask);
    }

    #[test]
    fn test_goal_mask_iteration_yields_set_goals() {
        let mask = GoalMask::LOWEST_RATE | GoalMask::LOWEST_PAYMENT | GoalMask::LOWEST_APR;
        let goals: Vec<GoalMask> = mask.iter_goals().collect();
        assert_eq!(goals.len(), 3);
        assert!(goals.contains(&GoalMask::LOWEST_RATE));
        assert!(goals.contains(&GoalMask::LOWEST_PAYMENT));
        assert!(goals.contains(&GoalMask::LOWEST_APR));
    }

    #[test]
    fn test_admin_can_disable_individual_goals() {
        let original = GoalMask::DEFAULT_CONSUMER;
        let without_apr = original.disable(GoalMask::LOWEST_APR);
        assert_eq!(without_apr.active_count(), 4);
        assert!(!without_apr.contains(GoalMask::LOWEST_APR));
        assert!(without_apr.contains(GoalMask::LOWEST_RATE));
    }

    #[test]
    fn test_goal_mask_enable_and_toggle() {
        let base = GoalMask::DEFAULT_CONSUMER;
        let with_assumability = base.enable(GoalMask::MAXIMUM_ASSUMABILITY);
        assert_eq!(with_assumability.active_count(), 6);
        let toggled = with_assumability.toggle_goal(GoalMask::MAXIMUM_ASSUMABILITY);
        assert_eq!(toggled, base);
    }

    #[test]
    fn test_persona_filters() {
        let mixed = GoalMask::DEFAULT_CONSUMER | GoalMask::HIGHEST_CASH_ON_CASH_RETURN;
        let consumer_only = mixed.consumer_goals();
        assert_eq!(consumer_only, GoalMask::DEFAULT_CONSUMER);
        let investor_only = mixed.investor_goals();
        assert!(investor_only.contains(GoalMask::HIGHEST_CASH_ON_CASH_RETURN));
        assert_eq!(investor_only.active_count(), 1);
    }

    #[test]
    fn test_is_consumer_mode_and_investor_mode() {
        assert!(GoalMask::DEFAULT_CONSUMER.is_consumer_mode());
        assert!(!GoalMask::DEFAULT_CONSUMER.is_investor_mode());
        assert!(GoalMask::DEFAULT_INVESTOR.is_investor_mode());
        assert!(!GoalMask::DEFAULT_INVESTOR.is_consumer_mode());
        // Shared goals alone = consumer mode
        assert!(GoalMask::ZERO_PREPAYMENT_PENALTY.is_consumer_mode());
    }

    #[test]
    fn test_name_of_and_description_of() {
        assert_eq!(GoalMask::name_of(GoalMask::LOWEST_RATE), Some("Lowest Rate"));
        assert_eq!(GoalMask::name_of(GoalMask::LOWEST_PAYMENT), Some("Lowest Payment"));
        assert!(GoalMask::description_of(GoalMask::LOWEST_RATE).unwrap().contains("note rate"));
        // Composite mask returns None
        assert!(GoalMask::name_of(GoalMask::DEFAULT_CONSUMER).is_none());
        // Empty returns None
        assert!(GoalMask::name_of(GoalMask::empty()).is_none());
    }

    #[test]
    fn test_persona_of() {
        assert_eq!(GoalMask::persona_of(GoalMask::LOWEST_RATE), Some(GoalPersona::Consumer));
        assert_eq!(
            GoalMask::persona_of(GoalMask::HIGHEST_CASH_ON_CASH_RETURN),
            Some(GoalPersona::Investor)
        );
        assert_eq!(
            GoalMask::persona_of(GoalMask::ZERO_PREPAYMENT_PENALTY),
            Some(GoalPersona::Shared)
        );
        assert!(GoalMask::persona_of(GoalMask::DEFAULT_CONSUMER).is_none());
    }

    #[test]
    fn test_describe_active_returns_correct_count() {
        let mask = GoalMask::DEFAULT_CONSUMER;
        let descriptions = mask.describe_active();
        assert_eq!(descriptions.len(), 5);
        let names: Vec<&str> = descriptions.iter().map(|(n, _, _)| *n).collect();
        assert!(names.contains(&"Lowest Rate"));
        assert!(names.contains(&"Lowest Payment"));
    }

    #[test]
    fn test_investor_goals_are_all_distinct_bits() {
        let investor_goals = [
            GoalMask::HIGHEST_CASH_ON_CASH_RETURN,
            GoalMask::HIGHEST_MONTHLY_CASH_FLOW,
            GoalMask::MAXIMUM_LEVERAGE_AT_TARGET_DSCR,
            GoalMask::HIGHEST_IRR_AT_HORIZON,
            GoalMask::LOWEST_HOLDING_EXPENSE,
            GoalMask::MINIMIZE_TITLE_SEASONING,
            GoalMask::LOWEST_RESERVE_REQUIREMENT,
            GoalMask::NON_RECOURSE_LIABILITY,
        ];
        for (i, a) in investor_goals.iter().enumerate() {
            for (j, b) in investor_goals.iter().enumerate() {
                if i != j {
                    assert!((*a & *b).is_empty(), "investor goals {i} and {j} share a bit");
                }
            }
        }
    }

    #[test]
    fn test_all_34_individual_goals_are_distinct_bits() {
        let all_individual: Vec<GoalMask> = (0u32..34)
            .map(|i| GoalMask::from_bits(1u64 << i).expect("all 34 bits must be valid"))
            .collect();
        for (i, g) in all_individual.iter().enumerate() {
            assert_eq!(g.active_count(), 1, "bit {i} must have exactly 1 active goal");
        }
        let union = all_individual.iter().fold(GoalMask::empty(), |acc, &g| acc | g);
        assert_eq!(union.active_count(), 34);
    }

    #[test]
    fn test_goal_table_covers_all_34_bits() {
        assert_eq!(GOAL_TABLE.len(), 34, "GOAL_TABLE must have one entry per assigned goal");
        for (i, info) in GOAL_TABLE.iter().enumerate() {
            assert_eq!(info.bits, 1u64 << i, "GOAL_TABLE entry {i} has wrong bit value");
            assert!(!info.short_name.is_empty());
            assert!(!info.description.is_empty());
        }
    }

    #[test]
    fn test_goal_mask_u64_storage_all_34_bits() {
        let all_current = (0u32..34)
            .fold(GoalMask::empty(), |acc, i| {
                acc | GoalMask::from_bits_truncate(1u64 << i)
            });
        assert_eq!(all_current.active_count(), 34);
    }

    #[test]
    fn test_goal_mask_empty_and_all() {
        assert!(GoalMask::empty().is_empty());
        assert!(!GoalMask::empty().is_any_active());
        assert!(GoalMask::DEFAULT_CONSUMER.is_any_active());
        assert_eq!(GoalMask::empty().active_count(), 0);
        assert!(GoalMask::empty().describe_active().is_empty());
    }
}
