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
        #[ensures(self.count == old(self.count) + 2)]
        fn next_even(&mut self) {
            self.count += 2;
        }

        // Manually express the invariant in terms of `ret` since `self.count` is mutably borrowed.
        #[requires(is_even(self.count))]
        #[ensures(is_even(*ret))]
        #[ensures(*ret == old(self.count) + 2)]
        fn next_even_and_get<'a>(&'a mut self) -> &'a mut usize {
            self.count += 2;
            &mut self.count
        }

        #[invariant(is_even(self.count))]
        #[requires(self.count >= 2)]
        #[ensures(self.count == old(self.count) - 2)]
        fn prev_even(&mut self) {
            self.count -= 2;
        }

        #[invariant(is_even(self.count))]
        fn this_var_collision(&mut self) -> usize {
            #[allow(unused_variables)]
            let (this, this__) = (42, 42);
            self.count
        }
    }

    let mut adder = EvenAdder { count: 0 };

    adder.next_even();
    adder.next_even();

    adder.prev_even();
    adder.prev_even();

    assert_eq!(*adder.next_even_and_get(), 2);
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

        #[ensures(self.count == old(self.count) + 2)]
        fn next_even(&mut self) {
            self.count += Self::step();
        }

        #[requires(self.count >= 2)]
        #[ensures(self.count == old(self.count) - 2)]
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
