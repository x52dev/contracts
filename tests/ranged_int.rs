/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! Test implementing a `RangedInt` type.

use contracts::*;
use std::ops::{Add, Deref};

#[cfg(feature = "mirai_assertions")]
mod mirai_assertion_mocks;

#[derive(Copy, Clone, Debug)]
pub struct Range {
    min: usize,
    max: usize,
}

impl Range {
    #[requires(min < max)]
    pub fn new(min: usize, max: usize) -> Self {
        Range { min, max }
    }

    #[ensures(ret.min == self.min.min(other.min))]
    #[ensures(ret.max == self.max.max(other.max))]
    pub fn merge(self, other: Range) -> Range {
        let min = self.min.min(other.min);
        let max = self.max.max(other.max);

        Range::new(min, max)
    }

    pub fn contains(self, val: usize) -> bool {
        self.min <= val && val <= self.max
    }
}

#[derive(Copy, Clone, Debug)]
pub struct RangedInt {
    range: Range,
    value: usize,
}

#[invariant(self.range.contains(self.value))]
impl RangedInt {
    #[requires(range.contains(val))]
    pub fn new(range: Range, val: usize) -> Self {
        RangedInt { range, value: val }
    }

    pub fn to_usize(self) -> usize {
        self.value
    }

    pub fn range(self) -> Range {
        self.range
    }

    #[ensures(ret.range.contains(ret.value))]
    pub fn extend(self, range: Range) -> Self {
        let new_range = self.range.merge(range);

        RangedInt::new(new_range, self.value)
    }
}

impl Add<Self> for RangedInt {
    type Output = RangedInt;

    #[requires(self.range.merge(rhs.range).contains(self.value + rhs.value))]
    #[ensures(ret.range.contains(ret.value))]
    #[ensures(ret.value == self.value + rhs.value)]
    fn add(self, rhs: Self) -> Self::Output {
        let mut new_ranged = self.extend(rhs.range);

        new_ranged.value += rhs.value;

        new_ranged
    }
}

impl Add<usize> for RangedInt {
    type Output = RangedInt;

    #[requires(self.range.contains(rhs))]
    #[ensures(ret.value == self.value + rhs)]
    fn add(self, rhs: usize) -> Self::Output {
        let mut new = self;
        new.value += rhs;
        new
    }
}

impl Deref for RangedInt {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

#[test]
fn ranged_int_add() {
    let days = RangedInt::new(Range::new(0, 6), 0);

    assert_eq!(*days, 0);

    assert_eq!(*(days + 1), 1);
    assert_eq!(*(days + 5), 5);
    assert_eq!(*(days + 6), 6);
}

#[test]
fn ranged_int_add_reassign() {
    let mut days = RangedInt::new(Range::new(0, 6), 0);
    assert_eq!(*days, 0);

    days = days + 3;
    assert_eq!(*days, 3);

    days = days + 2;
    assert_eq!(*days, 5);
}

#[test]
#[should_panic(expected = "Pre-condition of add violated")]
fn ranged_overflow() {
    let days = RangedInt::new(Range::new(0, 6), 0);

    // cause overflow
    let _ = days + 7;
}

#[test]
#[should_panic(expected = "Pre-condition of new violated")]
fn construction_underflow() {
    // value below of permitted range
    RangedInt::new(Range::new(4, 6), 0);
}

#[test]
#[should_panic(expected = "Pre-condition of new violated")]
fn construction_overflow() {
    // value above of permitted range
    RangedInt::new(Range::new(0, 6), 8);
}
