use crate::{create_ident, get_function_types, FunctionTypes};
use proc_macro::TokenStream as TS;
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{punctuated::Punctuated, token::Comma, ItemImpl, Pat};

pub fn persistent(org: TokenStream, nodes: ItemImpl) -> TS {
    let this_type = nodes.self_ty;

    let server_name = create_ident(&format!("{}Server", this_type.to_token_stream()));
    let arg_name = create_ident(&format!("__{}RpcArg", this_type.to_token_stream()));
    let client_name = create_ident(&format!("{}Client", this_type.to_token_stream()));
    let client_connection_name =
        create_ident(&format!("{}Connection", this_type.to_token_stream()));
    let mut arg_enum = quote!();
    let mut serve_match = quote!();
    let mut new_impl = quote!();

    let FunctionTypes {
        _generics,
        functions,
    }: FunctionTypes = get_function_types(nodes.items);

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

                    // Bottleneck here!
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
                let mut stream = self.__client_channels.get_mut(&id).ok_or(::rusty_procedure_call::RpcError::MissingClient)?;
                stream.send(bytes).await?;
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

    let server_struct = quote! {
        pub struct #server_name {
            port: u16,
            auth: u64,
            connections: std::collections::HashMap<u64, #client_connection_name>,
        }
        impl #server_name {
            pub fn new(port: u16) -> Self {
                Self {
                    port,
                    auth: u64::MAX,
                    connections: std::collections::HashMap::new()
                }
            }
            pub fn serve(mut self) -> Result<std::thread::JoinHandle<()>, ::rusty_procedure_call::RpcError> {
                use message_io::{
                    network::{NetEvent, Transport::*},
                    node::{self, NodeEvent},
                };

                let (handler, listener) = node::split::<()>();
                let addr = format!("127.0.0.1:{}",self.port);
                handler.network().listen(FramedTcp, &addr)?;
                handler.network().listen(Udp, &addr)?;

                let t = std::thread::spawn(move || {
                    listener.for_each(move |event| match event.network() {
                        NetEvent::Connected(_, _) => unreachable!(), // Used for explicit connections.
                        NetEvent::Accepted(endpoint, _listener) => {
                            // self.connections.insert(endpoint, listener);
                        }, // Tcp or Ws
                        NetEvent::Message(endpoint, data) => {
                            let message: Message = postcard::from_bytes(&data).unwrap();
                            match message {
                                Message::ConnectTcp(id) => {
                                    println!("{RED}SERVER{RESET} - Starting TCP connection from: {endpoint}");
                                    if let Some(_) = self.connections.insert(id, #client_connection_name {
                                        tcp: endpoint,
                                        udp: unsafe { std::mem::zeroed() },
                                    }) {
                                        panic!("TODO: Double TCP connection! Please add more info here!");
                                    } else {
                                        if self.auth == u64::MAX {
                                            self.auth = id;
                                            println!("{RED}SERVER{RESET} - setting connection {id} as authorative client");
                                        }
                                    }
                                }
                                Message::ConnectUdp(id) => {
                                    println!("{RED}SERVER{RESET} - Starting UDP connection from: {endpoint}");
                                    if let Some(r) = self.connections.get_mut(&id){
                                        if r.udp == unsafe { std::mem::zeroed() } {
                                            r.udp = endpoint;
                                            let ret = postcard::to_allocvec(&Message::OkUdp).unwrap();
                                            handler.network().send(r.tcp, &ret);
                                        } else {
                                            panic!("TODO: Double UDP connection!");
                                        }
                                    } else {
                                        panic!("TODO: Unknown UDP connection!!");
                                    }
                                }
                                _ => {
                                    let ret = postcard::to_allocvec(&Message::Ok).unwrap();
                                    handler.network().send(endpoint, &ret);
                                }
                            };
                        }
                        NetEvent::Disconnected(_endpoint) => println!("Client disconnected"), //Tcp or Ws
                    });
                });

                Ok(t)
            }
        }
    };

    let client_connection = quote! {
        #[derive(Debug)]
        pub struct #client_connection_name {
            udp: message_io::network::Endpoint,
            tcp: message_io::network::Endpoint
        }
    };

    let client_struct = quote! {
        pub struct #client_name {
            inner: #this_type,
            addr: std::net::SocketAddr
        }
        impl #client_name {
            pub fn new(inner: #this_type, addr: impl std::net::ToSocketAddrs) -> Self {
                Self {
                    inner,
                    // TODO remove this unwrap
                    addr: addr.to_socket_addrs().unwrap().next().unwrap()
                }
            }
        }
    };
    let arg_enum = quote! {
        #[allow(non_camel_case_types)]
        #[derive(serde::Deserialize,serde::Serialize, Debug)]
        enum #arg_name {
            #arg_enum
        }
    };

    quote! {
        #org
        #arg_enum
        #client_connection
        #server_struct
        #client_struct
    }
    .into()
}
