use thiserror::Error;

#[derive(Error, Debug)]
pub enum AmqpError {
    #[error("Connection error: {0}")]
    ConnectionError(#[from] lapin::Error),

    #[error("Pool error: {0}")]
    PoolError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Channel error: {0}")]
    ChannelError(String),
}

impl From<String> for AmqpError {
    fn from(msg: String) -> Self {
        AmqpError::ChannelError(msg)
    }
}

impl From<&str> for AmqpError {
    fn from(msg: &str) -> Self {
        AmqpError::ChannelError(msg.to_string())
    }
}

pub type Result<T> = std::result::Result<T, AmqpError>;
