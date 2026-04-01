use alloy::primitives::Address;
use alloy::providers::{Provider, ProviderBuilder, WsConnect};
use alloy::rpc::types::Filter;
use alloy::sol;
use alloy::sol_types::SolEvent;

use futures_util::StreamExt;
use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::util::Timeout;
use serde::Serialize;

use dotenvy::dotenv;
use std::env;
use std::time::Duration;

use sqlx::PgPool;

sol! {
    event Transfer(address indexed from, address indexed to, uint256 value);
}

#[derive(Serialize)]
struct TransferMessage {
    tx_hash: String,
    sender: String,
    receiver: String,
    amount: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    println!("Connecting to db");

    let pool = PgPool::connect(&db_url).await?;
    println!("Connected to DB");

    let alchemy_ws_url =
        env::var("ALCHEMY_WS_URL").expect("ALCHEMY_WS_URL must be set in .env file");
    let kafka_bootstrap_servers =
        env::var("KAFKA_BOOTSTRAP_SERVERS").unwrap_or_else(|_| "localhost:9093".to_string());
    let kafka_topic = env::var("KAFKA_TOPIC").unwrap_or_else(|_| "usdt_alerts".to_string());

    println!("Connecting to Ethereum node ");

    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", &kafka_bootstrap_servers)
        .create()
        .expect("Failed to create kafka producer");

    println!(
        "Kafka producer ready on {} for topic {}",
        kafka_bootstrap_servers, kafka_topic
    );

    let ws = WsConnect::new(alchemy_ws_url);
    let provider = ProviderBuilder::new().connect_ws(ws).await?;

    let target_address: Address = "0xdAC17F958D2ee523a2206206994597C13D831ec7".parse()?;

    let filter = Filter::new()
        .address(target_address)
        .event_signature(Transfer::SIGNATURE_HASH);

    let sub = provider.subscribe_logs(&filter).await?;
    let mut stream = sub.into_stream();

    println!("Listening for new events on the USDT contract... (Waiting for a block to be mined)");

    while let Some(log) = stream.next().await {
        match log.log_decode::<Transfer>() {
            Ok(decode_log) => {
                let transfer = decode_log.inner;

                let tx_hash = log.transaction_hash.unwrap_or_default().to_string();
                let sender = transfer.from.to_string();
                let receiver = transfer.to.to_string();
                let amount = transfer.value.to_string();

                let result = sqlx::query!(
                    "INSERT INTO usdt_transfers (tx_hash, sender, receiver, amount) VALUES ($1, $2, $3, $4)",
                    tx_hash,
                    sender,
                    receiver,
                    amount
                )
                .execute(&pool)
                .await;

                println!("USDT TRANSFER DETECTED");
                println!("TRANSFER FROM: {}", transfer.from);
                println!("TRANSFER TO: {}", transfer.to);

                println!("RAW AMOUNT: {}", transfer.value);
                println!("------------------------------------------");

                let kafka_message = TransferMessage {
                    tx_hash: tx_hash.clone(),
                    sender: sender.clone(),
                    receiver: receiver.clone(),
                    amount: amount.clone(),
                };

                if let Ok(payload) = serde_json::to_string(&kafka_message) {
                    let delivery = producer
                        .send(
                            FutureRecord::to(&kafka_topic)
                                .key(&tx_hash)
                                .payload(&payload),
                            Timeout::After(Duration::from_secs(5)),
                        )
                        .await;

                    match delivery {
                        Ok(_) => println!("Published to Kafka topic: {}", kafka_topic),
                        Err((e, _)) => println!("Failed to publish to Kafka: {}", e),
                    }
                } else {
                    println!("Failed to serialize transfer for Kafka publish");
                }

                match result {
                    Ok(_) => println!("Saved to DB: {} sent to {}", sender, receiver),
                    Err(e) => println!("Failed to save to DB: {:?}", e),
                }
            }
            Err(e) => {
                println!("Failed to decode a log: {:?}", e);
            }
        }
    }

    Ok(())
}
