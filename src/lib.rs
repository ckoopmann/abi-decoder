use ethabi::token::Token;

mod decoder;
#[cfg(test)]
mod tests;
mod transaction_data;
mod utils;

use decoder::chunk_and_decode_data;
use transaction_data::get_encoded_arguments;
pub use utils::print_parse_tree;

pub async fn decode_transaction_calldata(
    tx_hash: &str,
    provider_rpc_url: Option<&str>,
) -> Vec<Token> {
    let arguments_encoded = get_encoded_arguments(tx_hash, provider_rpc_url).await;
    if arguments_encoded.is_empty() {
        println!("Not a valid function call");
        return Vec::new();
    }

    chunk_and_decode_data(&arguments_encoded)
}
