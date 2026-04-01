use alloy::primitives::Address;
use alloy::providers::{Provider, ProviderBuilder, WsConnect};
use alloy::rpc::types::Filter;
use alloy::sol;

use alloy::sol_types::SolEvent;
use dotenvy::dotenv;
use futures_util::StreamExt;
use std::env;

sol! {
    event Transfer(address indexed from, address indexed to, uint256 value);
}

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

    let filter = Filter::new().address(target_address).event_signature(Transfer::SIGNATURE_HASH);

    let sub = provider.subscribe_logs(&filter).await?;
    let mut stream = sub.into_stream();

    println!("Listening for new events on the USDT contract... (Waiting for a block to be mined)");

    while let Some(log) = stream.next().await {
        match log.log_decode::<Transfer>() {
            Ok(decode_log) => {
                let transfer = decode_log.inner;

                println!("USDT TRANSFER DETECTED");
                println!("TRANSFER FROM: {}", transfer.from);
                println!("TRANSFER TO: {}", transfer.to);

                println!("RAW AMOUNT: {}", transfer.value);
                println!("------------------------------------------");
            }
            Err(e) => {
                println!("Failed to decode a log: {:?}", e);
            }
        }
    }

    Ok(())
}