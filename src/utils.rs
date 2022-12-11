use ethabi::token::Token;
use ethereum_types::{H160, U256};
use ethers::providers::{Http, Middleware, Provider};
use eyre::Result;
use reqwest;
use std::{env};

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

pub async fn get_etherscan_contract(address: &str, domain: &str) -> Result<String> {
    let api_key = env::var("ETHERSCAN_API_KEY")
        .expect("Could not get ETHERSCAN_API_KEY from environment");

    let abi_url = format!(
        "http://api.{}/api?module=contract&action=getabi&address={:}&format=raw&apikey={}",
        domain, address, api_key,
    );
    println!("ABI URL: {:?}", abi_url);
    let abi = reqwest::get(abi_url).await?.text().await?;

    if abi.starts_with("Contract source code not verified") {
        eyre::bail!("Contract source code not verified: {:?}", address);
    }
    if abi.starts_with('{') && abi.contains("Max rate limit reached") {
        eyre::bail!(
            "Max rate limit reached, please use etherscan API Key for higher rate limit: {:?}",
            address
        );
    }

    Ok(abi)
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
