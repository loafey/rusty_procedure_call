extern crate proc_macro;
use proc_macro::TokenStream as TS;
use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{
    punctuated::Punctuated, token::Comma, GenericParam, ImplItem, ItemImpl, Lifetime,
    LifetimeParam, ReturnType, Type, TypeParam, Visibility,
};

fn get_generics(ty: &Type) -> Punctuated<GenericParam, Comma> {
    let mut res = Punctuated::new();
    res
}

type Function = (Ident, Vec<TypeParam>, Option<Type>);
struct FunctionTypes {
    generics: Punctuated<GenericParam, Comma>,
    functions: Vec<Function>,
}
fn get_function_types(items: Vec<ImplItem>) -> FunctionTypes {
    let mut generics = Punctuated::new();
    let mut functions = Vec::new();
    for item in items {
        if let ImplItem::Fn(func) = item {
            if !matches!(func.vis, Visibility::Public(..)) {
                continue;
            }
            let func_name = func.sig.ident;
            let mut args = Vec::new();
            let mut is_self_ref = false;
            for arg in func.sig.inputs.into_iter() {
                match arg {
                    syn::FnArg::Receiver(r) => {
                        is_self_ref = r.reference.is_some();
                    }
                    syn::FnArg::Typed(t) => {
                        let t = t.ty;
                        generics.extend(get_generics(&t));
                        args.push(t);
                    }
                }
            }
            let ret_type = match func.sig.output {
                ReturnType::Default => None,
                ReturnType::Type(_, t) => Some(*t),
            };
            if is_self_ref {
                functions.push((func_name, Vec::new(), ret_type));
            }
        }
    }
    FunctionTypes {
        generics,
        functions,
    }
}

fn parse_impl_block(org: TokenStream, nodes: ItemImpl) -> TS {
    let mut arg_enum = quote!();
    let mut res_enum = quote!();
    let this_type = nodes.self_ty;
    let mut args_generics: Punctuated<GenericParam, Comma> = Punctuated::new();
    for item in nodes.items {
        if let syn::ImplItem::Fn(func) = item {
            if !matches!(func.vis, Visibility::Public(..)) {
                continue;
            }
            let func_name = func.sig.ident;
            let mut args = quote!();
            let mut is_self_ref = false;
            for arg in func.sig.inputs.into_iter() {
                match arg {
                    syn::FnArg::Receiver(r) => {
                        is_self_ref = r.reference.is_some();
                    }
                    syn::FnArg::Typed(t) => {
                        let t = t.ty;
                        args_generics.extend(get_generics(&t));
                        args = quote! {
                            #args
                            #t,
                        }
                    }
                }
            }
            let ret_type = match func.sig.output {
                ReturnType::Default => None,
                ReturnType::Type(_, t) => Some(*t),
            };
            if is_self_ref {
                arg_enum = quote! {
                    #arg_enum
                    #func_name ( #args ),
                };
                if let Some(ret_type) = ret_type {
                    res_enum = quote! {
                        #res_enum
                        #func_name ( #ret_type ),
                    };
                } else {
                    res_enum = quote! {
                        #res_enum
                        #func_name,
                    }
                }
            }
        }
    }

    let arg_name = syn::Ident::new(
        &format!("{}RpcArg", this_type.to_token_stream()),
        proc_macro2::Span::call_site(),
    );
    let res_name = syn::Ident::new(
        &format!("{}RpcRes", this_type.to_token_stream()),
        proc_macro2::Span::call_site(),
    );

    let arg_enum = if args_generics.is_empty() {
        quote!(
            #[allow(non_camel_case_types)]
            #[derive(serde::Deserialize,serde::Serialize)]
            enum #arg_name {
                #arg_enum
            }
        )
    } else {
        let mut punc = Punctuated::new();
        punc.extend(args_generics);
        let generics = syn::Generics {
            lt_token: None,
            params: punc,
            gt_token: None,
            where_clause: None,
        };
        quote!(
            #[allow(non_camel_case_types)]
            #[derive(serde::Deserialize,serde::Serialize)]
            enum #arg_name #generics {
                #arg_enum
            }
        )
    };

    let res_enum = quote! {
        #[allow(non_camel_case_types)]
        #[derive(serde::Deserialize,serde::Serialize)]
        enum #res_name {
            #res_enum
        }
    };

    let struct_name = syn::Ident::new(
        &format!("{}Rpc", this_type.to_token_stream()),
        proc_macro2::Span::call_site(),
    );
    let structy = quote! {
        struct #struct_name<A: tokio::net::ToSocketAddrs> {
            addr: A
        }
    };

    let output = quote! {
        #org
        #structy
        #arg_enum
        #res_enum
    };

    output.into()
}

#[proc_macro_attribute]
pub fn rpc(_attr: TS, item: TS) -> TS {
    let org = TokenStream::from(item.clone());
    if let Ok(nodes) = syn::parse::<ItemImpl>(item) {
        parse_impl_block(org, nodes)
    } else {
        todo!()
    }
}
