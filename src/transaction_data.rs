use ethers::providers::{Http, Middleware, Provider};
use std::env;

pub async fn get_encoded_arguments(tx_hash: &str) -> String {
    let calldata = get_calldata(tx_hash).await;
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

pub async fn get_calldata(tx_hash: &str) -> String {
    let mut tx_hash_bytes: [u8; 32] = [0; 32];
    hex::decode_to_slice(tx_hash, &mut tx_hash_bytes).expect("Decoding failed");
    println!("Getting trransaction: {:?}", tx_hash);
    let provider = get_provider();
    let tx = provider
        .get_transaction(tx_hash_bytes)
        .await
        .unwrap()
        .unwrap();
    hex::encode(tx.input.0)
}

pub fn get_provider() -> Provider<Http> {
    let rpc_url = env::var("RPC_URL").expect("RPC_URL must be set");
    Provider::<Http>::try_from(rpc_url).expect("could not instantiate HTTP Provider")
}
