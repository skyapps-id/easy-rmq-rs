# easy_rmq

Rust AMQP library with connection pool, publisher, subscriber, and dependency injection support.

## Features

- **Connection Pool**: Efficiently manages AMQP connections using deadpool
- **Publisher**: Send messages to exchanges with routing keys
- **Subscriber**: Receive messages from queues with handlers
- **Worker Registry**: Register and manage multiple workers with a clean pattern
- **Auto Setup**: Automatically creates exchanges and queues
- **Retry Mechanism**: Automatic retry with delay for failed messages
- **Single Active Consumer**: Ensure only one consumer processes messages at a time
- **Prefetch Control**: AMQP prefetch (QoS) configuration
- **Parallel Processing**: Configurable worker concurrency with async/blocking spawn
- **Middleware**: Custom middleware for logging, metrics, and distributed tracing
- **Distributed Tracing**: Built-in trace ID generation with OpenTelemetry support
- **Dependency Injection**: Support for trait-based DI pattern
- **Type Safe**: Strong error handling with thiserror
- **Async**: Full async support using tokio

## Installation

```toml
[dependencies]
easy_rmq = { path = "./easy_rmq" }
```

## Quick Start

### 1. Start RabbitMQ

```bash
docker run -d --name rabbitmq \
  -p 5672:5672 \
  -p 15672:15672 \
  rabbitmq:3-management
```

### 2. Subscriber Example (Run First!)

Open terminal 1 - Subscriber sets up queue & binding:
```bash
cargo run --example subscriber
```

### 3. Publisher Example (Run Second)

Open terminal 2:
```bash
cargo run --example publisher
```

Press `Ctrl+C` on subscriber for graceful shutdown.

## Architecture & Best Practices

🎯 **Simple & Clean:**
- **Default Exchange**: `amq.direct` (RabbitMQ built-in)
- **Publisher**: Auto-create exchange + send messages
- **Subscriber**: Auto-create exchange + queue + binding
- **Worker Registry**: Register multiple workers with clean pattern
- **Retry**: Automatic retry with delay for failed messages
- **Prefetch**: AMQP QoS control for message buffering
- **Concurrency**: Parallel worker processing
- **Full Auto-Setup**: No manual infrastructure needed

This follows AMQP best practices:
- Producer → Send to exchange (auto-created if not exists)
- Consumer → Auto-create everything + consume
- Registry → Manage multiple workers with consistent pattern

## Basic Usage

### Creating a Client

```rust
use easy_rmq::AmqpClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = AmqpClient::new(
        "amqp://guest:guest@localhost:5672".to_string(),
        10  // max pool size
    )?;

    Ok(())
}
```

### Publisher

Publisher **simple** - send to default exchange:

```rust
use easy_rmq::AmqpClient;

let client = AmqpClient::new("amqp://guest:guest@localhost:5672".to_string(), 10)?;

let publisher = client.publisher();

// Publish text
publisher.publish_text(
    "order.created",    // routing key
    "Hello, AMQP!"
).await?;

// Publish JSON
#[derive(serde::Serialize)]
struct Order {
    id: String,
    total: f64,
}

let order = Order {
    id: "123".to_string(),
    total: 100.0,
};

publisher.publish_json("order.created", &order).await?;
```

✅ **Auto send to default exchange** (`amq.direct`)
✅ **Auto-create exchange** if not exists (durable)
✅ **No manual setup needed**

### Multiple Exchanges

```rust
use lapin::ExchangeKind;

let client = AmqpClient::new("...", 10)?;

// Publisher 1 - Direct exchange
let pub1 = client.publisher().with_exchange("orders", ExchangeKind::Direct);
pub1.publish_text("order.created", "Order data").await?;

// Publisher 2 - Topic exchange
let pub2 = client.publisher().with_exchange("logs", ExchangeKind::Topic);
pub2.publish_text("order.created", "Log data").await?;

// Publisher 3 - Fanout exchange
let pub3 = client.publisher().with_exchange("broadcast", ExchangeKind::Fanout);
pub3.publish_text("any", "Broadcast data").await?;

// Shortcut methods
let pub4 = client.publisher().with_topic("logs");
let pub5 = client.publisher().with_direct("orders");
let pub6 = client.publisher().with_fanout("events");
```

