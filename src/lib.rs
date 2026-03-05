pub mod error;
pub mod middleware;
pub mod pool;
pub mod publisher;
pub mod registry;
pub mod subscriber;
pub mod traits;
pub mod utils;
pub mod worker;

pub use error::{AmqpError, Result};
pub use middleware::{Middleware, get_execution_time_us, get_headers};
pub use pool::{AmqpConnectionManager, AmqpPool, ChannelPool, create_pool};
pub use publisher::Publisher;
pub use registry::{HandlerFn, SubscriberRegistry};
pub use subscriber::Subscriber;
pub use traits::AmqpPublisher;
pub use utils::generate_trace_id;
pub use worker::{
    BuiltWorker, RetryConfig, SpawnFn, WorkerBuilder,
};

use std::sync::Arc;

pub struct AmqpClient {
    channel_pool: Arc<ChannelPool>,
}

impl AmqpClient {
    pub fn new(uri: String, max_size: usize) -> Result<Self> {
        let pool = Arc::new(create_pool(uri, max_size)?);
        let channel_pool = Arc::new(ChannelPool::new(pool));

        Ok(Self { channel_pool })
    }

    pub async fn get_channel(&self) -> Result<lapin::Channel> {
        self.channel_pool.get_channel().await
    }

    pub fn publisher(&self) -> Publisher {
        Publisher::new(self.channel_pool.clone())
    }

    pub fn channel_pool(&self) -> Arc<ChannelPool> {
        self.channel_pool.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_client() {
        let client = AmqpClient::new("amqp://guest:guest@localhost:5672".to_string(), 10);
        assert!(client.is_ok());
    }
}
