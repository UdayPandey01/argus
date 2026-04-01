use rdkafka::Message;
use rdkafka::admin::{AdminClient, AdminOptions, NewTopic, TopicReplication};
use rdkafka::client::DefaultClientContext;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer, StreamConsumer};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bootstrap_servers =
        env::var("KAFKA_BOOTSTRAP_SERVERS").unwrap_or_else(|_| "localhost:9093".to_string());
    let topic = env::var("KAFKA_TOPIC").unwrap_or_else(|_| "usdt_alerts".to_string());

    println!("Worker started, waiting for messages...");
    println!("Using Kafka bootstrap servers: {}", bootstrap_servers);

    let admin: AdminClient<DefaultClientContext> = ClientConfig::new()
        .set("bootstrap.servers", &bootstrap_servers)
        .create()
        .expect("Failed to create Kafka admin client");

    let new_topic = NewTopic::new(topic.as_str(), 1, TopicReplication::Fixed(1));
    let create_results = admin
        .create_topics(&[new_topic], &AdminOptions::new())
        .await;

    match create_results {
        Ok(results) => {
            for result in results {
                match result {
                    Ok(created_topic) => println!("Kafka topic ready: {}", created_topic),
                    Err((existing_topic, err)) => {
                        println!("Kafka topic create status for {}: {}", existing_topic, err)
                    }
                }
            }
        }
        Err(err) => println!("Kafka topic creation request failed: {}", err),
    }

    let consumer: StreamConsumer = ClientConfig::new()
        .set("bootstrap.servers", &bootstrap_servers)
        .set("group.id", "argus-worker-group")
        .set("auto.offset.reset", "earliest")
        .set("allow.auto.create.topics", "true")
        .create()
        .expect("Failed to create kafka consumer");

    println!("Connected to kafka");

    consumer
        .subscribe(&[topic.as_str()])
        .expect("Failed to subscribe to topic");

    println!("Listening for messages on '{}'...", topic);

    loop {
        match consumer.recv().await {
            Err(e) => println!("Kafka error: {}", e),
            Ok(message) => {
                if let Some(payload) = message.payload() {
                    if let Ok(raw_string) = std::str::from_utf8(payload) {
                        println!("CAUGHT A MESSAGE!");
                        println!("Raw Data: {}", raw_string);
                        println!("------------------------------------------------");
                    }
                }
            }
        }
    }
}