✅ **Explicit** - exchange type clear from parameters
✅ **Flexible** - Direct, Topic, Fanout, Headers
✅ **Auto-create** exchange with appropriate type

### Subscriber with Worker Registry

Use `SubscriberRegistry` to manage multiple workers:

```rust
use easy_rmq::{AmqpClient, SubscriberRegistry, WorkerBuilder};
use lapin::ExchangeKind;

#[tokio::main]
async fn main() -> easy_rmq::Result<()> {
    let client = AmqpClient::new("amqp://admin:password@localhost:5672".to_string(), 10)?;
    let pool = client.channel_pool();

    let worker = SubscriberRegistry::new()
        .register({
            let pool = pool.clone();
            move |_count| {
                println!("📝 Registering worker #{}", _count);
                WorkerBuilder::new(ExchangeKind::Direct)
                    .pool(pool)
                    .with_exchange("order.events.v1")
                    .queue("order.process")
                    .build(handle_order_event)
            }
        })
        .register({
            let pool = pool.clone();
            move |_count| {
                println!("📝 Registering worker #{}", _count);
                WorkerBuilder::new(ExchangeKind::Topic)
                    .pool(pool)
                    .with_exchange("logs.v1")
                    .routing_key("order.*")
                    .queue("api_logs")
                    .build(handle_log_event)
            }
        });

    worker.run().await?;
    Ok(())
}

fn handle_order_event(data: Vec<u8>) -> easy_rmq::Result<()> {
    let msg = String::from_utf8_lossy(&data);
    println!("📦 Order: {}", msg);
    Ok(())
}

fn handle_log_event(data: Vec<u8>) -> easy_rmq::Result<()> {
    let msg = String::from_utf8_lossy(&data);
    println!("📊 Log: {}", msg);
    Ok(())
}
```

**Queue Format per Exchange Type:**

| Exchange | Parameter | Queue Name | Routing Key |
|----------|-----------|------------|-------------|
| **Direct** | `.queue("rk")` | `rk.job` | `rk` |
| **Topic** | `.routing_key("rk")` + `.queue("q")` | `q` | `rk` |
| **Fanout** | `.queue("q")` | `q` | `""` |

✅ **Auto-created** exchange + queue + binding
✅ **Direct**: queue auto-formatted with `.job` suffix
✅ **Topic/Fanout**: full control over queue name

### Advanced Worker Configuration

#### Retry Mechanism

Automatically retry failed messages with delay:

```rust
WorkerBuilder::new(ExchangeKind::Direct)
    .pool(pool)
    .with_exchange("order.events.v1")
    .queue("order.process")
    .retry(3, 5000)  // max 3 retries, 5 second delay
    .build(handler)
```

**How it works:**
- Failed messages sent to `{queue}.retry` with TTL
- After delay, message returns to original queue
- After max retries exceeded, sent to `{queue}.dlq` (Dead Letter Queue)
- Retry count tracked in message headers: `x-retry-count`

#### Single Active Consumer

Enable single active consumer mode to ensure only one consumer processes messages from a queue at a time. This is crucial for scenarios requiring strict message ordering and avoiding race conditions, such as inventory management:

```rust
WorkerBuilder::new(ExchangeKind::Direct)
    .pool(pool)
    .with_exchange("stock.events.v1")
    .queue("stock.event")
    .single_active_consumer(true)
    .prefetch(1)                  // ⚠️ Must be 1 to avoid race conditions
    .concurrency(1)               // ⚠️ Must be 1 to avoid race conditions
    .build(handler)
```

**Single Active Consumer behavior:**
- Only one consumer actively receives messages from the queue
- Other consumers remain standby and take over if the active consumer fails
- Useful for:
  - **Inventory/stock updates** - prevent overselling by processing sequentially
  - **Payment processing** - ensure transactions are processed in order
  - **Workflow orchestration** - maintain strict execution order
  - High availability scenarios with automatic failover

