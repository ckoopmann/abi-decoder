//! Decode unknown abi encoded data
//!
//! This crate a methods to ecode abi encoded data without the abi specification.
//! It was inspired by [this excellent talk](https://www.youtube.com/watch?v=RZytWxtKODg) and primarily served as a learning project to better understand abi encoding and learn rust programming.
//!
//! ```rust
//! # use abi_decoder::decode_transaction_calldata;
//! #
//! #[tokio::main]
//! async fn main() {
//!    let tx_hash = "0x53ad65f13d7abec1423e1663e0d2c6852d7a60651248e565471ab722d1da9bed";
//!    // Ideally use your own rpc endpoint url (for example using infura / alchemy etc key)
//!    let rpc_url = "https://rpc.ankr.com/eth";
//!    let decoded_tokens: Vec<ethabi::Token> = decode_transaction_calldata(tx_hash, Some(rpc_url)).await;
//!
//! }
//! ```
//!
//! The majority of examples and code snippets in this crate assume that they are
//! inside an async block as written above.
#![warn(missing_docs)]
use ethabi::token::Token;

mod decoder;
#[cfg(test)]
mod test_utils;
#[cfg(test)]
mod tests;
mod transaction_data;
mod utils;

use decoder::chunk_and_decode_data;
use transaction_data::get_encoded_arguments;
#[doc(hidden)]
pub use utils::print_parse_tree;

/// Decodes the calldata of the given transaction
///
/// This function gets the calldata from of the given transaction (if it contains any), split off
/// the first 4 bytes (which is the function signature) and then decodes the remaining data.
///
/// Example
/// ```rust
/// tokio_test::block_on(async {
///   let tx_hash = "0x53ad65f13d7abec1423e1663e0d2c6852d7a60651248e565471ab722d1da9bed";
///   // Ideally use your own rpc endpoint url (for example using infura / alchemy etc key)
///   let rpc_url = "https://rpc.ankr.com/eth";
///   let decoded_tokens: Vec<ethabi::Token> = abi_decoder::decode_transaction_calldata(tx_hash, Some(rpc_url)).await;
/// })
/// ```
///
pub async fn decode_transaction_calldata(
    tx_hash: &str,
    provider_rpc_url: Option<&str>,
) -> Vec<Token> {
    let tx_hash = tx_hash.trim_start_matches("0x");
    let arguments_encoded = get_encoded_arguments(tx_hash, provider_rpc_url).await;
    if arguments_encoded.is_empty() {
        println!("Not a valid function call");
        return Vec::new();
    }

    chunk_and_decode_data(&arguments_encoded)
}
