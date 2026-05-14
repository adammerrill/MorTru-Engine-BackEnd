//! `LtvBasisPoints` — loan-to-value ratio in true basis points.
//!
//! **Encoding**: one stored unit equals **0.01%** (one true basis point).
//! So `95.00%` is stored as `LtvBasisPoints(9500)` and `97.00%` is stored as
//! `LtvBasisPoints(9700)`. The validating constructor rejects values above
//! `11000` (110.00%) since any LTV beyond that is implausible — VA loans
//! can finance the funding fee into the loan and go slightly above 100%,
//! but anything past 110% indicates corrupt input data.

use std::fmt;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::cents::Cents;
use crate::error::ParseError;

/// Loan-to-value ratio stored at 2-decimal-place precision.
#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[repr(transparent)]
pub struct LtvBasisPoints(pub u32);

impl LtvBasisPoints {
    /// Zero LTV (fully cash-funded, no loan needed).
    pub const ZERO: Self = LtvBasisPoints(0);

    /// 110.00% — the upper plausible bound. VA can finance the funding fee
    /// above 100% LTV; anything past 110% is rejected as invalid input.
    pub const MAX_PLAUSIBLE: Self = LtvBasisPoints(11000);

    /// Validating constructor. Returns `Err` if `value > 11000` (110.00%).
    ///
    /// Direct tuple-struct construction (`LtvBasisPoints(15000)`) bypasses
    /// this check and is reserved for trusted internal contexts. Always
    /// prefer `LtvBasisPoints::new(value)` for any value derived from
    /// external input.
    pub fn new(value: u32) -> Result<Self, ParseError> {
        if value > Self::MAX_PLAUSIBLE.0 {
            return Err(ParseError::LtvOutOfRange(value));
        }
        Ok(LtvBasisPoints(value))
    }

    /// Compute LTV from loan amount and property value, rounding half-up to
    /// the nearest basis point. Returns `Err` if the property value is zero
    /// or negative, or if the resulting LTV exceeds 110%.
    pub fn from_loan_and_value(loan: Cents, value: Cents) -> Result<Self, ParseError> {
        if value.0 <= 0 {
            return Err(ParseError::ZeroPropertyValue);
        }
        // LTV bps = round(loan / value * 10_000)
        // Compute as (loan * 10_000 + value/2) / value in i128 to avoid overflow.
        let numerator = i128::from(loan.0) * 10_000;
        let denominator = i128::from(value.0);
        let rounded = if loan.0 >= 0 {
            (numerator + denominator / 2) / denominator
        } else {
            (numerator - denominator / 2) / denominator
        };
        if rounded < 0 {
            return Err(ParseError::LtvOutOfRange(0));
        }
        let as_u32 = u32::try_from(rounded).map_err(|_| ParseError::LtvOutOfRange(u32::MAX))?;
        Self::new(as_u32)
    }

    /// Convert to a `Decimal` percentage. `LtvBasisPoints(9500)` returns `95.00`.
    #[must_use]
    pub fn to_decimal_percent(self) -> Decimal {
        Decimal::new(i64::from(self.0), 2)
    }

    /// Convert to a `Decimal` rate (fraction of 1). `LtvBasisPoints(9500)` returns `0.9500`.
    #[must_use]
    pub fn to_decimal_rate(self) -> Decimal {
        Decimal::new(i64::from(self.0), 4)
    }

    /// Checked addition. Result is saturated by the 110% cap; values above
    /// the cap return `None`.
    #[must_use]
    pub fn checked_add(self, other: LtvBasisPoints) -> Option<Self> {
        let sum = self.0.checked_add(other.0)?;
        Self::new(sum).ok()
    }

    /// Checked subtraction. Returns `None` on underflow.
    #[must_use]
    pub const fn checked_sub(self, other: LtvBasisPoints) -> Option<Self> {
        match self.0.checked_sub(other.0) {
            Some(v) => Some(LtvBasisPoints(v)),
            None => None,
        }
    }
}

impl fmt::Display for LtvBasisPoints {
    /// Format as `"95.00%"` with exactly two decimal places.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let whole = self.0 / 100;
        let frac = self.0 % 100;
        write!(f, "{whole}.{frac:02}%")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_ltv_capped_at_one_hundred_ten() {
        // Up to and including 11000 (110.00%) is accepted
        assert_eq!(LtvBasisPoints::new(9500).unwrap(), LtvBasisPoints(9500));
        assert_eq!(LtvBasisPoints::new(10_000).unwrap(), LtvBasisPoints(10_000));
        assert_eq!(LtvBasisPoints::new(10_500).unwrap(), LtvBasisPoints(10_500));
        assert_eq!(LtvBasisPoints::new(11_000).unwrap(), LtvBasisPoints(11_000));

        // 11001 and above is rejected
        assert!(LtvBasisPoints::new(11_001).is_err());
        assert!(LtvBasisPoints::new(15_000).is_err());
        assert!(LtvBasisPoints::new(u32::MAX).is_err());

        // The error variant carries the offending value for diagnostics
        match LtvBasisPoints::new(15_000) {
            Err(ParseError::LtvOutOfRange(v)) => assert_eq!(v, 15_000),
            other => panic!("expected LtvOutOfRange, got {other:?}"),
        }
    }