⚠️ **Important:** 
- **MUST** set `.prefetch(1)` and `.concurrency(1)` to avoid race conditions
- Messages MUST be processed sequentially (one at a time)
- Cannot be changed on existing queues (delete queue first if needed)
- Requires RabbitMQ 3.12+ with `single-active-consumer` plugin enabled
- Use `rabbitmq-plugins enable rabbitmq_single_active_consumer` to enable

**Why prefetch(1) and concurrency(1)?**
- Single active consumer ensures only ONE consumer is active
- If prefetch > 1: Single consumer buffers multiple messages, risking race conditions
- If concurrency > 1: Single consumer runs parallel workers, breaking message ordering
- Both MUST be 1 to guarantee sequential, ordered processing

**Example: Stock Update Race Condition**
```
Without SAC (parallel processing):
  Message 1: "Item A stock +10" → Consumer 1 reads stock: 50
  Message 2: "Item A stock -5"  → Consumer 2 reads stock: 50
  Consumer 1 writes: 60
  Consumer 2 writes: 45  ❌ Wrong! Should be 55

With SAC (sequential processing):
  Message 1: "Item A stock +10" → reads: 50, writes: 60
  Message 2: "Item A stock -5"  → reads: 60, writes: 55  ✓ Correct!
```

#### Prefetch (QoS) Control

Control how many messages pre-fetched from broker:

```rust
WorkerBuilder::new(ExchangeKind::Direct)
    .pool(pool)
    .queue("order.process")
    .prefetch(10)  // Buffer 10 messages
    .build(handler)
```

**Prefetch behavior:**
- Without `.concurrency()`: Messages buffered, processed sequentially 1-by-1
- With `.concurrency()`: Buffer size for parallel workers

#### Parallel Processing

Run multiple workers concurrently with controlled parallelism:

```rust
WorkerBuilder::new(ExchangeKind::Direct)
    .pool(pool)
    .queue("order.process")
    .prefetch(50)              // Buffer 50 messages
    .concurrency(10)           // Spawn 10 parallel workers
    .parallelize(tokio::task::spawn)  // Async tasks
    .build(handler)
```

**Configuration breakdown:**
- `.prefetch(N)` - AMQP prefetch count (buffer size from broker)
- `.concurrency(N)` - Number of parallel worker tasks
- `.parallelize(spawn_fn)` - Spawn function for task creation

**Spawn function options:**

```rust
// Async I/O tasks (default, good for database/HTTP calls)
.parallelize(tokio::task::spawn)

// CPU-intensive or blocking operations
.parallelize(tokio::task::spawn_blocking)
```

**Worker model:**
- Each worker runs its own consumer loop with unique consumer tag
- Workers compete for messages from the same queue
- Prefetch divides evenly among workers (e.g., prefetch=50, 10 workers → 5 per worker)

**Configuration Comparison:**

| Scenario | `.prefetch()` | `.concurrency()` | `.parallelize()` | Behavior |
|----------|---------------|------------------|------------------|----------|
| Sequential | Not set / 1 | Not set | Not set | 1 message at a time |
| Buffered | 10 | Not set | Not set | Buffer 10, process 1-by-1 |
| Parallel Async | 50 | 10 | `tokio::task::spawn` | 10 workers, async execution |
| Parallel Blocking | 50 | 10 | `tokio::task::spawn_blocking` | 10 workers, blocking threads |

#### Complete Example with All Features

```rust
WorkerBuilder::new(ExchangeKind::Direct)
    .pool(pool)
    .with_exchange("stock.events.v1")
    .queue("stock.event")
    .single_active_consumer(true) // Single active consumer mode
    .retry(3, 5000)               // 3 retries, 5s delay
    .prefetch(1)                  // Must be 1 with SAC
    .concurrency(1)               // Must be 1 with SAC
    .parallelize(tokio::task::spawn)  // Async execution
    .build(handle_stock_event)
```

⚠️ **Important:** `.concurrency()` requires `.parallelize()` to be set

### Middleware

Add middleware for logging, metrics, and distributed tracing:

