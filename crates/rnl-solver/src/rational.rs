//! Exact rational numbers for every decision of the solver (standard 8.5, 11.7).
//!
//! The resolution of 8.5 and the coordinate LPs compare numbers, so they compute with [`Q`], an
//! exact fraction: no decision depends on how floating point rounds, and native and WebAssembly
//! builds decide alike. Numbers come from the problem as `f64` and go back to the
//! record as `f64`, written by `fmt_num` (11.7, chapter 17):
//!
//! - [`from_f64`] reads a number as its shortest decimal representation, the digits `fmt_num`
//!   writes and RNL reads, so `0.1` is 1/10 and not the binary fraction nearest to it;
//! - [`to_f64`] rounds to the nearest `f64`, ties to even, so a number that came in goes back as
//!   the same `f64`, and one the solver computed reads back as the value it chose.

// Used by `simplex`, `model` and `resolve`; what only the coordinates use waits for stage S3.
#![cfg_attr(not(test), allow(dead_code))]

use std::cmp::Ordering;
use std::fmt;
use std::iter::Sum;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use num_bigint::{BigInt, BigUint, Sign};
use num_integer::Integer;
use num_rational::Ratio;
use num_traits::{One, Signed, ToPrimitive, Zero};
use rnl_runtime::geom::fmt_num;

/// An exact rational number. The denominator is positive and the fraction is in lowest terms, in
/// machine integers while it fits them, which is nearly always: the same value is always the same
/// form, so that comparing forms compares values. On overflow the operation is done again in big
/// integers, so no result differs from that of big integers alone.
#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) enum Q {
    /// The numerator, never `i64::MIN`, and the denominator, above 0.
    Small(i64, i64),
    /// One that does not fit.
    Big(Box<Ratio<BigInt>>),
}

fn gcd_u128(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

impl Q {
    /// The fraction `n / d`, `d` not 0, from machine integers wide enough to hold it.
    fn reduced(n: i128, d: i128) -> Q {
        assert!(d != 0, "denominator == 0");
        let (n, d) = if d < 0 { (-n, -d) } else { (n, d) };
        let g = gcd_u128(n.unsigned_abs(), d as u128).max(1) as i128;
        let (n, d) = (n / g, d / g);
        match (i64::try_from(n), i64::try_from(d)) {
            (Ok(a), Ok(b)) if a != i64::MIN => Q::Small(a, b),
            _ => Q::Big(Box::new(Ratio::new_raw(BigInt::from(n), BigInt::from(d)))),
        }
    }

    /// The form of a value in big integers, in lowest terms.
    fn of_big(r: Ratio<BigInt>) -> Q {
        match (r.numer().to_i64(), r.denom().to_i64()) {
            (Some(a), Some(b)) if a != i64::MIN => Q::Small(a, b),
            _ => Q::Big(Box::new(r)),
        }
    }

    fn big(&self) -> Ratio<BigInt> {
        match self {
            Q::Small(n, d) => Ratio::new_raw(BigInt::from(*n), BigInt::from(*d)),
            Q::Big(r) => (**r).clone(),
        }
    }

    /// The integer `n`.
    pub(crate) fn from_integer(n: BigInt) -> Q {
        Q::of_big(Ratio::from_integer(n))
    }

    /// The fraction `n / d`, reduced; `d` must not be 0.
    pub(crate) fn new(n: BigInt, d: BigInt) -> Q {
        Q::of_big(Ratio::new(n, d))
    }

    pub(crate) fn numer(&self) -> BigInt {
        match self {
            Q::Small(n, _) => BigInt::from(*n),
            Q::Big(r) => r.numer().clone(),
        }
    }

    pub(crate) fn denom(&self) -> BigInt {
        match self {
            Q::Small(_, d) => BigInt::from(*d),
            Q::Big(r) => r.denom().clone(),
        }
    }

    pub(crate) fn is_integer(&self) -> bool {
        match self {
            Q::Small(_, d) => *d == 1,
            Q::Big(r) => r.is_integer(),
        }
    }

    /// The integer of a number that is one; of another, its integer part.
    pub(crate) fn to_integer(&self) -> BigInt {
        match self {
            Q::Small(n, d) => BigInt::from(*n / *d),
            Q::Big(r) => r.to_integer(),
        }
    }

    pub(crate) fn floor(&self) -> Q {
        match self {
            Q::Small(n, d) => Q::reduced(i128::from(n.div_euclid(*d)), 1),
            Q::Big(r) => Q::of_big(r.floor()),
        }
    }

    pub(crate) fn ceil(&self) -> Q {
        match self {
            Q::Small(n, d) => Q::reduced(-i128::from((-*n).div_euclid(*d)), 1),
            Q::Big(r) => Q::of_big(r.ceil()),
        }
    }

    pub(crate) fn round(&self) -> Q {
        Q::of_big(self.big().round())
    }

    pub(crate) fn recip(&self) -> Q {
        match self {
            Q::Small(n, d) => Q::reduced(i128::from(*d), i128::from(*n)),
            Q::Big(r) => Q::of_big(r.recip()),
        }
    }

    pub(crate) fn abs(&self) -> Q {
        match self {
            Q::Small(n, d) => Q::Small(n.abs(), *d),
            Q::Big(r) => Q::of_big(r.abs()),
        }
    }

    pub(crate) fn signum(&self) -> Q {
        int(if self.is_positive() {
            1
        } else if self.is_negative() {
            -1
        } else {
            0
        })
    }

    pub(crate) fn is_positive(&self) -> bool {
        match self {
            Q::Small(n, _) => *n > 0,
            Q::Big(r) => r.is_positive(),
        }
    }

    pub(crate) fn is_negative(&self) -> bool {
        match self {
            Q::Small(n, _) => *n < 0,
            Q::Big(r) => r.is_negative(),
        }
    }
}

impl fmt::Debug for Q {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl fmt::Display for Q {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Q::Small(n, 1) => write!(f, "{n}"),
            Q::Small(n, d) => write!(f, "{n}/{d}"),
            Q::Big(r) => write!(f, "{r}"),
        }
    }
}

