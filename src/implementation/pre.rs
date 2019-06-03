/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use crate::implementation::{
    attributes_to_asserts, generate_fn_checks, parse_attributes, ContractMode,
};
use proc_macro::TokenStream;
use syn::ItemFn;

pub(crate) fn pre(mode: ContractMode, attr: TokenStream, toks: TokenStream) -> TokenStream {
    let (conds, desc) = parse_attributes(attr);

    let item: ItemFn = syn::parse_macro_input!(toks as ItemFn);
    let fn_name = item.ident.to_string();

    let desc = if let Some(desc) = desc {
        format!("Pre-condition of {} violated - {:?}", fn_name, desc)
    } else {
        format!("Pre-condition of {} violated", fn_name)
    };

    let pre = attributes_to_asserts(mode, conds, desc);
    let post = quote::quote! {};

    generate_fn_checks(item, pre, post)
}
