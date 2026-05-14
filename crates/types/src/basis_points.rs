//! `BasisPoints` — interest rate stored at 4-digit precision.
//!
//! **Encoding**: one stored unit equals **0.001%** (one-thousandth of a
//! percentage point, or one-tenth of a true basis point). So `6.875%` is
//! stored as `BasisPoints(6875)`, and `7.000%` is stored as `BasisPoints(7000)`.
//!
//! This is finer than a true basis point because mortgage rates are quoted to
//! three decimal places after the percent sign (`6.875%`, `6.999%`, `7.125%`)
//! and we want exact-integer representation without rounding at storage.

use std::fmt;
use std::str::FromStr;

use rust_decimal::prelude::ToPrimitive;
use rust_decimal::{Decimal, RoundingStrategy};
use serde::{Deserialize, Serialize};

use crate::error::ParseError;

/// Interest rate stored at 4-digit precision (0.001% per stored unit).
///
/// # Examples
///
/// ```text
/// 6.875%  -> BasisPoints(6875)
/// 7.000%  -> BasisPoints(7000)
/// 0.250%  -> BasisPoints(250)      // typical ARM margin
/// 0.001%  -> BasisPoints(1)        // smallest representable rate
/// ```
#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[repr(transparent)]
pub struct BasisPoints(pub u32);

impl BasisPoints {
    /// Zero rate.
    pub const ZERO: Self = BasisPoints(0);

    /// Maximum representable rate. `u32::MAX / 1000` ≈ 4.29 million percent,
    /// which is well beyond any plausible loan rate.
    pub const MAX: Self = BasisPoints(u32::MAX);

    /// Parse from a percentage string. `"6.875"` → `BasisPoints(6875)`.
    /// A trailing `%` sign is permitted but optional. Leading/trailing
    /// whitespace is ignored. Negative percentages are rejected since rates
    /// are non-negative; for signed price values use [`crate::PriceTicks`].
    pub fn from_percentage_str(s: &str) -> Result<Self, ParseError> {
        let cleaned = s.trim().trim_end_matches('%').trim();
        if cleaned.is_empty() {
            return Err(ParseError::InvalidPercentageString(s.to_string()));
        }
        let d = Decimal::from_str(cleaned)
            .map_err(|_| ParseError::InvalidPercentageString(s.to_string()))?;
        if d.is_sign_negative() {
            return Err(ParseError::InvalidPercentageString(s.to_string()));
        }
        let scaled = (d * Decimal::from(1000))
            .round_dp_with_strategy(0, RoundingStrategy::MidpointAwayFromZero);
        let bps = scaled
            .to_u32()
            .ok_or_else(|| ParseError::InvalidPercentageString(s.to_string()))?;
        Ok(BasisPoints(bps))
    }

    /// Convert to a `Decimal` rate (as a fraction of 1, not a percentage).
    /// `BasisPoints(6875)` returns `0.06875`.
    #[must_use]
    pub fn to_decimal_rate(self) -> Decimal {
        Decimal::new(i64::from(self.0), 5)
    }

    /// Convert to a `Decimal` percentage. `BasisPoints(6875)` returns `6.875`.
    #[must_use]
    pub fn to_decimal_percent(self) -> Decimal {
        Decimal::new(i64::from(self.0), 3)
    }

    // ── Task 1.8: named decimal bridge methods ────────────────────────────

    /// Inverse of [`Self::to_decimal_rate`]. Converts a decimal rate fraction
    /// back to the `BasisPoints` integer encoding.
    ///
    /// `Decimal("0.06875")` → `BasisPoints(6875)`.
    /// Rounds half-up at the 0.001% precision boundary.
    /// Returns `Err` if the rate is negative or too large to represent.
    pub fn from_decimal_rate(rate: Decimal) -> Result<Self, ParseError> {
        if rate.is_sign_negative() {
            return Err(ParseError::InvalidPercentageString(rate.to_string()));
        }
        // rate × 100_000 gives our 4-digit-precision integer encoding
        let scaled = (rate * Decimal::from(100_000))
            .round_dp_with_strategy(0, RoundingStrategy::MidpointAwayFromZero);
        let bps = scaled
            .to_u32()
            .ok_or_else(|| ParseError::InvalidPercentageString(rate.to_string()))?;
        Ok(BasisPoints(bps))
    }

