/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use proc_macro::TokenStream;
use syn::{parse::Parser, punctuated::Punctuated, Expr, ExprLit, Lit, Token};

/// Parse attributes into a list of expression and an optional description of the assert
pub(crate) fn parse_attributes(
    attrs: TokenStream,
) -> (Vec<Expr>, Option<String>) {
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
