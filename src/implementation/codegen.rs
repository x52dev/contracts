/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use proc_macro2::{Ident, Span, TokenStream};
use quote::ToTokens;
use syn::{
    spanned::Spanned,
    visit::{visit_return_type, Visit},
    visit_mut::{self as visitor, visit_block_mut, visit_expr_mut, VisitMut},
    Attribute, Expr, ExprCall, ReturnType, TypeImplTrait,
};

use crate::implementation::{Contract, ContractMode, ContractType, FuncWithContracts};

/// Substitution for `old()` expressions.
pub(crate) struct OldExpr {
    /// Name of the variable binder.
    pub(crate) name: String,
    /// Expression to be evaluated.
    pub(crate) expr: Expr,
}

/// Extract calls to the pseudo-function `old()` in post-conditions,
/// which evaluates an expression in a context *before* the
/// to-be-checked-function is executed.
pub(crate) fn extract_old_calls(contracts: &mut [Contract]) -> Vec<OldExpr> {
    struct OldExtractor {
        last_id: usize,
        olds: Vec<OldExpr>,
    }

    // if the call is a call to old() then the argument will be
    // returned.
    fn get_old_data(call: &ExprCall) -> Option<Expr> {
        // must have only one argument
        if call.args.len() != 1 {
            return None;
        }

        if let Expr::Path(path) = &*call.func {
            if path.path.is_ident("old") {
                Some(call.args[0].clone())
            } else {
                None
            }
        } else {
            None
        }
    }

    impl visitor::VisitMut for OldExtractor {
        fn visit_expr_mut(&mut self, expr: &mut Expr) {
            if let Expr::Call(call) = expr {
                if let Some(mut old_arg) = get_old_data(call) {
                    // if it's a call to old() then add to list of
                    // old expressions and continue to check the
                    // argument.

                    self.visit_expr_mut(&mut old_arg);

                    let id = self.last_id;
                    self.last_id += 1;

                    let old_var_name = format!("__contract_old_{}", id);

                    let old_expr = OldExpr {
                        name: old_var_name.clone(),
                        expr: old_arg,
                    };

                    self.olds.push(old_expr);

                    // override the original expression with the new variable
                    // identifier
                    *expr = {
                        let span = expr.span();

                        let ident = syn::Ident::new(&old_var_name, span);

                        let toks = quote::quote_spanned! { span=> #ident };

                        syn::parse(toks.into()).unwrap()
                    };
                } else {
                    // otherwise continue visiting the expression call
                    visitor::visit_expr_call_mut(self, call);
                }
            } else {
                visitor::visit_expr_mut(self, expr);
            }
        }
    }

    let mut extractor = OldExtractor {
        last_id: 0,
        olds: vec![],
    };

    for contract in contracts {
        if contract.ty != ContractType::Ensures {
            continue;
        }

        for assertion in &mut contract.assertions {
            extractor.visit_expr_mut(assertion);
        }
    }

    extractor.olds
}

fn get_assert_macro(
    ctype: ContractType, // only Pre/Post allowed.
    mode: ContractMode,
    span: Span,
) -> Option<Ident> {
    if cfg!(feature = "mirai_assertions") {
        match (ctype, mode) {
            (ContractType::Requires, ContractMode::Always) => {
                Some(Ident::new("checked_precondition", span))
            }
            (ContractType::Requires, ContractMode::Debug) => {
                Some(Ident::new("debug_checked_precondition", span))
            }
            (ContractType::Requires, ContractMode::Test) => {
                Some(Ident::new("debug_checked_precondition", span))
            }
            (ContractType::Requires, ContractMode::Disabled) => {
                Some(Ident::new("precondition", span))
            }
            (ContractType::Requires, ContractMode::LogOnly) => {
                Some(Ident::new("precondition", span))
            }
            (ContractType::Ensures, ContractMode::Always) => {
                Some(Ident::new("checked_postcondition", span))
            }
            (ContractType::Ensures, ContractMode::Debug) => {
                Some(Ident::new("debug_checked_postcondition", span))
            }
            (ContractType::Ensures, ContractMode::Test) => {
                Some(Ident::new("debug_checked_postcondition", span))
            }
            (ContractType::Ensures, ContractMode::Disabled) => {
                Some(Ident::new("postcondition", span))
            }
            (ContractType::Ensures, ContractMode::LogOnly) => {
                Some(Ident::new("postcondition", span))
            }
            (ContractType::Invariant, _) => {
                panic!("expected Invariant to be narrowed down to Pre/Post")
            }
        }
    } else {
        match mode {
            ContractMode::Always => Some(Ident::new("assert", span)),
            ContractMode::Debug => Some(Ident::new("debug_assert", span)),
            ContractMode::Test => Some(Ident::new("debug_assert", span)),
            ContractMode::Disabled => None,
            ContractMode::LogOnly => None,
        }
    }
}

/// Generate the resulting code for this function by inserting assertions.
pub(crate) fn generate(
    mut func: FuncWithContracts,
    docs: Vec<Attribute>,
    olds: Vec<OldExpr>,
) -> TokenStream {
    let func_name = func.function.sig.ident.to_string();

    // creates an assertion appropriate for the current mode
    let make_assertion = |mode: ContractMode,
                          ctype: ContractType,
                          display: TokenStream,
                          exec_expr: &Expr,
                          desc: &str| {
        let span = display.span();
        let mut result = TokenStream::new();

        let format_args = quote::quote_spanned! { span=>
            concat!(concat!(#desc, ": "), stringify!(#display))
        };

        if mode == ContractMode::LogOnly {
            result.extend(quote::quote_spanned! { span=>
                #[allow(clippy::nonminimal_bool)]
                {
                    if !(#exec_expr) {
                        log::error!("{}", #format_args);
                    }
                }
            });
        }

        if let Some(assert_macro) = get_assert_macro(ctype, mode, span) {
            result.extend(quote::quote_spanned! { span=>
                #[allow(clippy::nonminimal_bool)] {
                    #assert_macro!(#exec_expr, "{}", #format_args);
                }
            });
        }

        if mode == ContractMode::Test {
            quote::quote_spanned! { span=>
              #[cfg(test)] {
                #result
              }
            }
        } else {
            result
        }
    };

    //
    // generate assertion code for pre-conditions
    //

    let pre = func
        .contracts
        .iter()
        .filter(|c| c.ty == ContractType::Requires || c.ty == ContractType::Invariant)
        .flat_map(|c| {
            let contract_type_name = if c.ty == ContractType::Invariant {
                format!("{} (as pre-condition)", c.ty.message_name())
            } else {
                c.ty.message_name().to_string()
            };

            let desc = if let Some(desc) = c.desc.as_ref() {
                format!("{} of {} violated: {}", contract_type_name, func_name, desc)
            } else {
                format!("{} of {} violated", contract_type_name, func_name)
            };

            c.assertions
                .iter()
                .zip(c.streams.iter())
                .map(move |(expr, display)| {
                    let mode = c.mode.final_mode();

                    make_assertion(
                        mode,
                        ContractType::Requires,
                        display.clone(),
                        expr,
                        &desc.clone(),
                    )
                })
        })
        .collect::<TokenStream>();

    //
    // generate assertion code for post-conditions
    //

    let post = func
        .contracts
        .iter()
        .filter(|c| c.ty == ContractType::Ensures || c.ty == ContractType::Invariant)
        .flat_map(|c| {
            let contract_type_name = if c.ty == ContractType::Invariant {
                format!("{} (as post-condition)", c.ty.message_name())
            } else {
                c.ty.message_name().to_string()
            };

            let desc = if let Some(desc) = c.desc.as_ref() {
                format!("{} of {} violated: {}", contract_type_name, func_name, desc)
            } else {
                format!("{} of {} violated", contract_type_name, func_name)
            };

            c.assertions
                .iter()
                .zip(c.streams.iter())
                .map(move |(expr, display)| {
                    let mode = c.mode.final_mode();

                    make_assertion(
                        mode,
                        ContractType::Ensures,
                        display.clone(),
                        expr,
                        &desc.clone(),
                    )
                })
        })
        .collect::<TokenStream>();

    //
    // bind "old()" expressions
    //

    let olds = {
        let mut toks = TokenStream::new();

        for old in olds {
            let span = old.expr.span();

            let name = syn::Ident::new(&old.name, span);

            let expr = old.expr;

            let binding = quote::quote_spanned! { span=>
                let #name = #expr;
            };

            toks.extend(Some(binding));
        }

        toks
    };

    //
    // wrap the function body in a block so that we can use its return value
    //

    let body = 'blk: {
        let mut block = func.function.block.clone();
        visit_block_mut(&mut ReturnReplacer, &mut block);

        let mut impl_detector = ImplDetector { found_impl: false };
        visit_return_type(&mut impl_detector, &func.function.sig.output);

        if !impl_detector.found_impl {
            if let ReturnType::Type(.., ref return_type) = func.function.sig.output {
                break 'blk quote::quote! {
                    let ret: #return_type = 'run: #block;
                };
            }
        }

        quote::quote! {
            let ret = 'run: #block;
        }
    };

    //
    // create a new function body containing all assertions
    //

    let new_block = quote::quote! {

        {
            #pre

            #olds

            #body

            #post

            ret
        }

    };

    // insert documentation attributes

    func.function.attrs.extend(docs);

    // replace the old function body with the new one

    func.function.block = Box::new(syn::parse_quote!(#new_block));

    func.function.into_token_stream()
}

struct ReturnReplacer;

impl VisitMut for ReturnReplacer {
    fn visit_expr_mut(&mut self, node: &mut Expr) {
        if let Expr::Return(ret_expr) = node {
            let ret_expr_expr = ret_expr.expr.clone();
            *node = syn::parse_quote!(break 'run #ret_expr_expr);
        }

        visit_expr_mut(self, node);
    }
}

struct ImplDetector {
    found_impl: bool,
}

impl<'a> Visit<'a> for ImplDetector {
    fn visit_type_impl_trait(&mut self, _node: &'a TypeImplTrait) {
        self.found_impl = true;
    }
}
