/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! A crate implementing ["Design by Contract"][dbc] via procedural macros.
//!
//! This crate is heavily inspired by the [`libhoare`] compiler plugin.
//!
//! The main use of this crate is to annotate functions and methods using
//! "contracts" in the form of [*pre-conditions* (`requires`)][precond],
//! [*post-conditions* (`ensures`)][postcond] and [*invariants*][invariant].
//!
//! Each "contract" annotation that is violated will cause an assertion failure.
//!
//! The attributes use "function call form" and can contain 1 or more conditions
//! to check.
//! If the last argument to an attribute is a string constant it will be
//! inserted into the assertion message.
//!
//! ## Example
//!
//! ```rust
//! # use contracts::*;
//! #[requires(x > 0, "x must be in the valid input range")]
//! #[ensures(ret.is_some() -> ret.unwrap() * ret.unwrap() == x)]
//! fn integer_sqrt(x: u64) -> Option<u64> {
//!    // ...
//! # unimplemented!()
//! }
//! ```
//!
//! ```rust
//! # use std::collections::HashSet;
//! # use contracts::*;
//! pub struct Library {
//!     available: HashSet<String>,
//!     lent: HashSet<String>,
//! }
//!
//! impl Library {
//!     fn book_exists(&self, book_id: &str) -> bool {
//!         self.available.contains(book_id)
//!             || self.lent.contains(book_id)
//!     }
//!
//!     #[debug_requires(!self.book_exists(book_id), "Book IDs are unique")]
//!     #[debug_ensures(self.available.contains(book_id), "Book now available")]
//!     #[ensures(self.available.len() == old(self.available.len()) + 1)]
//!     #[ensures(self.lent.len() == old(self.lent.len()), "No lent change")]
//!     pub fn add_book(&mut self, book_id: &str) {
//!         self.available.insert(book_id.to_string());
//!     }
//!
//!     #[debug_requires(self.book_exists(book_id))]
//!     #[ensures(ret -> self.available.len() == old(self.available.len()) - 1)]
//!     #[ensures(ret -> self.lent.len() == old(self.lent.len()) + 1)]
//!     #[debug_ensures(ret -> self.lent.contains(book_id))]
//!     #[debug_ensures(!ret -> self.lent.contains(book_id), "Book already lent")]
//!     pub fn lend(&mut self, book_id: &str) -> bool {
//!         if self.available.contains(book_id) {
//!             self.available.remove(book_id);
//!             self.lent.insert(book_id.to_string());
//!             true
//!         } else {
//!             false
//!         }
//!     }
//!
//!     #[debug_requires(self.lent.contains(book_id), "Can't return a non-lent book")]
//!     #[ensures(self.lent.len() == old(self.lent.len()) - 1)]
//!     #[ensures(self.available.len() == old(self.available.len()) + 1)]
//!     #[debug_ensures(!self.lent.contains(book_id))]
//!     #[debug_ensures(self.available.contains(book_id), "Book available again")]
//!     pub fn return_book(&mut self, book_id: &str) {
//!         self.lent.remove(book_id);
//!         self.available.insert(book_id.to_string());
//!     }
//! }
//! ```
//!
//! ## Attributes
//!
//! This crate exposes the `requires`, `ensures` and `invariant` attributes.
//!
//! - `requires` are checked before a function/method is executed.
//! - `ensures` are checked after a function/method ran to completion
//! - `invariant`s are checked both before *and* after a function/method ran.
//!
//! Additionally, all those attributes have versions with different "modes". See
//! [the Modes section](#modes) below.
//!
//! For `trait`s and trait `impl`s the `contract_trait` attribute can be used.
//!
//! ## Pseudo-functions and operators
//!
//! ### `old()` function
//!
//! One unique feature that this crate provides is an `old()` pseudo-function which
//! allows to perform checks using a value of a parameter before the function call
//! happened. This is only available in `ensures` attributes.
//!
//! ```rust
//! # use contracts::*;
//! #[ensures(*x == old(*x) + 1, "after the call `x` was incremented")]
//! fn incr(x: &mut usize) {
//!     *x += 1;
//! }
//! ```
//!
//! ### `->` operator
//!
//! For more complex functions it can be useful to express behaviour using logical
//! implication. Because Rust does not feature an operator for implication, this
//! crate adds this operator for use in attributes.
//!
//! ```rust
//! # use contracts::*;
//! #[ensures(person_name.is_some() -> ret.contains(person_name.unwrap()))]
//! fn geeting(person_name: Option<&str>) -> String {
//!     let mut s = String::from("Hello");
//!     if let Some(name) = person_name {
//!         s.push(' ');
//!         s.push_str(name);
//!     }
//!     s.push('!');
//!     s
//! }
//! ```
//!
//! This operator is right-associative.
//!
//! **Note**: Because of the design of `syn`, it is tricky to add custom operators
//! to be parsed, so this crate performs a rewrite of the `TokenStream` instead.
//! The rewrite works by separating the expression into a part that's left of the
//! `->` operator and the rest on the right side. This means that
//! `if a -> b { c } else { d }` will not generate the expected code.
//! Explicit grouping using parenthesis or curly-brackets can be used to avoid this.
//!
//! ## Modes
//!
//! All the attributes (requires, ensures, invariant) have `debug_*` and `test_*` versions.
//!
//! - `debug_requires`/`debug_ensures`/`debug_invariant` use `debug_assert!`
//!   internally rather than `assert!`
//! - `test_requires`/`test_ensures`/`test_invariant` guard the `assert!` with an
//!   `if cfg!(test)`.
//!   This should mostly be used for stating equivalence to "slow but obviously
//!   correct" alternative implementations or checks.
//!   
//!   For example, a merge-sort implementation might look like this
//!   ```rust
//!   # use contracts::*;
//!   # fn is_sorted<T>(x: T) -> bool { true }
//!   #[test_ensures(is_sorted(input))]
//!   fn merge_sort<T: Ord + Copy>(input: &mut [T]) {
//!       // ...
//!   }
//!   ```
//!
//! ## Feature flags
//!
//! Following feature flags are available:
//!  - `disable_contracts` - disables all checks and assertions.
//!  - `override_debug` - changes all contracts (except `test_` ones) into
//!    `debug_*` versions
//!  - `override_log` - changes all contracts (except `test_` ones) into a
//!    `log::error!()` call if the condition is violated.
//!    No abortion happens.
//! - `mirai_assertions` - instead of regular assert! style macros, emit macros
//!   used by the [MIRAI] static analyzer.
//!
//! [dbc]: https://en.wikipedia.org/wiki/Design_by_contract
//! [`libhoare`]: https://github.com/nrc/libhoare
//! [precond]: attr.requires.html
//! [postcond]: attr.ensures.html
//! [invariant]: attr.invariant.html
//! [MIRAI]: https://github.com/facebookexperimental/MIRAI

