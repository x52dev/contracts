use crate::implementation::{
    Contract, ContractMode, ContractType, FuncWithContracts,
};

use proc_macro::TokenStream;
use quote::ToTokens;
use syn::{
    spanned::Spanned, visit_mut as visitor, Attribute, Block, Expr, ExprCall,
    ReturnType,
};

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
        if contract.ty != ContractType::Post {
            continue;
        }

        for assertion in &mut contract.assertions {
            use visitor::VisitMut;
            extractor.visit_expr_mut(assertion);
        }
    }

    extractor.olds
}

/// Generate the resulting code for this function by inserting assertions.
pub(crate) fn generate(
    mut func: FuncWithContracts,
    docs: Vec<Attribute>,
    olds: Vec<OldExpr>,
) -> TokenStream {
    let func_name = func.function.ident.to_string();

    // creates an assertion appropriate for the current mode
    let make_assertion = |mode: ContractMode,
                          display_expr: &Expr,
                          exec_expr: &Expr,
                          desc: &str| {
        let span = display_expr.span();

        let format_args = quote::quote_spanned! { span=>
            concat!(concat!(#desc, ": "), stringify!(#display_expr))
        };

        match mode {
            ContractMode::Always => {
                quote::quote_spanned! { span=>
                    assert!(#exec_expr, #format_args);
                }
            }
            ContractMode::Disabled => {
                quote::quote! {}
            }
            ContractMode::Debug => {
                quote::quote_spanned! { span=>
                    debug_assert!(#exec_expr, #format_args);
                }
            }
            ContractMode::Test => {
                quote::quote_spanned! { span=>
                    #[cfg(test)]
                    {
                        assert!(#exec_expr, #format_args);
                    }
                }
            }
            ContractMode::LogOnly => {
                quote::quote_spanned! { span=>
                    if !(#exec_expr) {
                        log::error!(#format_args);
                    }
                }
            }
        }
    };

    //
    // generate assertion code for pre-conditions
    //

    let pre: proc_macro2::TokenStream = func
        .contracts
        .iter()
        .filter(|c| {
            c.ty == ContractType::Pre || c.ty == ContractType::Invariant
        })
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

            c.assertions.iter().zip(c.display_assertions.iter()).map(
                move |(expr, display_expr)| {
                    let mode = c.mode.final_mode();

                    make_assertion(mode, display_expr, expr, &desc.clone())
                },
            )
        })
        .collect();

    //
    // generate assertion code for post-conditions
    //

    let post: proc_macro2::TokenStream = func
        .contracts
        .iter()
        .filter(|c| {
            c.ty == ContractType::Post || c.ty == ContractType::Invariant
        })
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

            c.assertions.iter().zip(c.display_assertions.iter()).map(
                move |(expr, display_expr)| {
                    let mode = c.mode.final_mode();

                    make_assertion(mode, display_expr, expr, &desc.clone())
                },
            )
        })
        .collect();

    //
    // bind "old()" expressions
    //

    let olds = {
        let mut toks = proc_macro2::TokenStream::new();

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
    // wrap the function body in a closure
    //

    let block = func.function.block.clone();

    let ret_ty = if let ReturnType::Type(_, ty) = &func.function.decl.output {
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

            #olds

            #body

            #post

            ret
        }

    }
    .into();

    // insert documentation attributes

    func.function.attrs.extend(docs);

    // replace the old function body with the new one

    func.function.block = Box::new(syn::parse_macro_input!(new_block as Block));

    let res = func.function.into_token_stream();

    res.into()
}
