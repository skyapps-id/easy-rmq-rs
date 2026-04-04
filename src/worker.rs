use crate::{ChannelPool, Result, Subscriber, default_exchange_for_kind, middleware::Middleware, registry::HandlerFn};
use lapin::ExchangeKind;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

pub struct Data<T>(pub Arc<T>);

impl<T> Data<T> {
    pub fn new(data: T) -> Self {
        Self(Arc::new(data))
    }
}

impl<T> Clone for Data<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> AsRef<T> for Data<T> {
    fn as_ref(&self) -> &T {
        &self.0
    }
}

impl<T> std::ops::Deref for Data<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct WorkerBuilderWithData<T> {
    builder: WorkerBuilder,
    data: Arc<T>,
}

impl<T> WorkerBuilderWithData<T>
where
    T: Send + Sync + 'static + Clone,
{
    pub fn pool(mut self, pool: Arc<ChannelPool>) -> Self {
        self.builder.channel_pool = Some(pool);
        self
    }

    pub fn with_exchange(mut self, exchange: impl Into<String>) -> Self {
        self.builder.exchange = exchange.into();
        self
    }

    pub fn routing_key(mut self, routing_key: impl Into<String>) -> Self {
        self.builder.routing_key = Some(routing_key.into());
        self
    }

    pub fn queue(mut self, queue: impl Into<String>) -> Self {
        self.builder.queue = Some(queue.into());
        self
    }

    pub fn retry(mut self, max_retries: u32, delay_ms: u64) -> Self {
        self.builder.retry_enabled = true;
        self.builder.max_retries = max_retries;
        self.builder.retry_delay = Duration::from_millis(delay_ms);
        self
    }

    pub fn prefetch(mut self, count: u16) -> Self {
        self.builder.prefetch = count;
        self
    }

    pub fn concurrency(mut self, count: u16) -> Self {
        self.builder.concurrency = Some(count);
        self
    }

    pub fn parallelize<F>(mut self, spawn_fn: F) -> Self
    where
        F: Fn(
                Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>>,
            ) -> tokio::task::JoinHandle<Result<()>>
            + Send
            + Sync
            + 'static,
    {
        self.builder.spawn_fn = Some(Arc::new(spawn_fn));
        self
    }

    pub fn single_active_consumer(mut self, enabled: bool) -> Self {
        self.builder.single_active_consumer = enabled;
        self
    }

    pub fn middleware<M>(mut self, middleware: M) -> Self
    where
        M: Middleware + 'static,
    {
        self.builder.middlewares.push(Arc::new(middleware));
        self
    }

    pub fn build<F, Fut>(self, handler: F) -> BuiltWorker
    where
        F: Fn(Data<T>, Vec<u8>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let pool = self.builder.channel_pool.expect("Pool must be set with .pool()");

        let data = self.data.clone();
        let handler_arc = Arc::new(handler);

        let wrapped_handler = move |payload: Vec<u8>| -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>> {
            let data = data.clone();
            let handler = handler_arc.clone();
            Box::pin(async move {
                handler(Data(data), payload).await
            })
        };

        let subscriber = Subscriber::new(pool, self.builder.exchange_kind.clone())
            .with_exchange(&self.builder.exchange)
            .with_single_active_consumer(self.builder.single_active_consumer);

        BuiltWorker {
            exchange_kind: self.builder.exchange_kind,
            subscriber,
            routing_key: self.builder.routing_key,
            queue: self.builder.queue.expect("queue required"),
            handler: Box::new(wrapped_handler),
            retry_max_retries: if self.builder.retry_enabled { Some(self.builder.max_retries) } else { None },
            retry_delay: if self.builder.retry_enabled { Some(self.builder.retry_delay) } else { None },
            prefetch: self.builder.prefetch,
            concurrency: self.builder.concurrency,
            spawn_fn: self.builder.spawn_fn,
            middlewares: self.builder.middlewares,
        }
    }
}

pub type SpawnFn = Arc<
    dyn Fn(
            Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>>,
        ) -> tokio::task::JoinHandle<Result<()>>
        + Send
        + Sync,
>;

pub struct WorkerBuilder {
    exchange_kind: ExchangeKind,
    exchange: String,
    channel_pool: Option<Arc<ChannelPool>>,
    routing_key: Option<String>,
    queue: Option<String>,
    retry_enabled: bool,
    max_retries: u32,
    retry_delay: Duration,
    prefetch: u16,
    concurrency: Option<u16>,
    spawn_fn: Option<SpawnFn>,
    single_active_consumer: bool,
    middlewares: Vec<Arc<dyn Middleware>>,
}

