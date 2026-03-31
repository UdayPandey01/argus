use alloy::primitives::Address;
use alloy::providers::{Provider, ProviderBuilder, WsConnect};
use alloy::rpc::types::Filter;

use dotenvy::dotenv;
use futures_util::StreamExt;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let alchemy_ws_url =
        env::var("ALCHEMY_WS_URL").expect("ALCHEMY_WS_URL must be set in .env file");

    println!("Connecting to Ethereum node ");

    let ws = WsConnect::new(alchemy_ws_url);
    let provider = ProviderBuilder::new().connect_ws(ws).await?;

    let target_address: Address =
        "0xdAC17F958D2ee523a2206206994597C13D831ec7".parse()?;

    let filter = Filter::new().address(target_address);

    let sub = provider.subscribe_logs(&filter).await?;
    let mut stream = sub.into_stream();

    println!("Listening for new events on the USDT contract... (Waiting for a block to be mined)");

    while let Some(log) = stream.next().await {
        println!("NEW EVENT DETECTED!");
        println!("Block Number: {:?}", log.block_number);
        println!("Transaction Hash: {:?}", log.transaction_hash);
        println!("Raw Data: {:?}\n", log.data());
    }

    Ok(())
}