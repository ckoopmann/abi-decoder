use crate::decoder;
use ethabi::Token;
use ethereum_types::{H160, U256};
use std::str::FromStr;

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

pub fn address_token_from_string(address: &str) -> Token {
    Token::Address(H160::from_str(address).unwrap())
}

pub fn bytes_token_from_string(bytes: &str) -> Token {
    Token::Bytes(hex::decode(bytes).unwrap())
}

pub fn fixed_bytes_token_from_string(bytes: &str) -> Token {
    Token::FixedBytes(hex::decode(bytes).unwrap())
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

pub fn print_chunked_data(label: &str, data: &str) {
    println!("{}", label);
    let chunks = decoder::preprocessing::chunk_data(data);
    for (i, chunk) in chunks.iter().enumerate() {
        println!(
            "{}: {} - {}",
            i,
            chunk,
            u64::from_str_radix(chunk.trim_start_matches('0'), 16).unwrap_or(0)
        );
    }
}

pub(crate) use parameterize;
