use crate::error::Result;
use async_trait::async_trait;

#[async_trait]
pub trait AmqpPublisher: Send + Sync {
    async fn publish(&self, exchange: &str, routing_key: &str, payload: &[u8]) -> Result<()>;
}
