/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

extern crate proc_macro;
use proc_macro::TokenStream;
use syn::{Block, Expr, FnArg, ImplItem, ImplItemMethod, Item, ItemFn, ItemImpl};

#[proc_macro_attribute]
pub fn pre(attr: TokenStream, toks: TokenStream) -> TokenStream {
    let assert: Expr = syn::parse_macro_input!(attr as Expr);

    let item: ItemFn = syn::parse_macro_input!(toks as ItemFn);

    let pre = quote::quote! {
        assert!(#assert, "Pre-condition violated: {}", stringify!(#assert));
    };
    let post = quote::quote! {};

    impl_fn_checks(item, pre, post)
}

#[proc_macro_attribute]
pub fn post(attr: TokenStream, toks: TokenStream) -> TokenStream {
    let assert: Expr = syn::parse_macro_input!(attr as Expr);

    let item: ItemFn = syn::parse_macro_input!(toks as ItemFn);

    let pre = quote::quote! {};
    let post = quote::quote! {
        assert!(#assert, "Post-condition violated: {}", stringify!(#assert));
    };

    impl_fn_checks(item, pre, post)
}

#[proc_macro_attribute]
pub fn invariant(attr: TokenStream, toks: TokenStream) -> TokenStream {
    let assert: Expr = syn::parse_macro_input!(attr as Expr);

    let item: Item = syn::parse_macro_input!(toks as Item);

    match item {
        Item::Fn(fn_) => {
            let pre = quote::quote! {
                assert!(#assert, "Invariant violated: {}", stringify!(#assert));
            };

            let post = pre.clone();

            impl_fn_checks(fn_, pre, post)
        }
        Item::Impl(impl_) => impl_impl_invariant(assert, impl_),
        _ => unimplemented!("The #[invariant] attribute only works on functions impl-blocks."),
    }
}

fn impl_fn_checks(
    mut fn_def: ItemFn,
    pre: proc_macro2::TokenStream,
    post: proc_macro2::TokenStream,
) -> TokenStream {
    let block = fn_def.block.clone();

    let new_block = quote::quote! {

        {
            #pre

            let ret = {
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

fn impl_impl_invariant(invariant: Expr, mut impl_def: ItemImpl) -> TokenStream {
    // all that is done is prefix all the function definitions with
    // the invariant attribute.
    // The following expansion of the attributes will then implement the invariant
    // just like it's done for functions.

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