    /// Convert an APR or note rate computed by the Newton–Raphson solver
    /// (expressed as an `f64` decimal fraction) to `BasisPoints`, rounding
    /// to the nearest 0.001% (one unit of our 4-digit encoding).
    ///
    /// `0.0606442` → `BasisPoints(6064)` (= 6.064% in our 0.001% units).
    ///
    /// **Precision note:** the spec originally cited `BasisPoints(606)` using
    /// traditional 0.01% basis points. This implementation uses our canonical
    /// 0.001% precision, so the same input yields `BasisPoints(6064)`.
    ///
    /// Returns `None` for NaN, infinity, negative rates, or values that
    /// overflow `u32`.
    #[must_use]
    pub fn from_apr_f64(rate: f64) -> Option<Self> {
        if !rate.is_finite() || rate < 0.0 {
            return None;
        }
        // Use Decimal for the final rounding step to avoid f64 precision loss
        // at the rounding boundary.
        let d = Decimal::from_f64_retain(rate)?;
        let scaled = (d * Decimal::from(100_000))
            .round_dp_with_strategy(0, RoundingStrategy::MidpointAwayFromZero);
        let bps = scaled.to_u32()?;
        Some(BasisPoints(bps))
    }

    /// Lossy conversion to `f64` decimal rate. **For Newton–Raphson and
    /// other intermediate floating-point calculations only.**
    ///
    /// `BasisPoints(6875).as_f64_rate()` returns `0.06875_f64`.
    ///
    /// f64 has 53-bit mantissa. The smallest distinguishable rate increment
    /// at 6% is ~1e-15, far finer than our 0.001% = 1e-5 storage precision.
    /// Precision loss is negligible for any iterative rate solver.
    #[must_use]
    pub fn as_f64_rate(self) -> f64 {
        self.0 as f64 / 100_000.0
    }

    /// Checked addition. Returns `None` on overflow.
    #[must_use]
    pub const fn checked_add(self, other: BasisPoints) -> Option<Self> {
        match self.0.checked_add(other.0) {
            Some(v) => Some(BasisPoints(v)),
            None => None,
        }
    }

    /// Checked subtraction. Returns `None` on overflow.
    #[must_use]
    pub const fn checked_sub(self, other: BasisPoints) -> Option<Self> {
        match self.0.checked_sub(other.0) {
            Some(v) => Some(BasisPoints(v)),
            None => None,
        }
    }

    /// Saturating addition.
    #[must_use]
    pub const fn saturating_add(self, other: BasisPoints) -> Self {
        BasisPoints(self.0.saturating_add(other.0))
    }

    /// Saturating subtraction.
    #[must_use]
    pub const fn saturating_sub(self, other: BasisPoints) -> Self {
        BasisPoints(self.0.saturating_sub(other.0))
    }
}

impl fmt::Display for BasisPoints {
    /// Format as `"6.875%"` with exactly three decimal places.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let whole = self.0 / 1000;
        let frac = self.0 % 1000;
        write!(f, "{whole}.{frac:03}%")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_basis_points_from_percentage_string() {
        assert_eq!(
            BasisPoints::from_percentage_str("6.875").unwrap(),
            BasisPoints(6875)
        );
        assert_eq!(
            BasisPoints::from_percentage_str("7.000").unwrap(),
            BasisPoints(7000)
        );
        assert_eq!(
            BasisPoints::from_percentage_str("7").unwrap(),
            BasisPoints(7000)
        );
        assert_eq!(
            BasisPoints::from_percentage_str("0.250").unwrap(),
            BasisPoints(250)
        );
        assert_eq!(
            BasisPoints::from_percentage_str("0.001").unwrap(),
            BasisPoints(1)
        );
    }

    #[test]
    fn test_basis_points_from_percentage_string_with_percent_sign() {
        assert_eq!(
            BasisPoints::from_percentage_str("6.875%").unwrap(),
            BasisPoints(6875)
        );
        assert_eq!(
            BasisPoints::from_percentage_str("7%").unwrap(),
            BasisPoints(7000)
        );
    }

    #[test]
    fn test_basis_points_from_percentage_string_with_whitespace() {
        assert_eq!(
            BasisPoints::from_percentage_str("  6.875  ").unwrap(),
            BasisPoints(6875)
        );
        assert_eq!(
            BasisPoints::from_percentage_str(" 6.875 % ").unwrap(),
            BasisPoints(6875)
        );
    }

