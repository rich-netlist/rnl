use super::*;
use crate::rational::{frac, int};

#[test]
fn halves_go_up_to_the_grid() {
    let g = Some(int(1));
    assert_eq!(round(&frac(9, 2), &g), int(5));
    assert_eq!(round(&frac(-9, 2), &g), int(-4));
    assert_eq!(round(&frac(21, 5), &g), int(4));
    assert_eq!(round(&frac(21, 5), &None), frac(21, 5));
    assert_eq!(round(&frac(13, 10), &Some(frac(1, 2))), frac(3, 2));
}
