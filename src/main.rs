use error::RpcError;
use rpc_derive::rpc;
use std::{error::Error, fmt::format};
use tokio::{
    io::AsyncWrite,
    net::{TcpListener, TcpStream},
};

mod error;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let ports = 8080..8082;
    for port in ports.clone() {
        let ports = ports.clone();
        std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed building the Runtime")
                .block_on(rpc(port, ports.filter(|p| *p != port).collect()))
        });
    }
}

async fn rpc(port: u16, others: Vec<u16>) -> Result<(), RpcError> {
    let listener = TcpListener::bind(&format!("127.0.0.1:{port}")).await?;
    let mut me = Node { value: 0 };
    let others = others
        .into_iter()
        .map(|n| NodeRpc::new(format!("127.0.0.1:{n}")))
        .collect::<Vec<_>>();

    for other in others {
        tokio::task::spawn(async {
            // tokio::
        });
    }

    loop {
        let (mut socket, _) = listener.accept().await?;
        me.serve(&mut socket).await?;
    }
}

struct Node {
    value: usize,
}
#[rpc]
impl Node {
    fn r#priv(&self) -> bool {
        false
    }

    pub fn alive(&self) -> bool {
        true
    }

    pub fn add_one(&self, i: usize) -> usize {
        i + 1
    }

    pub fn print(&self, string: String) {}
}
