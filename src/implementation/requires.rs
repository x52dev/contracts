/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use proc_macro2::TokenStream;
use syn::ItemFn;

use crate::implementation::{emit_error, ContractMode, ContractType, FuncWithContracts};

pub(crate) fn requires(mode: ContractMode, attr: TokenStream, toks: TokenStream) -> TokenStream {
    let ty = ContractType::Requires;

    let func: ItemFn = match syn::parse2(toks.clone()) {
        Ok(func) => func,
        Err(err) => return emit_error(err, toks),
    };

    let f = FuncWithContracts::new_with_initial_contract(func, ty, mode, attr);

    f.generate()
}
