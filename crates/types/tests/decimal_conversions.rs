//! Integration tests for the Task 1.8 decimal conversion helpers.
//!
//! These tests verify the precision contract that governs every financial
//! calculation in the engine:
//!
//! - Monetary values are stored as integer `Cents`; `f64` or `Decimal` is used
//!   only for intermediate computation and is always rounded back to `Cents`
//!   before any disclosed figure is produced.
//! - Rate values are stored as integer `BasisPoints`; APR Newton–Raphson uses
//!   `f64` internally and rounds to the nearest 0.001% unit on output.
//!
//! Spec-required tests:
//! - `test_cents_round_half_up_to_nearest_cent`
//! - `test_basis_points_to_decimal_rate_precision`
//! - `test_decimal_to_cents_no_loss`
//! - `test_apr_round_to_nearest_basis_point`
//! - `prop_round_trip_decimal_cents`

use proptest::prelude::*;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use types::{BasisPoints, Cents};

// ─────────────────────────────────────────────────────────────────────────────
// Cents — decimal bridge
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_cents_round_half_up_to_nearest_cent() {
    // Half-up rounding (TRID convention): $1.005 rounds to $1.01 (up)
    assert_eq!(
        Cents::from_decimal_dollars(dec!(1.005)).unwrap(),
        Cents(101),
        "$1.005 must round up to $1.01"
    );
    // $1.004 rounds to $1.00 (down)
    assert_eq!(
        Cents::from_decimal_dollars(dec!(1.004)).unwrap(),
        Cents(100),
        "$1.004 must round down to $1.00"
    );
    // Exactly halfway — half-UP means this rounds to $1.01
    assert_eq!(
        Cents::from_decimal_dollars(dec!(1.005)).unwrap(),
        Cents(101)
    );
    // Negative: -$1.005 rounds away from zero → -$1.01
    assert_eq!(
        Cents::from_decimal_dollars(dec!(-1.005)).unwrap(),
        Cents(-101)
    );
    // -$1.004 rounds toward zero → -$1.00
    assert_eq!(
        Cents::from_decimal_dollars(dec!(-1.004)).unwrap(),
        Cents(-100)
    );
    // Zero
    assert_eq!(Cents::from_decimal_dollars(dec!(0)).unwrap(), Cents(0));
}

#[test]
fn test_decimal_to_cents_no_loss() {
    // Decimal("123.45") → Cents(12345) → Decimal("123.45") — lossless
    let d = dec!(123.45);
    let c = Cents::from_decimal_dollars(d).unwrap();
    assert_eq!(c, Cents(12345));
    let back = c.to_decimal_dollars();
    assert_eq!(
        back, d,
        "Decimal → Cents → Decimal must be lossless for exact cent values"
    );

    // A few more exact values
    let pairs: &[(Decimal, Cents)] = &[
        (dec!(0.01), Cents(1)),
        (dec!(0.99), Cents(99)),
        (dec!(1000.00), Cents(100_000)),
        (dec!(-50.25), Cents(-5025)),
    ];
    for (decimal, expected_cents) in pairs {
        let cents = Cents::from_decimal_dollars(*decimal).unwrap();
        assert_eq!(cents, *expected_cents, "from_decimal_dollars({decimal})");
        let back = cents.to_decimal_dollars();
        assert_eq!(back, *decimal, "to_decimal_dollars({cents:?})");
    }
}

#[test]
fn test_cents_to_decimal_dollars_and_from_are_inverses() {
    // to_decimal_dollars and from_decimal_dollars are strict inverses for
    // any integer number of cents.
    let values = [
        Cents(0),
        Cents(1),
        Cents(-1),
        Cents(12345),
        Cents(-99_999_999),
    ];
    for c in values {
        let d = c.to_decimal_dollars();
        let back = Cents::from_decimal_dollars(d).unwrap();
        assert_eq!(c, back, "roundtrip failed for {c:?}");
    }
}

