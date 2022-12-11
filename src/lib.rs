use ethabi::{Contract, Token};
use std::env;

use ethereum_types::H160;
use ethers::providers::Middleware;

use std::convert::TryFrom;

pub mod decoder;
pub mod utils;

pub async fn decode_transaction_calldata(tx_hash: &str) -> Vec<Token> {
    let arguments_encoded = get_encoded_arguments(tx_hash).await;
    if arguments_encoded.is_empty() {
        println!("Not a valid function call");
        return Vec::new();
    }

    chunk_and_decode_data(&arguments_encoded)
}

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
    let provider = utils::get_provider();
    let tx = provider
        .get_transaction(tx_hash_bytes)
        .await
        .unwrap()
        .unwrap();
    hex::encode(tx.input.0)
}

fn chunk_and_decode_data(encoded_arguments: &str) -> Vec<Token> {
    if encoded_arguments.is_empty() {
        return Vec::new();
    }

    let encoded_arguments = decoder::add_padding(encoded_arguments);
    let chunks = decoder::chunk_data(&encoded_arguments);
    for (i, chunk) in chunks.iter().enumerate() {
        println!(
            "{}: {} - {}",
            i,
            chunk,
            u64::from_str_radix(chunk.trim_start_matches('0'), 16).unwrap_or(0)
        );
    }
    decoder::decode_chunks(chunks)
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
    parameterize!(
        can_re_encode_single_transaction,
        [
            // These opensea transactions contain extra data appeneded after the encoded_arguments which
            // will always mess up this algorithm
            (
                nft_bulk_transfer,
                "0x32cf9e754e4e2400886bb9119130de3c826132921cd444ad882efe670f29cc23"
            ),
            (
                cockpunch_mint_with_trailing_bytes,
                "a848faf90566f79928e8a01cf483f2f1e899fced845e9e5cc164b79a295becd5"
            ),
            (
                opensea_cancel_listing,
                "0xe65afe90ca425074a68231a64c30e743878c46e0bed15307561c31d1acbce297"
            ),
            // Transaction with huge calldata that gets the algorithm stuck in infinite recursion
            // TODO: Try to reproduce with smaller example
            // (
            //     opensea_fullfill_multiple_orders_long,
            //     "0x9360601719fa9c412e402dde237a384ff7517e64cb47258b775b237c8d88827f"
            // ),
            (
                opensea_fullfill_multiple_orders_shorter,
                "0x7c1531482c3c1d1d42638016e8912ed7d12ba709efdb0db77790308f4af8a531"
            ),
            (
                random_nft_claim,
                "0x8d0ad5358f0a402906b8ee9cfdcb466915f52abb97c2b036ca6e292983e03a7d"
            )
        ]
    );

    #[tokio::main]
    async fn same_decoding_as_etherscan(tx_hash: &str) {
        let tx_hash = tx_hash.trim_start_matches("0x");
        let expected_tokens =
            utils::remove_single_top_level_tuple(decode_tx_via_etherscan(tx_hash).await.unwrap());

        // let expected_tokens_reencoded = hex::encode(ethabi::encode(&expected_tokens));
        // println!("Checking reencoded tokens");
        // assert_eq!(arguments_encoded, expected_tokens_reencoded);

        let arguments_encoded = decoder::add_padding(&get_encoded_arguments(tx_hash).await);
        print_chunked_data("#### ENCODED ARGUMENTS ####", &arguments_encoded);

        println!();
        println!("#### Expected Tokens ####");
        for token in &expected_tokens {
            utils::print_parse_tree(token, 0);
        }

        let tokens = decode_transaction_calldata(tx_hash).await;
        println!();
        println!("#### Decoded Tokens ####");
        for token in &tokens {
            utils::print_parse_tree(token, 0);
        }
        println!("### DONE ##");

        let cleaned_expected_tokens = utils::replace_zero_value_with_uint(expected_tokens);
        assert_eq!(tokens, cleaned_expected_tokens);
    }

    #[tokio::main]
    async fn can_re_encode_single_transaction(tx_hash: &str) {
        let tx_hash = tx_hash.trim_start_matches("0x");
        let arguments_encoded = decoder::add_padding(&get_encoded_arguments(tx_hash).await);
        print_chunked_data("#### ENCODED ARGUMENTS ####", &arguments_encoded);

        let expected_tokens =
            utils::remove_single_top_level_tuple(decode_tx_via_etherscan(tx_hash).await.unwrap());
        println!("#### Expected Tokens ####");
        for token in &expected_tokens {
            utils::print_parse_tree(token, 0);
        }

        let tokens = decode_transaction_calldata(tx_hash).await;
        println!("#### Decoded Tokens ####");
        for token in &tokens {
            utils::print_parse_tree(token, 0);
        }
        println!("### DONE ##");
        let tokens_reencoded = hex::encode(ethabi::encode(&tokens));
        println!("Reencoded tokens length: {}", tokens_reencoded.len());
        print_chunked_data("#### RE-ENCODED ARGUMENTS ####", &tokens_reencoded);

        assert_eq!(tokens_reencoded, arguments_encoded);
    }

    // Opensea/Seaport transactions often are troublesome since they contain complex nested
    // data and added data after the encoded arguments. This makes it hard to decode them correctly
    #[tokio::main]
    #[test]
    async fn can_re_encode_all_transactions_to_seaport() {
        let start_block = 16136002;
        // TODO: Transactions with very large calldata to seaport methods can cause the algorithm
        // to get stuck - investigate
        // Increase this value to find the smallest problematic transaction for debugging
        let max_calldata_size = 64 * 100;
        let num_blocks = 5;
        let seaport_address =
            H160::from_slice(&hex::decode("00000000006c3852cbef3e08e8df289169ede581").unwrap());
        let provider = utils::get_provider();
        for block_number in start_block..start_block + num_blocks {
            println!("Testing transactions from block: {}", block_number);
            let block = provider
                .get_block_with_txs(block_number)
                .await
                .unwrap()
                .unwrap();
            println!(
                "Number of transactions in block: {}",
                block.transactions.len()
            );
            for (i, tx) in block.transactions.iter().enumerate() {
                if tx.to == Some(seaport_address) {
                    println!("Tx index: {}", i);
                    let tx_hash = hex::encode(tx.hash.0);
                    let calldata = hex::encode(&tx.input.0);
                    let encoded_arguments =
                        decoder::add_padding(split_off_encoded_arguments(&calldata));
                    print_chunked_data("#### ENCODED ARGUMENTS ####", &encoded_arguments);
                    if encoded_arguments.len() > max_calldata_size {
                        println!("Skipping transaction with huge calldata: {}", tx_hash);
                        continue;
                    }
                    println!("Encoded arguments length: {}", encoded_arguments.len());
                    println!("Decoding tx: {}", tx_hash);
                    let tokens = decode_transaction_calldata(&tx_hash).await;
                    println!();
                    println!("#### Decoded Tokens ####");
                    for token in &tokens {
                        utils::print_parse_tree(token, 0);
                    }
                    println!("### DONE ##");
                    let tokens_reencoded = hex::encode(ethabi::encode(&tokens));
                    println!("Reencoded tokens length: {}", tokens_reencoded.len());
                    print_chunked_data("#### RE-ENCODED ARGUMENTS ####", &tokens_reencoded);
                    assert_eq!(tokens_reencoded, encoded_arguments);
                }
            }
        }
    }
    #[tokio::main]
    #[test]
    async fn can_re_encode_all_transactions_not_to_seaport() {
        let start_block = 16136001;
        let num_blocks = 1;
        let seaport_address =
            H160::from_slice(&hex::decode("00000000006c3852cbef3e08e8df289169ede581").unwrap());
        let provider = utils::get_provider();
        for block_number in start_block..start_block + num_blocks {
            println!("Testing transactions from block: {}", block_number);
            let block = provider
                .get_block_with_txs(block_number)
                .await
                .unwrap()
                .unwrap();
            println!(
                "Number of transactions in block: {}",
                block.transactions.len()
            );
            for (i, tx) in block.transactions.iter().enumerate() {
                if tx.to != Some(seaport_address) {
                    println!("Tx index: {}", i);
                    let tx_hash = hex::encode(tx.hash.0);
                    let calldata = hex::encode(&tx.input.0);
                    let encoded_arguments =
                        decoder::add_padding(split_off_encoded_arguments(&calldata));
                    print_chunked_data("#### ENCODED ARGUMENTS ####", &encoded_arguments);
                    println!("Encoded arguments length: {}", encoded_arguments.len());
                    println!("Decoding tx: {}", tx_hash);
                    let tokens = decode_transaction_calldata(&tx_hash).await;
                    println!();
                    println!("#### Decoded Tokens ####");
                    for token in &tokens {
                        utils::print_parse_tree(token, 0);
                    }
                    println!("### DONE ##");
                    let tokens_reencoded = hex::encode(ethabi::encode(&tokens));
                    println!("Reencoded tokens length: {}", tokens_reencoded.len());
                    print_chunked_data("#### RE-ENCODED ARGUMENTS ####", &tokens_reencoded);
                    assert_eq!(tokens_reencoded, encoded_arguments);
                }
            }
        }
        let num_blocks = 1;
        let provider = utils::get_provider();
        for block_number in start_block..start_block + num_blocks {
            println!("Testing transactions from block: {}", block_number);
            let block = provider
                .get_block_with_txs(block_number)
                .await
                .unwrap()
                .unwrap();
            println!(
                "Number of transactions in block: {}",
                block.transactions.len()
            );
            for (i, tx) in block.transactions.iter().enumerate() {
                println!("Tx index: {}", i);
                let tx_hash = hex::encode(tx.hash.0);
                println!("Decoding tx: {}", tx_hash);
                let tokens = decode_transaction_calldata(&tx_hash).await;
                println!();
                println!("#### Decoded Tokens ####");
                for token in &tokens {
                    utils::print_parse_tree(token, 0);
                }
                println!("### DONE ##");
                let tokens_reencoded = hex::encode(ethabi::encode(&tokens));
                println!("Reencoded tokens length: {}", tokens_reencoded.len());
                print_chunked_data("#### RE-ENCODED ARGUMENTS ####", &tokens_reencoded);
                let calldata = hex::encode(&tx.input.0);
                let encoded_arguments =
                    decoder::add_padding(split_off_encoded_arguments(&calldata));
                println!("Encoded arguments length: {}", encoded_arguments.len());
                print_chunked_data("#### ENCODED ARGUMENTS ####", &encoded_arguments);
                assert_eq!(tokens_reencoded, encoded_arguments);
            }
        }
    }

    async fn decode_tx_via_etherscan(tx_hash: &str) -> Option<Vec<Token>> {
        let tx_hash = tx_hash.trim_start_matches("0x");
        let provider = utils::get_provider();

        let mut tx_hash_bytes: [u8; 32] = [0; 32];
        hex::decode_to_slice(tx_hash, &mut tx_hash_bytes).expect("Decoding failed");
        let tx = provider
            .get_transaction(tx_hash_bytes)
            .await
            .unwrap()
            .unwrap();

        let contract_address = format!("0x{:}", hex::encode(tx.to.unwrap()));
        let contract_abi = utils::get_etherscan_contract(&contract_address, "etherscan.io")
            .await
            .unwrap();
        let contract = Contract::load(contract_abi.as_bytes()).unwrap();
        let selector = &tx.input.0[0..4];
        for function in contract.functions.values().flatten() {
            let signature = function.short_signature();
            if selector == signature {
                let decoded = function.decode_input(&tx.input.0[4..]).unwrap();
                return Some(decoded);
            }
        }
        None
    }

    fn print_chunked_data(label: &str, data: &str) {
        println!("{}", label);
        let chunks = decoder::chunk_data(data);
        for (i, chunk) in chunks.iter().enumerate() {
            println!(
                "{}: {} - {}",
                i,
                chunk,
                u64::from_str_radix(chunk.trim_start_matches('0'), 16).unwrap_or(0)
            );
        }
    }
}
