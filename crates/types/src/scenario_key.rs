//! `ScenarioKey` — 8-byte packed identifier for a single pricing scenario.
//!
//! The engine enumerates tens of thousands of scenarios (product × term ×
//! rate × MI option) per analysis. This key must:
//!
//! - Fit in a single CPU register (u64)
//! - Hash and compare in O(1) with no heap allocation
//! - Pack all scenario-distinguishing fields with no wasted bits
//!
//! # Layout (little-endian byte order)
//!
//! | Bytes | Bits  | Field              | Type  | Notes                          |
//! |-------|-------|--------------------|-------|--------------------------------|
//! | 0     | 0–7   | product            | u8    | `LoanProduct` discriminant     |
//! | 1     | 8–15  | tier               | u8    | `Tier` discriminant            |
//! | 2     | 16–23 | balance_type       | u8    | `BalanceType` discriminant     |
//! | 3–4   | 24–39 | term_months        | u16   | 120–360 inclusive              |
//! | 5–6   | 40–55 | rate_quarter_bps   | u16   | rate × 4 in basis points       |
//! | 7     | 56–63 | mi_option          | u8    | MI selection index 0–15        |
//!
//! # Rate encoding
//!
//! `rate_quarter_bps` stores the interest rate as an integer multiple of
//! **0.25 basis points** (0.0025%). One stored unit = 0.0025%. A rate of
//! 6.875% = 6875 bps ÷ 4 = 1718.75... so this field uses 0.25 bp precision
//! which is the smallest increment that appears on US rate sheets:
//!
//! - 6.875% → 6875 bps × (1/0.25) ... actually: stored = bps * 4 / 4:
//!   `rate_quarter_bps = (rate_bps * 4)` where 1 unit = 0.25 bp = 0.0025%.
//! - 6.875% = 6875 bps; stored = 6875 (already integer at 1-bp precision).
//!   For finer: 6.8725% = 6872.5 bps → store as 27490 quarter-bps.
//!
//! The u16 range 0–65535 covers 0–16383.75% — far exceeding any plausible
//! mortgage rate.
//!
//! # Safe packing via u64 transparency
//!
//! `#[repr(C, packed)]` on structs with u16 fields creates unaligned
//! references that are UB in safe Rust (clippy::unaligned_references).
//! We pack into a `u64` instead — identical layout, safe derives.

use crate::enums::loan_product::LoanProduct;
use crate::enums::misc::{BalanceType, Tier};
use crate::term_months::TermMonths;
use serde::{Deserialize, Serialize};

/// 8-byte packed scenario identifier.
///
/// Use [`ScenarioKey::new`] to construct from typed fields.
/// Use the accessor methods (`product()`, `tier()`, etc.) to unpack.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(transparent)]
pub struct ScenarioKey(u64);

// Compile-time size guard — identical intent to the spec's:
//   const _: () = assert!(std::mem::size_of::<ScenarioKey>() == 8);
const _SCENARIO_KEY_SIZE_CHECK: () = {
    assert!(size_of::<ScenarioKey>() == 8);
};

impl ScenarioKey {
    // ── Bit offsets ───────────────────────────────────────────────────────
    const PRODUCT_SHIFT: u32 = 0;
    const TIER_SHIFT: u32 = 8;
    const BALANCE_TYPE_SHIFT: u32 = 16;
    const TERM_MONTHS_SHIFT: u32 = 24;
    const RATE_SHIFT: u32 = 40;
    const MI_OPTION_SHIFT: u32 = 56;

    const U8_MASK: u64 = 0xFF;
    const U16_MASK: u64 = 0xFFFF;
    const MI_MASK: u64 = 0x0F; // mi_option is 0–15 (4 bits used, stored in 8)

    /// Pack all fields into a single `ScenarioKey`.
    ///
    /// # Panics (debug only)
    ///
    /// Panics if `mi_option > 15` in debug builds. In release the value
    /// is silently truncated to 4 bits.
    #[must_use]
    pub fn new(
        product: LoanProduct,
        tier: Tier,
        balance_type: BalanceType,
        term: TermMonths,
        rate_quarter_bps: u16,
        mi_option: u8,
    ) -> Self {
        debug_assert!(mi_option <= 15, "mi_option {mi_option} exceeds maximum 15");
        ScenarioKey(
            ((product as u64) << Self::PRODUCT_SHIFT)
                | ((tier as u64) << Self::TIER_SHIFT)
                | ((balance_type as u64) << Self::BALANCE_TYPE_SHIFT)
                | ((term.0 as u64) << Self::TERM_MONTHS_SHIFT)
                | ((rate_quarter_bps as u64) << Self::RATE_SHIFT)
                | (((mi_option & 0x0F) as u64) << Self::MI_OPTION_SHIFT),
        )
    }

    /// Raw u64 value (for serialisation / hash-bucket experiments).
    #[must_use]
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    // ── Field accessors ───────────────────────────────────────────────────

    /// `LoanProduct` discriminant (0–17).
    #[must_use]
    pub fn product_raw(self) -> u8 {
        ((self.0 >> Self::PRODUCT_SHIFT) & Self::U8_MASK) as u8
    }

    /// `Tier` discriminant (0–1).
    #[must_use]
    pub fn tier_raw(self) -> u8 {
        ((self.0 >> Self::TIER_SHIFT) & Self::U8_MASK) as u8
    }

    /// `BalanceType` discriminant (0–3).
    #[must_use]
    pub fn balance_type_raw(self) -> u8 {
        ((self.0 >> Self::BALANCE_TYPE_SHIFT) & Self::U8_MASK) as u8
    }

    /// Term in months (120–360).
    #[must_use]
    pub fn term_months(self) -> u16 {
        ((self.0 >> Self::TERM_MONTHS_SHIFT) & Self::U16_MASK) as u16
    }

