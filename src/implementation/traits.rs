/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use proc_macro::TokenStream;
use syn::{
    FnArg, ImplItem, ItemImpl, ItemTrait, Pat, TraitItem, TraitItemMethod,
};

/// Name used for the "re-routed" method.
fn contract_method_impl_name(name: &str) -> String {
    format!("__contracts_impl_{}", name)
}

/// Modifies a trait item in a way that it includes contracts.
pub(crate) fn contract_trait_item_trait(
    _attrs: TokenStream,
    mut trait_: ItemTrait,
) -> TokenStream {
    /// Just rename the method to have an internal, generated name.
    fn create_method_rename(method: &TraitItemMethod) -> TraitItemMethod {
        let mut m: TraitItemMethod = (*method).clone();

        // transform method
        {
            // remove all attributes and rename
            let name = m.sig.ident.to_string();

            let new_name = contract_method_impl_name(&name);

            m.attrs.clear();
            m.sig.ident = syn::Ident::new(&new_name, m.sig.ident.span());
        }

        m
    }

    /// Create a wrapper function which has a default implementation and
    /// includes contracts.
    ///
    /// This new function forwards the call to the actual implementation.
    fn create_method_wrapper(method: &TraitItemMethod) -> TraitItemMethod {
        struct ArgInfo {
            call_toks: proc_macro2::TokenStream,
        }

        // Calculate name and pattern tokens
        fn arg_pat_info(pat: &Pat) -> ArgInfo {
            match pat {
                Pat::Ident(ident) => {
                    let toks = quote::quote! {
                        #ident
                    };
                    ArgInfo { call_toks: toks }
                }
                Pat::Tuple(tup) => {
                    let infos = tup.elems.iter().map(arg_pat_info);

                    let toks = {
                        let mut toks = proc_macro2::TokenStream::new();

                        for info in infos {
                            toks.extend(info.call_toks);
                            toks.extend(quote::quote!(,));
                        }

                        toks
                    };

                    ArgInfo {
                        call_toks: quote::quote!((#toks)),
                    }
                }
                Pat::TupleStruct(_tup) => unimplemented!(),
                p => panic!("Unsupported pattern type: {:?}", p),
            }
        }

        let mut m: TraitItemMethod = (*method).clone();

        let argument_data = m
            .sig
            .inputs
            .clone()
            .into_iter()
            .map(|t: FnArg| match &t {
                FnArg::Receiver(_) => quote::quote!(self),
                FnArg::Typed(p) => {
                    let info = arg_pat_info(&p.pat);

                    info.call_toks
                }
            })
            .collect::<Vec<_>>();

        let arguments = {
            let mut toks = proc_macro2::TokenStream::new();

            for arg in argument_data {
                toks.extend(arg);
                toks.extend(quote::quote!(,));
            }

            toks
        };

        let body: TokenStream = {
            let name = contract_method_impl_name(&m.sig.ident.to_string());
            let name = syn::Ident::new(&name, m.sig.ident.span());

            let toks = quote::quote! {
                {
                    Self::#name(#arguments)
                }
            };

            toks.into()
        };

        {
            let block: syn::Block =
                syn::parse_macro_input::parse(body).unwrap();
            m.default = Some(block);
            m.semi_token = None;
        }

        m
    }

    // create method wrappers and renamed items
    let funcs = trait_
        .items
        .iter()
        .filter_map(|item| {
            if let TraitItem::Method(m) = item {
                let rename = create_method_rename(m);
                let wrapper = create_method_wrapper(m);

                Some(vec![
                    TraitItem::Method(rename),
                    TraitItem::Method(wrapper),
                ])
            } else {
                None
            }
        })
        .flatten()
        .collect::<Vec<_>>();

    // remove all previous methods
    trait_.items = trait_
        .items
        .into_iter()
        .filter(|item| {
            if let TraitItem::Method(_) = item {
                false
            } else {
                true
            }
        })
        .collect();

    // add back new methods
    trait_.items.extend(funcs);

    let toks = quote::quote! {
        #trait_
    };

    toks.into()
}

/// Rename all methods inside an `impl` to use the "internal implementation"
/// name.
pub(crate) fn contract_trait_item_impl(
    _attrs: TokenStream,
    impl_: ItemImpl,
) -> TokenStream {
    let new_impl = {
        let mut impl_: ItemImpl = impl_;

        impl_.items.iter_mut().for_each(|it| {
            if let ImplItem::Method(method) = it {
                let new_name =
                    contract_method_impl_name(&method.sig.ident.to_string());
                let new_ident =
                    syn::Ident::new(&new_name, method.sig.ident.span());

                method.sig.ident = new_ident;
            }
        });

        impl_
    };

    let toks = quote::quote! {
        #new_impl
    };

    toks.into()
}
