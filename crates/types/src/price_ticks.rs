//! `PriceTicks` — rate-sheet price in 1/10000ths of a percentage point.
//!
//! **Encoding**: one stored unit equals **0.0001 percentage points**, i.e.,
//! ten-thousandths of a percentage point. So `-3.281` (a discount price) is
//! stored as `PriceTicks(-32810)`, and `+0.500` (a small premium) is stored
//! as `PriceTicks(5000)`. Par is `PriceTicks(0)`.
//!
//! The type is **signed** because rate-sheet prices can be either premiums
//! (positive — lender pays a credit) or discounts (negative — borrower pays
//! points). The combined net price after LLPAs and adjustors can also flip
//! sign depending on the scenario.

use std::fmt;
use std::str::FromStr;

use rust_decimal::prelude::ToPrimitive;
use rust_decimal::{Decimal, RoundingStrategy};
use serde::{Deserialize, Serialize};

use crate::cents::Cents;
use crate::error::ParseError;

/// Rate-sheet price in 1/10000ths of a percentage point. Signed.
#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[repr(transparent)]
pub struct PriceTicks(pub i32);

impl PriceTicks {
    /// Zero, equivalent to [`Self::PAR`].
    pub const ZERO: Self = PriceTicks(0);

    /// Par pricing: no premium, no discount.
    pub const PAR: Self = PriceTicks(0);

    /// Parse from a percentage-points string. `"-3.281"` → `PriceTicks(-32810)`.
    /// A trailing `%` sign is permitted but optional.
    pub fn from_percentage_points_str(s: &str) -> Result<Self, ParseError> {
        let cleaned = s.trim().trim_end_matches('%').trim();
        if cleaned.is_empty() {
            return Err(ParseError::InvalidPercentageString(s.to_string()));
        }
        let d = Decimal::from_str(cleaned)
            .map_err(|_| ParseError::InvalidPercentageString(s.to_string()))?;
        let scaled = (d * Decimal::from(10000))
            .round_dp_with_strategy(0, RoundingStrategy::MidpointAwayFromZero);
        let ticks = scaled
            .to_i32()
            .ok_or_else(|| ParseError::InvalidPercentageString(s.to_string()))?;
        Ok(PriceTicks(ticks))
    }

    /// Convert to a `Decimal` in percentage points. `PriceTicks(-32810)`
    /// returns `-3.2810`.
    #[must_use]
    pub fn to_decimal_pp(self) -> Decimal {
        Decimal::new(i64::from(self.0), 4)
    }

    /// Apply this price to a loan amount and return the resulting credit
    /// (positive) or cost (negative) in [`Cents`]. A price of `-3.281` on a
    /// `$200,000` loan produces `Cents(-656_200)` — the borrower pays `$6,562`
    /// in discount points.
    ///
    /// Intermediate multiplication uses `i128` so the result is correct even
    /// for very large loans where `i64 * i32` would overflow.
    #[must_use]
    pub fn apply_to_loan(self, loan: Cents) -> Cents {
        // Result in cents = loan_cents * (ticks / 10_000) / 100
        //                 = loan_cents * ticks / 1_000_000
        let product = i128::from(loan.0) * i128::from(self.0);
        let result = product / 1_000_000;
        // Clamp to i64 range; absurd values would only occur with corrupt input.
        let clamped = result.clamp(i128::from(i64::MIN), i128::from(i64::MAX));
        Cents(clamped as i64)
    }

    /// Checked addition.
    #[must_use]
    pub const fn checked_add(self, other: PriceTicks) -> Option<Self> {
        match self.0.checked_add(other.0) {
            Some(v) => Some(PriceTicks(v)),
            None => None,
        }
    }

    /// Checked subtraction.
    #[must_use]
    pub const fn checked_sub(self, other: PriceTicks) -> Option<Self> {
        match self.0.checked_sub(other.0) {
            Some(v) => Some(PriceTicks(v)),
            None => None,
        }
    }

    /// Saturating addition.
    #[must_use]
    pub const fn saturating_add(self, other: PriceTicks) -> Self {
        PriceTicks(self.0.saturating_add(other.0))
    }

    /// Saturating subtraction.
    #[must_use]
    pub const fn saturating_sub(self, other: PriceTicks) -> Self {
        PriceTicks(self.0.saturating_sub(other.0))
    }

    /// True if this is a discount price (borrower pays points).
    #[must_use]
    pub const fn is_discount(self) -> bool {
        self.0 < 0
    }

    /// True if this is a premium price (lender credit).
    #[must_use]
    pub const fn is_premium(self) -> bool {
        self.0 > 0
    }

    /// True if this price is exactly par.
    #[must_use]
    pub const fn is_par(self) -> bool {
        self.0 == 0
    }
}