    /// Rate stored as quarter-basis-points (1 unit = 0.25 bp = 0.0025%).
    #[must_use]
    pub fn rate_quarter_bps(self) -> u16 {
        ((self.0 >> Self::RATE_SHIFT) & Self::U16_MASK) as u16
    }

    /// MI selection index (0–15).
    #[must_use]
    pub fn mi_option(self) -> u8 {
        ((self.0 >> Self::MI_OPTION_SHIFT) & Self::MI_MASK) as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enums::loan_product::LoanProduct;
    use crate::enums::misc::{BalanceType, Tier};
    use crate::term_months::TermMonths;

    fn sample_key() -> ScenarioKey {
        ScenarioKey::new(
            LoanProduct::FixedConv21To30,
            Tier::Standard,
            BalanceType::Conforming,
            TermMonths(360),
            6875, // 6.875% in quarter-bps (6875 bps)
            0,
        )
    }

    #[test]
    fn test_scenario_key_is_exactly_8_bytes() {
        // This is also enforced at compile time by the const assertion above.
        assert_eq!(size_of::<ScenarioKey>(), 8);
    }

    #[test]
    fn test_scenario_key_pack_unpack_roundtrip() {
        let key = sample_key();
        assert_eq!(key.product_raw(), LoanProduct::FixedConv21To30 as u8);
        assert_eq!(key.tier_raw(), Tier::Standard as u8);
        assert_eq!(key.balance_type_raw(), BalanceType::Conforming as u8);
        assert_eq!(key.term_months(), 360);
        assert_eq!(key.rate_quarter_bps(), 6875);
        assert_eq!(key.mi_option(), 0);
    }

    #[test]
    fn test_scenario_key_pack_unpack_varied_fields() {
        let key = ScenarioKey::new(
            LoanProduct::Arm5_6Sofr,
            Tier::Elite,
            BalanceType::HighBalance,
            TermMonths(300),
            28_000, // ~7.000% in quarter-bps
            3,
        );
        assert_eq!(key.product_raw(), LoanProduct::Arm5_6Sofr as u8);
        assert_eq!(key.tier_raw(), Tier::Elite as u8);
        assert_eq!(key.balance_type_raw(), BalanceType::HighBalance as u8);
        assert_eq!(key.term_months(), 300);
        assert_eq!(key.rate_quarter_bps(), 28_000);
        assert_eq!(key.mi_option(), 3);
    }

    #[test]
    fn test_scenario_key_mi_option_boundary() {
        for mi in 0u8..=15 {
            let key = ScenarioKey::new(
                LoanProduct::FixedConv21To30,
                Tier::Standard,
                BalanceType::Conforming,
                TermMonths(360),
                6875,
                mi,
            );
            assert_eq!(key.mi_option(), mi);
        }
    }

    #[test]
    fn test_scenario_key_different_fields_produce_different_keys() {
        let base = sample_key();

        let different_product = ScenarioKey::new(
            LoanProduct::FixedConv11To15,
            Tier::Standard,
            BalanceType::Conforming,
            TermMonths(360),
            6875,
            0,
        );
        let different_term = ScenarioKey::new(
            LoanProduct::FixedConv21To30,
            Tier::Standard,
            BalanceType::Conforming,
            TermMonths(240),
            6875,
            0,
        );
        let different_rate = ScenarioKey::new(
            LoanProduct::FixedConv21To30,
            Tier::Standard,
            BalanceType::Conforming,
            TermMonths(360),
            7000,
            0,
        );

        assert_ne!(base, different_product);
        assert_ne!(base, different_term);
        assert_ne!(base, different_rate);
    }

    #[test]
    fn test_scenario_key_hash_distribution() {
        use std::collections::HashMap;
        // Generate 100k distinct keys spanning product × term × rate
        let mut map: HashMap<ScenarioKey, u32> = HashMap::with_capacity(100_000);
        let mut count = 0u32;
        'outer: for rate in (4000u16..=9000).step_by(1) {
            for term in (120u16..=360).step_by(1) {
                for mi in 0u8..=4 {
                    let key = ScenarioKey::new(
                        LoanProduct::FixedConv21To30,
                        Tier::Standard,
                        BalanceType::Conforming,
                        TermMonths(term),
                        rate,
                        mi,
                    );
                    map.insert(key, count);
                    count += 1;
                    if count >= 100_000 {
                        break 'outer;
                    }
                }
            }
        }
        // All 100k keys must be distinct (no accidental collisions in the key space)
        assert_eq!(map.len(), 100_000, "all 100k keys must be unique");
    }

    #[test]
    fn test_one_million_keys_fit_in_8_megabytes() {
        // 1_000_000 × 8 bytes = 8_000_000 bytes = 8 MB
        let bytes_per_key = size_of::<ScenarioKey>();
        let total = 1_000_000usize * bytes_per_key;
        assert_eq!(bytes_per_key, 8);
        assert_eq!(total, 8_000_000);

        // Verify we can actually allocate a Vec of 1M keys
        let keys: Vec<ScenarioKey> = (0u32..1_000_000)
            .map(|i| {
                ScenarioKey::new(
                    LoanProduct::FixedConv21To30,
                    Tier::Standard,
                    BalanceType::Conforming,
                    TermMonths(120 + (i % 241) as u16),
                    4000 + (i % 5001) as u16,
                    (i % 16) as u8,
                )
            })
            .collect();
        assert_eq!(keys.len(), 1_000_000);
    }

    #[test]
    fn test_scenario_key_serde_json() {
        let key = sample_key();
        let json = serde_json::to_string(&key).unwrap();
        let back: ScenarioKey = serde_json::from_str(&json).unwrap();
        assert_eq!(back, key);
    }
}