extern crate proc_macro;

mod implementation;

use implementation::ContractMode;
use proc_macro::TokenStream;

/// Pre-conditions are checked before the function body is run.
///
/// ## Example
///
/// ```rust
/// # use contracts::*;
/// #[requires(elems.len() >= 1)]
/// fn max<T: Ord + Copy>(elems: &[T]) -> T {
///    // ...
/// # unimplemented!()
/// }
/// ```
#[proc_macro_attribute]
pub fn requires(attr: TokenStream, toks: TokenStream) -> TokenStream {
    let attr = attr.into();
    let toks = toks.into();
    implementation::requires(ContractMode::Always, attr, toks).into()
}

/// Same as [`requires`], but uses `debug_assert!`.
///
/// [`requires`]: attr.requires.html
#[proc_macro_attribute]
pub fn debug_requires(attr: TokenStream, toks: TokenStream) -> TokenStream {
    let attr = attr.into();
    let toks = toks.into();
    implementation::requires(ContractMode::Debug, attr, toks).into()
}

/// Same as [`requires`], but is only enabled in `#[cfg(test)]` environments.
///
/// [`requires`]: attr.requires.html
#[proc_macro_attribute]
pub fn test_requires(attr: TokenStream, toks: TokenStream) -> TokenStream {
    let attr = attr.into();
    let toks = toks.into();
    implementation::requires(ContractMode::Test, attr, toks).into()
}

/// Post-conditions are checked after the function body is run.
///
/// The result of the function call is accessible in conditions using the `ret`
/// identifier.
///
/// A "pseudo-function" named `old` can be used to evaluate expressions in a
/// context *prior* to function execution.
/// This function takes only a single argument and the result of it will be
/// stored in a variable before the function is called. Because of this,
/// handling references might require special care.
///
/// ## Examples
///
/// ```rust
/// # use contracts::*;
/// #[ensures(ret > x)]
/// fn incr(x: usize) -> usize {
///     x + 1
/// }
/// ```
///
/// ```rust
/// # use contracts::*;
/// #[ensures(*x == old(*x) + 1, "x is incremented")]
/// fn incr(x: &mut usize) {
///     *x += 1;
/// }
/// ```
#[proc_macro_attribute]
pub fn ensures(attr: TokenStream, toks: TokenStream) -> TokenStream {
    let attr = attr.into();
    let toks = toks.into();
    implementation::ensures(ContractMode::Always, attr, toks).into()
}

