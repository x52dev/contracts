/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use crate::implementation::{Contract, ContractMode};
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::ToTokens;
use syn::{parse::Parser, Attribute, Expr};

pub(crate) fn generate_attributes(contracts: &[Contract]) -> Vec<Attribute> {
    let mut attrs = vec![];

    fn make_attribute(content: &str) -> Attribute {
        let span = Span::call_site();

        let content_str = syn::LitStr::new(content, span);

        let toks: TokenStream =
            quote::quote_spanned!( span=> #[doc = #content_str] ).into();

        let parser = Attribute::parse_outer;

        let mut attributes = parser.parse(toks).unwrap();

        attributes.remove(0)
    }

    fn print_expr(expr: &Expr) -> String {
        let toks = expr.into_token_stream();
        toks.to_string()
    }

    // header
    attrs.push(make_attribute("# Contracts"));

    for contract in contracts {
        let ty = contract.ty;
        let mode = match contract.mode {
            ContractMode::Always => None,
            ContractMode::Disabled => None,
            ContractMode::Debug => Some("debug"),
            ContractMode::Test => Some("test"),
            ContractMode::LogOnly => None,
        };

        if let Some(desc) = &contract.desc {
            // document all assertions under the description

            let header_txt = if let Some(name) = mode {
                format!("{} - {}: {}", ty.message_name(), name, desc)
            } else {
                format!("{}: {}", ty.message_name(), desc)
            };

            attrs.push(make_attribute(&header_txt));

            for assert in &contract.assertions {
                attrs.push(make_attribute(&format!(
                    " - `{}`",
                    print_expr(assert)
                )));
            }

            attrs.push(make_attribute(""));
        } else {
            // document each assertion on its own

            for assert in &contract.assertions {
                let doc_str = if let Some(name) = mode {
                    format!(
                        "{} - {}: `{}`",
                        ty.message_name(),
                        name,
                        print_expr(assert)
                    )
                } else {
                    format!("{}: `{}`", ty.message_name(), print_expr(assert))
                };

                attrs.push(make_attribute(&doc_str));
                attrs.push(make_attribute(""));
            }
        }
    }

    attrs
}
