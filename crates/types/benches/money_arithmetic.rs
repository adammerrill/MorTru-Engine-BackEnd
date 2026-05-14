//! Criterion benchmarks for money/rate arithmetic.
//!
//! Run with: `cargo bench --bench money_arithmetic`
//!
//! Target: every arithmetic operation under 5 ns with zero allocations on
//! the hot path. Display formatting allocates (it returns a String); the
//! goal there is to stay sub-microsecond.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use types::{BasisPoints, Cents, LtvBasisPoints, PriceTicks};

fn bench_cents_add(c: &mut Criterion) {
    c.bench_function("cents_checked_add", |b| {
        let x = Cents(123_456);
        let y = Cents(789_012);
        b.iter(|| black_box(x).checked_add(black_box(y)));
    });
    c.bench_function("cents_saturating_add", |b| {
        let x = Cents(123_456);
        let y = Cents(789_012);
        b.iter(|| black_box(x).saturating_add(black_box(y)));
    });
}

fn bench_cents_mul(c: &mut Criterion) {
    c.bench_function("cents_checked_mul", |b| {
        let x = Cents(123_456);
        b.iter(|| black_box(x).checked_mul(black_box(360)));
    });
}

fn bench_basis_points_to_decimal(c: &mut Criterion) {
    c.bench_function("basis_points_to_decimal_rate", |b| {
        let bp = BasisPoints(6875);
        b.iter(|| black_box(bp).to_decimal_rate());
    });
    c.bench_function("basis_points_to_decimal_percent", |b| {
        let bp = BasisPoints(6875);
        b.iter(|| black_box(bp).to_decimal_percent());
    });
}

fn bench_basis_points_parse(c: &mut Criterion) {
    c.bench_function("basis_points_from_str_6_875", |b| {
        b.iter(|| BasisPoints::from_percentage_str(black_box("6.875")));
    });
}

fn bench_price_ticks_apply(c: &mut Criterion) {
    c.bench_function("price_ticks_apply_to_loan", |b| {
        let price = PriceTicks(-32810);
        let loan = Cents(20_000_000); // $200,000
        b.iter(|| black_box(price).apply_to_loan(black_box(loan)));
    });
}

fn bench_ltv_calculation(c: &mut Criterion) {
    c.bench_function("ltv_from_loan_and_value", |b| {
        let loan = Cents(28_500_000); // $285,000
        let value = Cents(30_000_000); // $300,000
        b.iter(|| LtvBasisPoints::from_loan_and_value(black_box(loan), black_box(value)));
    });
}

fn bench_cents_display(c: &mut Criterion) {
    c.bench_function("cents_to_string_1234_56", |b| {
        let x = Cents(123_456);
        b.iter(|| black_box(x).to_string());
    });
    c.bench_function("cents_to_string_large_with_commas", |b| {
        let x = Cents(123_456_789_012); // $1,234,567,890.12
        b.iter(|| black_box(x).to_string());
    });
}

criterion_group!(
    money_arithmetic,
    bench_cents_add,
    bench_cents_mul,
    bench_basis_points_to_decimal,
    bench_basis_points_parse,
    bench_price_ticks_apply,
    bench_ltv_calculation,
    bench_cents_display,
);
criterion_main!(money_arithmetic);
