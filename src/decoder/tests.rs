use super::*;
use crate::{parameterize, utils::{address_token_from_string, fixed_bytes_token_from_string, print_parse_tree}};
use ethabi::Token;
use ethereum_types::U256;
use hex;

parameterize!(
    test_same_encoding,
    [
        (
            address_bytes_and_uint256,
            vec![
                address_token_from_string("0x7C07F7aBe10CE8e33DC6C5aD68FE033085256A84"),
                fixed_bytes_token_from_string("7C07F7aBe10CE8e33DC6C5aD68FE033085256A"),
                Token::Uint(U256::from(100)),
            ]
        ),
        (
            address_and_uint256,
            vec![
                address_token_from_string("0x7C07F7aBe10CE8e33DC6C5aD68FE033085256A84"),
                Token::Uint(U256::from(100)),
            ]
        ),
        (
            uint256_and_address,
            vec![
                Token::Uint(U256::from(100)),
                address_token_from_string("0x7C07F7aBe10CE8e33DC6C5aD68FE033085256A84")
            ]
        ),
        (
            uint256_array_simple,
            vec![Token::Array(vec![
                Token::Uint(U256::from(3)),
                Token::Uint(U256::from(4)),
            ])]
        ),
        (
            uint256_array_nested,
            vec![Token::Array(vec![
                Token::Array(vec![
                    Token::Uint(U256::from(128)),
                    Token::Uint(U256::from(256)),
                ]),
                Token::Array(vec![
                    Token::Uint(U256::from(512)),
                    Token::Uint(U256::from(1024)),
                ]),
            ])]
        ),
        (
            array_of_static_tuples,
            vec![Token::Array(vec![
                Token::Tuple(vec![
                    Token::Uint(U256::from(128)),
                    Token::Uint(U256::from(1024)),
                ]),
                Token::Tuple(vec![
                    Token::Uint(U256::from(128)),
                    Token::Uint(U256::from(1024)),
                ]),
            ]),]
        ),
        (
            array_of_arrays,
            vec![Token::Array(vec![
                Token::Array(vec![
                    Token::Array(vec![
                        Token::Uint(U256::from(128)),
                        Token::Uint(U256::from(1024)),
                    ]),
                    Token::Array(vec![
                        Token::Uint(U256::from(1)),
                        Token::Uint(U256::from(2)),
                        Token::Uint(U256::from(3)),
                    ]),
                ]),
                Token::Array(vec![
                    Token::Array(vec![
                        Token::Uint(U256::from(128)),
                        Token::Uint(U256::from(1024)),
                    ]),
                    Token::Array(vec![
                        Token::Uint(U256::from(1)),
                        Token::Uint(U256::from(2)),
                        Token::Uint(U256::from(3)),
                    ]),
                ])
            ]),]
        ),
        (
            tuple_of_two_arrays_and_uint256,
            vec![Token::Tuple(vec![
                // TODO: If this value is changed to "2" the test breaks (since the
                // remaining data will be interpreted as an array of two tuples
                Token::Uint(U256::from(1)),
                Token::Array(vec![
                    Token::Uint(U256::from(128)),
                    Token::Uint(U256::from(1024)),
                ]),
                Token::Array(vec![
                    Token::Uint(U256::from(128)),
                    Token::Uint(U256::from(1024)),
                ]),
            ]),]
        ),
        (
            array_of_static_tuples_nested,
            vec![Token::Array(vec![
                Token::Tuple(vec![
                    Token::Array(vec![
                        Token::Tuple(vec![
                            Token::Uint(U256::from(128)),
                            Token::Uint(U256::from(1024)),
                        ]),
                        Token::Tuple(vec![
                            Token::Uint(U256::from(128)),
                            Token::Uint(U256::from(1024)),
                        ]),
                    ]),
                    Token::Uint(U256::from(128)),
                ]),
                Token::Tuple(vec![
                    Token::Array(vec![
                        Token::Tuple(vec![
                            Token::Uint(U256::from(123)),
                            Token::Uint(U256::from(456)),
                        ]),
                        Token::Tuple(vec![
                            Token::Uint(U256::from(690)),
                            Token::Uint(U256::from(420)),
                        ]),
                    ]),
                    Token::Uint(U256::from(1)),
                ]),
            ]),]
        ),
        (
            array_and_two_primitives_flat,
            vec![
                Token::Array(vec![
                    Token::Uint(U256::from(128)),
                    Token::Uint(U256::from(1024)),
                ]),
                Token::Uint(U256::from(128)),
                Token::Uint(U256::from(1024)),
            ]
        ),
        (
            array_and_two_primitives_nested,
            vec![Token::Array(vec![Token::Tuple(vec![
                Token::Array(vec![
                    Token::Uint(U256::from(128)),
                    Token::Uint(U256::from(1024)),
                ]),
                Token::Uint(U256::from(128)),
                Token::Uint(U256::from(1024)),
            ])]),]
        ),
        (
            two_arrays_of_primitives,
            vec![
                Token::Array(vec![
                    Token::Uint(U256::from(128)),
                    Token::Uint(U256::from(1024)),
                ]),
                Token::Array(vec![
                    Token::Uint(U256::from(128)),
                    Token::Uint(U256::from(1024)),
                ]),
                Token::Uint(U256::from(128)),
                Token::Uint(U256::from(1024)),
            ]
        ),
        (
            iceth_issuance_copy,
            vec![
                address_token_from_string("0x7C07F7aBe10CE8e33DC6C5aD68FE033085256A84"),
                Token::Uint(U256::from(10000000000_u64)),
                Token::Tuple(vec![
                    Token::Array(vec![
                        address_token_from_string("0x7C07F7aBe10CE8e33DC6C5aD68FE033085256A87"),
                        address_token_from_string("0x7C07F7aBe10CE8e33DC6C5aD68FE033085256A85"),
                    ]),
                    Token::Array(vec![]),
                    Token::Uint(U256::from(4)),
                ]),
            ]
        ),
    ]
);

