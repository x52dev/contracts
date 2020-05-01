/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use contracts::*;

#[cfg(feature = "mirai_assertions")]
mod mirai_assertion_mocks;

#[test]
fn adder_example() {
    #[contract_trait]
    trait Adder {
        fn tell(&self) -> usize;

        #[pre((self.tell() + val) < 20)]
        fn add(&mut self, val: usize);
    }

    struct MyAdder(usize);

    #[contract_trait]
    impl Adder for MyAdder {
        fn tell(&self) -> usize {
            self.0
        }

        fn add(&mut self, val: usize) {
            self.0 += val;
        }
    }

    let mut add = MyAdder(0);

    add.add(3);
    add.add(16);

    // this would violate the contract
    // add.add(2);
}

#[test]
fn interpolate_example() {
    #[contract_trait]
    trait Interpolate {
        #[pre(0.0 <= val, val <= 1.0)]
        #[pre(min < max)]
        #[post(min <= ret, ret <= max)]
        fn interpolate(min: f64, max: f64, val: f64) -> f64;
    }

    struct Linear;

    #[contract_trait]
    impl Interpolate for Linear {
        fn interpolate(min: f64, max: f64, val: f64) -> f64 {
            min + (val * (max - min))
        }
    }

    struct Quadratic;

    #[contract_trait]
    impl Interpolate for Quadratic {
        fn interpolate(min: f64, max: f64, val: f64) -> f64 {
            let val = val * val;

            Linear::interpolate(min, max, val)
        }
    }

    let min = 12.00;
    let max = 24.00;

    let val = 0.4;

    Linear::interpolate(min, max, val);
    Quadratic::interpolate(min, max, val);
}
