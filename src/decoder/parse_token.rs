use ethabi::param_type::ParamType;
use ethabi::token::{LenientTokenizer, Token, Tokenizer};
use hex;
use std::collections::HashMap;

use super::parse_marker::{
    add_disallowed_marker, generate_parse_markers, get_index, Location, MarkerType, ParseMarker,
};

#[derive(Debug, Clone)]
pub enum TokenOrTopLevel {
    Token(Token),
    TopLevel(Vec<Token>),
}

impl TokenOrTopLevel {
    fn to_token(&self) -> Token {
        match self {
            TokenOrTopLevel::Token(token) => (*token).clone(),
            TokenOrTopLevel::TopLevel(_) => panic!("Expected token, got top level"),
        }
    }
}

pub fn parse_token(
    parse_marker: &ParseMarker,
    chunks: &[&str],
    disallowed_markers: &HashMap<usize, MarkerType>,
    recurse_disallow_markers: bool,
) -> Option<TokenOrTopLevel> {
    match parse_marker {
        ParseMarker::Tuple(ref location) => parse_tuple(location, chunks),
        ParseMarker::Word(location) => parse_word(location, chunks),
        ParseMarker::DynamicBytes(padding, location) => {
            parse_dynamic_bytes(padding, location, chunks)
        }
        ParseMarker::StaticArray(element_size, ref location) => parse_static_array(
            element_size,
            location,
            chunks,
            disallowed_markers,
            recurse_disallow_markers,
        ),
        _ => parse_nested_token(
            parse_marker,
            chunks,
            disallowed_markers,
            recurse_disallow_markers,
        ),
    }
}

fn parse_static_array(
    element_size: &usize,
    location: &Location,
    chunks: &[&str],
    disallowed_markers: &HashMap<usize, MarkerType>,
    recurse_disallow_markers: bool,
) -> Option<TokenOrTopLevel> {
    let mut parse_tree = Vec::new();
    let data_to_parse = chunks[location.start..location.end].to_vec();
    let mut i = 0;
    while i < data_to_parse.len() {
        let new_parse_marker = if *element_size == 1 {
            ParseMarker::Word(i)
        } else {
            ParseMarker::Tuple(Location {
                start: i,
                end: i + element_size,
            })
        };

        parse_tree.push(
            parse_token(
                &new_parse_marker,
                &data_to_parse,
                disallowed_markers,
                recurse_disallow_markers,
            )?
            .to_token(),
        );
        i += element_size;
    }
    let result = Token::Array(parse_tree);
    Some(TokenOrTopLevel::Token(result))
}

fn parse_dynamic_bytes(
    padding: &usize,
    location: &Location,
    chunks: &[&str],
) -> Option<TokenOrTopLevel> {
    let mut decoded_bytes: Vec<u8> = chunks[location.start..location.end]
        .iter()
        .flat_map(|chunk| hex::decode(chunk).expect("Failed to decode dynamic bytes"))
        .collect();
    decoded_bytes.truncate(decoded_bytes.len().saturating_sub(*padding));
    Some(TokenOrTopLevel::Token(Token::Bytes(decoded_bytes)))
}

fn parse_word(location: &usize, chunks: &[&str]) -> Option<TokenOrTopLevel> {
    Some(TokenOrTopLevel::Token(tokenize_argument(chunks[*location])))
}

fn parse_tuple(location: &Location, chunks: &[&str]) -> Option<TokenOrTopLevel> {
    let data_to_parse = chunks[location.start..location.end].to_vec();
    let elements = data_to_parse.iter().map(|x| tokenize_argument(x)).collect();
    Some(TokenOrTopLevel::Token(Token::Tuple(elements)))
}

