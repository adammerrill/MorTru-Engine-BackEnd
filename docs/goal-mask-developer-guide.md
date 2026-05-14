# GoalMask Developer Guide

**File:** `crates/types/src/goal_mask.rs`  
**Task:** 1.9  
**Owner:** `types` crate  

---

## What is GoalMask?

`GoalMask` is a `u64` bitmask that encodes the optimisation objectives for a
single analysis run. Each bit represents one goal — a dimension on which the
engine should rank loan scenarios.

The engine does **not** return a single best loan. It returns a
**Pareto-optimal frontier**: the set of scenarios where no other scenario is
strictly better on every active goal simultaneously. The operator (or the
borrower, via UI toggles) configures which goals are active; the engine does
the rest.

```
Borrower request
    ↓
AnalysisRequest { goals: GoalMask, ... }
    ↓
Engine enumerates ~40k scenarios
    ↓
Each scenario is scored for every active goal
    ↓
Pareto frontier eliminates dominated scenarios
    ↓
AnalysisResult with ranked scenarios per goal
```

---

## Goal Personas

Goals belong to one of three personas:

| Persona  | Who uses it | Bits |
|----------|-------------|------|
| `Consumer` | Owner-occupant homebuyers | 0–22 |
| `Investor` | Real-estate investors | 23–29 |
| `Shared` | Both, with different intent | 30–33 |

**Shared goals** have the same bit but different ranking context: a *consumer*
enabling `ZERO_PREPAYMENT_PENALTY` cares about refinancing optionality; an
*investor* enabling the same goal cares about unpenalised asset liquidation.
The scoring engine receives the full context and weights accordingly.

---

## Built-in Defaults

### DEFAULT_CONSUMER (5 goals)

```rust
GoalMask::LOWEST_HORIZON_COST
| GoalMask::LOWEST_PAYMENT
| GoalMask::LOWEST_CASH_TO_CLOSE
| GoalMask::LOWEST_RATE
| GoalMask::LOWEST_APR
```

Use for owner-occupant purchase or refinance when no explicit goals are
specified. Covers the five dimensions LOs discuss most frequently.

### DEFAULT_INVESTOR (4 goals)

```rust
GoalMask::HIGHEST_CASH_ON_CASH_RETURN
| GoalMask::HIGHEST_MONTHLY_CASH_FLOW
| GoalMask::MAXIMUM_LEVERAGE_AT_TARGET_DSCR
| GoalMask::LOWEST_CASH_TO_CLOSE
```

Use for non-owner-occupied and DSCR product requests.

---

## Common Usage Patterns

### Consumer: Basic homebuyer

```rust
let goals = GoalMask::DEFAULT_CONSUMER;
```

### Consumer: Cash-constrained first-time buyer

```rust
let goals = GoalMask::DEFAULT_CONSUMER
    .enable(GoalMask::LOWEST_UPFRONT_MI)
    .enable(GoalMask::FASTEST_MI_CANCEL);
```

### Consumer: Rate-and-term refi — break-even focus

```rust
let goals = GoalMask::DEFAULT_CONSUMER
    .enable(GoalMask::FASTEST_BREAK_EVEN)
    .disable(GoalMask::LOWEST_CASH_TO_CLOSE);
```

### Consumer: ARM comparison with payment-shock concern

```rust
let goals = GoalMask::DEFAULT_CONSUMER
    .enable(GoalMask::HIGHEST_PAYMENT_STABILITY)
    .enable(GoalMask::MINIMIZE_ARM_PAYMENT_SHOCK);
```

### Consumer: Itemising high-earner (Schedule A deduction)

```rust
let goals = GoalMask::DEFAULT_CONSUMER
    .enable(GoalMask::MAX_MORTGAGE_INTEREST_DEDUCTION)
    .enable(GoalMask::LOWEST_LIFETIME_COST);
```

### Consumer: FHA buyer focused on exiting MIP

```rust
let goals = GoalMask::DEFAULT_CONSUMER
    .enable(GoalMask::FASTEST_EIGHTY_LTV)
    .enable(GoalMask::FASTEST_MI_CANCEL)
    .enable(GoalMask::LOWEST_MI_COST);
```

### Consumer: VA assumability play (sell at premium in a rising-rate market)

```rust
let goals = GoalMask::DEFAULT_CONSUMER
    .enable(GoalMask::MAXIMUM_ASSUMABILITY)
    .enable(GoalMask::ZERO_PREPAYMENT_PENALTY);
```

### Investor: BRRRR strategy

```rust
// Buy, rehab, rent, refinance, repeat — needs fast seasoning for cash-out
let goals = GoalMask::DEFAULT_INVESTOR
    .enable(GoalMask::MINIMIZE_TITLE_SEASONING)
    .enable(GoalMask::LOWEST_RESERVE_REQUIREMENT);
```

### Investor: DSCR buy-and-hold, liability isolation

```rust
let goals = GoalMask::DEFAULT_INVESTOR
    .enable(GoalMask::NON_RECOURSE_LIABILITY)
    .enable(GoalMask::HIGHEST_IRR_AT_HORIZON);
```

### Investor: Fix-and-flip (short hold, exit under 24 months)

```rust
let goals = GoalMask::empty()
    .enable(GoalMask::ZERO_PREPAYMENT_PENALTY)
    .enable(GoalMask::LOWEST_HOLDING_EXPENSE)
    .enable(GoalMask::HIGHEST_CASH_ON_CASH_RETURN);
```

