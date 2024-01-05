use crate::{create_ident, get_function_types, FunctionTypes};
use proc_macro::TokenStream as TS;
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{punctuated::Punctuated, token::Comma, ItemImpl, Pat};

pub fn persistent(org: TokenStream, nodes: ItemImpl) -> TS {
    let this_type = nodes.self_ty;

    let server_name = create_ident(&format!("{}Server", this_type.to_token_stream()));
    let client_name = create_ident(&format!("{}Client", this_type.to_token_stream()));

    let server_struct = quote! {
        pub struct #server_name {
            port: u16,
            inner: #this_type
        }
        impl #server_name {
            pub fn new(inner: #this_type, port: u16) -> Self {
                Self {
                    port,
                    inner
                }
            }
            pub fn serve(self) -> Result<std::thread::JoinHandle<()>, ::rusty_procedure_call::RpcError> {
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
                        NetEvent::Accepted(_endpoint, _listener) => println!("Client connected"), // Tcp or Ws
                        NetEvent::Message(endpoint, data) => {
                            println!("Server - Received: {}", String::from_utf8_lossy(data));
                            handler.network().send(endpoint, data);
                        }
                        NetEvent::Disconnected(_endpoint) => println!("Client disconnected"), //Tcp or Ws
                    });
                });

                Ok(t)
            }
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

    quote! {
        #org
        #server_struct
        #client_struct
    }
    .into()
}
