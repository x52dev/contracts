/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use proc_macro2::{Spacing, TokenStream, TokenTree};
use syn::{Expr, ExprLit, Lit};

/// Parse attributes into a list of expression and an optional description of
/// the assert
pub(crate) fn parse_attributes(
    attrs: TokenStream,
) -> (Vec<Expr>, Vec<TokenStream>, Option<String>) {
    let segments = segment_input(attrs.into());
    let mut segments_stream: Vec<TokenStream> = segments
        .iter()
        .map(|x| x.iter().cloned().collect::<TokenStream>())
        .collect();
    let rewritten_segs: Vec<_> = segments.into_iter().map(rewrite).collect();

    let mut conds: Vec<Expr> = vec![];

    for seg in rewritten_segs {
        let expr = match syn::parse2::<Expr>(seg) {
            Ok(val) => val,
            Err(err) => Expr::Verbatim(err.to_compile_error()),
        };
        conds.push(expr);
    }

    let desc = conds
        .last()
        .map(|expr| match expr {
            Expr::Lit(ExprLit {
                lit: Lit::Str(str), ..
            }) => Some(str.value()),
            _ => None,
        })
        .unwrap_or(None);

    if desc.is_some() {
        conds.pop();
        segments_stream.pop();
    }

    let exprs = conds.into_iter().map(|e| e).collect();

    (exprs, segments_stream, desc)
}

// This function rewrites a list of TokenTrees so that the "pseudooperator" for
// implication `==>` gets transformed into an `if` expression.
//
// This has to happen on a TokenStream/Tree because it's not possible to easily
// add new syntax to the syn parsers without basically re-writing the whole
// expression parsing functions from scratch.
//
// The input gets classified into "before `==>` op" and "after `==>` op". Those
// two groups are then used to create an `if` that has implication semantics.
// However, because the input is only split based on the operator, no precedence
// is respected, including keywords such as `if`. This means the implication
// operator should only be used in grouped expressions.
// This also has the effect that implication is right-associative, which is the
// expected behaviour.
fn rewrite(segments: Vec<TokenTree>) -> proc_macro2::TokenStream {
    let mut lhs = vec![];
    let mut rhs: Option<_> = None;
    let mut span: Option<_> = None;

    let mut idx = 0;

    'segment: while let Some(tt) = segments.get(idx) {
        match tt {
            TokenTree::Group(group) => {
                let stream: Vec<_> = group.stream().into_iter().collect();

                let new_stream: TokenStream =
                    rewrite(stream).into_iter().collect();

                let mut new_group =
                    proc_macro2::Group::new(group.delimiter(), new_stream);
                new_group.set_span(group.span());

                lhs.push(TokenTree::Group(new_group));
                idx += 1;
            }
            TokenTree::Ident(_) => {
                lhs.push(tt.clone());
                idx += 1;
            }
            TokenTree::Literal(_) => {
                lhs.push(tt.clone());
                idx += 1;
            }
            TokenTree::Punct(_) => {
                let punct = |idx: usize, c: char, s: Spacing| -> bool {
                    let tt = if let Some(val) = segments.get(idx) {
                        val
                    } else {
                        return false;
                    };

                    if let TokenTree::Punct(p) = tt {
                        p.as_char() == c && p.spacing() == s
                    } else {
                        false
                    }
                };

                if punct(idx, '=', Spacing::Joint)
                    && punct(idx + 1, '=', Spacing::Joint)
                    && punct(idx + 2, '>', Spacing::Alone)
                {
                    // found the implication
                    let rest = Vec::from(&segments[idx + 3..]);
                    let rhs_stream = rewrite(rest);

                    rhs = Some(rhs_stream);
                    span = Some(segments[idx + 2].span());
                    break 'segment;
                } else {
                    // consume all so that =========> would not match with
                    // implication
                    'op: while let Some(tt) = segments.get(idx) {
                        match tt {
                            TokenTree::Punct(p) => {
                                if p.spacing() == Spacing::Alone {
                                    // read this one but finish afterwards
                                    lhs.push(tt.clone());
                                    idx += 1;
                                    break 'op;
                                } else {
                                    // this punctuation is a joint-punctuation
                                    // so needs to be read and then continue
                                    lhs.push(tt.clone());
                                    idx += 1;
                                }
                            }
                            _ => {
                                // not a punktuation, so do not increase idx
                                break 'op;
                            }
                        }
                    }
                }
            }
        }
    }

    match (rhs, span) {
        (None, None) => lhs.into_iter().collect(),
        (None, Some(_)) => {
            unreachable!("If there's a span there should be an implication")
        }
        (Some(_), None) => unreachable!("Invalid spans"),
        (Some(rhs), Some(span)) => {
            let lhs: TokenStream = lhs.into_iter().collect();

            quote::quote_spanned! {
                span =>
                if !#lhs { true } else { #rhs }
            }
        }
    }
}

// The tokenstream can contain multiple expressions to be checked, separated by
// a comma. This function "pulls" those expressions apart.
fn segment_input(tts: TokenStream) -> Vec<Vec<TokenTree>> {
    let mut groups = vec![];

    let mut group = vec![];

    for tt in tts {
        match tt {
            TokenTree::Punct(p)
                if p.as_char() == ',' && p.spacing() == Spacing::Alone =>
            {
                groups.push(group);
                group = vec![];
            }
            t => group.push(t),
        }
    }

    if !group.is_empty() {
        groups.push(group);
    }

    groups
}
