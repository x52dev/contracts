/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

pub(crate) mod invariant;
pub(crate) mod post;
pub(crate) mod pre;
pub(crate) mod traits;

use proc_macro::TokenStream;
use quote::ToTokens;
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Block, Expr, ExprLit, ItemFn, Lit, ReturnType, Token};

pub(crate) use invariant::invariant;
pub(crate) use post::post;
pub(crate) use pre::pre;
use proc_macro2::{Span, TokenTree};
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

/// The different contract types.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum ContractType {
    Pre,
    Post,
    Invariant,
}

impl ContractType {
    /// Get the name that is used as a message-prefix on violation of a contract.
    pub(crate) fn message_name(self) -> &'static str {
        match self {
            ContractType::Pre => "Pre-condition",
            ContractType::Post => "Post-condition",
            ContractType::Invariant => "Invariant",
        }
    }

    /// Determine the type and mode of an identifier.
    pub(crate) fn contract_type_and_mode(ident: &str) -> Option<(ContractType, ContractMode)> {
        match ident {
            "pre" => Some((ContractType::Pre, ContractMode::Always)),
            "post" => Some((ContractType::Post, ContractMode::Always)),
            "invariant" => Some((ContractType::Invariant, ContractMode::Always)),
            "debug_pre" => Some((ContractType::Pre, ContractMode::Debug)),
            "debug_post" => Some((ContractType::Post, ContractMode::Debug)),
            "debug_invariant" => Some((ContractType::Invariant, ContractMode::Debug)),
            "test_pre" => Some((ContractType::Pre, ContractMode::Test)),
            "test_post" => Some((ContractType::Post, ContractMode::Test)),
            "test_invariant" => Some((ContractType::Invariant, ContractMode::Test)),
            _ => None,
        }
    }
}

/// Representation of a contract
#[derive(Debug)]
pub(crate) struct Contract {
    pub(crate) span: Span,
    pub(crate) ty: ContractType,
    pub(crate) mode: ContractMode,
    pub(crate) assertions: Vec<Expr>,
    pub(crate) desc: Option<String>,
}

impl Contract {
    pub(crate) fn from_toks(ty: ContractType, mode: ContractMode, toks: TokenStream) -> Self {
        let (assertions, desc) = parse_attributes(toks);

        let span = Span::call_site();

        Self {
            span,
            ty,
            mode,
            assertions,
            desc,
        }
    }
}

/// A function that is annotated with contracts
#[derive(Debug)]
pub(crate) struct FuncWithContracts {
    pub(crate) contracts: Vec<Contract>,
    pub(crate) function: ItemFn,
}

impl FuncWithContracts {
    /// Create a `FuncWithContracts` value from the attribute-tokens of the first
    /// contract and a parsed version of the function.
    ///
    /// The initial contract is parsed from the tokens, others will be read from parsed function.
    pub(crate) fn new_with_initial_contract(
        mut func: ItemFn,
        cty: ContractType,
        cmode: ContractMode,
        ctoks: TokenStream,
    ) -> Self {
        // add in the first attribute
        let mut contracts: Vec<Contract> = {
            let initial_contract = Contract::from_toks(cty, cmode, ctoks);
            vec![initial_contract]
        };

        // find all other attributes

        let contract_attrs = func
            .attrs
            .iter()
            .filter_map(|a| {
                let name = a.path.segments.last().unwrap().value().ident.to_string();
                let (ty, mode) = ContractType::contract_type_and_mode(&name)?;
                Some((ty, mode, a))
            })
            .map(|(ty, mode, a)| {
                // the tts on attributes contains the out parenthesis, so some code might
                // be mistakenly parsed as tuples, that's not good!
                //
                // this is a hack to get to the inner token stream.

                let tok_tree = a.tts.clone().into_iter().next().unwrap();
                let toks = match tok_tree {
                    TokenTree::Group(group) => group.stream(),
                    TokenTree::Ident(i) => i.into_token_stream(),
                    TokenTree::Punct(p) => p.into_token_stream(),
                    TokenTree::Literal(l) => l.into_token_stream(),
                };

                Contract::from_toks(ty, mode, toks.into())
            });

        contracts.extend(contract_attrs);

        // remove contract attributes
        {
            let attrs = std::mem::replace(&mut func.attrs, vec![]);

            let other_attrs = attrs
                .into_iter()
                .filter(|attr| {
                    ContractType::contract_type_and_mode(
                        &attr.path.segments.last().unwrap().value().ident.to_string(),
                    )
                    .is_none()
                })
                .collect();

            func.attrs = other_attrs;
        }

        Self {
            function: func,
            contracts,
        }
    }

