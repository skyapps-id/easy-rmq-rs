use std::time::Duration;

use easy_rmq_rs::{AmqpClient, Data, Result, SubscriberRegistry, WorkerBuilder};
use lapin::ExchangeKind;

#[derive(Clone)]
struct EmailService {
    smtp_server: String,
}

impl EmailService {
    fn new(smtp_server: String) -> Self {
        Self { smtp_server }
    }

    fn send_email(&self, data: &[u8]) -> Result<()> {
        let msg = String::from_utf8_lossy(data);
        println!("📧 Sending email via SMTP: {}", self.smtp_server);
        println!("   Data: {}", msg);
        Ok(())
    }
}

fn send_email(service: Data<EmailService>, data: &[u8]) -> Result<()> {
    service.send_email(data)
}

fn handle_order_plain(data: &[u8]) -> Result<()> {
    let msg = String::from_utf8_lossy(data);
    println!("📦 Processing order (no dependency): {}", msg);
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let client = AmqpClient::new("amqp://admin:password@localhost:5672".to_string(), 10)?;
    let pool = client.channel_pool();

    let email_service = Data::new(EmailService::new("smtp.gmail.com:587".to_string()));

    let worker = SubscriberRegistry::new()
        .register({
            let pool = pool.clone();
            move |_count| {
                WorkerBuilder::new(ExchangeKind::Direct)
                    .pool(pool.clone())
                    .with_exchange("orders.v1")
                    .queue("orders.process")
                    .build(handle_order_plain)
            }
        })
        .register({
            let pool = pool.clone();
            let email_service = email_service.clone();
            move |_count| {
                WorkerBuilder::new(ExchangeKind::Direct)
                    .pool(pool.clone())
                    .with_exchange("emails.v1")
                    .queue("emails.send")
                    .data(email_service.clone())
                    .build(send_email)
            }
        });

    println!("✓ Dependency injection example setup complete");
    println!("  - Order handler without dependency");
    println!("  - Email handler with EmailService dependency");

    tokio::select! {
        result = worker.run() => {
            if let Err(e) = result {
                eprintln!("❌ Registry error: {:?}", e);
            }
        }
        _ = tokio::signal::ctrl_c() => {
            println!("\n🛑 Shutting down gracefully...");
        }
    }

    tokio::time::sleep(Duration::from_millis(500)).await;
    println!("✓ Shutdown complete");

    Ok(())
}