fn parse_nested_token(
    outer_parse_marker: &ParseMarker,
    chunks: &[&str],
    disallowed_markers: &HashMap<usize, MarkerType>,
    recurse_disallow_markers: bool,
) -> Option<TokenOrTopLevel> {
    let data_to_parse = match outer_parse_marker {
        ParseMarker::TopLevel => chunks,
        ParseMarker::DynamicArray(..) => &chunks[1..],
        ParseMarker::DynamicOffset(_, ref location) => {
            if disallowed_markers.contains_key(&0) && disallowed_markers[&0] == MarkerType::Tuple {
                return None;
            }
            &chunks[location.start..location.end + 1]
        }
        _ => panic!("Non nested marker passed to parse_nested_token"),
    };
    let (parse_markers, tokens) = generate_tokens(
        outer_parse_marker,
        disallowed_markers,
        data_to_parse,
        recurse_disallow_markers,
    )?;
    let wrapped_token = match outer_parse_marker {
        ParseMarker::TopLevel => TokenOrTopLevel::TopLevel(tokens.clone()),
        ParseMarker::DynamicArray(..) => TokenOrTopLevel::Token(Token::Array(tokens.clone())),
        ParseMarker::DynamicOffset(_, _) => TokenOrTopLevel::Token(Token::Tuple(tokens.clone())),
        _ => panic!("Non nested marker passed to parse_nested_token"),
    };
    strip_invalid_tokens(
        outer_parse_marker,
        &parse_markers,
        wrapped_token,
        tokens,
        disallowed_markers,
        chunks,
        recurse_disallow_markers,
    )
}

fn generate_tokens(
    outer_parse_marker: &ParseMarker,
    disallowed_markers: &HashMap<usize, MarkerType>,
    inner_data: &[&str],
    recurse_disallow_markers: bool,
) -> Option<(Vec<ParseMarker>, Vec<Token>)> {
    let mut tokens = Vec::new();
    let parse_markers = generate_parse_markers(
        outer_parse_marker,
        disallowed_markers.clone(),
        inner_data,
        matches!(outer_parse_marker, ParseMarker::DynamicOffset(..)),
    );
    for parse_marker in parse_markers.clone() {
        let result = parse_token(
            &parse_marker,
            inner_data,
            disallowed_markers,
            recurse_disallow_markers,
        );

        if let Some(wrapped_token) = result {
            tokens.push(wrapped_token.to_token());
        } else if recurse_disallow_markers {
            let mut new_disallowed_markers = disallowed_markers.clone();
            add_disallowed_marker(&mut new_disallowed_markers, &parse_marker).ok()?;
            return generate_tokens(
                outer_parse_marker,
                &new_disallowed_markers,
                inner_data,
                recurse_disallow_markers,
            );
        } else {
            return None;
        }
    }
    Some((parse_markers, tokens))
}

fn strip_invalid_tokens(
    parse_marker: &ParseMarker,
    parse_markers: &[ParseMarker],
    token: TokenOrTopLevel,
    mut tokens: Vec<Token>,
    disallowed_markers: &HashMap<usize, MarkerType>,
    data_to_parse: &[&str],
    recurse_disallow_markers: bool,
) -> Option<TokenOrTopLevel> {
    let invalid_token_markers = get_invalid_token_markers(parse_markers, &tokens);
    if !invalid_token_markers.is_empty() {
        if recurse_disallow_markers {
            let result = rerun_with_invalid_token_markers(
                parse_marker,
                &invalid_token_markers,
                disallowed_markers,
                data_to_parse,
                false,
            );

            if result.is_some() {
                result
            } else {
                let result = rerun_with_invalid_token_markers(
                    parse_marker,
                    &invalid_token_markers,
                    disallowed_markers,
                    data_to_parse,
                    true,
                );
                if result.is_some() {
                    result
                } else {
                    None
                }
            }
        } else {
            None
        }
    } else {
        match token {
            TokenOrTopLevel::TopLevel(_) => {
                tokens = tokens
                    .iter()
                    .map(|e| remove_single_element_tuples((*e).clone()))
                    .collect();

                Some(TokenOrTopLevel::TopLevel(tokens))
            }
            _ => Some(token),
        }
    }
}

