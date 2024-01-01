use crate::{create_ident, get_function_types, FunctionTypes};
use proc_macro::TokenStream as TS;
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{punctuated::Punctuated, token::Comma, ItemImpl, Pat};

pub fn persistent(org: TokenStream, nodes: ItemImpl) -> TS {
    let mut arg_enum = quote!();
    let mut new_impl = quote! {
        pub async fn new(addr: A) -> Result<Self, RpcError> {
            let stream = tokio::net::TcpStream::connect(&addr).await?;
            Ok(Self { addr, stream })
        }
    };
    let mut serve_impl = quote! {match value };
    let this_type = nodes.self_ty;

    let arg_name = create_ident(&format!("__{}RpcArg", this_type.to_token_stream()));
    let struct_name = create_ident(&format!("{}Rpc", this_type.to_token_stream()));

    let FunctionTypes {
        _generics,
        functions,
    }: FunctionTypes = get_function_types(nodes.items);

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
                // TODO should respect mutability here!
                pub async fn #i (&mut self, #t ) -> Result< #ret_string , crate::RpcError > {
                    use tokio::net::TcpStream;
                    use tokio::io::{ AsyncWriteExt, AsyncReadExt };

                    let stream = &mut self.stream;

                    let value = ::rusty_procedure_call::postcard::to_allocvec(& #arg_name :: #i #args)?;
                    let len = value.len() as u64;

                    stream.write_u64(len).await?;
                    stream.write_all(&value[..]).await?;

                    let len = stream.read_u64().await? as usize;
                    let mut buf = vec![0; len];
                    stream.read_exact(&mut buf).await?;

                    let res = ::rusty_procedure_call::postcard :: from_bytes :: < #ret_string >(&buf[..])?;

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
        #serve_impl {
            #serve_match
        };
        Ok(())
    );

    let arg_enum = quote! {
        #[allow(non_camel_case_types)]
        #[derive(serde::Deserialize,serde::Serialize, Debug)]
        enum #arg_name {
            #arg_enum
        }
    };

    let rpc_struct = quote! {
        pub struct #struct_name<A: tokio::net::ToSocketAddrs> {
            addr: A,
            stream: tokio::net::TcpStream,
        }
        impl<A: tokio::net::ToSocketAddrs> #struct_name<A> {
            pub fn serve(&mut self) {

            }
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

        #rpc_struct

        impl<A: tokio::net::ToSocketAddrs> #struct_name<A> {
            #new_impl
        }
    };

    let output = quote! {
        #org
        #structy
        #arg_enum
    };

    output.into()
}