impl fmt::Display for PriceTicks {
    /// Format with sign and four decimal places: `-3.2810`, `0.0000`, `2.2500`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let n = self.0;
        let (sign, magnitude) = if n < 0 {
            ("-", n.unsigned_abs())
        } else {
            ("", n as u32)
        };
        let whole = magnitude / 10000;
        let frac = magnitude % 10000;
        write!(f, "{sign}{whole}.{frac:04}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_price_ticks_signed() {
        // Discount: borrower pays points
        let discount = PriceTicks(-32810);
        assert!(discount.is_discount());
        assert!(!discount.is_premium());
        assert!(!discount.is_par());
        assert_eq!(discount.0, -32810);

        // Premium: lender pays a credit
        let premium = PriceTicks(22500);
        assert!(!premium.is_discount());
        assert!(premium.is_premium());
        assert!(!premium.is_par());

        // Par
        let par = PriceTicks::PAR;
        assert!(!par.is_discount());
        assert!(!par.is_premium());
        assert!(par.is_par());
        assert_eq!(par.0, 0);
    }

    #[test]
    fn test_price_ticks_from_string() {
        assert_eq!(
            PriceTicks::from_percentage_points_str("-3.281").unwrap(),
            PriceTicks(-32810)
        );
        assert_eq!(
            PriceTicks::from_percentage_points_str("0.000").unwrap(),
            PriceTicks(0)
        );
        assert_eq!(
            PriceTicks::from_percentage_points_str("2.250").unwrap(),
            PriceTicks(22500)
        );
        assert_eq!(
            PriceTicks::from_percentage_points_str("-0.500").unwrap(),
            PriceTicks(-5000)
        );
    }

    #[test]
    fn test_price_ticks_to_decimal_pp() {
        assert_eq!(PriceTicks(-32810).to_decimal_pp(), dec!(-3.2810));
        assert_eq!(PriceTicks(22500).to_decimal_pp(), dec!(2.2500));
        assert_eq!(PriceTicks(0).to_decimal_pp(), dec!(0.0000));
    }

    #[test]
    fn test_price_ticks_apply_to_loan() {
        // $200,000 loan with -3.281 price = $6,562 cost to borrower
        let loan = Cents(20_000_000);
        let price = PriceTicks(-32810);
        let credit = price.apply_to_loan(loan);
        assert_eq!(credit, Cents(-656_200));

        // $200,000 loan with par = $0 effect
        assert_eq!(PriceTicks::PAR.apply_to_loan(loan), Cents(0));

        // $200,000 loan with +1.000 = $2,000 credit to borrower
        assert_eq!(PriceTicks(10_000).apply_to_loan(loan), Cents(200_000));
    }

    #[test]
    fn test_price_ticks_apply_to_loan_large_amount_no_overflow() {
        // $10 million loan with -3.281 — verify i128 intermediate avoids overflow
        let loan = Cents(1_000_000_000);
        let price = PriceTicks(-32810);
        let credit = price.apply_to_loan(loan);
        assert_eq!(credit, Cents(-32_810_000));
    }

    #[test]
    fn test_price_ticks_display() {
        assert_eq!(PriceTicks(-32810).to_string(), "-3.2810");
        assert_eq!(PriceTicks(22500).to_string(), "2.2500");
        assert_eq!(PriceTicks(0).to_string(), "0.0000");
        assert_eq!(PriceTicks(-1).to_string(), "-0.0001");
        assert_eq!(PriceTicks(10000).to_string(), "1.0000");
    }

    #[test]
    fn test_price_ticks_arithmetic() {
        assert_eq!(
            PriceTicks(-32810).checked_add(PriceTicks(5000)),
            Some(PriceTicks(-27810))
        );
        assert_eq!(
            PriceTicks(-32810).checked_sub(PriceTicks(5000)),
            Some(PriceTicks(-37810))
        );
        assert_eq!(PriceTicks(i32::MAX).checked_add(PriceTicks(1)), None);
        assert_eq!(PriceTicks(i32::MIN).checked_sub(PriceTicks(1)), None);
    }

    #[test]
    fn test_price_ticks_serde_json() {
        let p = PriceTicks(-32810);
        let json = serde_json::to_string(&p).unwrap();
        assert_eq!(json, "-32810");
        let back: PriceTicks = serde_json::from_str(&json).unwrap();
        assert_eq!(back, p);
    }

    #[test]
    fn test_price_ticks_invalid_string_rejected() {
        assert!(PriceTicks::from_percentage_points_str("abc").is_err());
        assert!(PriceTicks::from_percentage_points_str("").is_err());
        assert!(PriceTicks::from_percentage_points_str("1.2.3").is_err());
    }

    #[test]
    fn test_price_ticks_repr_transparent() {
        assert_eq!(
            std::mem::size_of::<PriceTicks>(),
            std::mem::size_of::<i32>()
        );
    }
}