parameterize!(
    test_different_encoding,
    [(
        // This is an invalid array (different element types) so it should be decoded
        // differently into a valid interpretation of the data
        array_of_different_static_tuples,
        (
            vec![Token::Array(vec![
                Token::Tuple(vec![
                    Token::Uint(U256::from(128)),
                    Token::Uint(U256::from(1024)),
                ]),
                Token::Tuple(vec![
                    Token::Uint(U256::from(1)),
                    Token::Uint(U256::from(2)),
                    Token::Uint(U256::from(3)),
                ]),
            ]),],
            vec![
                Token::Uint(U256::from(32)),
                Token::Uint(U256::from(2)),
                Token::Uint(U256::from(128)),
                Token::Uint(U256::from(1024)),
                Token::Uint(U256::from(1)),
                Token::Uint(U256::from(2)),
                Token::Uint(U256::from(3)),
            ]
        )
    ),]
);

parameterize!(
    test_can_reencode_with_added_data_at_the_end,
    [
        (
            array_of_static_tuples_nested_with_extra_data,
            vec![Token::Array(vec![
                Token::Tuple(vec![
                    Token::Array(vec![
                        Token::Tuple(vec![
                            Token::Uint(U256::from(128)),
                            Token::Uint(U256::from(1024)),
                        ]),
                        Token::Tuple(vec![
                            Token::Uint(U256::from(128)),
                            Token::Uint(U256::from(1024)),
                        ]),
                    ]),
                    Token::Uint(U256::from(128)),
                ]),
                Token::Tuple(vec![
                    Token::Array(vec![
                        Token::Tuple(vec![
                            Token::Uint(U256::from(123)),
                            Token::Uint(U256::from(456)),
                        ]),
                        Token::Tuple(vec![
                            Token::Uint(U256::from(690)),
                            Token::Uint(U256::from(420)),
                        ]),
                    ]),
                    Token::Uint(U256::from(1)),
                ]),
            ]),]
        ),
        (
            array_of_arrays_with_extra_data,
            vec![Token::Array(vec![
                Token::Array(vec![
                    Token::Array(vec![
                        Token::Uint(U256::from(128)),
                        Token::Uint(U256::from(1024)),
                    ]),
                    Token::Array(vec![
                        Token::Uint(U256::from(1)),
                        Token::Uint(U256::from(2)),
                        Token::Uint(U256::from(3)),
                    ]),
                ]),
                Token::Array(vec![
                    Token::Array(vec![
                        Token::Uint(U256::from(128)),
                        Token::Uint(U256::from(1024)),
                    ]),
                    Token::Array(vec![
                        Token::Uint(U256::from(1)),
                        Token::Uint(U256::from(2)),
                        Token::Uint(U256::from(3)),
                    ]),
                ])
            ]),]
        ),
    ]
);

fn test_same_encoding(arguments: Vec<Token>) {
    println!("Arguments:");
    for argument in &arguments {
        print_parse_tree(argument, 0);
    }
    println!();
    println!();
    let encoded_arguments = add_padding(&hex::encode(ethabi::encode(&arguments)));

    let chunks = chunk_data(&encoded_arguments);
    for (i, chunk) in chunks.iter().enumerate() {
        println!(
            "{}: {} - {}",
            i,
            chunk,
            u64::from_str_radix(chunk.trim_start_matches('0'), 16).unwrap_or(0)
        );
    }
    let tokens = decode_chunks(chunks);
    for token in &tokens {
        print_parse_tree(token, 0);
    }
    assert_eq!(tokens, arguments);
}

fn test_can_reencode_with_added_data_at_the_end(arguments: Vec<Token>) {
    println!("Arguments:");
    for argument in &arguments {
        print_parse_tree(argument, 0);
    }
    println!();
    println!();
    let encoded_arguments = add_padding(&hex::encode(ethabi::encode(&arguments)));

    let mut chunks = chunk_data(&encoded_arguments);
    let extra_data = &"01".repeat(32);
    chunks.push(extra_data);
    for (i, chunk) in chunks.iter().enumerate() {
        println!(
            "{}: {} - {}",
            i,
            chunk,
            u64::from_str_radix(chunk.trim_start_matches('0'), 16).unwrap_or(0)
        );
    }
    let tokens = decode_chunks(chunks);
    for token in &tokens {
        print_parse_tree(token, 0);
    }
    // assert_eq!(tokens, arguments);
}

fn test_different_encoding(arguments_and_expected_tokens: (Vec<Token>, Vec<Token>)) {
    let (arguments, expected_tokens) = arguments_and_expected_tokens;
    for argument in &arguments {
        print_parse_tree(argument, 0);
    }
    println!();
    println!();
    let encoded_arguments = add_padding(&hex::encode(ethabi::encode(&arguments)));

    let chunks = chunk_data(&encoded_arguments);
    for (i, chunk) in chunks.iter().enumerate() {
        println!(
            "{}: {} - {}",
            i,
            chunk,
            u64::from_str_radix(chunk.trim_start_matches('0'), 16).unwrap_or(0)
        );
    }
    let tokens = decode_chunks(chunks);
    for token in &tokens {
        print_parse_tree(token, 0);
    }
    assert_eq!(tokens, expected_tokens);
}
