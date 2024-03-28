use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, ToTokens};
use std::str::FromStr;
use syn::punctuated::Punctuated;
use syn::token::{Brace, Bracket, Comma, Extern, Let, Mut, Paren, PathSep, Pound, Semi, Star, Unsafe};
use syn::{
    Abi, AttrStyle, Attribute, Block, Expr, ExprCall, ExprPath, FnArg,
    ForeignItem, ForeignItemFn, Item, ItemFn, ItemMod, LitStr, Local, LocalInit, Meta, Pat,
    PatIdent, Path, PathArguments, PathSegment, Stmt, Type, TypePtr,
    Visibility,
};

#[proc_macro_attribute]
pub fn dotnet(
    _args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let item = syn::parse_macro_input!(item as ItemMod);

    let output = dotnet_internal(item);

    output.into()
}

pub(crate) fn dotnet_internal(item: ItemMod) -> TokenStream {
    let (_, content) = item.content.unwrap();

    let mut functions = Vec::new();
    for item in &content {
        match item {
            Item::Struct(_s) => {}
            Item::ForeignMod(f) => {
                f.items.iter().for_each(|item| match item {
                    ForeignItem::Fn(f) => create_c_func_from_extern_rust_func(f, &mut functions),
                    ForeignItem::Type(_ty) => {}
                    _ => {}
                });
            }
            _ => {}
        }
    }

    let mod_name = item.ident;

    quote! {
        mod #mod_name {
            #(#functions)*
        }
    }
}

fn create_c_func_from_extern_rust_func(f: &ForeignItemFn, functions: &mut Vec<ItemFn>) {
    let mut cloned_sig = f.sig.clone();
    let mut fn_args = Punctuated::new();
    let mut lines = Vec::new();

    let args = arguments(&cloned_sig.inputs, &mut fn_args, &mut lines)
        .iter()
        .map(|ident| Expr::Verbatim(ident.to_token_stream()))
        .collect();

    cloned_sig.inputs = fn_args;
    cloned_sig.abi = Some(Abi {
        name: Some(LitStr::new("C", Span::call_site())),
        extern_token: Extern::default(),
    });
    cloned_sig.unsafety = Some(Unsafe::default());

    let call = Stmt::Expr(
        Expr::Call(ExprCall {
            attrs: vec![],
            func: Box::new(Expr::Path(ExprPath {
                attrs: vec![],
                path: Path {
                    leading_colon: None,
                    segments: Punctuated::from_iter([
                        PathSegment {
                            ident: Ident::new("super", Span::call_site()),
                            arguments: PathArguments::None,
                        },
                        PathSegment {
                            ident: Ident::new(&f.sig.ident.to_string(), Span::call_site()),
                            arguments: PathArguments::None,
                        },
                    ]),
                },
                qself: None,
            })),
            args,
            paren_token: Paren::default(),
        }),
        None,
    );

    lines.push(call);
    let fun = ItemFn {
        attrs: vec![Attribute {
            pound_token: Pound::default(),
            style: AttrStyle::Outer,
            bracket_token: Bracket::default(),
            meta: Meta::Path(Path {
                leading_colon: None,
                segments: Punctuated::from_iter([PathSegment {
                    ident: Ident::new("no_mangle", Span::call_site()),
                    arguments: Default::default(),
                }]),
            }),
        }],
        vis: Visibility::Inherited,
        sig: cloned_sig,
        block: Box::new(Block {
            brace_token: Brace::default(),
            stmts: lines,
        }),
    };

    functions.push(fun);
}

fn arguments<'a>(
    inputs: &'a Punctuated<FnArg, Comma>,
    new_inputs: &mut Punctuated<FnArg, Comma>,
    new_lines: &mut Vec<Stmt>,
) -> Punctuated<&'a Ident, Comma> {
    let mut args = Punctuated::new();
    for input in inputs {
        let FnArg::Typed(input) = &input else {
            unimplemented!("Methods are not supported!")
        };
        let Pat::Ident(w) = input.pat.as_ref() else {
            unreachable!()
        };

        let mut new_input = input.clone();

        match new_input.ty.as_ref() {
            Type::Path(ty) => {
                let last = ty.path.segments.last().unwrap();
                if last.ident.to_string() == "String" {
                    *new_input.ty.as_mut() = Type::Ptr(TypePtr {
                        star_token: Star(Span::call_site()),
                        const_token: None,
                        mutability: Some(Mut(Span::call_site())),
                        elem: Box::new(Type::Verbatim(
                            TokenStream::from_str("::rust_sharp::RustString").unwrap(),
                        )),
                    });
                    new_lines.push(Stmt::Local(Local {
                        attrs: vec![],
                        pat: Pat::Ident(PatIdent {
                            attrs: vec![],
                            ident: Ident::new(&w.ident.to_string(), Span::call_site()),
                            mutability: None,
                            by_ref: None,
                            subpat: None,
                        }),
                        let_token: Let::default(),
                        init: Some(LocalInit {
                            expr: Box::new(Expr::Call(ExprCall {
                                attrs: vec![],
                                paren_token: Paren::default(),
                                args: Punctuated::from_iter([Expr::Verbatim(
                                    TokenStream::from_str(&format!(
                                        "(*{}).as_mut_string()",
                                        w.ident.to_string()
                                    ))
                                    .unwrap(),
                                )]),
                                func: Box::new(Expr::Path(ExprPath {
                                    attrs: vec![],
                                    path: Path {
                                        leading_colon: Some(PathSep::default()),
                                        segments: Punctuated::from_iter([
                                            PathSegment {
                                                ident: Ident::new("std", Span::call_site()),
                                                arguments: PathArguments::None
                                            },
                                            PathSegment {
                                                ident: Ident::new("mem", Span::call_site()),
                                                arguments: PathArguments::None
                                            },
                                            PathSegment {
                                                ident: Ident::new("take", Span::call_site()),
                                                arguments: PathArguments::None
                                            }
                                        ]),
                                    },
                                    qself: None
                                })),
                            })),
                            diverge: None,
                            eq_token: Default::default(),
                        }),
                        semi_token: Semi::default(),
                    }));
                }
            }
            _ => {}
        }

        new_inputs.push(FnArg::Typed(new_input));

        args.push(&w.ident);
    }
    args
}

mod test {
    #[test]
    fn test_dotnet_internal() {
        use crate::dotnet_internal;

        let parsed = syn::parse_quote! {
        mod bridge {
            #[repr(C)]
            struct RustStruct {
                vector: Vec<i32>
            }

            extern "Rust" {
                type OpaqueType;

                fn function_def() -> i32;
                fn function_def_with_args(arg: String) -> i32;
            }
        }
    };

        let output = dotnet_internal(parsed);

        let item = syn::parse2(output).unwrap();
        let file = syn::File {
            attrs: vec![],
            items: vec![item],
            shebang: None,
        };

        println!("{:#}", prettyplease::unparse(&file))
    }
}
