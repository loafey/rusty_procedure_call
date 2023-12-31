#![allow(unused)]
use crate::RpcError;
use tokio::net::TcpListener;

fn main() {
    let ports = 8080..8082;
    let mut threads = Vec::new();
    for port in ports.clone() {
        let ports = ports.clone();
        threads.push(std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed building the Runtime")
                .block_on(rpc(port, ports.filter(|p| *p != port).collect()))
        }));
    }

    threads.into_iter().for_each(|j| {
        let _ = j.join();
    });
}

async fn rpc(port: u16, others: Vec<u16>) -> Result<(), RpcError> {
    let listener = TcpListener::bind(&format!("127.0.0.1:{port}")).await?;
    let mut me = Node { value: 0 };
    let others = others
        .into_iter()
        .map(|n| NodeRpc::new(format!("127.0.0.1:{n}")))
        .collect::<Vec<_>>();

    for other in others {
        tokio::task::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                println!("{port} {:?}", other.alive().await);
                println!(
                    "{port} {:?}",
                    other.print(format!("Dingus {port} says hello")).await
                );
                println!("{port} {:?}", other.add_to_me(10).await)
            }
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
#[crate::rpc]
impl Node {
    pub fn alive(&self) -> bool {
        true
    }

    pub fn add_to_me(&mut self, i: usize) -> usize {
        self.value += i;
        self.value
    }

    pub fn print(&self, string: String) {
        println!("{string}");
    }
}
