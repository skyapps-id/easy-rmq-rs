use crate::{
    error::{AmqpError, Result},
    pool::ChannelPool,
    traits::AmqpPublisher,
};
use lapin::{BasicProperties, options::*, types::FieldTable};
use std::sync::Arc;

#[derive(Clone)]
pub struct Publisher {
    channel_pool: Arc<ChannelPool>,
    exchange: String,
    trace_id: Option<String>,
}

impl Publisher {
    pub fn new(channel_pool: Arc<ChannelPool>) -> Self {
        Self {
            channel_pool,
            exchange: "amq.direct".to_string(),
            trace_id: None,
        }
    }

    pub fn with_exchange(mut self, exchange: impl Into<String>) -> Self {
        self.exchange = exchange.into();
        self
    }

    pub fn with_trace_id(mut self, trace_id: String) -> Self {
        self.trace_id = Some(trace_id);
        self
    }

    pub fn with_auto_trace_id(mut self) -> Self {
        self.trace_id = Some(crate::generate_trace_id());
        self
    }

    pub async fn publish_json<T: serde::Serialize>(
        &self,
        routing_key: &str,
        payload: &T,
    ) -> Result<()> {
        let json = serde_json::to_vec(payload).map_err(AmqpError::SerializationError)?;
        self.publish(&self.exchange, routing_key, &json).await
    }

    pub async fn publish_json_to<T: serde::Serialize>(
        &self,
        exchange: &str,
        routing_key: &str,
        payload: &T,
    ) -> Result<()> {
        let json = serde_json::to_vec(payload).map_err(AmqpError::SerializationError)?;
        self.publish(exchange, routing_key, &json).await
    }

    pub async fn publish_text(&self, routing_key: &str, payload: &str) -> Result<()> {
        self.publish(&self.exchange, routing_key, payload.as_bytes())
            .await
    }

    pub async fn publish_text_to(
        &self,
        exchange: &str,
        routing_key: &str,
        payload: &str,
    ) -> Result<()> {
        self.publish(exchange, routing_key, payload.as_bytes())
            .await
    }

    async fn publish_with_trace(&self, exchange: &str, routing_key: &str, payload: &[u8]) -> Result<()> {
        let channel = self.channel_pool.get_channel().await?;

        let mut headers = FieldTable::default();
        if let Some(ref trace_id) = self.trace_id {
            headers.insert(
                "x-trace-id".into(),
                lapin::types::AMQPValue::LongString(trace_id.clone().into()),
            );
        }

        let props = BasicProperties::default()
            .with_delivery_mode(2)
            .with_headers(headers);

        channel
            .basic_publish(
                exchange.into(),
                routing_key.into(),
                BasicPublishOptions::default(),
                payload,
                props,
            )
            .await
            .map_err(AmqpError::ConnectionError)?
            .await
            .map_err(AmqpError::ConnectionError)?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl AmqpPublisher for Publisher {
    async fn publish(&self, exchange: &str, routing_key: &str, payload: &[u8]) -> Result<()> {
        if self.trace_id.is_some() {
            self.publish_with_trace(exchange, routing_key, payload).await
        } else {
            let channel = self.channel_pool.get_channel().await?;

            let props = BasicProperties::default().with_delivery_mode(2);

            channel
                .basic_publish(
                    exchange.into(),
                    routing_key.into(),
                    BasicPublishOptions::default(),
                    payload,
                    props,
                )
                .await
                .map_err(AmqpError::ConnectionError)?
                .await
                .map_err(AmqpError::ConnectionError)?;

            Ok(())
        }
    }
}
