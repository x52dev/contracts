/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! Testing of simple functions.

use contracts::{invariant, post, pre};
use quickcheck_macros::quickcheck;

#[test]
fn test_a_thing() {
    #[pre(x > 10, x < 20, "x must be in valid range")]
    #[post(ret > x, "result will be bigger than input")]
    fn a(x: usize) -> usize {
        x + 1
    }

    a(15);
}

#[quickcheck]
fn test_sort(input: Vec<usize>) {
    fn is_sorted(input: &[usize]) -> bool {
        if input.len() < 2 {
            return true;
        }

        for i in 1..input.len() {
            if input[i - 1] > input[i] {
                return false;
            }
        }

        true
    }

    #[post(ret.len() == input.len())]
    #[post(is_sorted(&ret))]
    fn sort(input: &[usize]) -> Vec<usize> {
        let mut vec = input.to_owned();

        vec.sort();

        vec
    }

    sort(&input);
}

#[test]
fn test_invariant() {
    #[invariant(*val <= 10)]
    fn add_to_10(val: &mut usize) {
        if *val >= 10 {
            return;
        }
        *val += 1;
    }

    let mut val = 8;

    add_to_10(&mut val);
    add_to_10(&mut val);
    add_to_10(&mut val);
    add_to_10(&mut val);
    add_to_10(&mut val);
}
