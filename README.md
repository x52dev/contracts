# *Design By Contract* for Rust

[![License][license]][LICENSE]
![Build status][build]
![Lines of Code][loc]

[license]: https://img.shields.io/badge/license-MPL%202.0-blue.svg
[build]: https://gitlab.com/karroffel/contracts/badges/master/build.svg
[loc]: https://tokei.rs/b1/gitlab/karroffel/contracts?category=code

Annotate functions and methods with "contracts", using *invariants*,
*pre-conditions* and *post-conditions*.

[Design by contract][dbc] is a popular method to augment code with formal
interface specifications.
These specifications are used to increase the correctness of the code by
checking them as assertions at runtime.

[dbc]: https://en.wikipedia.org/wiki/Design_by_contract

```rust
pub struct Library {
    available: HashSet<String>,
    lent: HashSet<String>,
}

impl Library {
    fn book_exists(&self, book_id: &str) -> bool {
        self.available.contains(book_id)
            || self.lent.contains(book_id)
    }

    #[debug_pre(!self.book_exists(book_id), "Book IDs are unique")]
    #[debug_post(self.available.contains(book_id), "Book now available")]
    #[post(self.available.len() == old(self.available.len()) + 1)]
    #[post(self.lent.len() == old(self.lent.len()), "No lent change")]
    pub fn add_book(&mut self, book_id: &str) {
        self.available.insert(book_id.to_string());
    }

    #[debug_pre(self.book_exists(book_id))]
    #[post(ret ==> self.available.len() == old(self.available.len()) - 1)]
    #[post(ret ==> self.lent.len() == old(self.lent.len()) + 1)]
    #[debug_post(ret ==> self.lent.contains(book_id))]
    #[debug_post(!ret ==> self.lent.contains(book_id), "Book already lent")]
    pub fn lend(&mut self, book_id: &str) -> bool {
        if self.available.contains(book_id) {
            self.available.remove(book_id);
            self.lent.insert(book_id.to_string());
            true
        } else {
            false
        }
    }

    #[debug_pre(self.lent.contains(book_id), "Can't return a non-lent book")]
    #[post(self.lent.len() == old(self.lent.len()) - 1)]
    #[post(self.available.len() == old(self.available.len()) + 1)]
    #[debug_post(!self.lent.contains(book_id))]
    #[debug_post(self.available.contains(book_id), "Book available again")]
    pub fn return_book(&mut self, book_id: &str) {
        self.lent.remove(book_id);
        self.available.insert(book_id.to_string());
    }
}
```

## Attributes

This crate exposes the `pre`, `post` and `invariant` attributes.

- `pre`-conditions are checked before a function/method is executed.
- `post`-conditions are checked after a function/method ran to completion
- `invariant`s are checked both before *and* after a function/method ran.

Additionally, all those attributes have versions with different "modes". See
[the Modes section][#Modes] below.

For `trait`s and trait `impl`s the `contract_trait` attribute can be used.

More specific information can be found in the crate documentation.

## Pseudo-functions and operators

### `old()` function

One unique feature that this crate provides is an `old()` pseudo-function which
allows to perform checks using a value of a parameter before the function call
happened. This is only available in `post` attributes.

```rust
#[post(*x == old(*x) + 1, "after the call `x` was incremented")]
fn incr(x: &mut usize) {
    *x += 1;
}
```

### `==>` operator

For more complex functions it can be useful to express behaviour using logical
implication. Because Rust does not feature an operator for implication, this
crate adds this operator for use in attributes.

```rust
#[post(person_name.is_some() ==> ret.contains(person_name.unwrap()))]
fn geeting(person_name: Option<&str>) -> String {
    let mut s = String::from("Hello");
    if let Some(name) = person_name {
        s.push(' ');
        s.push_str(name);
    }
    s.push('!');
    s
}
```

This operator is right-associative.

**Note**: Because of the design of `syn`, it is tricky to add custom operators
to be parsed, so this crate performs a rewrite of the `TokenStream` instead.
The rewrite works by separating the expression into a part that's left of the
`==>` operator and the rest on the right side. This means that
`if a ==> b { c } else { d }` will not generate the expected code.
Explicit grouping using parenthesis or curly-brackets can be used to avoid this.


## Modes

All the attributes (pre, post, invariant) have `debug_*` and `test_*` versions.

- `debug_pre`/`debug_post`/`debug_invariant` use `debug_assert!` internally
  rather than `assert!`
- `test_pre`/`test_post`/`test_invariant` guard the `assert!` with an
  `if cfg!(test)`.
  This should mostly be used for stating equivalence to "slow but obviously
  correct" alternative implementations or checks.
  
  For example, a merge-sort implementation might look like this
  ```rust
  #[test_post(is_sorted(input))]
  fn merge_sort<T: Ord + Copy>(input: &mut [T]) {
      // ...
  }
  ```

## Set-up

To install the latest version, add `contracts` to the dependency section of the
`Cargo.toml` file.

```
[dependencies]
contracts = "0.4.0"
```

To then bring all procedural macros into scope, you can add `use contracts::*;`
in all files you plan to use the contract attributes.

Alternatively use the "old-style" of importing macros to have them available
project-wide.

```rust
#[macro_use]
extern crate contracts;
```

## Configuration

This crate exposes a number of feature flags to configure the assertion behavior.

 - `disable_contracts` - disables all checks and assertions.
 - `override_debug` - changes all contracts (except `test_` ones) into `debug_*`
   versions
 - `override_log` - changes all contracts (except `test_` ones) into a
   `log::error!()` call if the condition is violated.
   No abortion happens.
 - `mirai_assertions` - instead of regular assert! style macros, emit macros
   used by the [MIRAI] static analyzer. For more documentation of this usage, 
   head to the [MIRAI] repo.

[MIRAI]: https://github.com/facebookexperimental/MIRAI

## TODOs

 - implement more contracts for traits.
 - add a static analyzer à la SPARK for whole-projects using the contracts to
   make static assertions.
