use alloy::providers::{
    ProviderBuilder,
    WsConnect
};

use std::env;
use dotenvy::dotenv;

#[tokio::main]
async fn amin() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let alchemy_ws_url = env::var("ALCHEMY_WS_URL").expect("ALCHEMY_WS_URL must be set in .env file");

    println!("Connecting to Ethereum node ")

    
}