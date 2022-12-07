use ethabi::{Contract, Token};
use ethers::prelude::abigen;
use ethers::providers::{Middleware, Provider, Http};
use std::convert::TryFrom;
use ethereum_types::{U256, H160, H256};
use std::str::FromStr;
use hex;
use tokio;

pub mod utils;
pub mod decoder;

pub async fn decode_transaction_calldata(tx_hash: &str) -> Vec<Token> {
    let provider = Provider::<Http>::try_from(
    "https://mainnet.infura.io/v3/c60b0bb42f8a4c6481ecd229eddaca27"
).expect("could not instantiate HTTP Provider");
    let mut tx_hash_bytes: [u8; 32] = [0; 32];
    hex::decode_to_slice(tx_hash, &mut tx_hash_bytes).expect("Decoding failed");
    let tx = provider.get_transaction(tx_hash_bytes).await.unwrap().unwrap();
    let calldata = hex::encode(tx.input.0);
    let (selector, arguments_encoded) = calldata.split_at(8);
    let decoded = chunk_and_decode_data(arguments_encoded);
    return decoded;
}

fn chunk_and_decode_data(encoded_arguments: &str) -> Vec<Token> {
        let chunks = decoder::chunk_data(encoded_arguments);
        for (i, chunk) in chunks.iter().enumerate() {
            println!("{}: {} - {}", i, chunk, u64::from_str_radix(chunk.trim_start_matches("0"), 16).unwrap_or(0));
        }
        let decoded_data = decoder::decode_chunks(chunks);
        return decoded_data;
}

macro_rules! parameterize {
    ($test_fn:expr, [$(($name:ident, $input:expr)), * $(,)? ]) => {
        $(
            #[test]
            fn $name() {
                $test_fn($input);
            }
        )*
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    parameterize!(
        same_decoding_as_etherscan,
        [
            (
                erc20_transfer,
                "0x53ad65f13d7abec1423e1663e0d2c6852d7a60651248e565471ab722d1da9bed"
            ),
            (
                erc20_approval,
                "0xa2d187d1f4038bd2e7498940ffc063a10bb68bb16f025e10ae88c03579fb38b3"
            ),
            ( 
                uniswap_router_swap,
                "0xd901784e01299fe2481714e53ac13be41e827b6752670a9d98e8c00daabdc2c1"
            ),
            ( 
                iceth_issuance,
                "0x78737c10ef3008251795ae134c6671f23fbaf5486cd1b14f378111e141f49cc0"
            ),
            ( 
                zeroex_exchange_issuance,
                "0x001f464ec829d10f87c77c6589dff342fb36873a8e59a0ad21a102f8a50576f9"
            ),
        ]
    );

    #[tokio::main]
    async fn same_decoding_as_etherscan(tx_hash: &str) {
        let tx_hash = tx_hash.trim_start_matches("0x");
        let expected_tokens = utils::remove_single_top_level_tuple(decode_tx_via_etherscan(tx_hash).await.unwrap());

        let tokens = decode_transaction_calldata(tx_hash).await;
        println!("");
        println!("#### Expected Tokens ####");
        for token in &expected_tokens {
            utils::print_parse_tree(&token, 0);
        }
        println!("");
        println!("#### Decoded Tokens ####");
        for token in &tokens {
            utils::print_parse_tree(&token, 0);
        }
        println!("### DONE ##");

        let cleaned_expected_tokens = utils::replace_zero_value_with_uint(expected_tokens);
        assert_eq!(tokens, cleaned_expected_tokens);
    }
}

async fn decode_tx_via_etherscan(tx_hash: &str) -> Option<Vec<Token>> {
        let tx_hash = tx_hash.trim_start_matches("0x");
        let provider = Provider::<Http>::try_from(
            "https://mainnet.infura.io/v3/c60b0bb42f8a4c6481ecd229eddaca27"
        ).expect("could not instantiate HTTP Provider");

        let mut tx_hash_bytes: [u8; 32] = [0; 32];
        hex::decode_to_slice(tx_hash, &mut tx_hash_bytes).expect("Decoding failed");

        let tx = provider.get_transaction(tx_hash_bytes).await.unwrap().unwrap();
        let contract_address = format!("0x{:}", hex::encode(tx.to.unwrap()));
        let contract_abi = utils::get_etherscan_contract(&contract_address, "etherscan.io").await.unwrap();
        let contract = Contract::load(contract_abi.as_bytes()).unwrap();
        let selector = &tx.input.0[0..4];
        for function in contract.functions.values().flatten() {
            let signature = function.short_signature();
            if selector == signature  {
                let decoded = function.decode_input(&tx.input.0[4..]).unwrap();
                return Some(decoded);
            }
        }
        None
}

