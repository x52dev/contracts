/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use crate::implementation::{ContractMode, ContractType, FuncWithContracts};
use proc_macro::TokenStream;
use syn::ItemFn;

pub(crate) fn post(
    mode: ContractMode,
    attr: TokenStream,
    toks: TokenStream,
) -> TokenStream {
    let ty = ContractType::Post;

    let func = syn::parse_macro_input!(toks as ItemFn);

    let f = FuncWithContracts::new_with_initial_contract(func, ty, mode, attr);

    f.generate()
}
