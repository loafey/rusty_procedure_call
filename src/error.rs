use thiserror::Error;

#[derive(Error, Debug)]
pub enum RpcError {
    #[error("io error")]
    IOError(#[from] std::io::Error),

    #[error("serialize error")]
    SerializeError(#[from] postcard::Error),
}
