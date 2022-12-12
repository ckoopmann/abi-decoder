use crate::test_utils::{address_token_from_string, bytes_token_from_string};
use ethabi::Token;

pub fn token_hack_from_youtube_talk_expected_result() -> Vec<ethabi::Token> {
    vec![
                    Token::Array(vec![
                        Token::Tuple(vec![
                            address_token_from_string("0xcd62dde0e5acbc1d596b1c1699c8b2a5f1327693"),
                            bytes_token_from_string("d9b184950000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000600000000000000000000000000cd62dde0e5acbc1d596b1c1699c8b2a5f132769300000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000544146b8253000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000005000000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000008000000000000000000000000000000000000000000000000000000000000001e000000000000000000000000000000000000000000000000000000000000002a0000000000000000000000000000000000000000000000000000000000000042000000000000000000000000091191a15e778d46255fc9acd37d028228d97e786000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000e44885b2540000000000000000000000004b64f382aa063c07f1c55cf53c66cce3b6fd0bb0000000000000000000000000000000000000000000000000000000000000006000000000000000000000000000000000000000000000000000000000000000a00000000000000000000000000000000000000000000000000000000000000001000000000000000000000000cd62dde0e5acbc1d596b1c1699c8b2a5f132769300000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000002afb7c108d64a9177c00000000000000000000000000000000000000000000000000000000000000000000000000000000000091191a15e778d46255fc9acd37d028228d97e78600000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000044095ea7b300000000000000000000000010ed43c718714eb63d5aa57b78b54704e256024effffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff0000000000000000000000000000000000000000000000000000000000000000000000000000000010ed43c718714eb63d5aa57b78b54704e256024e000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000001045c11d7950000000000000000000000000000000000000000002afb7c108d64a9177c0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000a0000000000000000000000000cd62dde0e5acbc1d596b1c1699c8b2a5f1327693000000000000000000000000000000000000000000000000000000e8d4a50fff000000000000000000000000000000000000000000000000000000000000000200000000000000000000000091191a15e778d46255fc9acd37d028228d97e786000000000000000000000000bb4cdb9cbd36b01bd1cbaebf2de08d9173bc095c000000000000000000000000000000000000000000000000000000000000000000000000000000005df712fb47651986b1f972a8c71e364a37b207d100000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000004919840ad000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000000000000000000000cd62dde0e5acbc1d596b1c1699c8b2a5f13276930000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000000434c2c13000000000000000000000000000000000000000000000000000000000")
                        ]),
                        Token::Tuple(vec![
                            address_token_from_string("0xcd62dde0e5acbc1d596b1c1699c8b2a5f1327693"),
                            bytes_token_from_string("8014bad30000000000000000000000000000000000000000000000000000000000000040000000000000000000000000e183128b965ac347a5a082080cb9b6635faf90430000000000000000000000000000000000000000000000000000000000000001000000000000000000000000bb4cdb9cbd36b01bd1cbaebf2de08d9173bc095c"),
                        ]),
                        Token::Tuple(vec![
                            address_token_from_string("0xcd62dde0e5acbc1d596b1c1699c8b2a5f1327693"),
                            bytes_token_from_string("c230df73"),
                        ]),
                        Token::Tuple(vec![
                            address_token_from_string("0xcd62dde0e5acbc1d596b1c1699c8b2a5f1327693"),
                            bytes_token_from_string("c4d1776100000000000000000000000000000000000000000000000000000000015b4ae4"),
                        ]),
                        Token::Tuple(vec![
                            address_token_from_string("0x5df712fb47651986b1f972a8c71e364a37b207d1"),
                            bytes_token_from_string("919840ad"),
                        ]),
                    ])
                  ]
}
