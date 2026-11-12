//! Compact membership keys for search deduplication, never candidate ordering.
//! A collision may skip a candidate; hard facts are still checked independently.

use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;

use crate::rational::Q;

pub(super) type Fingerprint = (u64, u64);

pub(super) struct Look(DefaultHasher, DefaultHasher);

impl Look {
    pub(super) fn new() -> Self {
        let mut a = DefaultHasher::new();
        let mut b = DefaultHasher::new();
        a.write(&[0]);
        b.write(&[1]);
        Self(a, b)
    }

    fn write(&mut self, bytes: &[u8]) {
        self.0.write(bytes);
        self.1.write(bytes);
    }

    /// Explicit lengths and little-endian numbers keep the input identical on
    /// native and wasm; derived Hash can depend on usize and BigInt limb widths.
    pub(super) fn row(&mut self, row: &[Q]) {
        self.write(&(row.len() as u64).to_le_bytes());
        for q in row {
            match q {
                Q::Small(n, d) => {
                    self.write(&[0]);
                    self.write(&n.to_le_bytes());
                    self.write(&d.to_le_bytes());
                }
                Q::Big(r) => {
                    self.write(&[1]);
                    for n in [r.numer(), r.denom()] {
                        let bytes = n.to_signed_bytes_le();
                        self.write(&(bytes.len() as u64).to_le_bytes());
                        self.write(&bytes);
                    }
                }
            }
        }
    }

    pub(super) fn finish(self) -> Fingerprint {
        (self.0.finish(), self.1.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rational::int;

    #[test]
    fn rows_signs_and_large_rationals_have_distinct_keys() {
        let key = |rows: &[Vec<Q>]| {
            let mut look = Look::new();
            for row in rows {
                look.row(row);
            }
            look.finish()
        };
        let big = Q::from_integer(num_bigint::BigInt::from(1u64) << 130);
        let cases = [
            vec![],
            vec![vec![]],
            vec![vec![int(1), int(2)]],
            vec![vec![int(1)], vec![int(2)]],
            vec![vec![int(-1), int(2)]],
            vec![vec![big.clone()]],
            vec![vec![-big]],
        ];
        let keys: std::collections::BTreeSet<_> = cases.iter().map(|rows| key(rows)).collect();
        assert_eq!(keys.len(), cases.len());
        assert_eq!(key(&cases[2]), key(&cases[2]));
    }
}
