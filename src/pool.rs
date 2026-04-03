use crate::error::{AmqpError, Result};
use deadpool::managed::{Manager, Pool, RecycleError, RecycleResult};
use lapin::{Channel, Connection, ConnectionProperties};
use std::future::Future;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct AmqpConnectionManager {
    uri: String,
    connection_name: String,
}

impl AmqpConnectionManager {
    pub fn new(uri: String, connection_name: String) -> Self {
        Self { uri, connection_name }
    }
}

impl Manager for AmqpConnectionManager {
    type Type = Connection;
    type Error = lapin::Error;

    fn create(&self) -> impl Future<Output = std::result::Result<Self::Type, Self::Error>> + Send {
        let uri = self.uri.clone();
        let connection_name = self.connection_name.clone();
        async move {
            let opts = ConnectionProperties::default()
                .with_connection_name(connection_name.clone().into());

            Connection::connect(&uri, opts).await
        }
    }

    #[allow(clippy::manual_async_fn)]
    fn recycle(&self, conn: &mut Self::Type, _metrics: &deadpool::managed::Metrics) -> impl Future<Output = RecycleResult<Self::Error>> + Send {
        async move {
            if conn.status().connected() {
                Ok(())
            } else {
                Err(RecycleError::Backend(
                    std::io::Error::new(std::io::ErrorKind::ConnectionReset, "Connection not connected").into(),
                ))
            }
        }
    }
}

pub type AmqpPool = Pool<AmqpConnectionManager>;

pub fn create_pool(uri: String, connection_name: String, max_size: usize) -> Result<AmqpPool> {
    let manager = AmqpConnectionManager::new(uri, connection_name);
    let pool = Pool::builder(manager)
        .max_size(max_size)
        .build()
        .map_err(|e| AmqpError::PoolError(e.to_string()))?;

    Ok(pool)
}

#[derive(Clone)]
pub struct ChannelPool {
    pool: Arc<AmqpPool>,
    channel: Arc<Mutex<Option<Channel>>>,
}

impl ChannelPool {
    pub fn new(pool: Arc<AmqpPool>) -> Self {
        Self {
            pool,
            channel: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn get_channel(&self) -> Result<Channel> {
        let mut cached = self.channel.lock().await;

        if let Some(channel) = cached.as_ref()
            && channel.status().connected()
        {
            return Ok(channel.clone());
        }

        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| AmqpError::PoolError(e.to_string()))?;

        let channel = conn
            .create_channel()
            .await
            .map_err(AmqpError::ConnectionError)?;
        *cached = Some(channel.clone());

        Ok(channel)
    }
}
