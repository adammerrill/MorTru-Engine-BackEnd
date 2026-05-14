//! `Cents` — signed integer cents, the canonical money type for the engine.

use std::fmt;
use std::str::FromStr;

use rust_decimal::prelude::ToPrimitive;
use rust_decimal::{Decimal, RoundingStrategy};
use serde::{Deserialize, Serialize};

use crate::error::ParseError;

/// Money expressed as signed integer cents. `Cents(150)` is exactly `$1.50`,
/// always, by construction. Never use `f64` for money — float arithmetic
/// introduces rounding error that compounds and eventually produces disclosed
/// monetary amounts that drift from reality.
///
/// # Arithmetic
///
/// The type provides explicit `checked_*` and `saturating_*` methods. The
/// standard `+`, `-`, `*` operators are intentionally **not** implemented:
/// silent overflow in financial code is a defect class we eliminate at the
/// type level. Callers must choose `checked_add` (returns `Option<Cents>`)
/// or `saturating_add` (clamps at `i64::MIN..=i64::MAX`) per their needs.
///
/// # Range
///
/// `i64` provides a range of roughly ±92 quadrillion dollars — six orders of
/// magnitude beyond any plausible mortgage. The signed representation is
/// required because lender credits, seller credits, and net-after-credit
/// figures can legitimately be negative.
#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Cents(pub i64);

impl Cents {
    /// Zero dollars and cents.
    pub const ZERO: Self = Cents(0);

    /// Largest representable amount: roughly `$92 quadrillion`.
    pub const MAX: Self = Cents(i64::MAX);

    /// Smallest (most negative) representable amount.
    pub const MIN: Self = Cents(i64::MIN);

    /// Construct from a whole-dollar amount. Saturates on overflow.
    #[must_use]
    pub const fn from_dollars(dollars: i64) -> Self {
        Cents(dollars.saturating_mul(100))
    }

    /// Checked addition. Returns `None` on overflow.
    #[must_use]
    pub const fn checked_add(self, other: Cents) -> Option<Self> {
        match self.0.checked_add(other.0) {
            Some(v) => Some(Cents(v)),
            None => None,
        }
    }

    /// Checked subtraction. Returns `None` on overflow.
    #[must_use]
    pub const fn checked_sub(self, other: Cents) -> Option<Self> {
        match self.0.checked_sub(other.0) {
            Some(v) => Some(Cents(v)),
            None => None,
        }
    }

    /// Multiplication by a dimensionless scalar (e.g., a count of months).
    /// Returns `None` on overflow.
    #[must_use]
    pub const fn checked_mul(self, factor: i64) -> Option<Self> {
        match self.0.checked_mul(factor) {
            Some(v) => Some(Cents(v)),
            None => None,
        }
    }

    /// Saturating addition. Clamps at `i64::MIN..=i64::MAX`.
    #[must_use]
    pub const fn saturating_add(self, other: Cents) -> Self {
        Cents(self.0.saturating_add(other.0))
    }

    /// Saturating subtraction. Clamps at `i64::MIN..=i64::MAX`.
    #[must_use]
    pub const fn saturating_sub(self, other: Cents) -> Self {
        Cents(self.0.saturating_sub(other.0))
    }

    /// Saturating scalar multiplication. Clamps at `i64::MIN..=i64::MAX`.
    #[must_use]
    pub const fn saturating_mul(self, factor: i64) -> Self {
        Cents(self.0.saturating_mul(factor))
    }

    /// True if the amount is strictly greater than zero.
    #[must_use]
    pub const fn is_positive(self) -> bool {
        self.0 > 0
    }

    /// True if the amount is strictly less than zero.
    #[must_use]
    pub const fn is_negative(self) -> bool {
        self.0 < 0
    }

