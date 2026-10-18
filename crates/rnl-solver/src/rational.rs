//! Exact rational numbers for every decision of the solver (standard 8.5, 11.7).
//!
//! The resolution of 8.5 and the coordinate LPs compare numbers, so they compute with [`Q`], a
//! fraction of two big integers: no decision depends on how floating point rounds, and native
//! and WebAssembly builds decide alike. Numbers come from the problem as `f64` and go back to the
//! record as `f64`, written by `fmt_num` (11.7, chapter 17):
//!
//! - [`from_f64`] reads a number as its shortest decimal representation, the digits `fmt_num`
//!   writes and RNL reads, so `0.1` is 1/10 and not the binary fraction nearest to it;
//! - [`to_f64`] rounds to the nearest `f64`, ties to even, so a number that came in goes back as
//!   the same `f64`, and one the solver computed reads back as the value it chose.

// Used by `simplex`, `model` and `resolve`; what only the coordinates use waits for stage S3.
#![cfg_attr(not(test), allow(dead_code))]

use num_bigint::{BigInt, BigUint, Sign};
use num_integer::Integer;
use num_rational::Ratio;
use num_traits::{ToPrimitive, Zero};
use rnl_runtime::geom::fmt_num;

/// An exact rational number. The denominator is positive and the fraction is in lowest terms.
pub type Q = Ratio<BigInt>;

/// A reduced, exact fraction for configuration, hashes and score reports.
pub fn exact_text(q: &Q) -> String {
    if q.denom() == &BigInt::from(1) {
        q.numer().to_string()
    } else {
        format!("{}/{}", q.numer(), q.denom())
    }
}

/// The integer `n`.
pub(crate) fn int(n: i64) -> Q {
    Q::from_integer(BigInt::from(n))
}

/// The fraction `n / d`, reduced; `d` must not be 0.
pub(crate) fn frac(n: i64, d: i64) -> Q {
    Q::new(BigInt::from(n), BigInt::from(d))
}

/// A number of the problem as its shortest decimal representation, the one `fmt_num` writes:
/// `0.1` is exactly 1/10. None for NaN and the infinities, which RNL cannot write (3.7).
pub(crate) fn from_f64(v: f64) -> Option<Q> {
    if !v.is_finite() {
        return None;
    }
    // `Display` writes the shortest digits that read back as `v`, never with an exponent.
    parse_decimal(&format!("{v}"))
}

/// A decimal number as written: an optional sign, digits with an optional `.` and fraction
/// digits (at least one digit in all), and an optional exponent `e` or `E` with an optional sign.
/// None for anything else, and for exponents beyond ±4096, which no number of RNL needs.
pub(crate) fn parse_decimal(s: &str) -> Option<Q> {
    let (negative, rest) = match s.as_bytes().first()? {
        b'-' => (true, &s[1..]),
        b'+' => (false, &s[1..]),
        _ => (false, s),
    };
    let (mantissa, exponent) = match rest.find(['e', 'E']) {
        Some(i) => (&rest[..i], Some(&rest[i + 1..])),
        None => (rest, None),
    };
    let (whole, fraction) = match mantissa.find('.') {
        Some(i) => (&mantissa[..i], &mantissa[i + 1..]),
        None => (mantissa, ""),
    };
    let digits = |t: &str| t.bytes().all(|b| b.is_ascii_digit());
    if whole.len() + fraction.len() == 0 || !digits(whole) || !digits(fraction) {
        return None;
    }
    let mut exp: i64 = 0;
    if let Some(e) = exponent {
        let (sign, body) = match e.as_bytes().first()? {
            b'-' => (-1, &e[1..]),
            b'+' => (1, &e[1..]),
            _ => (1, e),
        };
        if body.is_empty() || body.len() > 4 || !digits(body) {
            return None;
        }
        exp = sign * body.parse::<i64>().ok()?;
        if exp.abs() > 4096 {
            return None;
        }
    }
    let all: String = [whole, fraction].concat();
    let mut n: BigInt = all.parse().ok()?;
    if negative {
        n = -n;
    }
    exp -= fraction.len() as i64;
    let ten = BigInt::from(10);
    let scale = num_traits::pow(ten, exp.unsigned_abs() as usize);
    Some(if exp >= 0 { Q::from_integer(n * scale) } else { Q::new(n, scale) })
}

/// The `f64` nearest to `q`, ties to even, as IEEE 754 rounds; infinite beyond the largest
/// `f64`. A number [`from_f64`] made comes back as the same `f64` (−0 as 0).
pub(crate) fn to_f64(q: &Q) -> f64 {
    let (n, d) = (q.numer(), q.denom());
    if n.is_zero() {
        return 0.0;
    }
    let negative = n.sign() == Sign::Minus;
    let (n, d): (&BigUint, &BigUint) = (n.magnitude(), d.magnitude());
    // e = floor(log2(n / d)): n / d lies in (2^(e0 - 1), 2^(e0 + 1)) with e0 the difference of the
    // bit lengths, and is at least 2^e0 exactly when n * 2^-e0 >= d.
    let e0 = n.bits() as i64 - d.bits() as i64;
    let e = if shl(n, (-e0).max(0)) >= shl(d, e0.max(0)) { e0 } else { e0 - 1 };
    let magnitude = if e > 1023 {
        f64::INFINITY
    } else {
        // the bits of precision at this exponent: 53, fewer for subnormal numbers
        let p = if e >= -1022 { 53 } else { 1075 + e };
        // m = n / d in units of the last place, 2^(e - p + 1), rounded half to even
        let s = p - 1 - e;
        let (num, den) = if s >= 0 { (shl(n, s), d.clone()) } else { (n.clone(), shl(d, -s)) };
        let (mut m, r) = num.div_rem(&den);
        let twice: BigUint = r << 1u32;
        if twice > den || (twice == den && m.is_odd()) {
            m += 1u32;
        }
        // m <= 2^53, exact as an f64; the product is exact or overflows to infinity
        let m = m.to_u64().map_or(f64::INFINITY, |m| m as f64);
        m * pow2(e - p + 1)
    };
    if negative {
        -magnitude
    } else {
        magnitude
    }
}

/// `q` as RNL writes it: rounded to the nearest `f64`, then by `fmt_num` (11.7).
pub(crate) fn fmt(q: &Q) -> String {
    fmt_num(to_f64(q))
}

fn shl(n: &BigUint, by: i64) -> BigUint {
    n << (by as u64)
}

/// 2^k for k from −1074 to 1023, the powers of two that are `f64`.
fn pow2(k: i64) -> f64 {
    debug_assert!((-1074..=1023).contains(&k));
    if k >= -1022 {
        f64::from_bits(((k + 1023) as u64) << 52)
    } else {
        f64::from_bits(1u64 << (k + 1074))
    }
}

#[cfg(test)]
#[path = "../tests/unit/rational.rs"]
mod tests;