    #[test]
    fn test_ltv_from_loan_and_value() {
        // $285,000 / $300,000 = 95.00%
        let loan = Cents(28_500_000);
        let value = Cents(30_000_000);
        let ltv = LtvBasisPoints::from_loan_and_value(loan, value).unwrap();
        assert_eq!(ltv, LtvBasisPoints(9500));

        // $194,000 / $200,000 = 97.00%
        let ltv =
            LtvBasisPoints::from_loan_and_value(Cents(19_400_000), Cents(20_000_000)).unwrap();
        assert_eq!(ltv, LtvBasisPoints(9700));

        // $100,000 / $100,000 = 100.00%
        let ltv =
            LtvBasisPoints::from_loan_and_value(Cents(10_000_000), Cents(10_000_000)).unwrap();
        assert_eq!(ltv, LtvBasisPoints(10_000));

        // Half-up rounding: $193,001 / $200,000 = 96.5005% → 9650 bps
        let ltv =
            LtvBasisPoints::from_loan_and_value(Cents(19_300_100), Cents(20_000_000)).unwrap();
        assert_eq!(ltv, LtvBasisPoints(9650));
    }

    #[test]
    fn test_ltv_from_loan_and_value_rejects_zero_value() {
        let loan = Cents(20_000_000);
        let zero = Cents(0);
        assert_eq!(
            LtvBasisPoints::from_loan_and_value(loan, zero),
            Err(ParseError::ZeroPropertyValue)
        );

        let negative = Cents(-1);
        assert_eq!(
            LtvBasisPoints::from_loan_and_value(loan, negative),
            Err(ParseError::ZeroPropertyValue)
        );
    }

    #[test]
    fn test_ltv_from_loan_and_value_rejects_implausible_ltv() {
        // $200,000 / $100,000 = 200% — rejected
        let loan = Cents(20_000_000);
        let value = Cents(10_000_000);
        assert!(LtvBasisPoints::from_loan_and_value(loan, value).is_err());
    }

    #[test]
    fn test_ltv_from_loan_and_value_no_overflow_on_large_values() {
        // $10 billion / $10 billion using i128 intermediate
        let loan = Cents(1_000_000_000_000);
        let value = Cents(1_000_000_000_000);
        let ltv = LtvBasisPoints::from_loan_and_value(loan, value).unwrap();
        assert_eq!(ltv, LtvBasisPoints(10_000));
    }

    #[test]
    fn test_ltv_to_decimal_percent() {
        assert_eq!(LtvBasisPoints(9500).to_decimal_percent(), dec!(95.00));
        assert_eq!(LtvBasisPoints(9700).to_decimal_percent(), dec!(97.00));
        assert_eq!(LtvBasisPoints(0).to_decimal_percent(), dec!(0.00));
        assert_eq!(LtvBasisPoints(11_000).to_decimal_percent(), dec!(110.00));
    }

    #[test]
    fn test_ltv_to_decimal_rate() {
        assert_eq!(LtvBasisPoints(9500).to_decimal_rate(), dec!(0.9500));
        assert_eq!(LtvBasisPoints(10_000).to_decimal_rate(), dec!(1.0000));
        assert_eq!(LtvBasisPoints(0).to_decimal_rate(), dec!(0.0000));
    }

    #[test]
    fn test_ltv_display() {
        assert_eq!(LtvBasisPoints(9500).to_string(), "95.00%");
        assert_eq!(LtvBasisPoints(9700).to_string(), "97.00%");
        assert_eq!(LtvBasisPoints(10_000).to_string(), "100.00%");
        assert_eq!(LtvBasisPoints(0).to_string(), "0.00%");
        assert_eq!(LtvBasisPoints(11_000).to_string(), "110.00%");
        assert_eq!(LtvBasisPoints(99).to_string(), "0.99%");
    }

    #[test]
    fn test_ltv_arithmetic() {
        // Addition respects the 110% cap
        assert_eq!(
            LtvBasisPoints(9500).checked_add(LtvBasisPoints(200)),
            Some(LtvBasisPoints(9700))
        );
        // Sum exceeds 110% → None
        assert_eq!(
            LtvBasisPoints(10_000).checked_add(LtvBasisPoints(2000)),
            None
        );

        // Subtraction
        assert_eq!(
            LtvBasisPoints(9700).checked_sub(LtvBasisPoints(200)),
            Some(LtvBasisPoints(9500))
        );
        assert_eq!(LtvBasisPoints(0).checked_sub(LtvBasisPoints(1)), None);
    }

    #[test]
    fn test_ltv_serde_json() {
        let ltv = LtvBasisPoints::new(9500).unwrap();
        let json = serde_json::to_string(&ltv).unwrap();
        assert_eq!(json, "9500");
        let back: LtvBasisPoints = serde_json::from_str(&json).unwrap();
        assert_eq!(back, ltv);
    }

    #[test]
    fn test_ltv_constants() {
        assert_eq!(LtvBasisPoints::ZERO, LtvBasisPoints(0));
        assert_eq!(LtvBasisPoints::MAX_PLAUSIBLE, LtvBasisPoints(11_000));
    }

    #[test]
    fn test_ltv_repr_transparent() {
        assert_eq!(size_of::<LtvBasisPoints>(), size_of::<u32>());
    }
}