    /// True if the amount is exactly zero.
    #[must_use]
    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }

    /// Absolute value. Saturates at `i64::MAX` when `self == Cents(i64::MIN)`
    /// to avoid overflow.
    #[must_use]
    pub const fn abs(self) -> Self {
        Cents(self.0.saturating_abs())
    }

    /// Convert to `rust_decimal::Decimal` with 2-decimal-place precision.
    /// `Cents(12345).to_decimal()` returns `Decimal::new(12345, 2)` (i.e., 123.45).
    #[must_use]
    pub fn to_decimal(self) -> Decimal {
        Decimal::new(self.0, 2)
    }

    /// Convert from a `rust_decimal::Decimal` dollar amount, rounding half-up
    /// to the nearest cent. This is the TRID rounding convention for
    /// disclosed monetary figures on the Loan Estimate.
    pub fn from_decimal_round_half_up(d: Decimal) -> Result<Self, ParseError> {
        let cents_decimal = (d * Decimal::from(100))
            .round_dp_with_strategy(0, RoundingStrategy::MidpointAwayFromZero);
        cents_decimal
            .to_i64()
            .map(Cents)
            .ok_or_else(|| ParseError::DecimalOutOfRange(d.to_string()))
    }

    // ── Task 1.8: named decimal bridge methods ────────────────────────────

    /// Named alias for [`Self::to_decimal`] that makes the unit explicit
    /// at call sites in the amortisation and APR calculation layers.
    /// `Cents(12345).to_decimal_dollars()` returns `Decimal("123.45")`.
    #[must_use]
    #[inline]
    pub fn to_decimal_dollars(self) -> Decimal {
        self.to_decimal()
    }

    /// Named alias for [`Self::from_decimal_round_half_up`] that makes the
    /// unit explicit at call sites. Rounds half-up per the TRID convention.
    #[inline]
    pub fn from_decimal_dollars(d: Decimal) -> Result<Self, ParseError> {
        Self::from_decimal_round_half_up(d)
    }

    /// Lossy conversion to `f64` dollars. **For Newton–Raphson and other
    /// intermediate floating-point calculations only.** Never use this for
    /// a disclosed monetary amount — always convert back to `Cents` via
    /// [`Self::from_decimal_dollars`] with explicit rounding before disclosure.
    ///
    /// Precision: f64 has 53 bits of mantissa, which gives exact
    /// representation for integers up to 2^53 ≈ $90 trillion. Beyond that,
    /// the conversion loses a few cents of precision — acceptable for the
    /// iterative solver but not for TRID forms.
    #[must_use]
    pub fn as_f64_dollars(self) -> f64 {
        self.0 as f64 / 100.0
    }
}

impl fmt::Display for Cents {
    /// Format as US-English dollars with thousands separators: `$1,234.56`,
    /// `-$1.50`, `$0.00`. Always shows exactly two decimal places.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let n = self.0;
        // unsigned_abs handles i64::MIN correctly (returns the magnitude as u64).
        let (sign, magnitude) = if n < 0 {
            ("-", n.unsigned_abs())
        } else {
            ("", n as u64)
        };
        let dollars = magnitude / 100;
        let cents = magnitude % 100;
        let dollars_str = format_with_commas(dollars);
        write!(f, "{sign}${dollars_str}.{cents:02}")
    }
}

impl FromStr for Cents {
    type Err = ParseError;

    /// Parse from a dollar-format string. Accepts `"1234.56"`, `"$1,234.56"`,
    /// `"-$1.50"`, `"-1234.56"`. Whitespace, `$` signs, and commas are stripped.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let cleaned: String = s.chars().filter(|c| !matches!(*c, '$' | ',')).collect();
        let cleaned = cleaned.trim();
        if cleaned.is_empty() {
            return Err(ParseError::InvalidMoneyString(s.to_string()));
        }
        let decimal = Decimal::from_str(cleaned)
            .map_err(|_| ParseError::InvalidMoneyString(s.to_string()))?;
        Cents::from_decimal_round_half_up(decimal)
    }
}

