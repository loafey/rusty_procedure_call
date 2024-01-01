# rusty_procedure_call
More or less just a proof of concept for generating async RPC interfaces using 
Rust's macro system. This is far from production ready and should not be used
in a professional manner. 

The idea is to make it easier to write RPC interfacing without getting bogged down with working on networking and such.

Built on top of [`tokio`](https://docs.rs/tokio/), and uses [`postcard`](https://docs.rs/postcard/) to serialize/deserialize messages.

## The gist of it
This crate provides two things; `RpcError`, which is simply the error type
for the library, and more excitingly the `#[rpc]` attribute macro.

This macro is applied to an `impl` block for a type, and will automatically
create two things; a serve function which you can pass `TcpStream`s to, and 
handles RPC calls, and a `<type>Rpc` struct. This struct acts as your 
clients connection point and includes RPC versions of all public self referential 
functions your `impl` block contains (i.e `pub fn func(&self)` etc),
wrapped in a `Result<T, RpcError>` to handle potential failure. 

See the example below for a working demonstration.

## The future
As said this crate is mostly a proof of concept, and could be improved in a number of ways. Listed here are some things I would like to work on:
| Goal | Status |
|------|--------|
| Allow for persistent connections. | ⌛ |
| The ability to use something other than `postcard`, if need be. | ❌ |
| Documentation. | ❌ |
| Support for generic parameters on the type we are using `#[rpc]` on (unsure if doable or desirable). | | 
| Any kind of built-in security. | ❌ | 
| Might be outside the scope of this project, but the ability to separate the client code and server code into two separate code bases. This might be beneficial in cases where the client does not need the server code and  vise-versa. | ❌ | 
| Proper macro errors and macro etiquette, still new to making macros. | ❌ | 
| Currently users have to add `tokio` and `serde` as dependencies of their projects, unsure if this is desired.  | ❌ |

Probably a lot more outside of these points.

## Example
See `src/tests/example.rs` for actual implementation.
```rs
use crate::RpcError;
use rpc_derive::rpc;
use tokio::net::TcpListener;
use std::future::Future;

struct Server {
    value: u128,
}
#[rpc]
impl Server {
    pub fn add_one(&mut self) {
        self.value += 1;
    }

    pub fn get_value(&self) -> u128 {
        self.value
    }
}

async fn server() -> Result<(), RpcError> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    let mut me = Server { value: 0 };

    loop {
        let (mut socket, _) = listener.accept().await?;
        me.serve(&mut socket).await?;
    }
}

fn block_async<T>(v: impl Future<Output = T>) -> T {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed building the Runtime")
        .block_on(v)
}

fn main() {
    std::thread::spawn(|| block_async(server()));
    block_async(async {
        // ServerRpc is created automatically by the #[rpc] macro
        let server = ServerRpc::new("127.0.0.1:8080");
        let max = 100;
        for _ in 0..max {
            // Call `add_one` on the server, incrementing it on the server.
            server.add_one().await.expect("failed to contact server");
        }
        // Get the server's value.
        let res = server.get_value().await.expect("failed to contact server");
        assert_eq!(max, res)
    });
}
```
