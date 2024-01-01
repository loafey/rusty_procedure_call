extern crate proc_macro;
use proc_macro::TokenStream as TS;
use proc_macro2::{Ident, TokenStream};
use quote::{quote, ToTokens};
use syn::{
    punctuated::Punctuated, token::Comma, GenericParam, ImplItem, ItemImpl, Pat, PatType,
    ReturnType, Type, Visibility,
};

use crate::Attr;

fn get_generics(_ty: &PatType) -> Punctuated<GenericParam, Comma> {
    Punctuated::new()
}

type Function = (Ident, Punctuated<PatType, Comma>, Option<Type>);
struct FunctionTypes {
    _generics: Punctuated<GenericParam, Comma>,
    functions: Vec<Function>,
}
fn get_function_types(items: Vec<ImplItem>) -> FunctionTypes {
    let mut _generics = Punctuated::new();
    let mut functions = Vec::new();
    for item in items {
        if let ImplItem::Fn(func) = item {
            if !matches!(func.vis, Visibility::Public(..)) {
                continue;
            }
            let func_name = func.sig.ident;
            let mut args: Punctuated<_, Comma> = Punctuated::new();
            let mut is_self_ref = false;
            for arg in func.sig.inputs.into_iter() {
                match arg {
                    syn::FnArg::Receiver(r) => {
                        is_self_ref = r.reference.is_some();
                    }
                    syn::FnArg::Typed(t) => {
                        _generics.extend(get_generics(&t));
                        args.push(t);
                    }
                }
            }
            let ret_type = match func.sig.output {
                ReturnType::Default => None,
                ReturnType::Type(_, t) => Some(*t),
            };
            if is_self_ref {
                functions.push((func_name, args, ret_type));
            }
        }
    }
    FunctionTypes {
        _generics,
        functions,
    }
}

fn create_ident(s: &str) -> Ident {
    syn::Ident::new(s, proc_macro2::Span::call_site())
}

pub fn non_persistent(attr: Attr, org: TokenStream, nodes: ItemImpl) -> TS {
    let mut arg_enum = quote!();
    let mut new_impl = quote! {
        pub fn new(addr: A) -> Self{
            Self { addr }
        }
    };
    let mut serve_impl = quote! {match value };
    let this_type = nodes.self_ty;

    let arg_name = create_ident(&format!("__{}RpcArg", this_type.to_token_stream()));
    let struct_name = create_ident(&format!("{}Rpc", this_type.to_token_stream()));

    let FunctionTypes {
        _generics,
        functions,
    } = get_function_types(nodes.items);

    let mut serve_match = quote!();

    for (i, t, r) in functions {
        let ret_string = if let Some(ret) = &r {
            quote!(#ret)
        } else {
            quote!(())
        };
        let args_without_types = t
            .clone()
            .into_iter()
            .filter_map(|p| {
                if let Pat::Ident(i) = *p.pat {
                    Some(i.ident)
                } else {
                    None
                }
            })
            .collect::<Punctuated<_, Comma>>();
        new_impl = {
            let args = if args_without_types.is_empty() {
                quote!()
            } else {
                quote!((#args_without_types))
            };

            quote! {
                #new_impl
                pub async fn #i (&self, #t ) -> Result< #ret_string , crate::RpcError > {
                    use tokio::net::TcpStream;
                    use tokio::io::{ AsyncWriteExt, AsyncReadExt };

                    let mut stream = TcpStream::connect(&self.addr).await?;

                    let value = ::rusty_procedure_call::postcard::to_allocvec(& #arg_name :: #i #args)?;
                    let len = value.len() as u64;

                    stream.write_u64(len).await?;
                    stream.write_all(&value[..]).await?;

                    let len = stream.read_u64().await? as usize;
                    let mut buf = vec![0; len];
                    stream.read_exact(&mut buf).await?;

                    let res = ::rusty_procedure_call::postcard :: from_bytes :: < #ret_string >(&buf[..])?;

                    stream.shutdown().await?;

                    Ok(res)
                }
            }
        };

        let res_call = if !args_without_types.is_empty() {
            quote!(
                let res = self. #i (#args_without_types);
            )
        } else {
            quote!(
                let res = self.#i();
            )
        };

        let m = if !args_without_types.is_empty() {
            quote!(#arg_name :: #i (#args_without_types))
        } else {
            quote!(#arg_name :: #i)
        };

        serve_match = quote! {
            #serve_match
            #m => {
                #res_call
                let bytes = ::rusty_procedure_call::postcard::to_allocvec(&res)?;
                let len = bytes.len() as u64;
                stream.write_u64(len).await?;
                stream.write_all(&bytes[..]).await?;
            },
        };

        if !t.is_empty() {
            let t = t
                .into_iter()
                .map(|t| t.ty)
                .collect::<Punctuated<_, Comma>>();
            arg_enum = quote! {
                #arg_enum
                #i (#t),
            };
        } else {
            arg_enum = quote! {
                #arg_enum
                #i,
            };
        }
    }

    serve_impl = quote!(
        #serve_impl { #serve_match };
        Ok(())
    );

    let arg_enum = quote! {
        #[allow(non_camel_case_types)]
        #[derive(serde::Deserialize,serde::Serialize, Debug)]
        enum #arg_name {
            #arg_enum
        }
    };

    let structy = quote! {
        impl #this_type {
            pub async fn serve(&mut self, stream: &mut tokio::net::TcpStream) -> Result<(), crate::RpcError> {
                use tokio::io::{AsyncWriteExt, AsyncReadExt};
                let len = stream.read_u64().await?;
                let mut buf = vec![0; len as usize];
                stream.read_exact(&mut buf).await?;
                let value = ::rusty_procedure_call::postcard::from_bytes(&buf[..])?;
                #serve_impl
            }
        }

        pub struct #struct_name<A: tokio::net::ToSocketAddrs> {
            addr: A
        }

        impl<A: tokio::net::ToSocketAddrs> #struct_name<A> {
            #new_impl
        }
    };

    let temp = format!("{attr:?}");
    let temp = quote!(const TEST: &str = #temp;);
    let output = quote! {
        #temp
        #org
        #structy
        #arg_enum
    };

    output.into()
}