#[test]
fn test_cents_as_f64_dollars_precision() {
    assert_eq!(Cents(0).as_f64_dollars(), 0.0_f64);
    assert_eq!(Cents(100).as_f64_dollars(), 1.0_f64);
    assert_eq!(Cents(12345).as_f64_dollars(), 123.45_f64);
    assert_eq!(Cents(-150).as_f64_dollars(), -1.5_f64);
    // Large values — f64 has ~15 significant digits, enough for any mortgage
    assert_eq!(Cents(1_000_000_000).as_f64_dollars(), 10_000_000.0_f64); // $10M
    assert_eq!(Cents(10_000_000_000).as_f64_dollars(), 100_000_000.0_f64); // $100M
}

// ─────────────────────────────────────────────────────────────────────────────
// BasisPoints — decimal bridge
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_basis_points_to_decimal_rate_precision() {
    // 6875 bps (6.875%) → 0.06875 exactly (no float rounding artefacts)
    assert_eq!(
        BasisPoints(6875).to_decimal_rate(),
        dec!(0.06875),
        "6875 bps must convert to exactly 0.06875"
    );
    // A few reference rates
    assert_eq!(BasisPoints(7000).to_decimal_rate(), dec!(0.07000));
    assert_eq!(BasisPoints(0).to_decimal_rate(), dec!(0.00000));
    assert_eq!(BasisPoints(1).to_decimal_rate(), dec!(0.00001)); // 0.001% smallest unit
    assert_eq!(BasisPoints(10_000).to_decimal_rate(), dec!(0.10000)); // 10.000%
}

#[test]
fn test_basis_points_from_decimal_rate_is_inverse_of_to_decimal_rate() {
    let samples = [
        BasisPoints(6875),
        BasisPoints(7000),
        BasisPoints(0),
        BasisPoints(1),
        BasisPoints(25_000),
    ];
    for bp in samples {
        let rate = bp.to_decimal_rate();
        let back = BasisPoints::from_decimal_rate(rate)
            .unwrap_or_else(|_| panic!("from_decimal_rate failed for {bp:?}"));
        assert_eq!(bp, back, "roundtrip failed for {bp:?}");
    }
}

#[test]
fn test_basis_points_from_decimal_rate_specific_values() {
    // 0.06875 → BasisPoints(6875)
    assert_eq!(
        BasisPoints::from_decimal_rate(dec!(0.06875)).unwrap(),
        BasisPoints(6875)
    );
    // 0.07 → BasisPoints(7000)
    assert_eq!(
        BasisPoints::from_decimal_rate(dec!(0.07)).unwrap(),
        BasisPoints(7000)
    );
    // Half-up rounding at the 0.001% boundary
    assert_eq!(
        BasisPoints::from_decimal_rate(dec!(0.068755)).unwrap(), // 6875.5 → round up
        BasisPoints(6876)
    );
    assert_eq!(
        BasisPoints::from_decimal_rate(dec!(0.068754)).unwrap(), // 6875.4 → round down
        BasisPoints(6875)
    );
    // Negative rejected
    assert!(BasisPoints::from_decimal_rate(dec!(-0.01)).is_err());
}

