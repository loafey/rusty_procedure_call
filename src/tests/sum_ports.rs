use super::block_async;
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
#[serial_test::serial]
fn sum_ports() {
    let sum = PORTS.into_iter().sum::<u64>();
    let nodes = PORTS
        .into_iter()
        .map(|port| PortHolderRpc::new(format!("127.0.0.1:{port}")));

    // Spin up X number of RPC servers waiting for one request
    for port in PORTS {
        std::thread::spawn(move || block_async(rpc(port, 1)));
    }
    let total = nodes.map(|node| block_async(node.my_port()).unwrap()).sum();
    assert_eq!(sum, total)
}

#[test]
#[serial_test::serial]
fn sum_ports_plus_one() {
    let sum = PORTS.into_iter().map(|p| p + 1).sum::<u64>();
    let nodes = PORTS
        .into_iter()
        .map(|port| PortHolderRpc::new(format!("127.0.0.1:{port}")));

    // Spin up X number of RPC servers waiting for one request
    for port in PORTS {
        std::thread::spawn(move || block_async(rpc(port, 2)));
    }
    let total = nodes
        .map(|node| {
            block_async(node.plus_one()).unwrap();
            block_async(node.my_port()).unwrap()
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
