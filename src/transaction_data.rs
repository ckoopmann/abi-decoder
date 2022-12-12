use ethers::providers::{Http, Middleware, Provider};
use std::env;

pub async fn get_encoded_arguments(tx_hash: &str, provider_rpc_url: Option<&str>) -> String {
    let calldata = get_calldata(tx_hash, provider_rpc_url).await;
    let arguments_encoded = split_off_encoded_arguments(&calldata);
    arguments_encoded.to_string()
}

pub fn split_off_encoded_arguments(calldata: &str) -> &str {
    if calldata.len() < 8 {
        return "";
    }
    let (_, arguments_encoded) = calldata.split_at(8);
    arguments_encoded
}

pub async fn get_calldata(tx_hash: &str, provider_rpc_url: Option<&str>) -> String {
    let mut tx_hash_bytes: [u8; 32] = [0; 32];
    hex::decode_to_slice(tx_hash, &mut tx_hash_bytes).expect("Decoding failed");
    println!("Getting trransaction: {:?}", tx_hash);
    let provider = get_provider(provider_rpc_url);
    let tx = provider
        .get_transaction(tx_hash_bytes)
        .await
        .unwrap()
        .unwrap();
    hex::encode(tx.input.0)
}

pub fn get_provider(rpc_url: Option<&str>) -> Provider<Http> {
    let rpc_url_unwrapped;
    let env_rpc_url = env::var("RPC_URL");
    if let Some(url) = rpc_url {
        rpc_url_unwrapped = url;
    } else if let Ok(ref url) = env_rpc_url {
        rpc_url_unwrapped = url;
    } else {
        panic!("No RPC URL provided");
    };
    Provider::<Http>::try_from(rpc_url_unwrapped).expect("could not instantiate HTTP Provider")
}
