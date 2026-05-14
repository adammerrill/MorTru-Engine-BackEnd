//! Property-based tests for the money/rate types.
//!
//! These tests use `proptest` to generate thousands of random inputs and
//! verify algebraic invariants that must hold for every value of the type.
//! Spec-required properties: `prop_cents_addition_is_commutative`,
//! `prop_basis_points_round_trip_through_decimal`, `prop_serde_roundtrip`
//! (one per type).

use proptest::prelude::*;
use types::{BasisPoints, Cents, CreditScore, DtiBasisPoints, LtvBasisPoints, PriceTicks};

proptest! {
    // ----- Cents -----

    /// Addition is commutative: for any a, b in i64, checked_add returns the
    /// same result regardless of operand order (including the None case on
    /// overflow).
    #[test]
    fn prop_cents_addition_is_commutative(a in any::<i64>(), b in any::<i64>()) {
        let cents_a = Cents(a);
        let cents_b = Cents(b);
        prop_assert_eq!(cents_a.checked_add(cents_b), cents_b.checked_add(cents_a));
    }

    /// Addition is associative when no intermediate result overflows.
    #[test]
    fn prop_cents_addition_associative(
        a in -1_000_000_000_000_i64..1_000_000_000_000,
        b in -1_000_000_000_000_i64..1_000_000_000_000,
        c in -1_000_000_000_000_i64..1_000_000_000_000,
    ) {
        let left = Cents(a)
            .checked_add(Cents(b))
            .and_then(|s| s.checked_add(Cents(c)));
        let right = Cents(b)
            .checked_add(Cents(c))
            .and_then(|s| Cents(a).checked_add(s));
        prop_assert_eq!(left, right);
    }

    /// Saturating addition is total — always returns a value, never panics.
    #[test]
    fn prop_cents_saturating_never_overflows(a in any::<i64>(), b in any::<i64>()) {
        // The only requirement is that the call completes without panicking.
        // The result is always a valid i64 by definition of the type.
        let _result = Cents(a).saturating_add(Cents(b));
    }

    /// Display roundtrips through parse for any value where parsing is lossless.
    /// We limit the range to ensure the formatted string can be parsed back
    /// (very large i64 values still work, but we constrain to be safe).
    #[test]
    fn prop_cents_display_roundtrip_through_parse(
        value in -1_000_000_000_000_i64..1_000_000_000_000,
    ) {
        let original = Cents(value);
        let s = original.to_string();
        let parsed: Cents = s.parse().expect("Display output must always parse back");
        prop_assert_eq!(original, parsed);
    }

    /// abs() is idempotent: abs(abs(x)) == abs(x).
    #[test]
    fn prop_cents_abs_idempotent(value in any::<i64>()) {
        let c = Cents(value);
        prop_assert_eq!(c.abs().abs(), c.abs());
    }

    /// Subtraction is the inverse of addition (within a safe range).
    #[test]
    fn prop_cents_sub_inverse_of_add(
        a in -1_000_000_000_i64..1_000_000_000,
        b in -1_000_000_000_i64..1_000_000_000,
    ) {
        let sum = Cents(a).checked_add(Cents(b)).unwrap();
        let diff = sum.checked_sub(Cents(b)).unwrap();
        prop_assert_eq!(diff, Cents(a));
    }

    // ----- BasisPoints -----

    /// Round-trip through `to_decimal_percent` → `from_percentage_str` is
    /// lossless for every value in the relevant range.
    #[test]
    fn prop_basis_points_round_trip_through_decimal(value in 0u32..1_000_000) {
        let bps = BasisPoints(value);
        let d = bps.to_decimal_percent();
        let back = BasisPoints::from_percentage_str(&d.to_string())
            .expect("decimal output must parse back to BasisPoints");
        prop_assert_eq!(bps, back);
    }

    /// Display roundtrips through `from_percentage_str`.
    #[test]
    fn prop_basis_points_display_roundtrip(value in 0u32..1_000_000) {
        let bps = BasisPoints(value);
        let s = bps.to_string();
        let parsed = BasisPoints::from_percentage_str(&s)
            .expect("Display output must always parse back");
        prop_assert_eq!(bps, parsed);
    }

    /// Adding zero is the identity (within non-overflowing range).
    #[test]
    fn prop_basis_points_zero_arithmetic(value in any::<u32>()) {
        let b = BasisPoints(value);
        prop_assert_eq!(b.checked_add(BasisPoints::ZERO), Some(b));
        prop_assert_eq!(b.checked_sub(BasisPoints::ZERO), Some(b));
    }

    // ----- Serde roundtrip (one per type) -----

    #[test]
    fn prop_serde_roundtrip_cents(value in any::<i64>()) {
        let original = Cents(value);
        let json = serde_json::to_string(&original).unwrap();
        let back: Cents = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(original, back);
    }

    #[test]
    fn prop_serde_roundtrip_basis_points(value in any::<u32>()) {
        let original = BasisPoints(value);
        let json = serde_json::to_string(&original).unwrap();
        let back: BasisPoints = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(original, back);
    }

    #[test]
    fn prop_serde_roundtrip_price_ticks(value in any::<i32>()) {
        let original = PriceTicks(value);
        let json = serde_json::to_string(&original).unwrap();
        let back: PriceTicks = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(original, back);
    }

    #[test]
    fn prop_serde_roundtrip_ltv(value in 0u32..=11_000) {
        let original = LtvBasisPoints::new(value).unwrap();
        let json = serde_json::to_string(&original).unwrap();
        let back: LtvBasisPoints = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(original, back);
    }

    #[test]
    fn prop_serde_roundtrip_dti(value in any::<u32>()) {
        let original = DtiBasisPoints::new(value);
        let json = serde_json::to_string(&original).unwrap();
        let back: DtiBasisPoints = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(original, back);
    }

    #[test]
    fn prop_serde_roundtrip_credit_score(value in 300u16..=850) {
        let original = CreditScore::new(value).unwrap();
        let json = serde_json::to_string(&original).unwrap();
        let back: CreditScore = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(original, back);
    }

    // ----- PriceTicks -----

    /// `apply_to_loan` is sign-respecting: a discount on a positive loan
    /// produces a negative effect (cost to borrower); a premium produces
    /// a positive effect (credit to borrower).
    #[test]
    fn prop_price_ticks_apply_sign(
        loan in 1_i64..100_000_000_000,  // up to $1B
        ticks in -100_000_i32..100_000,  // up to ±10 pp
    ) {
        let result = PriceTicks(ticks).apply_to_loan(Cents(loan));
        match ticks.cmp(&0) {
            std::cmp::Ordering::Greater => prop_assert!(result.0 >= 0),
            std::cmp::Ordering::Less    => prop_assert!(result.0 <= 0),
            std::cmp::Ordering::Equal   => prop_assert_eq!(result, Cents(0)),
        }
    }

    // ----- LtvBasisPoints -----

    /// LtvBasisPoints::new accepts exactly the values in 0..=11000 and
    /// rejects everything else.
    #[test]
    fn prop_ltv_accepts_iff_in_range(value in 0u32..20_000) {
        let result = LtvBasisPoints::new(value);
        if value <= 11_000 {
            prop_assert!(result.is_ok());
            prop_assert_eq!(result.unwrap().0, value);
        } else {
            prop_assert!(result.is_err());
        }
    }

    // ----- CreditScore -----

    /// CreditScore::new accepts exactly the values in 300..=850.
    #[test]
    fn prop_credit_score_accepts_iff_in_range(value in 0u16..1000) {
        let result = CreditScore::new(value);
        if (300..=850).contains(&value) {
            prop_assert!(result.is_ok());
            prop_assert_eq!(result.unwrap().0, value);
        } else {
            prop_assert!(result.is_err());
        }
    }

    /// `middle_of_three` returns a value that lies between the min and the
    /// max of its three inputs (inclusive).
    #[test]
    fn prop_credit_score_middle_is_bounded(
        a in 300u16..=850,
        b in 300u16..=850,
        c in 300u16..=850,
    ) {
        let cs_a = CreditScore::new(a).unwrap();
        let cs_b = CreditScore::new(b).unwrap();
        let cs_c = CreditScore::new(c).unwrap();
        let mid = CreditScore::middle_of_three(cs_a, cs_b, cs_c);
        let min_input = a.min(b).min(c);
        let max_input = a.max(b).max(c);
        prop_assert!(mid.0 >= min_input);
        prop_assert!(mid.0 <= max_input);
    }
}
