use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use easy_rmq_rs::{AmqpClient, Publisher};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize)]
struct Order {
    id: String,
    customer: String,
    total: f64,
    items: Vec<OrderItem>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OrderItem {
    product_id: String,
    quantity: u32,
    price: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct PublishResponse {
    success: bool,
    message: String,
}

struct AppState {
    publisher: Arc<Publisher>,
}

/// Publish order to order.events.v1 exchange (non-blocking JSON serialization)
async fn publish_order(
    state: web::Data<AppState>,
    order: web::Json<Order>,
) -> impl Responder {
    // Extract order ID before moving
    let order_id = order.id.clone();

    // Extract order data before moving
    let order_data = order.into_inner();

    // Use spawn_blocking for JSON serialization to avoid blocking Actix threads
    let order_json = match tokio::task::spawn_blocking(move || {
        serde_json::to_vec(&order_data)
            .map_err(|e| format!("JSON serialization failed: {}", e))
    })
    .await
    {
        Ok(Ok(bytes)) => bytes,
        Ok(Err(e)) => {
            return HttpResponse::InternalServerError().json(PublishResponse {
                success: false,
                message: e,
            })
        }
        Err(e) => {
            return HttpResponse::InternalServerError().json(PublishResponse {
                success: false,
                message: format!("Task join error: {}", e),
            })
        }
    };

    // Publish order using pre-configured publisher (exchange already set)
    let result = state
        .publisher
        .publish("order.process", order_json)
        .await;

    match result {
        Ok(_) => HttpResponse::Ok().json(PublishResponse {
            success: true,
            message: format!("Order {} published successfully", order_id),
        }),
        Err(e) => HttpResponse::InternalServerError().json(PublishResponse {
            success: false,
            message: format!("Failed to publish order: {}", e),
        }),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize logger
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    // Initialize RabbitMQ client
    let client = AmqpClient::new(
        "amqp://admin:password@localhost:5672".to_string(),
        10,
    )
    .expect("Failed to create RabbitMQ client");

    // Create publisher with order.events.v1 exchange
    let publisher = Arc::new(
        client.publisher()
            .with_exchange("order.events.v1")
    );

    let app_state = web::Data::new(AppState { publisher });

    let bind_address = "127.0.0.1:8080";
    println!("🚀 Actix-Web + easy-rmq-rs Integration");
    println!("=======================================\n");
    println!("📍 Server: http://{}\n", bind_address);
    println!("📰 Endpoint:");
    println!("  POST /publish - Publish order to order.events.v1 exchange\n");
    println!("📝 Configuration:");
    println!("  Exchange:   order.events.v1 (Direct)");
    println!("  Routing Key: order.process");
    println!("  Publisher:  Pre-configured with exchange\n");
    println!("✅ Non-blocking JSON serialization with spawn_blocking\n");

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .route("/publish", web::post().to(publish_order))
    })
    .bind(bind_address)?
    .run()
    .await
}