    /// Generate the resulting code for this function by inserting assertions.
    pub(crate) fn generate(mut self) -> TokenStream {
        let func_name = self.function.ident.to_string();

        // creates an assertion appropriate for the current mode
        let make_assertion = |mode: ContractMode, expr: &Expr, desc: &str| {
            let span = expr.span();

            let format_args = quote::quote_spanned! { span=>
                concat!(concat!(#desc, ": "), stringify!(#expr))
            };

            match mode {
                ContractMode::Always => {
                    quote::quote_spanned! { span=>
                        assert!(#expr, #format_args);
                    }
                }
                ContractMode::Disabled => {
                    quote::quote! {}
                }
                ContractMode::Debug => {
                    quote::quote_spanned! { span=>
                        debug_assert!(#expr, #format_args);
                    }
                }
                ContractMode::Test => {
                    quote::quote_spanned! { span=>
                        #[cfg(test)]
                        {
                            assert!(#expr, #format_args);
                        }
                    }
                }
                ContractMode::LogOnly => {
                    quote::quote_spanned! { span=>
                        if !(#expr) {
                            log::error!(#format_args);
                        }
                    }
                }
            }
        };

        //
        // generate assertion code for pre-conditions
        //

        let pre: proc_macro2::TokenStream = self
            .contracts
            .iter()
            .filter(|c| c.ty == ContractType::Pre || c.ty == ContractType::Invariant)
            .flat_map(|c| {
                let desc = if let Some(desc) = c.desc.as_ref() {
                    format!(
                        "{} of {} violated: {}",
                        c.ty.message_name(),
                        func_name,
                        desc
                    )
                } else {
                    format!("{} of {} violated", c.ty.message_name(), func_name)
                };

                c.assertions.iter().map(move |a| {
                    let mode = final_mode(c.mode);

                    make_assertion(mode, a, &desc.clone())
                })
            })
            .collect();

        //
        // generate assertion code for post-conditions
        //

        let post: proc_macro2::TokenStream = self
            .contracts
            .iter()
            .filter(|c| c.ty == ContractType::Post || c.ty == ContractType::Invariant)
            .flat_map(|c| {
                let desc = if let Some(desc) = c.desc.as_ref() {
                    format!(
                        "{} of {} violated: {}",
                        c.ty.message_name(),
                        func_name,
                        desc
                    )
                } else {
                    format!("{} of {} violated", c.ty.message_name(), func_name)
                };

                c.assertions.iter().map(move |a| {
                    let mode = final_mode(c.mode);

                    make_assertion(mode, a, &desc.clone())
                })
            })
            .collect();

        //
        // wrap the function body in a closure
        //

        let block = self.function.block.clone();

        let ret_ty = if let ReturnType::Type(_, ty) = &self.function.decl.output {
            let span = ty.span();
            quote::quote_spanned! { span=>
                #ty
            }
        } else {
            quote::quote! { () }
        };

        let body = quote::quote! {
            #[allow(unused_mut)]
            let mut run = || -> #ret_ty {
                #block
            };

            let ret = run();
        };

        //
        // create a new function body containing all assertions
        //

        let new_block = quote::quote! {

            {
                #pre

                #body

                #post

                ret
            }

        }
        .into();

        // replace the old function body with the new one

        self.function.block = Box::new(syn::parse_macro_input!(new_block as Block));

        let res = self.function.into_token_stream();

        res.into()
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
