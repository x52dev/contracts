use crate::implementation::{ContractMode, ContractType, FuncWithContracts};

use proc_macro::TokenStream;
use quote::ToTokens;
use syn::{spanned::Spanned, Attribute, Block, Expr, ReturnType};

/// Generate the resulting code for this function by inserting assertions.
pub(crate) fn generate(
    mut func: FuncWithContracts,
    docs: Vec<Attribute>,
) -> TokenStream {
    let func_name = func.function.ident.to_string();

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

            c.assertions.iter().map(move |a| {
                let mode = c.mode.final_mode();

                make_assertion(mode, a, &desc.clone())
            })
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

            c.assertions.iter().map(move |a| {
                let mode = c.mode.final_mode();

                make_assertion(mode, a, &desc.clone())
            })
        })
        .collect();

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