#[test]
fn test_apr_round_to_nearest_basis_point() {
    // APR from the Newton–Raphson solver (f64 output) → BasisPoints
    //
    // Precision note: this engine stores at 0.001% per unit, so the spec's
    // example "0.0606442 → BasisPoints(606)" uses traditional 0.01% basis
    // points. Our implementation returns BasisPoints(6064) — the same APR
    // expressed at our 0.001% precision (6.064% = 6064 units).
    let apr_rate: f64 = 0.0606442; // 6.06442%
    let bps = BasisPoints::from_apr_f64(apr_rate).expect("valid APR must convert");
    assert_eq!(
        bps,
        BasisPoints(6064),
        "6.06442% → BasisPoints(6064) at 0.001% precision"
    );

    // Clean round number: 7.000%
    assert_eq!(BasisPoints::from_apr_f64(0.07).unwrap(), BasisPoints(7000));

    // Exact half: 6.8755% — the f64 representation of 0.068755 is slightly
    // less than the mathematical value (0.068754999...), so it rounds DOWN.
    assert_eq!(
        BasisPoints::from_apr_f64(0.068755).unwrap(),
        BasisPoints(6875)
    );

    // Clearly above the boundary: 6.876%
    assert_eq!(
        BasisPoints::from_apr_f64(0.06876).unwrap(),
        BasisPoints(6876)
    );
    // Clearly below: 6.874%
    assert_eq!(
        BasisPoints::from_apr_f64(0.06874).unwrap(),
        BasisPoints(6874)
    );

    // Zero
    assert_eq!(BasisPoints::from_apr_f64(0.0).unwrap(), BasisPoints(0));

    // Invalid inputs return None
    assert!(BasisPoints::from_apr_f64(f64::NAN).is_none());
    assert!(BasisPoints::from_apr_f64(f64::INFINITY).is_none());
    assert!(BasisPoints::from_apr_f64(-0.01).is_none());
}

#[test]
fn test_basis_points_as_f64_rate() {
    assert_eq!(BasisPoints(6875).as_f64_rate(), 0.06875_f64);
    assert_eq!(BasisPoints(7000).as_f64_rate(), 0.07_f64);
    assert_eq!(BasisPoints(0).as_f64_rate(), 0.0_f64);

    // f64 roundtrip via from_apr_f64 is lossless for values at our precision
    for bps in [6875u32, 7000, 5500, 10_000, 1] {
        let rate_f64 = BasisPoints(bps).as_f64_rate();
        let back = BasisPoints::from_apr_f64(rate_f64).unwrap();
        assert_eq!(
            back,
            BasisPoints(bps),
            "f64 roundtrip failed for BasisPoints({bps})"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Property tests
// ─────────────────────────────────────────────────────────────────────────────

proptest! {
    /// Every integer number of cents round-trips losslessly through Decimal.
    /// Decimal can represent exact integers up to 10^28, far exceeding i64.
    #[test]
    fn prop_round_trip_decimal_cents(value in -99_999_999_999_i64..99_999_999_999) {
        let original = Cents(value);
        let d = original.to_decimal_dollars();
        let back = Cents::from_decimal_dollars(d)
            .expect("to_decimal_dollars output must always parse back");
        prop_assert_eq!(original, back, "Cents({}) did not roundtrip", value);
    }

    /// Every BasisPoints value round-trips through to_decimal_rate and
    /// from_decimal_rate within the representable range.
    #[test]
    fn prop_round_trip_decimal_rate(value in 0u32..100_000) {
        let original = BasisPoints(value);
        let rate = original.to_decimal_rate();
        let back = BasisPoints::from_decimal_rate(rate)
            .expect("to_decimal_rate output must always parse back");
        prop_assert_eq!(original, back, "BasisPoints({}) did not roundtrip", value);
    }

    /// from_apr_f64 and as_f64_rate are inverses for any BasisPoints value
    /// in the practical rate range (0–40%).
    #[test]
    fn prop_round_trip_apr_f64(value in 0u32..40_000) {
        let original = BasisPoints(value);
        let rate_f64 = original.as_f64_rate();
        let back = BasisPoints::from_apr_f64(rate_f64)
            .expect("as_f64_rate output must always convert back");
        prop_assert_eq!(original, back, "BasisPoints({}) f64 roundtrip failed", value);
    }

    /// from_decimal_dollars rejects values outside the Cents (i64) range.
    /// Values that fit in i64 cents must always succeed.
    #[test]
    fn prop_from_decimal_dollars_valid_range(
        dollars in -92_233_720_368_i64..92_233_720_368 // within i64 cents range
    ) {
        // Construct exact cent amount as Decimal
        let d = Decimal::from(dollars);
        let result = Cents::from_decimal_dollars(d);
        prop_assert!(result.is_ok(), "exact dollar amount {dollars} must succeed");
    }
}
