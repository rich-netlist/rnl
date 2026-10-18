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
