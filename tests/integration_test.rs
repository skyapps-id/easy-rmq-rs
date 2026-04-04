use easy_rmq_rs::{AmqpClient, AmqpPublisher};
use std::sync::Arc;

#[test]
fn test_create_client() {
    let client = AmqpClient::new(
        "amqp://guest:guest@localhost:5672".to_string(),
        "test-connection".to_string(),
        10,
    );
    assert!(client.is_ok());
}

#[test]
fn test_client_has_publisher() {
    let client = AmqpClient::new(
        "amqp://guest:guest@localhost:5672".to_string(),
        "test-connection".to_string(),
        10,
    )
    .unwrap();
    let publisher = client.publisher();
    drop(publisher);
}

#[test]
fn test_client_has_channel_pool() {
    let client = AmqpClient::new(
        "amqp://guest:guest@localhost:5672".to_string(),
        "test-connection".to_string(),
        10,
    )
    .unwrap();
    let pool = client.channel_pool();
    drop(pool);
}

#[test]
fn test_publisher_trait() {
    let client = AmqpClient::new(
        "amqp://guest:guest@localhost:5672".to_string(),
        "test-connection".to_string(),
        10,
    )
    .unwrap();
    let publisher: Arc<dyn AmqpPublisher> = Arc::new(client.publisher());
    drop(publisher);
}

#[test]
fn test_single_active_consumer_method_chain() {
    let client = AmqpClient::new(
        "amqp://guest:guest@localhost:5672".to_string(),
        "test-connection".to_string(),
        10,
    )
    .unwrap();
    let pool = client.channel_pool();

    use easy_rmq_rs::WorkerBuilder;
    use lapin::ExchangeKind;

    let _worker = WorkerBuilder::new(ExchangeKind::Direct)
        .pool(pool)
        .queue("test.queue")
        .single_active_consumer(true)
        .build(|_| async { Ok(()) });
}

#[test]
fn test_single_active_consumer_direct_builder() {
    let client = AmqpClient::new(
        "amqp://guest:guest@localhost:5672".to_string(),
        "test-connection".to_string(),
        10,
    )
    .unwrap();
    let pool = client.channel_pool();

    use easy_rmq_rs::WorkerBuilder;
    use lapin::ExchangeKind;

    let _worker = WorkerBuilder::new(ExchangeKind::Direct)
        .pool(pool)
        .queue("test.queue")
        .single_active_consumer(true)
        .build(|_| async { Ok(()) });
}

#[test]
fn test_single_active_consumer_topic_builder() {
    let client = AmqpClient::new(
        "amqp://guest:guest@localhost:5672".to_string(),
        "test-connection".to_string(),
        10,
    )
    .unwrap();
    let pool = client.channel_pool();

    use easy_rmq_rs::WorkerBuilder;
    use lapin::ExchangeKind;

    let _worker = WorkerBuilder::new(ExchangeKind::Topic)
        .pool(pool)
        .routing_key("test.routing.key")
        .queue("test.queue")
        .single_active_consumer(true)
        .build(|_| async { Ok(()) });
}

#[test]
fn test_single_active_consumer_fanout_builder() {
    let client = AmqpClient::new(
        "amqp://guest:guest@localhost:5672".to_string(),
        "test-connection".to_string(),
        10,
    )
    .unwrap();
    let pool = client.channel_pool();

    use easy_rmq_rs::WorkerBuilder;
    use lapin::ExchangeKind;

    let _worker = WorkerBuilder::new(ExchangeKind::Fanout)
        .pool(pool)
        .queue("test.queue")
        .single_active_consumer(true)
        .build(|_| async { Ok(()) });
}
