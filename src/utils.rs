use crate::decoder;
use ethabi::{Contract, Token};
use ethereum_types::{H160, U256};
use ethers::providers::{Http, Middleware, Provider};
use eyre::Result;
use reqwest;
use std::env;

pub fn print_with_indentation(indent: usize, s: &str) {
    for _i in 0..indent {
        print!("    ");
    }
    println!("{}", s);
}

pub fn print_parse_tree(parse_tree: &Token, indentation: usize) {
    match parse_tree {
        Token::Array(ref elements) => {
            print_with_indentation(indentation, "Array: ");
            for item in elements {
                print_parse_tree(item, indentation + 1);
            }
        }
        Token::Tuple(ref elements) => {
            print_with_indentation(indentation, "Tuple: ");
            for item in elements {
                print_parse_tree(item, indentation + 1);
            }
        }
        token => {
            print_with_indentation(indentation, &format!("Token: {:?}", token));
        }
    }
}

pub async fn get_etherscan_contract(address: &str, domain: &str) -> Result<String, String> {
    let api_key =
        env::var("ETHERSCAN_API_KEY").expect("Could not get ETHERSCAN_API_KEY from environment");

    let abi_url = format!(
        "http://api.{}/api?module=contract&action=getabi&address={:}&format=raw&apikey={}",
        domain, address, api_key,
    );
    println!("ABI URL: {:?}", abi_url);
    const MAX_ITERATION: u32 = 5;
    for iteration in 0..MAX_ITERATION {
        let abi = reqwest::get(&abi_url)
            .await
            .map_err(|e| format!("Error getting ABI from etherscan: {:?}", e))?
            .text()
            .await
            .map_err(|e| format!("Error getting ABI from etherscan: {:?}", e))?;

        if abi.starts_with("Contract source code not verified") {
            return Err("Contract source code not verified".to_string());
        }

        if abi.starts_with('{') && abi.contains("Max rate limit reached") {
            if iteration < MAX_ITERATION {
                println!(
                    "Max rate limit reached, sleeping for {} seconds",
                    2_u32.pow(iteration)
                );
                std::thread::sleep(std::time::Duration::from_secs(2_u32.pow(iteration).into()));
                continue;
            }
            return Err("max backoff reached".to_string());
        }
        return Ok(abi);
    }
    return Err("max iteration is zero".to_string());
}

pub fn remove_single_top_level_tuple(tokens: Vec<Token>) -> Vec<Token> {
    if tokens.len() == 1 {
        if let Token::Tuple(inner_tokens) = tokens[0].clone() {
            return inner_tokens;
        }
    }
    tokens
}

pub fn replace_zero_value_with_uint(tokens: Vec<Token>) -> Vec<Token> {
    tokens
        .iter()
        .map(|token| replace_zero_value_with_uint_single_token(token.clone()))
        .collect()
}

pub fn replace_zero_value_with_uint_single_token(token: Token) -> Token {
    match token {
        Token::Bool(false) => Token::Uint(U256::from(0)),
        Token::Address(address) => {
            if address == H160::zero() {
                Token::Uint(U256::from(0))
            } else {
                token
            }
        }
        Token::Array(tokens) => Token::Array(replace_zero_value_with_uint(tokens)),
        Token::Tuple(tokens) => Token::Tuple(replace_zero_value_with_uint(tokens)),
        Token::Int(int) => {
            if int == U256::zero() {
                Token::Uint(U256::from(0))
            } else {
                token
            }
        }
        _ => token,
    }
}

pub fn get_provider() -> Provider<Http> {
    let rpc_url = env::var("RPC_URL").expect("RPC_URL must be set");
    Provider::<Http>::try_from(rpc_url).expect("could not instantiate HTTP Provider")
}

pub async fn decode_tx_via_etherscan(tx_hash: &str) -> Option<Vec<Token>> {
    let tx_hash = tx_hash.trim_start_matches("0x");
    let provider = get_provider();

    let mut tx_hash_bytes: [u8; 32] = [0; 32];
    hex::decode_to_slice(tx_hash, &mut tx_hash_bytes).expect("Decoding failed");
    let tx = provider
        .get_transaction(tx_hash_bytes)
        .await
        .unwrap()
        .unwrap();

    let contract_address = format!("0x{:}", hex::encode(tx.to.unwrap()));
    let contract_abi = get_etherscan_contract(&contract_address, "etherscan.io")
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

pub fn print_chunked_data(label: &str, data: &str) {
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