impl Ord for Q {
    fn cmp(&self, other: &Q) -> Ordering {
        match (self, other) {
            (Q::Small(a, b), Q::Small(c, d)) => {
                (i128::from(*a) * i128::from(*d)).cmp(&(i128::from(*c) * i128::from(*b)))
            }
            _ => self.big().cmp(&other.big()),
        }
    }
}

impl PartialOrd for Q {
    fn partial_cmp(&self, other: &Q) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Default for Q {
    fn default() -> Q {
        Q::Small(0, 1)
    }
}

impl Zero for Q {
    fn zero() -> Q {
        Q::Small(0, 1)
    }

    fn is_zero(&self) -> bool {
        matches!(self, Q::Small(0, _))
    }
}

impl One for Q {
    fn one() -> Q {
        Q::Small(1, 1)
    }
}

fn add(x: &Q, y: &Q) -> Q {
    match (x, y) {
        (Q::Small(a, b), Q::Small(c, d)) => {
            let (a, b, c, d) = (i128::from(*a), i128::from(*b), i128::from(*c), i128::from(*d));
            Q::reduced(a * d + c * b, b * d)
        }
        _ => Q::of_big(x.big() + y.big()),
    }
}

fn sub(x: &Q, y: &Q) -> Q {
    match (x, y) {
        (Q::Small(a, b), Q::Small(c, d)) => {
            let (a, b, c, d) = (i128::from(*a), i128::from(*b), i128::from(*c), i128::from(*d));
            Q::reduced(a * d - c * b, b * d)
        }
        _ => Q::of_big(x.big() - y.big()),
    }
}

fn mul(x: &Q, y: &Q) -> Q {
    match (x, y) {
        (Q::Small(a, b), Q::Small(c, d)) => {
            Q::reduced(i128::from(*a) * i128::from(*c), i128::from(*b) * i128::from(*d))
        }
        _ => Q::of_big(x.big() * y.big()),
    }
}

fn div(x: &Q, y: &Q) -> Q {
    match (x, y) {
        (Q::Small(a, b), Q::Small(c, d)) => {
            Q::reduced(i128::from(*a) * i128::from(*d), i128::from(*b) * i128::from(*c))
        }
        _ => Q::of_big(x.big() / y.big()),
    }
}

macro_rules! q_op {
    ($tr:ident, $method:ident, $f:ident, $atr:ident, $amethod:ident) => {
        impl $tr<&Q> for &Q {
            type Output = Q;
            fn $method(self, o: &Q) -> Q {
                $f(self, o)
            }
        }
        impl $tr<Q> for &Q {
            type Output = Q;
            fn $method(self, o: Q) -> Q {
                $f(self, &o)
            }
        }
        impl $tr<&Q> for Q {
            type Output = Q;
            fn $method(self, o: &Q) -> Q {
                $f(&self, o)
            }
        }
        impl $tr<Q> for Q {
            type Output = Q;
            fn $method(self, o: Q) -> Q {
                $f(&self, &o)
            }
        }
        impl $atr<&Q> for Q {
            fn $amethod(&mut self, o: &Q) {
                *self = $f(self, o);
            }
        }
        impl $atr<Q> for Q {
            fn $amethod(&mut self, o: Q) {
                *self = $f(self, &o);
            }
        }
    };
}

q_op!(Add, add, add, AddAssign, add_assign);
q_op!(Sub, sub, sub, SubAssign, sub_assign);
q_op!(Mul, mul, mul, MulAssign, mul_assign);
q_op!(Div, div, div, DivAssign, div_assign);

impl Neg for Q {
    type Output = Q;
    fn neg(self) -> Q {
        -&self
    }
}

impl Neg for &Q {
    type Output = Q;
    fn neg(self) -> Q {
        match self {
            Q::Small(n, d) => Q::Small(-*n, *d),
            Q::Big(r) => Q::of_big(-(**r).clone()),
        }
    }
}

impl Sum for Q {
    fn sum<I: Iterator<Item = Q>>(iter: I) -> Q {
        iter.fold(Q::zero(), |a, b| a + b)
    }
}

impl<'a> Sum<&'a Q> for Q {
    fn sum<I: Iterator<Item = &'a Q>>(iter: I) -> Q {
        iter.fold(Q::zero(), |a, b| a + b)
    }
}