```rust
use easy_rmq::{WorkerBuilder, SubscriberRegistry};
use lapin::ExchangeKind;

// Define middleware functions
pub fn logging(_payload: &[u8], result: &Result<()>) -> Result<()> {
    match result {
        Ok(_) => tracing::info!("✓ Message processed successfully"),
        Err(e) => tracing::error!("✗ Message processing failed: {:?}", e),
    }
    Ok(())
}

pub fn metrics(_payload: &[u8], result: &Result<()>) -> Result<()> {
    static SUCCESS_COUNT: std::sync::atomic::AtomicU64 = 
        std::sync::atomic::AtomicU64::new(0);
    
    match result {
        Ok(_) => {
            let count = SUCCESS_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            tracing::info!("📊 Metrics: {} messages processed", count + 1);
        }
        Err(_) => tracing::warn!("✗ Message failed"),
    }
    Ok(())
}

// Register with middleware
let worker = SubscriberRegistry::new()
    .register({
        let pool = pool.clone();
        move |_count| {
            WorkerBuilder::new(ExchangeKind::Direct)
                .pool(pool)
                .with_exchange("orders")
                .queue("order.process")
                .middleware(logging)    // Add logging middleware
                .middleware(metrics)     // Add metrics middleware
                .build(handler)
        }
    });

worker.run().await?;
```

**Middleware execution order:**
1. `before()` - Called before handler (for timing, etc.)
2. Handler function executed
3. `after()` - Called after handler (for logging, metrics, etc.)

**Built-in middleware available:**
- `examples/common/middleware::logging` - Log message processing
- `examples/common/middleware::metrics` - Track execution metrics with timing
- `examples/common/middleware::tracing` - Extract and log trace IDs

### Exchange Types Detail

**Direct Exchange** - Queue name auto-formatted with `.job` suffix:

```rust
WorkerBuilder::new(ExchangeKind::Direct)
    .pool(pool)
    .with_exchange("order.events")
    .queue("order.created")  // routing_key
    .build(handler)
// Queue: "order.created.job"
// Binding: queue_bind("order.created.job", "order.events", "order.created")
```

**Topic Exchange** - Separate routing key and queue:

```rust
WorkerBuilder::new(ExchangeKind::Topic)
    .pool(pool)
    .with_exchange("logs")
    .routing_key("order.*")  // routing pattern
    .queue("api_logs")       // queue name
    .build(handler)
// Queue: "api_logs"
// Binding: queue_bind("api_logs", "logs", "order.*")
```

**Fanout Exchange** - Broadcast to all queues:

```rust
WorkerBuilder::new(ExchangeKind::Fanout)
    .pool(pool)
    .with_exchange("events")
    .queue("notification_q")
    .build(handler)
// Queue: "notification_q"
// Binding: queue_bind("notification_q", "events", "")
```

## Distributed Tracing

Built-in support for distributed tracing with automatic or custom trace ID generation, perfect for tracking message flows through your system.

### Publisher with Trace ID

```rust
use easy_rmq::AmqpClient;

let client = AmqpClient::new("amqp://guest:guest@localhost:5672".to_string(), 10)?;

// Option 1: Auto-generate trace ID (recommended for most cases)
client.publisher()
    .with_auto_trace_id()
    .publish_text("order.created", "Order data")
    .await?;

// Option 2: Use custom trace ID (e.g., from OpenTelemetry)
client.publisher()
    .with_trace_id("trace-from-otel-123".to_string())
    .publish_text("order.created", "Order data")
    .await?;

// Option 3: Generate standalone trace ID
use easy_rmq::generate_trace_id;
let trace_id = generate_trace_id();
client.publisher()
    .with_trace_id(trace_id)
    .publish_text("order.created", "Order data")
    .await?;
```

### Subscriber: Extract Trace ID

The subscriber automatically stores message headers in thread-local storage, accessible via `easy_rmq::get_headers()`:

```rust
use easy_rmq::Result;

// In your handler or middleware
pub fn extract_trace_id() -> Option<String> {
    easy_rmq::get_headers()
        .and_then(|h| h.inner().get("x-trace-id").cloned())
        .and_then(|v| match v {
            lapin::types::AMQPValue::LongString(s) => Some(s.to_string()),
            lapin::types::AMQPValue::ShortString(s) => Some(s.to_string()),
            _ => None,
        })
}

fn handle_event(data: Vec<u8>) -> Result<()> {
    let trace_id = extract_trace_id().unwrap_or_else(|| "unknown".to_string());
    tracing::info!("Processing message - trace-id: {}", trace_id);
    
    // Process message...
    Ok(())
}
```

