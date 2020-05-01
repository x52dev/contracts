/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use proc_macro2::TokenStream;
use syn::{FnArg, ImplItem, ImplItemMethod, Item, ItemFn, ItemImpl};

use crate::implementation::{ContractMode, ContractType, FuncWithContracts};

pub(crate) fn invariant(
    mode: ContractMode,
    attr: TokenStream,
    toks: TokenStream,
) -> TokenStream {
    let item: Item = syn::parse_quote!(#toks);

    let name = mode.name().unwrap().to_string() + "invariant";

    match item {
        Item::Fn(fn_) => invariant_fn(mode, attr, fn_),
        Item::Impl(impl_) => invariant_impl(mode, attr, impl_),
        _ => unimplemented!(
            "The #[{}] attribute only works on functions and impl-blocks.",
            name
        ),
    }
}

fn invariant_fn(
    mode: ContractMode,
    attr: TokenStream,
    func: ItemFn,
) -> TokenStream {
    let ty = ContractType::Invariant;

    let f = FuncWithContracts::new_with_initial_contract(func, ty, mode, attr);

    f.generate()
}

/// Generate the token-stream for an `impl` block with a "global" invariant.
fn invariant_impl(
    mode: ContractMode,
    invariant: TokenStream,
    mut impl_def: ItemImpl,
) -> TokenStream {
    // all that is done is prefix all the function definitions with
    // the invariant attribute.
    // The following expansion of the attributes will then implement the
    // invariant just like it's done for functions.

    // The mode received is "raw", so it can't be "Disabled" or "LogOnly"
    // but it can't hurt to deal with it anyway.
    let name = match mode.name() {
        Some(n) => n.to_string() + "invariant",
        None => {
            return quote::quote!( #impl_def ).into();
        }
    };

    let invariant_ident =
        syn::Ident::new(&name, proc_macro2::Span::call_site());

    let invariant: proc_macro2::TokenStream = invariant.into();

    fn method_uses_self(method: &ImplItemMethod) -> bool {
        let inputs = &method.sig.inputs;

        if !inputs.is_empty() {
            match inputs[0] {
                FnArg::Receiver(_) => true,
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
                #[#invariant_ident(#invariant)]
                #method
            };

            let met: ImplItemMethod = syn::parse_quote!(#method_toks);

            *method = met;
        }
    }

    let toks = quote::quote! {
        #impl_def
    };
    toks.into()
}
