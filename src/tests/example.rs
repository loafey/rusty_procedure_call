use super::block_async;
use crate::RpcError;
use rpc_derive::rpc;
use tokio::net::TcpListener;

#[test]
#[serial_test::serial]
fn example() {
    std::thread::spawn(|| block_async(server(8080)));
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

async fn server(port: u16) -> Result<(), RpcError> {
    let listener = TcpListener::bind(&format!("127.0.0.1:{port}")).await?;
    let mut me = Server { value: 0 };

    loop {
        let (mut socket, _) = listener.accept().await?;
        me.serve(&mut socket).await?;
    }
}