### Middleware: Automatic Trace ID Logging

Use the built-in `tracing` middleware for automatic trace ID extraction and logging:

```rust
use easy_rmq::{WorkerBuilder, SubscriberRegistry};
use lapin::ExchangeKind;

// Add tracing middleware
let worker = SubscriberRegistry::new()
    .register({
        let pool = pool.clone();
        move |_count| {
            WorkerBuilder::new(ExchangeKind::Direct)
                .pool(pool)
                .with_exchange("orders")
                .queue("order.process")
                .middleware(common::middleware::tracing)  // Auto-log trace IDs
                .build(handler)
        }
    });
```

**Sample output:**
```
INFO Message processed - trace-id: 19ca9a5f5e1-5e148b1f5008b7d8
WARN Message failed - trace-id: 19ca9a5f5e1-5e148b1f5008b7d8 | error: ...
```

### OpenTelemetry Integration

For production distributed tracing with OpenTelemetry:

```rust
use opentelemetry::trace::TraceContextExt;

// Get trace ID from current OTel context
let context = opentelemetry::Context::current();
let span = context.span();
let trace_id = span.span_context().trace_id().to_string();

// Pass trace ID through message pipeline
client.publisher()
    .with_trace_id(trace_id)
    .publish_text("order.created", payload)
    .await?;

// Or auto-generate when no OTel context available
client.publisher()
    .with_auto_trace_id()
    .publish_text("order.created", payload)
    .await?;
```

**Benefits:**
- ✅ Track messages across services
- ✅ Correlate logs with trace IDs
- ✅ Debug distributed systems
- ✅ Monitor message flows
- ✅ OTel-compatible

**Trace ID format:** `{timestamp_hex}-{random_hex}` (e.g., `19ca9a5f5e1-5e148b1f5008b7d8`)

**See also:**
- `examples/otel_integration.rs` - Complete OTel integration example
- `examples/trace_id_generator.rs` - Trace ID generation demo
- `examples/common/middleware.rs` - Built-in middleware implementations

## Dependency Injection

This library supports dependency injection using traits:

```rust
use easy_rmq::{AmqpPublisher, Result};
use std::sync::Arc;

struct OrderService {
    publisher: Arc<dyn AmqpPublisher>,
}

impl OrderService {
    fn new(publisher: Arc<dyn AmqpPublisher>) -> Self {
        Self { publisher }
    }

    async fn create_order(&self, order: Order) -> Result<()> {
        let payload = serde_json::to_vec(&order)?;
        self.publisher.publish("orders", "order.created", &payload).await?;
        Ok(())
    }
}

let client = AmqpClient::new("amqp://guest:guest@localhost:5672".to_string(), 10)?;
let publisher: Arc<dyn AmqpPublisher> = Arc::new(client.publisher());
let order_service = OrderService::new(publisher);
```

## Examples

See `examples/` folder for complete usage examples:

### Core Examples
- **`publisher.rs`** - Publisher with auto trace ID generation
- **`subscriber.rs`** - Multi-worker with middleware, retry, prefetch, concurrency, and SAC
- **`single_active_consumer.rs`** - Single active consumer demonstration

### Distributed Tracing Examples
- **`otel_integration.rs`** - OpenTelemetry integration patterns
- **`trace_id_generator.rs`** - Standalone trace ID generation demo

### Quick Start
```bash
# Terminal 1 - Start subscriber first
cargo run --example subscriber

# Terminal 2 - Then publisher
cargo run --example publisher

# Run OTel integration example
cargo run --example otel_integration

# Generate trace IDs
cargo run --example trace_id_generator
```

### Common Middleware
Located in `examples/common/middleware.rs`:
- `logging` - Log message processing results
- `metrics` - Track success/error counts with execution time
- `tracing` - Extract and log trace IDs from message headers

## Testing

```bash
cargo test
```

## Requirements

- Rust 1.70+
- RabbitMQ server (or Docker)

## License

ISC
