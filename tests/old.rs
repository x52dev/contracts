/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use contracts::*;

#[cfg(feature = "mirai_assertions")]
mod mirai_assertion_mocks;

#[test]
fn test_old_simple() {
    #[post(*x == old(*x) + 1, "x increments")]
    fn incr(x: &mut usize) {
        *x += 1;
    }

    let mut val = 0;
    incr(&mut val);
}

#[test]
fn test_old_nested() {
    #[post(*x == old(old(old(*x))) + 1, "x increments")]
    fn incr(x: &mut usize) {
        *x += 1;
    }

    let mut val = 0;
    incr(&mut val);
}

#[test]
#[should_panic(expected = "Post-condition of incr violated")]
fn test_violation() {
    #[post(*x == old(*x) + 1, "x increments")]
    fn incr(x: &mut usize) {
        *x += 0; // oops
    }

    let mut val = 0;
    incr(&mut val);
}