impl WorkerBuilder {
    pub fn new(exchange_kind: ExchangeKind) -> Self {
        let exchange = default_exchange_for_kind(&exchange_kind);

        Self {
            exchange_kind,
            exchange,
            channel_pool: None,
            routing_key: None,
            queue: None,
            retry_enabled: false,
            max_retries: 3,
            retry_delay: Duration::from_secs(60),
            prefetch: 1,
            concurrency: None,
            spawn_fn: None,
            single_active_consumer: false,
            middlewares: Vec::new(),
        }
    }

    pub fn pool(mut self, pool: Arc<ChannelPool>) -> Self {
        self.channel_pool = Some(pool);
        self
    }

    pub fn with_exchange(mut self, exchange: impl Into<String>) -> Self {
        self.exchange = exchange.into();
        self
    }

    pub fn routing_key(mut self, routing_key: impl Into<String>) -> Self {
        self.routing_key = Some(routing_key.into());
        self
    }

    pub fn queue(mut self, queue: impl Into<String>) -> Self {
        self.queue = Some(queue.into());
        self
    }

    pub fn retry(mut self, max_retries: u32, delay_ms: u64) -> Self {
        self.retry_enabled = true;
        self.max_retries = max_retries;
        self.retry_delay = Duration::from_millis(delay_ms);
        self
    }

    pub fn prefetch(mut self, count: u16) -> Self {
        self.prefetch = count;
        self
    }

    pub fn concurrency(mut self, count: u16) -> Self {
        self.concurrency = Some(count);
        self
    }

    pub fn parallelize<F>(mut self, spawn_fn: F) -> Self
    where
        F: Fn(
                Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>>,
            ) -> tokio::task::JoinHandle<Result<()>>
            + Send
            + Sync
            + 'static,
    {
        self.spawn_fn = Some(Arc::new(spawn_fn));
        self
    }

    pub fn single_active_consumer(mut self, enabled: bool) -> Self {
        self.single_active_consumer = enabled;
        self
    }

    pub fn middleware<M>(mut self, middleware: M) -> Self
    where
        M: Middleware + 'static,
    {
        self.middlewares.push(Arc::new(middleware));
        self
    }

    pub fn data<T>(self, data: Data<T>) -> WorkerBuilderWithData<T>
    where
        T: Send + Sync + 'static + Clone,
    {
        WorkerBuilderWithData {
            builder: self,
            data: data.0,
        }
    }

    pub fn build<F, Fut>(self, handler: F) -> BuiltWorker
    where
        F: Fn(Vec<u8>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let pool = self.channel_pool.expect("Pool must be set with .pool()");

        let handler_arc = Arc::new(handler);
        let wrapped_handler = move |payload: Vec<u8>| -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>> {
            let handler = handler_arc.clone();
            Box::pin(async move {
                handler(payload).await
            })
        };

        let subscriber = Subscriber::new(pool, self.exchange_kind.clone())
            .with_exchange(&self.exchange)
            .with_single_active_consumer(self.single_active_consumer);

        BuiltWorker {
            exchange_kind: self.exchange_kind,
            subscriber,
            routing_key: self.routing_key,
            queue: self.queue.expect("queue required"),
            handler: Box::new(wrapped_handler),
            retry_max_retries: if self.retry_enabled { Some(self.max_retries) } else { None },
            retry_delay: if self.retry_enabled { Some(self.retry_delay) } else { None },
            prefetch: self.prefetch,
            concurrency: self.concurrency,
            spawn_fn: self.spawn_fn,
            middlewares: self.middlewares,
        }
    }
}

pub struct BuiltWorker {
    exchange_kind: ExchangeKind,
    subscriber: Subscriber,
    routing_key: Option<String>,
    queue: String,
    handler: HandlerFn,
    retry_max_retries: Option<u32>,
    retry_delay: Option<Duration>,
    prefetch: u16,
    concurrency: Option<u16>,
    spawn_fn: Option<SpawnFn>,
    middlewares: Vec<Arc<dyn Middleware>>,
}

impl BuiltWorker {
    pub async fn run(self) -> Result<()> {
        let subscriber = if let (Some(max_retries), Some(delay)) = (self.retry_max_retries, self.retry_delay) {
            self.subscriber.with_retry(max_retries, delay)
        } else {
            self.subscriber
        };

        let subscriber = subscriber
            .with_prefetch(self.prefetch)
            .with_concurrency(self.concurrency)
            .with_spawn_fn(self.spawn_fn)
            .with_middlewares(self.middlewares);

        match self.exchange_kind {
            ExchangeKind::Direct => {
                subscriber.direct(&self.queue).build(self.handler).await
            }
            ExchangeKind::Topic => {
                let routing_key = self.routing_key.expect("routing_key required for Topic");
                subscriber.topic(&routing_key, &self.queue).build(self.handler).await
            }
            ExchangeKind::Fanout => {
                subscriber.fanout(&self.queue).build(self.handler).await
            }
            _ => panic!("Unsupported exchange kind"),
        }
    }
}
