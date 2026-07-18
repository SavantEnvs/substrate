//! Integration tests for sp-arithmetic's public API.
//!
//! These tests exercise the REAL mathematical operations of sp-arithmetic and
//! serve as the behavioral oracle for mayhem/test.sh: a patch that neuters
//! sp-arithmetic to exit(0) does not produce the correct arithmetic results and
//! fails these assertions. Compiled by build.sh (cargo test --no-run in the fuzz
//! workspace) and run by test.sh.
//!
//! The fuzz workspace (mayhem/fuzz/) has its own Cargo.lock that resolves fresh
//! compatible deps (hashbrown 0.17.x with bundled ahash, no legacy ahash 0.7.6).

use sp_arithmetic::*;
use sp_arithmetic::traits::{CheckedAdd, One, Zero};

// --- Permill ---
#[test]
fn permill_from_parts_roundtrip() {
    let p = Permill::from_parts(500_000);
    assert_eq!(p.deconstruct(), 500_000, "Permill 50% must preserve value");
}

#[test]
fn permill_multiplication_u64() {
    let half = Permill::from_parts(500_000);
    let result: u64 = half * 200u64;
    assert_eq!(result, 100, "50% of 200 must be 100");
}

#[test]
fn permill_from_rational_half() {
    let half = Permill::from_rational(1u32, 2u32);
    assert_eq!(half.deconstruct(), 500_000, "1/2 in Permill must be 500_000");
}

#[test]
fn permill_zero() {
    let z = Permill::zero();
    assert_eq!(z.deconstruct(), 0);
    assert!(z.is_zero());
}

#[test]
fn permill_one() {
    let one = Permill::one();
    assert_eq!(one.deconstruct(), 1_000_000, "Permill::one must be 1_000_000");
}

// --- Perbill ---
#[test]
fn perbill_from_parts_roundtrip() {
    let pb = Perbill::from_parts(750_000_000);
    assert_eq!(pb.deconstruct(), 750_000_000, "Perbill 75% must preserve value");
}

#[test]
fn perbill_zero_and_one() {
    assert_eq!(Perbill::zero().deconstruct(), 0);
    assert_eq!(Perbill::one().deconstruct(), 1_000_000_000);
}

#[test]
fn perbill_multiplication_zero() {
    let pb = Perbill::from_parts(500_000_000);
    let result: u128 = pb * 0u128;
    assert_eq!(result, 0, "50% of 0 must be 0");
}

#[test]
fn perbill_from_rational_quarter() {
    let q = Perbill::from_rational(1u32, 4u32);
    assert_eq!(q.deconstruct(), 250_000_000, "1/4 in Perbill must be 250_000_000");
}

// --- Percent ---
#[test]
fn percent_from_parts_roundtrip() {
    let pc = Percent::from_parts(75u8);
    assert_eq!(pc.deconstruct(), 75, "Percent 75 must preserve value");
}

#[test]
fn percent_multiplication_u64() {
    let half = Percent::from_parts(50u8);
    let result: u64 = half * 200u64;
    assert_eq!(result, 100, "50% of 200 must be 100");
}

// --- FixedU128 ---
#[test]
fn fixed_u128_addition() {
    let a = FixedU128::from_inner(1_500_000_000_000_000_000u128);
    let b = FixedU128::from_inner(1_000_000_000_000_000_000u128);
    let sum = a.checked_add(&b).expect("must not overflow for these values");
    assert_eq!(
        sum.into_inner(),
        2_500_000_000_000_000_000u128,
        "FixedU128 1.5 + 1.0 must equal 2.5"
    );
}

#[test]
fn fixed_u128_zero_and_one() {
    assert_eq!(FixedU128::zero().into_inner(), 0);
    assert_eq!(FixedU128::one().into_inner(), 1_000_000_000_000_000_000u128);
}

// --- Rational128 ---
#[test]
fn rational128_numerator_denominator() {
    let r = Rational128::from(3u128, 7u128);
    assert_eq!(r.n(), 3, "numerator must be 3");
    assert_eq!(r.d(), 7, "denominator must be 7");
}

// --- Saturating ops ---
#[test]
fn saturating_add_overflow() {
    let result: u32 = u32::MAX.saturating_add(1);
    assert_eq!(result, u32::MAX, "saturating_add at max must stay at max");
}

#[test]
fn saturating_mul_overflow() {
    let result: u32 = u32::MAX.saturating_mul(2);
    assert_eq!(result, u32::MAX, "saturating_mul overflow must saturate");
}