### Investor: Cash-out to retire revolving debt

```rust
let goals = GoalMask::DEFAULT_CONSUMER
    .enable(GoalMask::LOWEST_BLENDED_TOTAL_DEBT_RATE)
    .disable(GoalMask::LOWEST_CASH_TO_CLOSE);
```

---

## Inspecting Active Goals at Runtime

```rust
use types::GoalMask;

fn log_active_goals(mask: GoalMask) {
    for goal in mask.iter_goals() {
        if let (Some(name), Some(persona)) = (
            GoalMask::name_of(goal),
            GoalMask::persona_of(goal),
        ) {
            println!("[{persona:?}] {name}");
        }
    }
}

fn render_goal_tooltips(mask: GoalMask) -> Vec<String> {
    mask.describe_active()
        .into_iter()
        .map(|(name, desc, _)| format!("**{name}**: {desc}"))
        .collect()
}
```

---

## Persona Detection

```rust
fn validate_request(goals: GoalMask, occupancy: Occupancy) {
    if goals.is_investor_mode() && occupancy == Occupancy::PrimaryResidence {
        // Warn: investor goals on an owner-occupant property
    }
    if goals.is_consumer_mode() && occupancy == Occupancy::Investment {
        // Suggest adding investor goals
    }
}

// Decompose a mixed mask by persona
let investor_portion = goals.investor_goals();
let consumer_portion = goals.consumer_goals();
let shared_portion   = goals.shared_goals();
```

---

## Serialisation

`GoalMask` serialises as its raw `u64` integer:

```json
{ "goals": 31 }
```

`31` = `0b00011111` = bits 0–4 = `DEFAULT_CONSUMER`.

**Never re-assign or renumber an existing bit.** Persisted records and older
API responses store integer `GoalMask` values. Changing bit 5 would silently
corrupt historical data.

---

## Adding a New Goal

1. **Pick the next free bit.** Bits 0–33 are assigned; start at 34.

2. **Add the constant** inside `bitflags! { }`:

   ```rust
   /// One-sentence rustdoc describing what this goal optimises.
   const MY_NEW_GOAL = 1 << 34;
   ```

3. **Add a `GoalInfo` entry** to `GOAL_TABLE` (position 34, in bit order):

   ```rust
   GoalInfo {
       bits: 1 << 34,
       short_name: "My New Goal",
       description: "One sentence for UI and logs.",
       persona: GoalPersona::Consumer,
   },
   ```

4. **Update `CONSUMER_BITS`, `INVESTOR_BITS`, or `SHARED_BITS`** so that
   `consumer_goals()` / `investor_goals()` / `shared_goals()` return it.

5. **Add it to a default mask** only if it should be on by default (rare).

6. **Implement the scorer** in `crates/solver/src/goal_scorer.rs`:

   ```rust
   GoalMask::MY_NEW_GOAL => score_my_new_goal(scenario, context),
   ```

7. **Write a test** (see Testing Guidance below).

8. **Note the bit number in your PR description** so it can be recorded
   permanently in the changelog.

---

## Integration with the Ranking Engine (Epic 14)

### Scoring (Tasks 14.x)

For each scenario, the scorer produces a `GoalScore` per active goal.
`GoalScore` is a `f64` where **lower is better** for minimisation goals and
**higher is better** for maximisation goals.

```rust
pub fn score_scenario(
    scenario: &SolvedScenario,
    context: &ScenarioContext,
    goal: GoalMask,   // single-bit mask
) -> GoalScore { ... }
```

### Pareto Frontier (Task 14.9)

```rust
pub fn pareto_frontier(
    scored: &[(SolvedScenario, Vec<GoalScore>)],
    enabled_goals: GoalMask,  // which dimensions to compare
) -> Vec<&SolvedScenario>
```

Scenario A dominates B iff A is better-or-equal on **all** enabled goals and
strictly better on **at least one**. The frontier is the non-dominated set.
`enabled_goals.active_count()` tells the algorithm how many dimensions exist.

### ScenarioContext

```rust
pub struct ScenarioContext<'a> {
    pub goals: GoalMask,
    pub horizon_months: u16,
    pub target_payment_max: Option<Cents>,
    pub cash_available: Cents,
    // ...
}
```

The compound goals (`LOWEST_HORIZON_AT_TARGET_PAYMENT`, etc.) need the
constraint fields (`target_payment_max`, `cash_available`) to be set.
The scorer must check these are present before scoring compound goals.

---

## Reserved Bits

| Bits  | Status |
|-------|--------|
| 0–33  | Assigned (34 goals) |
| 34–63 | **Reserved** — free for future goals |

Add new goals sequentially from bit 34 upward. Do not skip bits.

---

## Testing Guidance

```rust
#[test]
fn test_my_new_goal_metadata() {
    let g = GoalMask::MY_NEW_GOAL;
    // Exactly one bit
    assert_eq!(g.active_count(), 1);
    // Metadata populated
    assert_eq!(GoalMask::name_of(g), Some("My New Goal"));
    assert!(GoalMask::description_of(g).is_some());
    assert_eq!(GoalMask::persona_of(g), Some(GoalPersona::Consumer));
    // Does not collide with any existing goal
    let existing = (0u32..34)
        .fold(GoalMask::empty(), |acc, i| acc | GoalMask::from_bits_truncate(1 << i));
    assert!((g & existing).is_empty());
}
```
