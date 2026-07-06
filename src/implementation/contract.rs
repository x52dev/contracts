/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use proc_macro2::{Ident, TokenStream};
use syn::{
    parenthesized,
    parse::{Parse, ParseStream},
    ItemFn, Token,
};

use crate::implementation::{emit_error, Contract, ContractType, FuncWithContracts};

pub(crate) fn contract(attr: TokenStream, toks: TokenStream) -> TokenStream {
    let clauses = match syn::parse2::<ContractClauses>(attr) {
        Ok(clauses) => clauses,
        Err(err) => return err.to_compile_error(),
    };

    let func: ItemFn = match syn::parse2(toks.clone()) {
        Ok(func) => func,
        Err(err) => return emit_error(err, toks),
    };

    FuncWithContracts::new_with_contracts(func, clauses.contracts).generate()
}

struct ContractClauses {
    contracts: Vec<Contract>,
}

impl Parse for ContractClauses {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut contracts = Vec::new();

        while !input.is_empty() {
            let ident = input.parse::<Ident>()?;
            let ident_str = ident.to_string();
            let (ty, mode) = ContractType::contract_type_and_mode(&ident_str).ok_or_else(|| {
                syn::Error::new_spanned(&ident, format!("unknown contract clause `{}`", ident_str))
            })?;

            let content;
            parenthesized!(content in input);
            let toks = content.parse::<TokenStream>()?;
            contracts.push(Contract::from_toks(ty, mode, toks));

            if input.is_empty() {
                break;
            }

            input.parse::<Token![,]>()?;
        }

        Ok(Self { contracts })
    }
}
