# *Design By Contract* for Rust

[![License][license]][LICENSE]
![Build status][build]
![Lines of Code][loc]

[license]: https://img.shields.io/badge/license-MPL%202.0-blue.svg
[build]: https://gitlab.com/karroffel/contracts/badges/master/build.svg
[loc]: https://tokei.rs/b1/gitlab/karroffel/contracts?category=code

Annotate functions and methods with "contracts", using *invariants*, *pre-conditions* and *post-conditions*.

[Design by contract][dbc] is a popular method to augment code with formal interface specifications.
These specifications are used to increase the correctness of the code by checking them as assertions at runtime.

[dbc]: https://en.wikipedia.org/wiki/Design_by_contract

```rust
struct Range {
    min: usize,
    max: usize,
}

impl Range {
    #[pre(min < max)]
    pub fn new(min: usize, max: usize) -> Self {
        Range {
            min,
            max,
        }
    }

    #[post(ret.min == self.min.min(other.min))]
    #[post(ret.max == self.max.max(other.max))]
    pub fn merge(self, other: Range) -> Range {
        let min = self.min.min(other.min);
        let max = self.max.max(other.max);

        Range::new(min, max)
    }
    
    pub fn contains(self, val: usize) -> bool {
        self.min <= val && val <= self.max
    }
}
```

(For a more complete example see [the RangedInt test][rit])

[rit]: tests/ranged_int.rs

## Modes

All the attributes (pre, post, invariant) have `debug_*` and `test_*` versions.

- `debug_pre`/`debug_post`/`debug_invariant` use `debug_assert!` internally rather than `assert!`
- `test_pre`/`test_post`/`test_invariant` guard the `assert!` with an `if cfg!(test)`.
  This should mostly be used for stating equivalence to "slow but obviously correct" alternative implementations or checks.
  
  For example, a merge-sort implementation might look like this
  ```rust
  #[post(is_sorted(input))]
  fn merge_sort<T: Ord + Copy>(input: &mut [T]) {
      // ...
  }
  ```

## Set-up

To install the latest version, add `contracts` to the dependency section of the `Cargo.toml` file.

```
[dependencies]
contracts = "0.2.2"
```

To then bring all procedural macros into scope, you can add `use contracts::*;` in all files you plan
to use the contract attributes.

Alternatively use the "old-style" of importing macros to have them available in project-wide.

```rust
#[macro_use]
extern crate contracts;
```

## Configuration

This crate exposes a number of feature flags to configure the assertion behavior.

 - `disable_contracts` - disables all checks and assertions.
 - `override_debug` - changes all contracts (except `test_` ones) into `debug_*` versions
 - `override_log` - changes all contracts (except `test_` ones) into a `log::error!()` call if the condition is violated.
   No abortion happens.


## TODOs

 - implement more contracts for traits.
 - add a static analyzer à la SPARK for whole-projects using the contracts to make static assertions.