/// Same as [`ensures`], but uses `debug_assert!`.
///
/// [`ensures`]: attr.ensures.html
#[proc_macro_attribute]
pub fn debug_ensures(attr: TokenStream, toks: TokenStream) -> TokenStream {
    let attr = attr.into();
    let toks = toks.into();
    implementation::ensures(ContractMode::Debug, attr, toks).into()
}

/// Same as [`ensures`], but is only enabled in `#[cfg(test)]` environments.
///
/// [`ensures`]: attr.ensures.html
#[proc_macro_attribute]
pub fn test_ensures(attr: TokenStream, toks: TokenStream) -> TokenStream {
    let attr = attr.into();
    let toks = toks.into();
    implementation::ensures(ContractMode::Test, attr, toks).into()
}

/// Invariants are conditions that have to be maintained at the "interface
/// boundaries".
///
/// Invariants can be supplied to functions (and "methods"), as well as on
/// `impl` blocks.
///
/// When applied to an `impl`-block all methods taking `self` (either by value
/// or reference) will be checked for the invariant.
///
/// ## Example
///
/// On a function:
///
/// ```rust
/// # use contracts::*;
/// /// Update `num` to the next bigger even number.
/// #[invariant(*num % 2 == 0)]
/// fn advance_even(num: &mut usize) {
///     *num += 2;
/// }
/// ```
///
/// On an `impl`-block:
///
/// ```rust
/// # use contracts::*;
/// struct EvenAdder {
///     count: usize,
/// }
///
/// #[invariant(self.count % 2 == 0)]
/// impl EvenAdder {
///     pub fn tell(&self) -> usize {
///         self.count
///     }
///
///     pub fn advance(&mut self) {
///         self.count += 2;
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn invariant(attr: TokenStream, toks: TokenStream) -> TokenStream {
    // Invariant attributes might apply to `impl` blocks as well, where the same
    // level is simply replicated on all methods.
    // Function expansions will resolve the actual mode themselves, so the
    // actual "raw" mode is passed here
    //
    // TODO: update comment when implemented for traits
    let attr = attr.into();
    let toks = toks.into();
    let mode = ContractMode::Always;
    implementation::invariant(mode, attr, toks).into()
}

/// Same as [`invariant`], but uses `debug_assert!`.
///
/// [`invariant`]: attr.invariant.html
#[proc_macro_attribute]
pub fn debug_invariant(attr: TokenStream, toks: TokenStream) -> TokenStream {
    let mode = ContractMode::Debug;
    let attr = attr.into();
    let toks = toks.into();
    implementation::invariant(mode, attr, toks).into()
}

/// Same as [`invariant`], but is only enabled in `#[cfg(test)]` environments.
///
/// [`invariant`]: attr.invariant.html
#[proc_macro_attribute]
pub fn test_invariant(attr: TokenStream, toks: TokenStream) -> TokenStream {
    let mode = ContractMode::Test;
    let attr = attr.into();
    let toks = toks.into();
    implementation::invariant(mode, attr, toks).into()
}

/// A "contract_trait" is a trait which ensures all implementors respect all
/// provided contracts.
///
/// When this attribute is applied to a `trait` definition, the trait gets
/// modified so that all invocations of methods are checked.
///
/// When this attribute is applied to an `impl Trait for Type` item, the
/// implementation gets modified so it matches the trait definition.
///
/// **When the `#[contract_trait]` is not applied to either the trait or an
/// `impl` it will cause compile errors**.
///
/// ## Example
///
/// ```rust
/// # use contracts::*;
/// #[contract_trait]
/// trait MyRandom {
///     #[requires(min < max)]
///     #[ensures(min <= ret, ret <= max)]
///     fn gen(min: f64, max: f64) -> f64;
/// }
///
/// // Not a very useful random number generator, but a valid one!
/// struct AlwaysMax;
///
/// #[contract_trait]
/// impl MyRandom for AlwaysMax {
///     fn gen(min: f64, max: f64) -> f64 {
///         max
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn contract_trait(attrs: TokenStream, toks: TokenStream) -> TokenStream {
    let attrs: proc_macro2::TokenStream = attrs.into();
    let toks: proc_macro2::TokenStream = toks.into();

    let item: syn::Item = syn::parse_quote!(#toks);

    let tts = match item {
        syn::Item::Trait(trait_) => implementation::contract_trait_item_trait(attrs, trait_),
        syn::Item::Impl(impl_) => {
            assert!(
                impl_.trait_.is_some(),
                "#[contract_trait] can only be applied to `trait` and `impl ... for` items"
            );
            implementation::contract_trait_item_impl(attrs, impl_)
        }
        _ => panic!("#[contract_trait] can only be applied to `trait` and `impl ... for` items"),
    };

    tts.into()
}
