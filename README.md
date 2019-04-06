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

## Set-up

At the moment, `contracts` is not on crates.io, so it has to be added as a git-dependency.

```
[dependencies]
contracts = { git = "https://gitlab.com/karroffel/contracts" }
```

To bring all procedural macros into scope, you can add `use contracts::*;` in all files you plan
to use the contract attributes.

Alternative use the "old-style" of importing macros to have them available in project-wide.

```rust
#[macro_use]
extern crate contracts;
```

## TODOs

 - add `test_pre`/`test_post`/`test_invariant` attributes which are only used in test configurations.
   This is useful to test implementations for "slow but obviously correct" alternative implementations.
 - add `debug_pre`/`debug_post`/`debug_invariant` attributes which use `debug_assert!` instead of `assert!`
 - add a static analyzer à la SPARK for whole-projects using the contracts to make static assertions.