/// Format an unsigned integer with US-English thousands separators.
fn format_with_commas(n: u64) -> String {
    let s = n.to_string();
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut result = Vec::with_capacity(len + len / 3);
    for (i, &b) in bytes.iter().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            result.push(b',');
        }
        result.push(b);
    }
    // SAFETY: input is ASCII digits and commas, both valid UTF-8.
    String::from_utf8(result).expect("ASCII bytes are valid UTF-8")
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_cents_add_normal() {
        assert_eq!(Cents(150).checked_add(Cents(50)), Some(Cents(200)));
        assert_eq!(Cents(0).checked_add(Cents(100)), Some(Cents(100)));
        assert_eq!(Cents(-150).checked_add(Cents(50)), Some(Cents(-100)));
        assert_eq!(
            Cents(123_456).checked_add(Cents(54_321)),
            Some(Cents(177_777))
        );
    }

    #[test]
    fn test_cents_overflow_returns_none() {
        assert_eq!(Cents(i64::MAX).checked_add(Cents(1)), None);
        assert_eq!(Cents(i64::MIN).checked_sub(Cents(1)), None);
        assert_eq!(Cents(i64::MAX).checked_mul(2), None);
        assert_eq!(Cents(i64::MIN).checked_mul(2), None);
    }

    #[test]
    fn test_cents_saturating_at_bounds() {
        assert_eq!(Cents(i64::MAX).saturating_add(Cents(1)), Cents(i64::MAX));
        assert_eq!(Cents(i64::MIN).saturating_sub(Cents(1)), Cents(i64::MIN));
        assert_eq!(Cents(i64::MAX).saturating_mul(2), Cents(i64::MAX));
        assert_eq!(Cents(i64::MIN).saturating_mul(2), Cents(i64::MIN));
    }

    #[test]
    fn test_cents_subtraction_normal() {
        assert_eq!(Cents(200).checked_sub(Cents(50)), Some(Cents(150)));
        assert_eq!(Cents(100).checked_sub(Cents(100)), Some(Cents(0)));
        assert_eq!(Cents(50).checked_sub(Cents(150)), Some(Cents(-100)));
    }

    #[test]
    fn test_cents_multiplication() {
        assert_eq!(Cents(100).checked_mul(12), Some(Cents(1200)));
        assert_eq!(Cents(150).checked_mul(0), Some(Cents(0)));
        assert_eq!(Cents(-100).checked_mul(3), Some(Cents(-300)));
        assert_eq!(Cents(100).checked_mul(-3), Some(Cents(-300)));
    }

    #[test]
    fn test_cents_display_formats_with_dollar_and_comma() {
        assert_eq!(Cents(123_456).to_string(), "$1,234.56");
        assert_eq!(Cents(50).to_string(), "$0.50");
        assert_eq!(Cents(100).to_string(), "$1.00");
        assert_eq!(Cents(1_000_000_000).to_string(), "$10,000,000.00");
        assert_eq!(Cents(0).to_string(), "$0.00");
        assert_eq!(Cents(99).to_string(), "$0.99");
        assert_eq!(Cents(999_999_999_999).to_string(), "$9,999,999,999.99");
    }

    #[test]
    fn test_cents_display_negative() {
        assert_eq!(Cents(-150).to_string(), "-$1.50");
        assert_eq!(Cents(-123_456).to_string(), "-$1,234.56");
        assert_eq!(Cents(-1).to_string(), "-$0.01");
        assert_eq!(Cents(-99).to_string(), "-$0.99");
        assert_eq!(Cents(-1_000_000_000).to_string(), "-$10,000,000.00");
    }

    #[test]
    fn test_cents_from_dollars() {
        assert_eq!(Cents::from_dollars(5), Cents(500));
        assert_eq!(Cents::from_dollars(-10), Cents(-1000));
        assert_eq!(Cents::from_dollars(0), Cents(0));
        // Saturates rather than overflowing
        assert_eq!(Cents::from_dollars(i64::MAX), Cents(i64::MAX));
    }

    #[test]
    fn test_cents_to_and_from_decimal() {
        assert_eq!(Cents(12345).to_decimal(), dec!(123.45));
        assert_eq!(Cents(0).to_decimal(), dec!(0.00));
        assert_eq!(Cents(-150).to_decimal(), dec!(-1.50));

        let back = Cents::from_decimal_round_half_up(dec!(123.45)).unwrap();
        assert_eq!(back, Cents(12345));
    }

    #[test]
    fn test_cents_from_decimal_rounds_half_up() {
        // Half rounds away from zero (TRID convention)
        assert_eq!(
            Cents::from_decimal_round_half_up(dec!(0.005)).unwrap(),
            Cents(1)
        );
        assert_eq!(
            Cents::from_decimal_round_half_up(dec!(0.004)).unwrap(),
            Cents(0)
        );
        assert_eq!(
            Cents::from_decimal_round_half_up(dec!(-0.005)).unwrap(),
            Cents(-1)
        );
        assert_eq!(
            Cents::from_decimal_round_half_up(dec!(1.235)).unwrap(),
            Cents(124)
        );
    }

    #[test]
    fn test_cents_parse_dollar_string() {
        assert_eq!("$1,234.56".parse::<Cents>().unwrap(), Cents(123_456));
        assert_eq!("1234.56".parse::<Cents>().unwrap(), Cents(123_456));
        assert_eq!("-$1.50".parse::<Cents>().unwrap(), Cents(-150));
        assert_eq!("$0.01".parse::<Cents>().unwrap(), Cents(1));
        assert_eq!("0".parse::<Cents>().unwrap(), Cents(0));
        assert_eq!("0.00".parse::<Cents>().unwrap(), Cents(0));
    }

    #[test]
    fn test_cents_parse_rejects_garbage() {
        assert!("abc".parse::<Cents>().is_err());
        assert!("".parse::<Cents>().is_err());
        assert!("$".parse::<Cents>().is_err());
        assert!("1.2.3".parse::<Cents>().is_err());
    }

    #[test]
    fn test_cents_serde_json() {
        let c = Cents(12345);
        let json = serde_json::to_string(&c).unwrap();
        assert_eq!(json, "12345");
        let back: Cents = serde_json::from_str(&json).unwrap();
        assert_eq!(back, c);

        let neg = Cents(-150);
        let json = serde_json::to_string(&neg).unwrap();
        assert_eq!(json, "-150");
        let back: Cents = serde_json::from_str(&json).unwrap();
        assert_eq!(back, neg);
    }

    #[test]
    fn test_cents_predicates() {
        assert!(Cents(1).is_positive());
        assert!(!Cents(0).is_positive());
        assert!(!Cents(-1).is_positive());
        assert!(Cents(-1).is_negative());
        assert!(!Cents(0).is_negative());
        assert!(!Cents(1).is_negative());
        assert!(Cents(0).is_zero());
        assert!(!Cents(1).is_zero());
        assert!(!Cents(-1).is_zero());
    }

    #[test]
    fn test_cents_abs() {
        assert_eq!(Cents(150).abs(), Cents(150));
        assert_eq!(Cents(-150).abs(), Cents(150));
        assert_eq!(Cents(0).abs(), Cents(0));
        // i64::MIN.abs() would overflow normal abs(); saturating_abs returns i64::MAX
        assert_eq!(Cents(i64::MIN).abs(), Cents(i64::MAX));
    }

    #[test]
    fn test_cents_constants() {
        assert_eq!(Cents::ZERO, Cents(0));
        assert_eq!(Cents::MAX, Cents(i64::MAX));
        assert_eq!(Cents::MIN, Cents(i64::MIN));
    }

    #[test]
    fn test_cents_repr_transparent_zero_overhead() {
        assert_eq!(size_of::<Cents>(), size_of::<i64>());
        assert_eq!(align_of::<Cents>(), align_of::<i64>());
    }

    #[test]
    fn test_cents_ordering() {
        assert!(Cents(100) > Cents(50));
        assert!(Cents(-100) < Cents(0));
        assert!(Cents(0) < Cents(1));
        let mut v = vec![Cents(50), Cents(-100), Cents(0), Cents(200)];
        v.sort();
        assert_eq!(v, vec![Cents(-100), Cents(0), Cents(50), Cents(200)]);
    }

    #[test]
    fn test_format_with_commas_helper() {
        assert_eq!(format_with_commas(0), "0");
        assert_eq!(format_with_commas(1), "1");
        assert_eq!(format_with_commas(100), "100");
        assert_eq!(format_with_commas(999), "999");
        assert_eq!(format_with_commas(1_000), "1,000");
        assert_eq!(format_with_commas(1_234_567), "1,234,567");
        assert_eq!(format_with_commas(1_234_567_890), "1,234,567,890");
    }
}
