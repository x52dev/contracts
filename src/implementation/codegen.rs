/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use proc_macro2::{Ident, Span, TokenStream};
use quote::ToTokens;
use syn::{
    spanned::Spanned, visit_mut as visitor, Attribute, Expr, ExprCall,
    ReturnType,
};

use crate::implementation::{
    Contract, ContractMode, ContractType, FuncWithContracts,
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
        if contract.ty != ContractType::Ensures {
            continue;
        }

        for assertion in &mut contract.assertions {
            use visitor::VisitMut;
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
                          display: proc_macro2::TokenStream,
                          exec_expr: &Expr,
                          desc: &str| {
        let span = display.span();
        let mut result = proc_macro2::TokenStream::new();

        let format_args = quote::quote_spanned! { span=>
            concat!(concat!(#desc, ": "), stringify!(#display))
        };

        if mode == ContractMode::LogOnly {
            result.extend(
                quote::quote_spanned! { span=>
                    if !(#exec_expr) {
                        log::error!(#format_args);
                    }
                }
                .into_iter(),
            );
        }

        if let Some(assert_macro) = get_assert_macro(ctype, mode, span) {
            result.extend(
                quote::quote_spanned! { span=>
                    #assert_macro!(#exec_expr, #format_args);
                }
                .into_iter(),
            );
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

    let pre: proc_macro2::TokenStream = func
        .contracts
        .iter()
        .filter(|c| {
            c.ty == ContractType::Requires || c.ty == ContractType::Invariant
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

            c.assertions.iter().zip(c.streams.iter()).map(
                move |(expr, display)| {
                    let mode = c.mode.final_mode();

                    make_assertion(
                        mode,
                        ContractType::Requires,
                        display.clone(),
                        expr,
                        &desc.clone(),
                    )
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
            c.ty == ContractType::Ensures || c.ty == ContractType::Invariant
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

            c.assertions.iter().zip(c.streams.iter()).map(
                move |(expr, display)| {
                    let mode = c.mode.final_mode();

                    make_assertion(
                        mode,
                        ContractType::Ensures,
                        display.clone(),
                        expr,
                        &desc.clone(),
                    )
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
    // wrap the function body in a closure if we have any postconditions
    //

    let body = if post.is_empty() {
        let block = &func.function.block;
        quote::quote! { let ret = #block; }
    } else {
        create_closure_body_and_adjust_signature(&func.function)
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

struct SelfReplacer<'a> {
    replace_with: &'a syn::Ident,
}

impl syn::visit_mut::VisitMut for SelfReplacer<'_> {
    fn visit_ident_mut(&mut self, i: &mut Ident) {
        if i == "self" {
            *i = self.replace_with.clone();
        }
    }
}

fn ty_contains_impl_trait(ty: &syn::Type) -> bool {
    use syn::visit::Visit;

    struct TyContainsImplTrait {
        seen_impl_trait: bool,
    }

    impl syn::visit::Visit<'_> for TyContainsImplTrait {
        fn visit_type_impl_trait(&mut self, _: &syn::TypeImplTrait) {
            self.seen_impl_trait = true;
        }
    }

    let mut vis = TyContainsImplTrait {
        seen_impl_trait: false,
    };
    vis.visit_type(ty);
    vis.seen_impl_trait
}

fn create_closure_body_and_adjust_signature(func: &syn::ItemFn) -> TokenStream {
    let is_method = func.sig.receiver().is_some();

    // If the function has a receiver (e.g. `self`, `&mut self`, or `self: Box<Self>`) rename it
    // to `this__` within the closure

    let mut block = func.block.clone();
    let mut closure_args = vec![];
    let mut arg_names = vec![];

    if is_method {
        let this_ident = syn::Ident::new("this__", Span::call_site());

        let mut receiver = func.sig.inputs[0].clone();
        match receiver {
            // self, &self, &mut self
            syn::FnArg::Receiver(rcv) => {
                // The `Self` type.
                let self_ty = Box::new(syn::Type::Path(syn::TypePath {
                    qself: None,
                    path: syn::Path::from(syn::Ident::new("Self", rcv.span())),
                }));

                let ty = if let Some((and_token, ref lifetime)) = rcv.reference
                {
                    Box::new(syn::Type::Reference(syn::TypeReference {
                        and_token,
                        lifetime: lifetime.clone(),
                        mutability: rcv.mutability,
                        elem: self_ty,
                    }))
                } else {
                    self_ty
                };

                let pat_mut = if rcv.reference.is_none() {
                    rcv.mutability
                } else {
                    None
                };

                // this__: [& [mut]] Self
                let new_rcv = syn::PatType {
                    attrs: rcv.attrs.clone(),
                    pat: Box::new(syn::Pat::Ident(syn::PatIdent {
                        attrs: vec![],
                        by_ref: None,
                        mutability: pat_mut,
                        ident: this_ident.clone(),
                        subpat: None,
                    })),
                    colon_token: syn::Token![:](rcv.span()),
                    ty,
                };

                receiver = syn::FnArg::Typed(new_rcv);
            }

            // self: Box<Self>
            syn::FnArg::Typed(ref mut pat) => {
                if let syn::Pat::Ident(ref mut ident) = *pat.pat {
                    if ident.ident == "self" {
                        ident.ident = this_ident.clone();
                    }
                }
            }
        }

        closure_args.push(receiver);
        arg_names.push(syn::Ident::new("self", Span::call_site()));

        // Replace any references to `self` in the function body with references to `this__`.
        syn::visit_mut::visit_block_mut(
            &mut SelfReplacer {
                replace_with: &this_ident,
            },
            &mut block,
        );
    }

    // Replace any pattern matching in the function signature with placeholder identifiers.
    // Pattern matching gets done in the closure instead.
    let args = func.sig.inputs.iter().skip(if is_method { 1 } else { 0 });
    for arg in args {
        match arg {
            syn::FnArg::Receiver(_) => unreachable!("Multiple receivers?"),

            syn::FnArg::Typed(syn::PatType { pat, ty, .. }) => {
                if !ty_contains_impl_trait(ty) {
                    if let syn::Pat::Ident(ident) = &**pat {
                        arg_names.push(ident.ident.clone());
                        closure_args.push(arg.clone());
                    }
                }
            }
        }
    }

    let ret_ty = match &func.sig.output {
        ReturnType::Type(_, ty) => {
            let span = ty.span();
            match ty.as_ref() {
                syn::Type::ImplTrait(_) => quote::quote! {},
                ty => quote::quote_spanned! { span=>
                    -> #ty
                },
            }
        }
        ReturnType::Default => quote::quote! {},
    };

    let closure_args = closure_args.iter();
    let arg_names = arg_names.iter();

    quote::quote! {
        #[allow(unused_mut)]
        let mut run = |#(#closure_args),*| #ret_ty #block;

        let ret = run(#(#arg_names),*);
    }
}
