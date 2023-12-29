use std::future::Future;

pub use error::RpcError;
pub use rpc_derive::rpc;

mod error;

fn block_async<T>(v: impl Future<Output = T>) -> T {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed building the Runtime")
        .block_on(v)
}

#[cfg(test)]
mod sum_ports {
    use tokio::net::TcpListener;

    struct PortHolder {
        port: u64,
    }
    #[crate::rpc]
    impl PortHolder {
        pub fn my_port(&self) -> u64 {
            self.port
        }

        // Yea, this one doesn't really make sense, but it showcases that we can
        // have mutability.
        pub fn plus_one(&mut self) {
            self.port += 1;
        }
    }

    const PORTS: [u64; 11] = [
        8080, 8081, 8082, 8083, 8084, 8085, 8086, 8087, 8088, 8089, 8090,
    ];

    #[test]
    fn sum_ports() {
        let sum = PORTS.into_iter().sum::<u64>();
        let nodes = PORTS
            .into_iter()
            .map(|port| PortHolderRpc::new(format!("127.0.0.1:{port}")));

        // Spin up X number of RPC servers waiting for one request
        for port in PORTS {
            std::thread::spawn(move || crate::block_async(rpc(port, 1)));
        }
        let total = nodes
            .map(|node| crate::block_async(node.my_port()).unwrap())
            .sum();
        assert_eq!(sum, total)
    }

    #[test]
    fn sum_ports_plus_one() {
        let sum = PORTS.into_iter().map(|p| p + 1).sum::<u64>();
        let nodes = PORTS
            .into_iter()
            .map(|port| PortHolderRpc::new(format!("127.0.0.1:{port}")));

        // Spin up X number of RPC servers waiting for one request
        for port in PORTS {
            std::thread::spawn(move || crate::block_async(rpc(port, 2)));
        }
        let total = nodes
            .map(|node| {
                crate::block_async(node.plus_one()).unwrap();
                crate::block_async(node.my_port()).unwrap()
            })
            .sum();
        assert_eq!(sum, total)
    }

    #[allow(unused)]
    async fn rpc(port: u64, serve_amount: usize) -> Result<(), crate::RpcError> {
        let listener = TcpListener::bind(format!("127.0.0.1:{port}")).await?;
        let mut me = PortHolder { port };
        for _ in 0..serve_amount {
            let (mut socket, _) = listener.accept().await?;
            me.serve(&mut socket).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod unused_test {
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
}
