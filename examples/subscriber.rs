use easy_rmq::{AmqpClient, Result, SubscriberRegistry, WorkerBuilder};
use lapin::ExchangeKind;
use std::time::Duration;
use tokio::signal;

mod common;
use common::middleware::{logging, metrics, tracing};

fn handle_order_event(data: Vec<u8>) -> Result<()> {
    let msg = String::from_utf8_lossy(&data);
    let event: serde_json::Value = serde_json::from_str(&msg)?;

    println!("📦 [Order] Event: {}", event);

    std::thread::sleep(std::time::Duration::from_millis(100));

    let order_id = event["id"].as_str().unwrap_or("unknown");
    let total = event["total"].as_f64().unwrap_or(0.0);
    println!(
        "❌ [Order] Processing failed: {} | Total: ${}",
        order_id, total
    );
    Err("Simulated processing error".into())
}

fn handle_stock_event(data: Vec<u8>) -> Result<()> {
    let msg = String::from_utf8_lossy(&data);
    let event: serde_json::Value = serde_json::from_str(&msg)?;

    println!("📦 [Stock] Event: {}", event);

    Ok(())
}

fn handle_log_event(data: Vec<u8>) -> Result<()> {
    let msg = String::from_utf8_lossy(&data);
    let log: serde_json::Value = serde_json::from_str(&msg)?;

    let level = log["level"].as_str().unwrap_or("INFO");
    let service = log["service"].as_str().unwrap_or("unknown");
    let message = log["message"].as_str().unwrap_or("-");

    println!("📊 [Log] [{}] {} | {}", level, service, message);

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let client = AmqpClient::new("amqp://admin:password@localhost:5672".to_string(), 10)?;

    println!("📝 Setting up AMQP subscribers...\n");

    let pool = client.channel_pool();

    let worker = SubscriberRegistry::new()
        .register({
            let pool = pool.clone();
            move |_count| {
                WorkerBuilder::new(ExchangeKind::Direct)
                    .pool(pool)
                    .with_exchange("order.events.v1")
                    .queue("order.process")
                    .retry(3, 5000)
                    .prefetch(10)
                    .concurrency(5)
                    .parallelize(tokio::task::spawn)
                    .middleware(logging)
                    .middleware(metrics)
                    .middleware(tracing)
                    .build(handle_order_event)
            }
        })
        .register({
            let pool = pool.clone();
            move |_count| {
                WorkerBuilder::new(ExchangeKind::Direct)
                    .pool(pool)
                    .with_exchange("stock.events.v1")
                    .queue("stock.event")
                    .single_active_consumer(true)
                    .retry(3, 5000)
                    .prefetch(1)
                    .concurrency(1)
                    .parallelize(tokio::task::spawn)
                    .middleware(logging)
                    .middleware(metrics)
                    .middleware(tracing)
                    .build(handle_stock_event)
            }
        })
        .register({
            let pool = pool.clone();
            move |_count| {
                WorkerBuilder::new(ExchangeKind::Topic)
                    .pool(pool)
                    .with_exchange("logs.v1")
                    .routing_key("order.*")
                    .queue("api_logs")
                    .retry(2, 10000)
                    .prefetch(20)
                    .concurrency(10)
                    .parallelize(tokio::task::spawn)
                    .middleware(logging)
                    .middleware(metrics)
                    .middleware(tracing)
                    .build(handle_log_event)
            }
        });

    println!("\n(Press Ctrl+C to exit)\n");

    tokio::select! {
        result = worker.run() => {
            if let Err(e) = result {
                eprintln!("❌ Registry error: {:?}", e);
            }
        }
        _ = signal::ctrl_c() => {
            println!("\n🛑 Shutting down gracefully...");
        }
    }

    tokio::time::sleep(Duration::from_millis(500)).await;
    println!("✓ Shutdown complete");

    Ok(())
}