    #[test]
    fn test_basis_points_to_decimal_rate() {
        assert_eq!(BasisPoints(6875).to_decimal_rate(), dec!(0.06875));
        assert_eq!(BasisPoints(7000).to_decimal_rate(), dec!(0.07000));
        assert_eq!(BasisPoints(0).to_decimal_rate(), dec!(0.00000));
        assert_eq!(BasisPoints(100).to_decimal_rate(), dec!(0.00100));
        // 0.001% (smallest unit) = 0.00001 as rate
        assert_eq!(BasisPoints(1).to_decimal_rate(), dec!(0.00001));
    }

    #[test]
    fn test_basis_points_to_decimal_percent() {
        assert_eq!(BasisPoints(6875).to_decimal_percent(), dec!(6.875));
        assert_eq!(BasisPoints(7000).to_decimal_percent(), dec!(7.000));
        assert_eq!(BasisPoints(0).to_decimal_percent(), dec!(0.000));
        assert_eq!(BasisPoints(250).to_decimal_percent(), dec!(0.250));
    }

    #[test]
    fn test_basis_points_display() {
        assert_eq!(BasisPoints(6875).to_string(), "6.875%");
        assert_eq!(BasisPoints(7000).to_string(), "7.000%");
        assert_eq!(BasisPoints(0).to_string(), "0.000%");
        assert_eq!(BasisPoints(250).to_string(), "0.250%");
        assert_eq!(BasisPoints(1).to_string(), "0.001%");
    }

    #[test]
    fn test_basis_points_negative_string_rejected() {
        assert!(BasisPoints::from_percentage_str("-1.5").is_err());
        assert!(BasisPoints::from_percentage_str("-0.001").is_err());
    }

    #[test]
    fn test_basis_points_invalid_string_rejected() {
        assert!(BasisPoints::from_percentage_str("abc").is_err());
        assert!(BasisPoints::from_percentage_str("").is_err());
        assert!(BasisPoints::from_percentage_str("1.2.3").is_err());
        assert!(BasisPoints::from_percentage_str("%").is_err());
    }

    #[test]
    fn test_basis_points_arithmetic() {
        assert_eq!(
            BasisPoints(6875).checked_add(BasisPoints(125)),
            Some(BasisPoints(7000))
        );
        assert_eq!(
            BasisPoints(7000).checked_sub(BasisPoints(125)),
            Some(BasisPoints(6875))
        );
        assert_eq!(BasisPoints(u32::MAX).checked_add(BasisPoints(1)), None);
        assert_eq!(BasisPoints(0).checked_sub(BasisPoints(1)), None);

        assert_eq!(
            BasisPoints(u32::MAX).saturating_add(BasisPoints(1)),
            BasisPoints(u32::MAX)
        );
        assert_eq!(
            BasisPoints(0).saturating_sub(BasisPoints(1)),
            BasisPoints(0)
        );
    }

    #[test]
    fn test_basis_points_serde_json() {
        let b = BasisPoints(6875);
        let json = serde_json::to_string(&b).unwrap();
        assert_eq!(json, "6875");
        let back: BasisPoints = serde_json::from_str(&json).unwrap();
        assert_eq!(back, b);
    }

    #[test]
    fn test_basis_points_constants() {
        assert_eq!(BasisPoints::ZERO, BasisPoints(0));
        assert_eq!(BasisPoints::MAX, BasisPoints(u32::MAX));
    }

    #[test]
    fn test_basis_points_repr_transparent() {
        assert_eq!(size_of::<BasisPoints>(), size_of::<u32>());
    }

    #[test]
    fn test_basis_points_ordering() {
        assert!(BasisPoints(7000) > BasisPoints(6875));
        let mut v = vec![BasisPoints(7000), BasisPoints(6500), BasisPoints(6875)];
        v.sort();
        assert_eq!(
            v,
            vec![BasisPoints(6500), BasisPoints(6875), BasisPoints(7000)]
        );
    }

    #[test]
    fn test_basis_points_high_precision_round() {
        // 6.8754% should round half-up to 6875 at 0.001% precision
        let result = BasisPoints::from_percentage_str("6.8754").unwrap();
        // Decimal at 3dp: 6.8754 * 1000 = 6875.4, round half-up = 6875
        assert_eq!(result, BasisPoints(6875));

        // 6.8755 should round to 6876
        let result = BasisPoints::from_percentage_str("6.8755").unwrap();
        assert_eq!(result, BasisPoints(6876));
    }
}
