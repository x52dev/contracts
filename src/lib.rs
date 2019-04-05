/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! A crate implementing ["Design by Contract"][dbc] via procedural macros.
//!
//! This crate is heavily inspired by the [`libhoare`] compiler plugin.
//!
//! The main use of this crate is to annotate functions and methods using
//! "contracts" in the form of [*pre-conditions*][precond],
//! [*postconditions*][postcond] and [*invariants*][invariant].
//!
//! Each "contract" annotation that is violated will cause an assertion failure.
//!
//! The attributes use "function call form" and can contain 1 or more conditions
//! to check.
//! If the last argument to an attribute is a string constant it will be inserted
//! into the assertion message.
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
//! [dbc]: https://en.wikipedia.org/wiki/Design_by_contract
//! [`libhoare`]: https://github.com/nrc/libhoare
//! [precond]: attr.pre.html
//! [postcond]: attr.pre.html
//! [invariant]: attr.invariant.html

extern crate proc_macro;

use proc_macro::TokenStream;
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::Token;
use syn::{
    Block, Expr, ExprLit, FnArg, ImplItem, ImplItemMethod, Item, ItemFn, ItemImpl, Lit, ReturnType,
};

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
    let (conds, desc) = parse_attributes(attr);

    let desc = if let Some(desc) = desc {
        format!("Pre-condition violated - {:?}", desc)
    } else {
        "Pre-condition violated".to_string()
    };

    let item: ItemFn = syn::parse_macro_input!(toks as ItemFn);

    let pre = attributes_to_asserts(conds, desc);
    let post = quote::quote! {};

    impl_fn_checks(item, pre, post)
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
    let (conds, desc) = parse_attributes(attr);

    let desc = if let Some(desc) = desc {
        format!("Post-condition violated - {:?}", desc)
    } else {
        "Post-condition violated".to_string()
    };

    let item: ItemFn = syn::parse_macro_input!(toks as ItemFn);

    let pre = quote::quote! {};
    let post = attributes_to_asserts(conds, desc);

    impl_fn_checks(item, pre, post)
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
    let item: Item = syn::parse_macro_input!(toks as Item);

    match item {
        Item::Fn(fn_) => {
            let (conds, desc) = parse_attributes(attr);

            let desc = if let Some(desc) = desc {
                format!("Invariant violated - {:?}", desc)
            } else {
                "Invariant violated".to_string()
            };

            let pre = attributes_to_asserts(conds, desc);
            let post = pre.clone();

            impl_fn_checks(fn_, pre, post)
        }
        Item::Impl(impl_) => impl_impl_invariant(attr, impl_),
        _ => unimplemented!("The #[invariant] attribute only works on functions impl-blocks."),
    }
}

/// Parse attributes into a list of expression and an optional description of the assert
fn parse_attributes(attrs: TokenStream) -> (Vec<Expr>, Option<String>) {
    let mut conds: Punctuated<Expr, Token![,]> = {
        let tokens = attrs;

        let parser = Punctuated::<Expr, Token![,]>::parse_separated_nonempty;

        let terminated = parser.parse(tokens.clone());

        if let Ok(res) = terminated {
            res
        } else {
            let parser = Punctuated::<Expr, Token![,]>::parse_terminated;

            parser.parse(tokens).unwrap()
        }
    };

    let desc = conds
        .last()
        .map(|x| {
            let expr = *x.value();

            if let Expr::Lit(ExprLit {
                lit: Lit::Str(str), ..
            }) = expr
            {
                Some(str.value())
            } else {
                None
            }
        })
        .unwrap_or(None);

    if desc.is_some() {
        conds.pop();
    }

    let exprs = conds.into_iter().map(|e| e).collect();

    (exprs, desc)
}

/// Create the token-stream for assert statements.
fn attributes_to_asserts(exprs: Vec<Expr>, desc: String) -> proc_macro2::TokenStream {
    let mut stream = proc_macro2::TokenStream::new();

    for expr in exprs {
        stream.extend(quote::quote! {
            assert!(#expr, concat!(concat!(#desc, ": "), stringify!(#expr)));
        });
    }

    stream
}

/// Generate the token-stream for the new function implementation
fn impl_fn_checks(
    mut fn_def: ItemFn,
    pre: proc_macro2::TokenStream,
    post: proc_macro2::TokenStream,
) -> TokenStream {
    let block = fn_def.block.clone();

    let ret_ty = if let ReturnType::Type(_, ty) = &fn_def.decl.output {
        quote::quote! {
            #ty
        }
    } else {
        quote::quote! { () }
    };

    let new_block = quote::quote! {

        {
            #pre

            let ret: #ret_ty = {
                #block
            };

            #post

            ret
        }

    }
    .into();

    fn_def.block = Box::new(syn::parse_macro_input!(new_block as Block));

    let res = quote::quote! {
        #fn_def
    };

    res.into()
}

/// Generate the token-stream for an `impl` block with a "global" invariant.
fn impl_impl_invariant(invariant: TokenStream, mut impl_def: ItemImpl) -> TokenStream {
    // all that is done is prefix all the function definitions with
    // the invariant attribute.
    // The following expansion of the attributes will then implement the invariant
    // just like it's done for functions.

    let invariant: proc_macro2::TokenStream = invariant.into();

    fn method_uses_self(method: &ImplItemMethod) -> bool {
        let inputs = &method.sig.decl.inputs;

        if !inputs.is_empty() {
            match inputs[0] {
                FnArg::SelfValue(_) | FnArg::SelfRef(_) => true,
                _ => false,
            }
        } else {
            false
        }
    }

    for item in &mut impl_def.items {
        if let ImplItem::Method(method) = item {
            // only implement invariants for methods that take `self`
            if !method_uses_self(method) {
                continue;
            }

            let method_toks = quote::quote! {
                #[invariant(#invariant)]
                #method
            }
            .into();

            let met = syn::parse_macro_input!(method_toks as ImplItemMethod);

            *method = met;
        }
    }

    let toks = quote::quote! {
        #impl_def
    };
    toks.into()
}