/// The integer `n`.
pub(crate) fn int(n: i64) -> Q {
    Q::reduced(i128::from(n), 1)
}

/// The fraction `n / d`, reduced; `d` must not be 0.
pub(crate) fn frac(n: i64, d: i64) -> Q {
    Q::reduced(i128::from(n), i128::from(d))
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
mod tests {
    use super::*;

    /// A fixed xorshift sequence, so the tests are the same on every run.
    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }
    }

    #[test]
    fn numbers_are_read_as_their_shortest_decimals() {
        assert_eq!(from_f64(0.1), Some(frac(1, 10)));
        assert_eq!(from_f64(-2.5), Some(frac(-5, 2)));
        assert_eq!(from_f64(1e-6), Some(frac(1, 1_000_000)));
        assert_eq!(from_f64(3.0), Some(int(3)));
        assert_eq!(from_f64(-0.0), Some(int(0)));
        assert_eq!(from_f64(0.3), Some(frac(3, 10)));
        let big = from_f64(1e300).unwrap();
        assert_eq!(big, Q::from_integer(num_traits::pow(BigInt::from(10), 300)));
        assert_eq!(from_f64(f64::NAN), None);
        assert_eq!(from_f64(f64::INFINITY), None);
        assert_eq!(from_f64(f64::NEG_INFINITY), None);
    }

    #[test]
    fn decimals_parse_in_every_written_form() {
        assert_eq!(parse_decimal("12"), Some(int(12)));
        assert_eq!(parse_decimal("-12.50"), Some(frac(-25, 2)));
        assert_eq!(parse_decimal("+.5"), Some(frac(1, 2)));
        assert_eq!(parse_decimal("5."), Some(int(5)));
        assert_eq!(parse_decimal("1e3"), Some(int(1000)));
        assert_eq!(parse_decimal("2.5E-2"), Some(frac(1, 40)));
        assert_eq!(parse_decimal("0.000001"), Some(frac(1, 1_000_000)));
        for bad in ["", "-", ".", "1.2.3", "1e", "e5", "1e+", "0x10", "1 ", " 1", "1e99999", "nan", "inf", "1_0"] {
            assert_eq!(parse_decimal(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn every_f64_comes_back_as_itself() {
        let special = [
            0.1,
            1.0 / 3.0,
            2.0 / 3.0,
            -7.25,
            1e-310,
            5e-324,
            f64::MIN_POSITIVE,
            f64::MIN_POSITIVE / 3.0,
            f64::MAX,
            -f64::MAX,
            f64::EPSILON,
            123_456_789.123_456_79,
            9007199254740993.0,
            1e22,
            1e23,
        ];
        for v in special {
            assert_eq!(to_f64(&from_f64(v).unwrap()).to_bits(), v.to_bits(), "{v:e}");
        }
        let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
        let mut seen = 0;
        while seen < 3000 {
            let v = f64::from_bits(rng.next());
            if !v.is_finite() {
                continue;
            }
            seen += 1;
            let back = to_f64(&from_f64(v).unwrap());
            assert_eq!(back.to_bits(), if v == 0.0 { 0.0f64.to_bits() } else { v.to_bits() }, "{v:e}");
        }
    }

    #[test]
    fn fractions_round_to_nearest_like_ieee_division() {
        // n / d with both exact in f64: the division is correctly rounded, an oracle for to_f64
        let mut rng = Rng(0x2545_f491_4f6c_dd1d);
        for _ in 0..3000 {
            let n = (rng.next() >> (11 + rng.next() % 50)) as i64;
            let d = ((rng.next() >> (11 + rng.next() % 50)) as i64).max(1);
            let n = if rng.next() % 2 == 0 { n } else { -n };
            assert_eq!(to_f64(&frac(n, d)).to_bits(), (n as f64 / d as f64).to_bits(), "{n}/{d}");
        }
    }

    #[test]
    fn long_decimals_round_like_the_standard_library() {
        // `str::parse::<f64>` rounds correctly, another oracle, with many digits and exponents
        let mut rng = Rng(0x1234_5678_9abc_def1);
        for _ in 0..2000 {
            let digits: String = (0..1 + rng.next() % 40).map(|_| char::from(b'0' + (rng.next() % 10) as u8)).collect();
            let point = (rng.next() % (digits.len() as u64 + 1)) as usize;
            let exp = (rng.next() % 700) as i64 - 350;
            let s = format!("{}.{}e{}", &digits[..point], &digits[point..], exp);
            let expected: f64 = s.parse().unwrap();
            assert_eq!(to_f64(&parse_decimal(&s).unwrap()).to_bits(), expected.to_bits(), "{s}");
        }
    }

    #[test]
    fn ties_go_to_even_and_extremes_saturate() {
        let one = BigInt::from(1);
        let two53 = BigInt::from(1u64 << 53);
        // 1 + 2^-53 is halfway between 1 and the next f64: to 1, whose last bit is even
        assert_eq!(to_f64(&Q::new(&two53 + &one, two53.clone())), 1.0);
        // 1 + 3 * 2^-53 is halfway between 1 + 2^-52 (odd) and 1 + 2^-51 (even)
        assert_eq!(to_f64(&Q::new(&two53 + BigInt::from(3), two53.clone())), 1.0 + 2f64.powi(-51));
        // just above halfway rounds up
        let two54 = BigInt::from(1u64 << 54);
        assert_eq!(to_f64(&Q::new(&two54 + BigInt::from(3), two54.clone())), 1.0 + 2f64.powi(-52));
        // beyond the largest f64, and below half the smallest subnormal
        let huge = Q::from_integer(num_traits::pow(BigInt::from(2), 1024));
        assert_eq!(to_f64(&huge), f64::INFINITY);
        assert_eq!(to_f64(&-huge), f64::NEG_INFINITY);
        let tiny = Q::new(one.clone(), num_traits::pow(BigInt::from(2), 1076));
        assert_eq!(to_f64(&tiny), 0.0);
        // exactly half the smallest subnormal goes to the even 0, a little more to 2^-1074
        let half = Q::new(one.clone(), num_traits::pow(BigInt::from(2), 1075));
        assert_eq!(to_f64(&half), 0.0);
        let more = Q::new(BigInt::from(3), num_traits::pow(BigInt::from(2), 1076));
        assert_eq!(to_f64(&more), 5e-324);
        // the largest f64, (2^53 − 1) 2^971 exactly, plus less than half an ulp stays the largest;
        // plus half an ulp is a tie, and its even neighbour 2^1024 is beyond every f64
        let max = Q::from_integer((num_traits::pow(BigInt::from(2), 53) - 1) << 971u32);
        assert_eq!(to_f64(&max), f64::MAX);
        let half_ulp = Q::from_integer(num_traits::pow(BigInt::from(2), 970));
        assert_eq!(to_f64(&(&max + &half_ulp - int(1))), f64::MAX);
        assert_eq!(to_f64(&(&max + &half_ulp)), f64::INFINITY);
    }

    #[test]
    fn written_as_rnl_writes_numbers() {
        assert_eq!(fmt(&frac(1, 10)), "0.1");
        assert_eq!(fmt(&int(-4)), "-4");
        assert_eq!(fmt(&frac(5, 2)), "2.5");
        assert_eq!(fmt(&frac(1, 3)), "0.3333333333333333");
    }
}

#[cfg(test)]
mod fast_path {
    use super::*;

    /// Every operation in machine integers gives what big integers give, near the edges of what fits
    /// too, on values that do not fit and on mixes of both, and the same value is always the same
    /// form.
    #[test]
    fn the_machine_integers_give_what_big_integers_give() {
        let edges =
            [0, 1, -1, 2, 3, 7, 10, 1 << 31, (1 << 31) + 1, i64::MAX, i64::MAX - 1, -i64::MAX, i64::MIN + 1, i64::MIN];
        let big = |n: i64, d: i64| Ratio::new(BigInt::from(n), BigInt::from(d));
        let mut values: Vec<(Q, Ratio<BigInt>)> = Vec::new();
        for &n in &edges {
            for &d in &[1, 2, 3, 10, 1 << 31, i64::MAX, i64::MAX - 1, -1, -3, i64::MIN] {
                values.push((frac(n, d), big(n, d)));
            }
        }
        // values that do not fit: 2^63 and its negation, 2^64 / 3, (2^63 + 1) / 2, 3 / 2^64
        let two63 = BigInt::from(1u64 << 63);
        let two64 = BigInt::from(1u128 << 64);
        for (n, d) in [
            (two63.clone(), BigInt::from(1)),
            (-two63.clone(), BigInt::from(1)),
            (two64.clone(), BigInt::from(3)),
            (two63.clone() + 1, BigInt::from(2)),
            (BigInt::from(3), two64.clone()),
        ] {
            let r = Ratio::new(n.clone(), d.clone());
            let q = Q::new(n, d);
            assert!(matches!(q, Q::Big(_)), "{q}");
            values.push((q, r));
        }
        let same = |q: &Q, r: &Ratio<BigInt>, what: &str| {
            assert_eq!(&q.big(), r, "{what}");
            assert_eq!(*q, Q::of_big(r.clone()), "{what}: the form");
            assert_eq!(q.to_string(), r.to_string(), "{what}: the text");
        };
        for (x, rx) in &values {
            same(x, rx, "from");
            same(&-x, &-rx.clone(), "neg");
            same(&x.abs(), &rx.abs(), "abs");
            same(&x.floor(), &rx.floor(), "floor");
            same(&x.ceil(), &rx.ceil(), "ceil");
            same(&x.round(), &rx.round(), "round");
            same(&x.signum(), &rx.signum(), "signum");
            assert_eq!(x.to_integer(), rx.to_integer());
            assert_eq!(&x.numer(), rx.numer());
            assert_eq!(&x.denom(), rx.denom());
            assert_eq!(x.is_integer(), rx.is_integer());
            assert_eq!(x.is_zero(), rx.is_zero());
            assert_eq!(x.is_positive(), rx.is_positive());
            assert_eq!(x.is_negative(), rx.is_negative());
            if !rx.is_zero() {
                same(&x.recip(), &rx.recip(), "recip");
            }
            for (y, ry) in &values {
                same(&(x + y), &(rx + ry), "add");
                same(&(x - y), &(rx - ry), "sub");
                same(&(x * y), &(rx * ry), "mul");
                if !ry.is_zero() {
                    same(&(x / y), &(rx / ry), "div");
                }
                assert_eq!(x.cmp(y), rx.cmp(ry));
            }
        }
    }
}
