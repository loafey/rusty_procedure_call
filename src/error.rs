use thiserror::Error;

#[derive(Error, Debug)]
pub enum RpcError {
    #[error("missing client")]
    MissingClient,

    #[error("io error")]
    IOError(#[from] std::io::Error),

    #[error("serialize error")]
    SerializeError(#[from] postcard::Error),

    #[error("mpsc send error")]
    MPSCSendError(#[from] tokio::sync::mpsc::error::SendError<Vec<u8>>),
}
