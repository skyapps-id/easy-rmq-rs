use easy_rmq::{AmqpClient, Result};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let client = AmqpClient::new("amqp://admin:password@localhost:5672".to_string(), 10)?;

    println!("📤 Starting dependency injection publishers...\n");

    // Publisher 1 - order events to orders.v1 (Direct exchange)
    let order_publisher = client.publisher().with_exchange("orders.v1");
    
    for i in 1..=3 {
        let order = serde_json::json!({
            "id": format!("ORD-{:04}", i),
            "customer": "John Doe",
            "total": (i as f64) * 99.99,
            "items": vec!["item-1", "item-2"],
            "status": "created",
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        println!("📦 [Order #{}] Publishing to orders.v1:", i);
        println!("   Routing Key: orders.process");
        println!("   Data: {}\n", order);
        
        order_publisher
            .clone()
            .with_auto_trace_id()
            .publish_text("orders.process", &order.to_string())
            .await?;
        
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    // Publisher 2 - email events to emails.v1 (Direct exchange)
    let email_publisher = client.publisher().with_exchange("emails.v1");

    for i in 1..=3 {
        let email = serde_json::json!({
            "to": format!("customer{}@example.com", i),
            "subject": "Order Confirmation",
            "body": "Your order has been processed successfully!",
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        println!("📧 [Email #{}] Publishing to emails.v1:", i);
        println!("   Routing Key: emails.send");
        println!("   Data: {}\n", email);

        email_publisher
            .clone()
            .with_auto_trace_id()
            .publish_text("emails.send", &email.to_string())
            .await?;
        
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    println!("✓ All messages published!");
    println!("\n💡 Make sure dependency_injection example is running:");
    println!("   Terminal 1: cargo run --example dependency_injection");
    println!("   Terminal 2: cargo run --example dependency_injection_publisher");
    
    tokio::time::sleep(Duration::from_secs(1)).await;
    println!("\n✓ Shutdown complete");

    Ok(())
}
