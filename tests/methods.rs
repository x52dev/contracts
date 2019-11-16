/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! Testing of methods and `impl`-blocks.

use contracts::*;

#[cfg(feature = "mirai_assertions")]
mod mirai_assertion_mocks;

#[test]
fn methods() {
    fn is_even(x: usize) -> bool {
        x % 2 == 0
    }

    struct EvenAdder {
        count: usize,
    }

    impl EvenAdder {
        #[invariant(is_even(self.count))]
        #[post(self.count == old(self.count) + 2)]
        fn next_even(&mut self) {
            self.count += 2;
        }

        #[invariant(is_even(self.count))]
        #[pre(self.count >= 2)]
        #[post(self.count == old(self.count) - 2)]
        fn prev_even(&mut self) {
            self.count -= 2;
        }
    }

    let mut adder = EvenAdder { count: 0 };

    adder.next_even();
    adder.next_even();

    adder.prev_even();
    adder.prev_even();
}

#[test]
fn impl_invariant() {
    fn is_even(x: usize) -> bool {
        x % 2 == 0
    }

    struct EvenAdder {
        count: usize,
    }

    #[invariant(is_even(self.count), "Count has to always be even")]
    impl EvenAdder {
        const fn step() -> usize {
            2
        }

        fn new() -> Self {
            EvenAdder { count: 0 }
        }

        #[post(self.count == old(self.count) + 2)]
        fn next_even(&mut self) {
            self.count += Self::step();
        }

        #[pre(self.count >= 2)]
        #[post(self.count == old(self.count) - 2)]
        fn prev_even(&mut self) {
            self.count -= Self::step();
        }
    }

    let mut adder = EvenAdder::new();

    adder.next_even();
    adder.next_even();

    adder.prev_even();
    adder.prev_even();
}