fn rerun_with_invalid_token_markers(
    parse_marker: &ParseMarker,
    invalid_token_markers: &Vec<(usize, MarkerType)>,
    disallowed_markers: &HashMap<usize, MarkerType>,
    data_to_parse: &[&str],
    recurse_disallow_markers: bool,
) -> Option<TokenOrTopLevel> {
    for invalid_token_marker in invalid_token_markers {
        if !disallowed_markers.contains_key(&invalid_token_marker.0)
            || invalid_token_marker.1 != disallowed_markers[&invalid_token_marker.0]
        {
            let mut new_disallowed_markers = disallowed_markers.clone();
            new_disallowed_markers.insert(invalid_token_marker.0, invalid_token_marker.1.clone());
            let new_result = parse_token(
                parse_marker,
                data_to_parse,
                &new_disallowed_markers,
                recurse_disallow_markers,
            );
            if new_result.is_some() {
                return new_result;
            }
        }
    }
    None
}

fn get_invalid_token_markers(
    parse_markers: &[ParseMarker],
    tokens: &[Token],
) -> Vec<(usize, MarkerType)> {
    let mut invalid_token_markers: Vec<(usize, MarkerType)> = Vec::new();
    for (i, token) in tokens.iter().enumerate() {
        if let Some(marker_type) = check_token(token, &parse_markers[i]) {
            let index = get_index(&parse_markers[i]);
            invalid_token_markers.push((index, marker_type));
        };
    }
    invalid_token_markers
}

fn remove_single_element_tuples(token: Token) -> Token {
    match token {
        Token::Tuple(tokens) => {
            if tokens.len() == 1 {
                remove_single_element_tuples(tokens[0].clone())
            } else {
                Token::Tuple(
                    tokens
                        .iter()
                        .map(|e| remove_single_element_tuples((*e).clone()))
                        .collect(),
                )
            }
        }
        Token::Array(tokens) => Token::Array(
            tokens
                .iter()
                .map(|e| remove_single_element_tuples((*e).clone()))
                .collect(),
        ),
        _ => token,
    }
}

pub fn tokenize_argument(argument: &str) -> Token {
    let trimmed_argument = argument.trim_start_matches('0');

    // If word does not start with a 0 then assume it is a left aligned type (bytes)
    if trimmed_argument.len() == argument.len() {
        let mut right_trimmed_argument = argument.trim_end_matches('0').to_owned();
        if right_trimmed_argument.len() % 2 == 1 {
            right_trimmed_argument.push('0');
        }
        let bytes_len = right_trimmed_argument.len() / 2;

        return LenientTokenizer::tokenize(
            &ParamType::FixedBytes(bytes_len),
            &right_trimmed_argument,
        )
        .expect("Failed to tokenize bytes argument");
    }
    // TODO: Maybe change default to still use bytes when only one leading zero ?

    // If bytes match an address that does not start with 0 byte then assume it is an address
    if let Ok(token) = LenientTokenizer::tokenize(&ParamType::Address, trimmed_argument) {
        return token;
    }

    if let Ok(token) = LenientTokenizer::tokenize(&ParamType::Uint(256), argument) {
        token
    } else {
        panic!("Could not tokenize argument: {}", argument);
    }
}

pub fn check_token(token: &Token, parse_marker: &ParseMarker) -> Option<MarkerType> {
    match (token, parse_marker) {
        (Token::Tuple(_), ParseMarker::DynamicOffset(..)) => {
            if contains_dynamic_type(token) {
                None
            } else {
                Some(MarkerType::Tuple)
            }
        }
        _ => None,
    }
}

pub fn contains_dynamic_type(token: &Token) -> bool {
    match token {
        Token::Tuple(tokens) => {
            for token in tokens {
                if contains_dynamic_type(token) {
                    return true;
                }
            }
            false
        }
        Token::Array(_) => true,
        Token::Bytes(_) => true,
        _ => false,
    }
}
