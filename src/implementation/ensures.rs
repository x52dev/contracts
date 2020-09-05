/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use crate::implementation::{ContractMode, ContractType, FuncWithContracts};
use proc_macro2::TokenStream;

pub(crate) fn ensures(
    mode: ContractMode,
    attr: TokenStream,
    toks: TokenStream,
) -> TokenStream {
    let ty = ContractType::Ensures;

    let func = syn::parse_quote!(#toks);

    let f = FuncWithContracts::new_with_initial_contract(func, ty, mode, attr);

    f.generate()
}
