/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! A crate implementing ["Design by Contract"][dbc] via procedural macros.
//!
//! This crate is heavily inspired by the [`libhoare`] compiler plugin.
//!
//! The main use of this crate is to annotate functions and methods using
//! "contracts" in the form of [*pre-conditions*][precond],
//! [*post-conditions*][postcond] and [*invariants*][invariant].
//!
//! Each "contract" annotation that is violated will cause an assertion failure.
//!
//! The attributes use "function call form" and can contain 1 or more conditions
//! to check.
//! If the last argument to an attribute is a string constant it will be inserted
//! into the assertion message.
//!
//! ## Example
//!
//! ```rust
//! # use contracts::*;
//! #[pre(x > 0, "x must be in the valid input range")]
//! #[post(ret.map(|s| s * s == x).unwrap_or(true))]
//! fn integer_sqrt(x: u64) -> Option<u64> {
//!    // ...
//! # unimplemented!()
//! }
//! ```
//!
//! ## Feature flags
//!
//! Following feature flags are available:
//!  - `disable_contracts` - disables all checks and assertions.
//!  - `override_debug` - changes all contracts (except `test_` ones) into `debug_*` versions
//!  - `override_log` - changes all contracts (except `test_` ones) into a `log::error!()` call if the condition is violated.
//!    No abortion happens.
//!
//! [dbc]: https://en.wikipedia.org/wiki/Design_by_contract
//! [`libhoare`]: https://github.com/nrc/libhoare
//! [precond]: attr.pre.html
//! [postcond]: attr.post.html
//! [invariant]: attr.invariant.html

extern crate proc_macro;

mod implementation;

use implementation::{final_mode, ContractMode};
use proc_macro::TokenStream;

/// Pre-conditions are checked before the function body is run.
///
/// ## Example
///
/// ```rust
/// # use contracts::*;
/// #[pre(elems.len() >= 1)]
/// fn max<T: Ord + Copy>(elems: &[T]) -> T {
///    // ...
/// # unimplemented!()
/// }
/// ```
#[proc_macro_attribute]
pub fn pre(attr: TokenStream, toks: TokenStream) -> TokenStream {
    let mode = final_mode(ContractMode::Always);
    implementation::pre(mode, attr, toks)
}

/// Same as [`pre`], but uses `debug_assert!`.
///
/// [`pre`]: attr.pre.html
#[proc_macro_attribute]
pub fn debug_pre(attr: TokenStream, toks: TokenStream) -> TokenStream {
    let mode = final_mode(ContractMode::Debug);
    implementation::pre(mode, attr, toks)
}

/// Same as [`pre`], but is only enabled in `#[cfg(test)]` environments.
///
/// [`pre`]: attr.pre.html
#[proc_macro_attribute]
pub fn test_pre(attr: TokenStream, toks: TokenStream) -> TokenStream {
    let mode = final_mode(ContractMode::Test);
    implementation::pre(mode, attr, toks)
}

/// Post-conditions are checked after the function body is run.
///
/// The result of the function call is accessible in conditions using the `ret` identifier.
///
/// ## Example
///
/// ```rust
/// # use contracts::*;
/// #[post(ret > x)]
/// fn incr(x: usize) -> usize {
///     x + 1
/// }
/// ```
#[proc_macro_attribute]
pub fn post(attr: TokenStream, toks: TokenStream) -> TokenStream {
    let mode = final_mode(ContractMode::Always);
    implementation::post(mode, attr, toks)
}

/// Same as [`post`], but uses `debug_assert!`.
///
/// [`post`]: attr.post.html
#[proc_macro_attribute]
pub fn debug_post(attr: TokenStream, toks: TokenStream) -> TokenStream {
    let mode = final_mode(ContractMode::Debug);
    implementation::post(mode, attr, toks)
}

/// Same as [`post`], but is only enabled in `#[cfg(test)]` environments.
///
/// [`post`]: attr.post.html
#[proc_macro_attribute]
pub fn test_post(attr: TokenStream, toks: TokenStream) -> TokenStream {
    let mode = final_mode(ContractMode::Test);
    implementation::post(mode, attr, toks)
}

/// Invariants are conditions that have to be maintained at the "interface boundaries".
///
/// Invariants can be supplied to functions (and "methods"), as well as on `impl` blocks.
///
/// When applied to an `impl`-block all methods taking `self` (either by value or reference)
/// will be checked for the invariant.
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
    // Function expansions will resolve the actual mode themselves, so the actual
    // "raw" mode is passed here
    //
    // TODO: update comment for traits
    let mode = ContractMode::Always;
    implementation::invariant(mode, attr, toks)
}

/// Same as [`invariant`], but uses `debug_assert!`.
///
/// [`invariant`]: attr.invariant.html
#[proc_macro_attribute]
pub fn debug_invariant(attr: TokenStream, toks: TokenStream) -> TokenStream {
    let mode = ContractMode::Debug;
    implementation::invariant(mode, attr, toks)
}

/// Same as [`invariant`], but is only enabled in `#[cfg(test)]` environments.
///
/// [`invariant`]: attr.invariant.html
#[proc_macro_attribute]
pub fn test_invariant(attr: TokenStream, toks: TokenStream) -> TokenStream {
    let mode = ContractMode::Test;
    implementation::invariant(mode, attr, toks)
}

/// A "contract_trait" is a trait which ensures all implementors respect all provided contracts.
///
/// When this attribute is applied to a `trait` definition, the trait gets modified so that all
/// invocations of methods are checked.
///
/// When this attribute is applied to an `impl Trait for Type` item, the implementation gets
/// modified so it matches the trait definition.
///
/// **When the `#[contract_trait]` is not applied to either the trait or an `impl` it will cause
/// compile errors**.
///
/// ## Example
///
/// ```rust
/// # use contracts::*;
/// #[contract_trait]
/// trait MyRandom {
///     #[pre(min < max)]
///     #[post(min <= ret, ret <= max)]
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
    let item: syn::Item = syn::parse_macro_input!(toks);

    match item {
        syn::Item::Trait(trait_) => implementation::contract_trait_item_trait(attrs, trait_),
        syn::Item::Impl(impl_) => {
            assert!(
                impl_.trait_.is_some(),
                "#[contract_trait] can only be applied to `trait` and `impl ... for` items"
            );
            implementation::contract_trait_item_impl(attrs, impl_)
        }
        _ => panic!("#[contract_trait] can only be applied to `trait` and `impl ... for` items"),
    }
}
