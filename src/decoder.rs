use ethabi::token::Token;
use std::collections::HashMap;

pub mod parse_marker;
pub mod parse_token;
pub mod preprocessing;
#[cfg(test)]
mod tests;

use parse_marker::ParseMarker;
use preprocessing::{add_padding, chunk_data};
// TODO: Add check that ensures arrays have elements of the same type

pub fn chunk_and_decode_data(encoded_arguments: &str) -> Vec<Token> {
    if encoded_arguments.is_empty() {
        return Vec::new();
    }

    let encoded_arguments = add_padding(encoded_arguments);
    let chunks = chunk_data(&encoded_arguments);
    println!("#### Encoded calldata (without function selector) ####");
    for (i, chunk) in chunks.iter().enumerate() {
        println!(
            "{}: {} - {}",
            i,
            chunk,
            u64::from_str_radix(chunk.trim_start_matches('0'), 16).unwrap_or(0)
        );
    }
    println!("\n");
    decode_chunks(chunks)
}

pub fn decode_chunks(chunks: Vec<&str>) -> Vec<Token> {
    let result = parse_token::parse_token(&ParseMarker::TopLevel, &chunks, &HashMap::new(), true);
    if let Some(parse_token::TokenOrTopLevel::TopLevel(tokens)) = result {
        tokens
    } else {
        panic!("Failed to parse arguments");
    }
}
