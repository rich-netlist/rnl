//! Stable row IDs in order, stored as nonzero 64-row blocks.

#[derive(Clone, Debug, Default)]
pub(super) struct RowSet {
    blocks: Vec<(usize, u64)>,
    len: usize,
}

impl RowSet {
    pub(super) fn insert(&mut self, row: usize) -> bool {
        let block = row / 64;
        let bit = 1u64 << (row % 64);
        match self.blocks.binary_search_by_key(&block, |(b, _)| *b) {
            Ok(i) => {
                if self.blocks[i].1 & bit != 0 {
                    return false;
                }
                self.blocks[i].1 |= bit;
            }
            Err(i) => self.blocks.insert(i, (block, bit)),
        }
        self.len += 1;
        true
    }

    pub(super) fn remove(&mut self, row: usize) -> bool {
        let block = row / 64;
        let bit = 1u64 << (row % 64);
        let Ok(i) = self.blocks.binary_search_by_key(&block, |(b, _)| *b) else { return false };
        if self.blocks[i].1 & bit == 0 {
            return false;
        }
        self.blocks[i].1 &= !bit;
        if self.blocks[i].1 == 0 {
            self.blocks.remove(i);
        }
        self.len -= 1;
        true
    }

    pub(super) fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub(super) fn iter(&self) -> Iter<'_> {
        Iter { blocks: self.blocks.iter(), base: 0, bits: 0, left: self.len }
    }
}

pub(super) struct Iter<'a> {
    blocks: std::slice::Iter<'a, (usize, u64)>,
    base: usize,
    bits: u64,
    left: usize,
}

impl Iterator for Iter<'_> {
    type Item = usize;

    fn next(&mut self) -> Option<usize> {
        if self.bits == 0 {
            let &(block, bits) = self.blocks.next()?;
            self.base = block * 64;
            self.bits = bits;
        }
        let bit = self.bits.trailing_zeros() as usize;
        self.bits &= self.bits - 1;
        self.left -= 1;
        Some(self.base + bit)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.left, Some(self.left))
    }
}

impl ExactSizeIterator for Iter<'_> {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn ordered_blocks_match_a_set_through_growth_removal_and_clone() {
        let mut rows = RowSet::default();
        let mut expected = BTreeSet::new();
        for row in [128, 0, 63, 64, 129, 4097, 64, usize::MAX] {
            assert_eq!(rows.insert(row), expected.insert(row));
            assert_eq!(rows.iter().len(), expected.len());
            assert_eq!(rows.iter().collect::<Vec<_>>(), expected.iter().copied().collect::<Vec<_>>());
        }
        let saved = rows.clone();
        for row in [63, 64, 64, 128, 129, 0, 4096, 4097, usize::MAX] {
            assert_eq!(rows.remove(row), expected.remove(&row));
            let mut iter = rows.iter();
            for &r in &expected {
                let before = iter.len();
                assert_eq!(iter.next(), Some(r));
                assert_eq!(iter.len(), before - 1);
            }
            assert_eq!(iter.next(), None);
            assert_eq!(iter.len(), 0);
        }
        assert!(rows.is_empty());
        assert_eq!(saved.iter().collect::<Vec<_>>(), [0, 63, 64, 128, 129, 4097, usize::MAX]);
    }

    #[test]
    fn deterministic_membership_changes_match_a_set() {
        let mut rows = RowSet::default();
        let mut expected = BTreeSet::new();
        let mut state = 0x9e37_79b9u32;
        for _ in 0..4000 {
            state = state.wrapping_mul(1664525).wrapping_add(1013904223);
            let row = (state >> 5) as usize % 1024;
            if state & 1 == 0 {
                assert_eq!(rows.insert(row), expected.insert(row));
            } else {
                assert_eq!(rows.remove(row), expected.remove(&row));
            }
            assert_eq!(rows.iter().collect::<Vec<_>>(), expected.iter().copied().collect::<Vec<_>>());
        }
    }
}
