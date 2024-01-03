use crate::{create_ident, get_function_types, FunctionTypes};
use proc_macro::TokenStream as TS;
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{punctuated::Punctuated, token::Comma, ItemImpl, Pat};

pub fn persistent(org: TokenStream, nodes: ItemImpl) -> TS {
    let mut arg_enum = quote!();
    let mut new_impl = quote! {
        pub async fn new(addr: A, my_id: u64) -> Result<Self, RpcError> {
            use tokio::io::AsyncWriteExt;
            let mut stream = tokio::net::TcpStream::connect(&addr).await?;
            stream.write_u64(my_id).await?;
            Ok(Self { addr, stream, my_id })
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

                    //println!("Writing message of len {len}");

                    stream.write_u64(len).await?;
                    stream.write_all(&value[..]).await?;

                    //println!("Wrote message of len {len}");

                    let time = std::time::Instant::now();
                    let len = stream.read_u64().await? as usize;
                    let mut buf = vec![0; len];
                    stream.read_exact(&mut buf).await?;

                    //println!("Got response of len {len}");

                    let res = ::rusty_procedure_call::postcard :: from_bytes :: < #ret_string >(&buf[..])?;
                    println!("client - reading response time: {}s", time.elapsed().as_secs_f32());

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
                let time = std::time::Instant::now();
                #res_call
                let bytes = ::rusty_procedure_call::postcard::to_allocvec(&res).unwrap();
                let mut stream = self.__client_channels.get_mut(&id).unwrap();
                stream.send(bytes).await?;
                println!("server - process time: {}s", time.elapsed().as_secs_f32());
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
            my_id: u64
        }
        impl<A: tokio::net::ToSocketAddrs> #struct_name<A> {
            pub fn serve(&mut self) {

            }
        }
    };

    let structy = quote! {
        impl #this_type {
            pub async fn serve(&mut self, mut stream: tokio::net::TcpStream, id: u64, mut receiver: tokio::sync::mpsc::Receiver::<Vec<u8>>) -> Result<(), crate::RpcError> {
                let sender = self.__sender.clone();
                tokio::spawn(async move {
                    loop {
                        use tokio::io::{AsyncWriteExt, AsyncReadExt};
                        let mut len_buf = [0; std::mem::size_of::<u64>()];
                        if let Ok(message) = receiver.try_recv() {
                            //println!("Sending over {message:?}");
                            let len = message.len();
                            stream.write_u64(len as u64).await.unwrap();
                            stream.write_all(&message).await.unwrap();
                        }
                        if let Ok(b) = stream.try_read(&mut len_buf) {
                            let len = u64::from_be_bytes(len_buf);
                            //println!("Server got message of len {len} and {b} bytes");
                            let mut buf = vec![0; len as usize];
                            stream.read_exact(&mut buf).await.unwrap();
                            //println!("{buf:?}");
                            let value = ::rusty_procedure_call::postcard::from_bytes(&buf[..]).unwrap();
                            sender.send(__MessageHandler::Message((value, id))).await.unwrap();
                        }
                        // this should not be needed! causes deadlock when removed
                        //tokio::time::sleep(std::time::Duration::from_secs_f32(1.0 / 300.0)).await;
                    }
                });
                Ok(())
            }

            pub async fn handle_messages(&mut self) -> Result<(), crate::RpcError> {
                if let Some(value) = self.__receiver.recv().await {
                    match value {
                        __MessageHandler::Message((value, id)) => {
                            #serve_impl
                        },
                        __MessageHandler::NewConnection(mut socket) => {
                            let id = socket.read_u64().await?;
                            println!("Got client: {id}");
                            let (sender, receiver) = tokio::sync::mpsc::channel(10);
                            self.__client_channels.insert(id, sender);
                            self.serve(socket, id, receiver).await?;
                        }
                    }
                    //println!("Handling message from {id}");
                }
                Ok(())
            }
        }

        #rpc_struct

        impl<A: tokio::net::ToSocketAddrs> #struct_name<A> {
            #new_impl
        }
    };

    let message_handler_message = create_ident("__MessageHandler");
    let message_handler_message = quote!(enum #message_handler_message {
        NewConnection(tokio::net::TcpStream),
        Message(( #arg_name , u64))
    });

    let output = quote! {
        #message_handler_message
        #org
        #structy
        #arg_enum
    };

    output.into()
}
