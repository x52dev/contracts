/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

pub(crate) mod invariant;
pub(crate) mod post;
pub(crate) mod pre;
pub(crate) mod traits;

use proc_macro::TokenStream;
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::{Block, Expr, ExprLit, ItemFn, Lit, ReturnType, Token};

pub(crate) use invariant::invariant;
pub(crate) use post::post;
pub(crate) use pre::pre;
pub(crate) use traits::{contract_trait_item_impl, contract_trait_item_trait};

/// Checking-mode of a contract.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub(crate) enum ContractMode {
    /// Always check contract
    Always,
    /// Never check contract
    Disabled,
    /// Check contract only in debug builds
    Debug,
    /// Check contract only in `#[cfg(test)]` configurations
    Test,
    /// Check the contract and print information upon violation, but don't abort the program.
    LogOnly,
}

impl ContractMode {
    /// Return the prefix of attributes of `self` mode.
    pub(crate) fn name(self) -> Option<&'static str> {
        match self {
            ContractMode::Always => Some(""),
            ContractMode::Disabled => None,
            ContractMode::Debug => Some("debug_"),
            ContractMode::Test => Some("test_"),
            ContractMode::LogOnly => None,
        }
    }
}

/// Computes the contract type based on feature flags.
pub(crate) fn final_mode(mode: ContractMode) -> ContractMode {
    // disabled ones can't be "forced", test ones should stay test, no matter what.
    if mode == ContractMode::Disabled || mode == ContractMode::Test {
        return mode;
    }

    if cfg!(feature = "disable_contracts") {
        ContractMode::Disabled
    } else if cfg!(feature = "override_debug") {
        // log is "weaker" than debug, so keep log
        if mode == ContractMode::LogOnly {
            mode
        } else {
            ContractMode::Debug
        }
    } else if cfg!(feature = "override_log") {
        ContractMode::LogOnly
    } else {
        mode
    }
}

/// Generate the token-stream for the new function implementation
pub(crate) fn generate_fn_checks(
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

            #[allow(unused_mut)]
            let mut run = || -> #ret_ty {
                #block
            };

            let ret = run();

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

/// Parse attributes into a list of expression and an optional description of the assert
pub(crate) fn parse_attributes(attrs: TokenStream) -> (Vec<Expr>, Option<String>) {
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

            match expr {
                Expr::Lit(ExprLit {
                    lit: Lit::Str(str), ..
                }) => Some(str.value()),
                _ => None,
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
pub(crate) fn attributes_to_asserts(
    mode: ContractMode,
    exprs: Vec<Expr>,
    desc: String,
) -> proc_macro2::TokenStream {
    let mut stream = proc_macro2::TokenStream::new();

    let generate = |expr: &Expr, desc: &str| {
        let format_args = quote::quote! {
            concat!(concat!(#desc, ": "), stringify!(#expr))
        };

        match mode {
            ContractMode::Always => {
                quote::quote! {
                    assert!(#expr, #format_args);
                }
            }
            ContractMode::Disabled => {
                quote::quote! {}
            }
            ContractMode::Debug => {
                quote::quote! {
                    debug_assert!(#expr, #format_args);
                }
            }
            ContractMode::Test => {
                quote::quote! {
                    #[cfg(test)]
                    {
                        assert!(#expr, #format_args);
                    }
                }
            }
            ContractMode::LogOnly => {
                quote::quote! {
                    if !(#expr) {
                        log::error!(#format_args);
                    }
                }
            }
        }
    };

    for expr in exprs {
        stream.extend(generate(&expr, &desc));
    }

    stream
}
