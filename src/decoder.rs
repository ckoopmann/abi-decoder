use ethabi::param_type::ParamType;
use ethabi::token::{LenientTokenizer, Token, Tokenizer};
use ethereum_types::{H160, H256, U256};
use ethers::contract;
use hex;
use std::cmp;
use std::collections::HashMap;
use std::i64;
use std::str::FromStr;

use crate::utils;

// TODO: Array tokens of different types ?
// TODO: Array of tuples

#[derive(Debug, Clone)]
pub struct Location {
    start: usize,
    end: usize,
}

#[derive(Debug, Clone)]
pub enum TokenOrTopLevel {
    Token(Token),
    TopLevel(Vec<Token>),
}

impl TokenOrTopLevel {
    fn to_token(self) -> Token {
        match self {
            TokenOrTopLevel::Token(token) => token,
            TokenOrTopLevel::TopLevel(_) => panic!("Expected token, got top level"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ParseMarker {
    Word(usize),
    DynamicBytes(usize, Location),      // Paddding, Location
    StaticArray(usize, Location),       // Element Size, Location
    DynamicArray(usize, Vec<Location>), // Array Starting index, Location
    Tuple(Location),
    DynamicOffset(usize, Location), // Pointer Index, Location
    TopLevel,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MarkerType {
    Word,
    Array,
    Tuple,
    DynamicArray,
    DynamicBytes,
    TopLevel,
}

// Adds padding to the end of the data if it contains trailing bytes
pub fn add_padding(encoded_data: &str) -> String {
    let mut encoded_data = encoded_data.to_string();
    if encoded_data.len() % 64 != 0 {
        let padding = 64 - (encoded_data.len() % 64);
        encoded_data.push_str(&"0".repeat(padding));
    }
    return encoded_data;
}

pub fn chunk_data(encoded_data: &str) -> Vec<&str> {
    let mut encoded_data = encoded_data;
    if encoded_data.len() % 64 != 0 {
        panic!("Invalid data length");
    }
    let mut chunks = Vec::new();
    while encoded_data.len() > 0 {
        let (word, rest) = encoded_data.split_at(64);
        chunks.push(word);
        encoded_data = rest;
    }
    return chunks;
}

pub fn decode_chunks(chunks: Vec<&str>) -> Vec<Token> {
    // let parse_markers = generate_parse_markers(chunks.clone());
    let result = generate_token(ParseMarker::TopLevel, chunks, HashMap::new(), true);
    if let Some(TokenOrTopLevel::TopLevel(tokens)) = result {
        return tokens;
    } else {
        panic!("Failed to parse arguments");
    }
}

pub fn generate_token(
    parse_marker: ParseMarker,
    chunks: Vec<&str>,
    disallowed_markers: HashMap<usize, MarkerType>,
    recurse_disallow_markers: bool,
) -> Option<TokenOrTopLevel> {
    // println!("generate_token(
    //         parse_marker: {:?},
    //         chunks: {:?} - {:?},
    //         disallowed_markers: {:?},
    //         recurse_disallow_markers: {:?},
    //         )", parse_marker, chunks[0], chunks[chunks.len() - 1], disallowed_markers, recurse_disallow_markers);
    // println!("Begin parsing parse_marker: {:?}", parse_marker);
    // println!(
    //     "chunks from: {:?} - to: {:?}",
    //     chunks[0],
    //     chunks[chunks.len() - 1]
    // );
    if disallowed_markers.keys().len() > 0 {
        // println!("Disallowed markers: {:?}", disallowed_markers);
    }
    let result = match parse_marker {
        ParseMarker::Tuple(ref location) => {
            let data_to_parse = chunks[location.start..location.end].to_vec();
            let elements = data_to_parse.iter().map(|x| tokenize_argument(x)).collect();
            Some(TokenOrTopLevel::Token(Token::Tuple(elements)))
        }
        ParseMarker::Word(location) => {
            Some(TokenOrTopLevel::Token(tokenize_argument(chunks[location])))
        }
        ParseMarker::DynamicBytes(padding, ref location) => {
            let mut decoded_bytes: Vec<u8> = chunks[location.start..location.end]
                .iter()
                .map(|chunk| hex::decode(chunk).expect("Failed to decode dynamic bytes"))
                .flatten()
                .collect();
            decoded_bytes.truncate(decoded_bytes.len().saturating_sub(padding));
            Some(TokenOrTopLevel::Token(Token::Bytes(decoded_bytes)))
        }
        ParseMarker::StaticArray(element_size, ref location) => {
            let mut parse_tree = Vec::new();
            let data_to_parse = chunks[location.start..location.end].to_vec();
            let mut i = 0;
            while i < data_to_parse.len() {
                parse_tree.push(
                    generate_token(
                        if element_size == 1 {
                            ParseMarker::Word(i)
                        } else {
                            ParseMarker::Tuple(Location {
                                start: i,
                                end: i + element_size,
                            })
                        },
                        data_to_parse.clone(),
                        disallowed_markers.clone(),
                        true,
                    )?
                    .to_token(),
                );
                i += element_size;
            }
            let result = Token::Array(parse_tree);
            Some(TokenOrTopLevel::Token(result))
        }
        ParseMarker::DynamicOffset(_, ref location) => {
            if disallowed_markers.contains_key(&0) && disallowed_markers[&0] == MarkerType::Tuple {
                return None;
            }
            let data_to_parse = chunks[location.start..location.end + 1].to_vec();
            let mut tokens = Vec::new();
            let parse_markers =
                generate_parse_markers(disallowed_markers.clone(), data_to_parse.clone(), true);
            let mut new_disallowed_markers = disallowed_markers.clone();

            for cur_parse_marker in &parse_markers {
                let result = generate_token(
                    cur_parse_marker.clone(),
                    data_to_parse.clone(),
                    disallowed_markers.clone(),
                    true,
                );

                // println!(
                //     "Parsemarker result: {:?}",
                //     result
                // );
                if result.is_some() {
                    tokens.push(result.unwrap().to_token());
                } else {
                    if recurse_disallow_markers {
                        add_disallowed_marker(&mut new_disallowed_markers, &cur_parse_marker)
                            .ok()?;
                        // println!("Recursing from dynamic offset loop");
                        return generate_token(
                            parse_marker.clone(),
                            chunks.clone(),
                            new_disallowed_markers.clone(),
                            true,
                        );
                    } else {
                        // println!("Return None for dynamic offset loop");
                        return None;
                    }
                }
            }
            // println!(
            //     "Recursing on dynamic offset with disallowed_markers: {:?}",
            //     disallowed_markers.len()
            // );
            strip_invalid_tokens(
                &parse_marker,
                &parse_markers,
                TokenOrTopLevel::Token(Token::Tuple(tokens.clone())),
                tokens,
                &disallowed_markers,
                &chunks,
                recurse_disallow_markers,
            )
        }
        ParseMarker::DynamicArray(_, ref locations) => {
            let mut tokens = Vec::new();
            let data_to_parse = chunks[1..].to_vec();

            let mut new_disallowed_markers = disallowed_markers.clone();
            let parse_markers: Vec<ParseMarker> = locations
                .iter()
                .enumerate()
                .map(|e| ParseMarker::DynamicOffset(e.0, e.1.clone()))
                .collect();

            for cur_parse_marker in &parse_markers {
                let result = generate_token(
                    cur_parse_marker.clone(),
                    data_to_parse.clone(),
                    disallowed_markers.clone(),
                    true,
                );

                // println!(
                //     "Parsemarker result: {:?}",
                //     result
                // );
                if result.is_some() {
                    tokens.push(result.unwrap().to_token());
                } else {
                    if recurse_disallow_markers {
                        // println!("Disallowed markers before: {:?}", new_disallowed_markers);
                        add_disallowed_marker(&mut new_disallowed_markers, &cur_parse_marker)
                            .ok()?;
                        // println!(
                        //     "Recursing from dynamic array loop: {:?}",
                        //     new_disallowed_markers
                        // );
                        return generate_token(
                            parse_marker.clone(),
                            chunks.clone(),
                            new_disallowed_markers.clone(),
                            recurse_disallow_markers,
                        );
                    } else {
                        // println!("Return None for dynamic offset loop");
                        return None;
                    }
                }
            }
            // println!(
            //     "Recursing on dynamic array with disallowed_markers: {:?}",
            //     disallowed_markers.len()
            // );
            strip_invalid_tokens(
                &parse_marker,
                &parse_markers,
                TokenOrTopLevel::Token(Token::Array(tokens.clone())),
                tokens,
                &disallowed_markers,
                &chunks,
                recurse_disallow_markers,
            )
        }
        ParseMarker::TopLevel => {
            let mut tokens = Vec::new();

            // println!(
            //     "Recursing on top level with disallowed_markers: {:?}",
            //     disallowed_markers
            // );
            let parse_markers =
                generate_parse_markers(disallowed_markers.clone(), chunks.clone(), false);
            let mut new_disallowed_markers = disallowed_markers.clone();
            // println!(
            //     "Looping over top level parse markers: {:?}",
            //     parse_markers
            // );
            for parse_marker in parse_markers.clone() {
                // println!(
                //     "Loop iteration parse marker: {:?}",
                //     parse_marker
                // );
                let result = generate_token(
                    parse_marker.clone(),
                    chunks.clone(),
                    disallowed_markers.clone(),
                    true,
                );

                // println!(
                //     "Parsemarker result: {:?}",
                //     result
                // );
                if result.is_some() {
                    tokens.push(result.unwrap().to_token());
                } else {
                    if recurse_disallow_markers {
                        add_disallowed_marker(&mut new_disallowed_markers, &parse_marker).ok()?;
                        // println!("Recursing from top level loop");
                        return generate_token(
                            ParseMarker::TopLevel,
                            chunks.clone(),
                            new_disallowed_markers.clone(),
                            recurse_disallow_markers,
                        );
                    } else {
                        return None;
                    }
                }
            }

            // println!("Stripping invalid tokens for top level: ");
            strip_invalid_tokens(
                &parse_marker,
                &parse_markers,
                TokenOrTopLevel::TopLevel(tokens.clone()),
                tokens,
                &disallowed_markers,
                &chunks,
                recurse_disallow_markers,
            )
        }
    };
    // println!("Finished parsing {:?}", parse_marker);
    // println!("with disallowed_markers {:?}", disallowed_markers);
    // println!("Result: {:?}", result);
    // println!("");
    return result;
}

fn add_disallowed_marker(
    disallowed_markers: &mut HashMap<usize, MarkerType>,
    parse_marker: &ParseMarker,
) -> Result<(), String> {
    let index = get_index(parse_marker);
    let marker_to_add = match parse_marker {
        ParseMarker::DynamicOffset(..) | ParseMarker::Tuple(..) => MarkerType::Tuple,
        ParseMarker::DynamicArray(..) => MarkerType::DynamicArray,
        ParseMarker::StaticArray(..) => MarkerType::Array,
        ParseMarker::DynamicBytes(..) => MarkerType::DynamicBytes,
        _ => {
            panic!("Cannot add disallowed marker for {:?}", parse_marker);
        }
    };
    if disallowed_markers.contains_key(&index) {
        return Err(format!(
            "Disallowed marker already exists for index {}",
            index
        ));
    }
    disallowed_markers.insert(index, marker_to_add);
    Ok(())
}

fn strip_invalid_tokens(
    parse_marker: &ParseMarker,
    parse_markers: &Vec<ParseMarker>,
    token: TokenOrTopLevel,
    mut tokens: Vec<Token>,
    disallowed_markers: &HashMap<usize, MarkerType>,
    data_to_parse: &Vec<&str>,
    recurse_disallow_markers: bool,
) -> Option<TokenOrTopLevel> {
    // println!("Strip invalid tokens - token: {:?}", token);
    let invalid_token_markers = get_invalid_token_markers(&parse_markers, &tokens);
    // println!("invalid_token_markers: {:?}", invalid_token_markers);
    if invalid_token_markers.len() > 0 {
        // println!("Invalid token in parse_marker: {:?}", parse_marker);
        // println!("Invalid token markers {:?}", invalid_token_markers);
        // println!("disallowed_markers: {:?}", disallowed_markers);
        if recurse_disallow_markers {
            let result = rerun_with_invalid_token_markers(
                &parse_marker,
                &invalid_token_markers,
                &disallowed_markers,
                &data_to_parse,
                false,
            );

            if result.is_some() {
                return result;
            } else {
                let result = rerun_with_invalid_token_markers(
                    &parse_marker,
                    &invalid_token_markers,
                    &disallowed_markers,
                    &data_to_parse,
                    true,
                );
                if result.is_some() {
                    return result;
                } else {
                    return None;
                }
            }
        } else {
            return None;
        }
    } else {
        match token {
            TokenOrTopLevel::TopLevel(_) => {
                tokens = tokens
                    .iter()
                    .map(|e| remove_single_element_tuples((*e).clone()))
                    .collect();

                return Some(TokenOrTopLevel::TopLevel(tokens));
            }
            _ => {
                return Some(token);
            }
        }
    }
}

fn rerun_with_invalid_token_markers(
    parse_marker: &ParseMarker,
    invalid_token_markers: &Vec<(usize, MarkerType)>,
    disallowed_markers: &HashMap<usize, MarkerType>,
    data_to_parse: &Vec<&str>,
    recurse_disallow_markers: bool,
) -> Option<TokenOrTopLevel> {
    // println!("rerun_with_invalid_token_markers({:?}, {:?}, {:?}, {:?}, {:?})", parse_marker, invalid_token_markers, disallowed_markers, data_to_parse, recurse_disallow_markers);
    for invalid_token_marker in invalid_token_markers {
        if !disallowed_markers.contains_key(&invalid_token_marker.0)
            || invalid_token_marker.1 != disallowed_markers[&invalid_token_marker.0]
        {
            let mut new_disallowed_markers = disallowed_markers.clone();
            new_disallowed_markers.insert(invalid_token_marker.0, invalid_token_marker.1.clone());
            let new_result = generate_token(
                parse_marker.clone(),
                data_to_parse.clone(),
                new_disallowed_markers,
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
    parse_markers: &Vec<ParseMarker>,
    tokens: &Vec<Token>,
) -> Vec<(usize, MarkerType)> {
    // println!("Length parse_markers: {:?}", parse_markers.len());
    // println!("Length tokens: {:?}", tokens.len());
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

pub fn get_index(marker: &ParseMarker) -> usize {
    match marker {
        ParseMarker::Word(location) => *location,
        ParseMarker::Tuple(location) => location.start - 1,
        ParseMarker::DynamicBytes(_, location) => location.start - 1,
        ParseMarker::StaticArray(element_size, location) => location.start - 1,
        ParseMarker::DynamicOffset(i, _) => *i,
        ParseMarker::DynamicArray(i, _) => *i,
        ParseMarker::TopLevel => {
            panic!("TopLevel marker should not be used");
        }
    }
}

pub fn check_token(token: &Token, parse_marker: &ParseMarker) -> Option<MarkerType> {
    // println!("Checking token: {:?}", token);
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

pub fn get_dynamic_offset_marker(
    parse_markers: &Vec<ParseMarker>,
    i: usize,
    chunks: &Vec<&str>,
    most_recent_tuple_offset: usize,
    data_length: usize,
    disallowed_markers: &HashMap<usize, MarkerType>,
) -> Option<(usize, usize, ParseMarker)> {
    if disallowed_markers.contains_key(&i) && disallowed_markers[&i] == MarkerType::Tuple {
        return None;
    }
    let chunk = chunks[i];
    let offset = get_dynamic_offset(0, i, chunk, most_recent_tuple_offset, data_length)?;

    let tuple_offset = offset;

    let tuple_location = parse_markers.len();
    let parse_marker = ParseMarker::DynamicOffset(
        i,
        Location {
            start: offset,
            end: data_length,
        },
    );

    return Some((tuple_offset, tuple_location, parse_marker));
}

pub fn generate_parse_markers(
    disallowed_markers: HashMap<usize, MarkerType>,
    chunks: Vec<&str>,
    in_dynamic_offset: bool,
) -> Vec<ParseMarker> {
    // println!("Generating parse markers: {:?}", chunks);
    let mut parse_markers: Vec<ParseMarker> = Vec::new();
    let mut most_recent_tuple_offset: usize = 0;
    let mut most_recent_tuple_location: Option<usize> = None;
    let data_length = chunks.len() - 1;
    let mut first_tuple = <usize>::max_value();
    let mut i = 0;

    while i <= data_length && i < first_tuple {
        // println!("LOOP i: {:?}", i);
        if let Some((tuple_offset, tuple_location, parse_marker)) = get_dynamic_offset_marker(
            &parse_markers,
            i,
            &chunks,
            most_recent_tuple_offset,
            data_length,
            &disallowed_markers,
        ) {
            update_tuple_variables(
                &mut parse_markers,
                tuple_offset,
                tuple_location,
                &mut most_recent_tuple_offset,
                &mut most_recent_tuple_location,
                &mut first_tuple,
            );
            parse_markers.push(parse_marker);
            i += 1;
        } else if let Some(bytes_marker) = get_dynamic_bytes_marker(
            i,
            &chunks,
            data_length,
            &mut first_tuple,
            in_dynamic_offset && i == 0,
        ) {
            if let ParseMarker::DynamicBytes(ref padding, ref location) = bytes_marker {
                // println!("Dynamic bytes marker at location: {:?} - with padding: {:?}", location, padding);
                i = location.end;
                parse_markers.push(bytes_marker);
            } else {
                panic!("Invalid bytes marker");
            }
        } else if let Some(array_marker) = get_array_marker(
            &parse_markers,
            i,
            &chunks,
            data_length,
            &mut most_recent_tuple_offset,
            &mut most_recent_tuple_location,
            &mut first_tuple,
            in_dynamic_offset && i == 0,
        ) {
            if let ParseMarker::StaticArray(element_size, ref location) = array_marker {
                i = location.end;
                parse_markers.push(array_marker);
            } else if let ParseMarker::DynamicArray(_, ref locations) = array_marker {
                i = locations[locations.len() - 1].end;
                parse_markers.push(array_marker);
            } else {
                panic!("Invalid array marker");
            }
        } else {
            parse_markers.push(ParseMarker::Word(i));
            i += 1;
        }
    }
    // println!("");
    // println!("##############################");
    // println!("Parse markers: {:?}", parse_markers);
    // println!("Disallowed marker: {:?}", disallowed_markers);
    return parse_markers;
}

fn update_tuple_variables(
    parse_markers: &mut Vec<ParseMarker>,
    tuple_offset: usize,
    tuple_location: usize,
    most_recent_tuple_offset: &mut usize,
    most_recent_tuple_location: &mut Option<usize>,
    first_tuple: &mut usize,
) {
    update_tuple_location(parse_markers, most_recent_tuple_location, tuple_offset - 1);

    if *first_tuple == <usize>::max_value() {
        *first_tuple = tuple_offset;
    }

    *most_recent_tuple_offset = tuple_offset;
    *most_recent_tuple_location = Some(tuple_location);
}

fn update_tuple_location(
    parse_markers: &mut Vec<ParseMarker>,
    most_recent_tuple_location: &mut Option<usize>,
    end: usize,
) {
    if let Some(location) = most_recent_tuple_location {
        match parse_markers[*location] {
            ParseMarker::DynamicOffset(_, ref mut loc) => {
                loc.end = end;
            }
            ParseMarker::DynamicArray(_, ref mut locs) => {
                let length = locs.len();
                if length == 0 {
                    panic!("Invalid parse marker for previous tuple in dynamic array");
                }
                locs[length - 1].end = end;
            }

            _ => {
                panic!("Invalid parse marker for previous tuple")
            }
        }
    }
}

fn get_dynamic_offset(
    ref_point: usize,
    i: usize,
    chunk: &str,
    most_recent_tuple: usize,
    data_length: usize,
) -> Option<usize> {
    if U256::from_str(&chunk).unwrap() > U256::from(data_length) * U256::from(32) {
        return None;
    }

    let decoded_num = U256::from_str(&chunk).unwrap().as_usize();

    if decoded_num % 32 != 0 {
        return None;
    }
    let offset = decoded_num / 32 + ref_point;
    if offset <= most_recent_tuple {
        return None;
    }
    if offset <= i {
        return None;
    }
    return Some(offset);
}

fn get_array_marker(
    parse_markers: &Vec<ParseMarker>,
    i: usize,
    chunks: &Vec<&str>,
    data_length: usize,
    most_recent_tuple_offset: &mut usize,
    most_recent_tuple_location: &mut Option<usize>,
    first_tuple: &mut usize,
    is_first_element_in_dynamic_offset: bool,
) -> Option<ParseMarker> {
    if !is_first_element_in_dynamic_offset {
        return None;
    }
    if let Some(marker) = get_array_marker_dynamic(
        parse_markers,
        i,
        chunks,
        data_length,
        most_recent_tuple_offset,
        most_recent_tuple_location,
        first_tuple,
    ) {
        return Some(marker);
    } else {
        return get_array_marker_static(i, chunks, data_length, first_tuple);
    }
}

fn get_dynamic_bytes_marker(
    i: usize,
    chunks: &Vec<&str>,
    data_length: usize,
    first_tuple: &mut usize,
    first_element_in_dynamic_offset: bool,
) -> Option<ParseMarker> {
    if !first_element_in_dynamic_offset {
        return None;
    }
    let remaining_data_length = std::cmp::min(data_length, *first_tuple - 1) - i;
    let raw_length = U256::from_str(&chunks[i]).ok()?;
    if raw_length > U256::from(<usize>::max_value()) {
        return None;
    }

    let parsed_length = raw_length.as_usize();

    // For zero length we prefer empty array over empty bytes
    // TODO: Review
    if parsed_length == 0 {
        return None;
    }

    let mut length_words = parsed_length / 32;
    if parsed_length % 32 != 0 {
        length_words += 1;
    }
    let padding = length_words * 32 - parsed_length;
    // println!("parsed_length = {}, length_words = {}, padding = {}", parsed_length, length_words, padding);

    if length_words + i != remaining_data_length {
        // println!("Invalid dynamic bytes marker");
        return None;
    }

    let last_word = chunks[i + length_words];
    let padding_bytes = &last_word[64 - padding * 2..];
    // println!("Checking padding_bytes = {}", padding_bytes);
    if padding_bytes != "0".repeat(padding * 2) {
        // println!("Padding contains non zero element cant be dynamic bytes");
        return None;
    }

    Some(ParseMarker::DynamicBytes(
        padding,
        Location {
            start: i + 1,
            end: i + 1 + length_words,
        },
    ))
}

fn get_array_marker_static(
    i: usize,
    chunks: &Vec<&str>,
    data_length: usize,
    first_tuple: &mut usize,
) -> Option<ParseMarker> {
    let (length, element_size) = get_array_length(i, chunks[i], data_length, false, first_tuple)?;

    // If length is zero static / dynamic arrays are the same
    if length == 0 {
        return Some(ParseMarker::StaticArray(
            0,
            Location {
                start: i + 1,
                end: i + 1,
            },
        ));
    }

    let marker = ParseMarker::StaticArray(
        element_size,
        Location {
            start: i + 1,
            end: i + length * element_size + 1,
        },
    );
    // println!("length: {:} - element_size: {:}", length, element_size);
    // println!("Static array marker = {:?}", marker);
    Some(marker)
}

fn get_array_marker_dynamic(
    parse_markers: &Vec<ParseMarker>,
    i: usize,
    chunks: &Vec<&str>,
    data_length: usize,
    most_recent_tuple_offset: &mut usize,
    most_recent_tuple_location: &mut Option<usize>,
    first_tuple: &mut usize,
) -> Option<ParseMarker> {
    let (length, _) = get_array_length(i, chunks[i], data_length, true, first_tuple)?;

    // If length is zero static / dynamic arrays are the same
    if length == 0 {
        // Make sure the array consumes all of its space
        let remaining_data_length = std::cmp::min(data_length, *first_tuple) - i;
        if length != remaining_data_length {
            return None;
        }
        return Some(ParseMarker::StaticArray(
            0,
            Location {
                start: i + 1,
                end: i + 1,
            },
        ));
    }

    let mut parse_marker = None;
    let mut most_recent_tuple_offset_copy = (*most_recent_tuple_offset).clone();
    let mut most_recent_tuple_location_copy = (*most_recent_tuple_location).clone();
    let mut first_tuple_copy = (*first_tuple).clone();
    let mut parse_markers_copy = (*parse_markers).clone();
    let limited_chunks = &chunks[i + 1..data_length].to_vec();
    if limited_chunks.len() == 0 {
        return None;
    }
    for j in 0..length {
        if let Some((tuple_offset, tuple_location, tuple_parse_marker)) = get_dynamic_offset_marker(
            &parse_markers_copy,
            j,
            limited_chunks,
            most_recent_tuple_offset_copy,
            data_length - i - 1,
            &HashMap::new(),
        ) {
            if let ParseMarker::DynamicOffset(_, ref location) = tuple_parse_marker {
                if j == 0 && location.start != i + length {
                    // println!("Invalid start location of first tuple in dynamic array");
                    // println!("location: {:?}", location);
                    // println!("length: {}", length);
                    // println!("i: {}", i);
                    return None;
                }
                // println!("Adding parse marker: {:?}", tuple_parse_marker);
                parse_markers_copy.push(tuple_parse_marker);
                update_tuple_variables(
                    &mut parse_markers_copy,
                    tuple_offset,
                    tuple_location,
                    &mut most_recent_tuple_offset_copy,
                    &mut most_recent_tuple_location_copy,
                    &mut first_tuple_copy,
                );
            } else {
                panic!("Invalid tuple marker");
            }
            if j == length - 1 {
                // At the end the dynamic array should fill up all of the space until the first
                // tuple
                let remaining_data_length = std::cmp::min(data_length, first_tuple_copy) - i;
                if length != remaining_data_length {
                    return None;
                }
                let mut locations = Vec::new();
                for marker in parse_markers_copy.iter().skip(parse_markers.len()) {
                    if let ParseMarker::DynamicOffset(j, ref location) = marker {
                        locations.push(location.clone());
                    } else {
                        panic!("Invalid tuple marker");
                    }
                }
                parse_marker = Some(ParseMarker::DynamicArray(j, locations));
                *most_recent_tuple_location = most_recent_tuple_location_copy;
                *most_recent_tuple_offset = most_recent_tuple_offset_copy;
                *first_tuple = first_tuple_copy;
                break;
            }
        } else {
            break;
        }
    }
    return parse_marker;
}

fn get_array_length(
    i: usize,
    chunk: &str,
    data_length: usize,
    is_dynamic: bool,
    first_tuple: &mut usize,
) -> Option<(usize, usize)> {
    if U256::from_str(&chunk).unwrap() + U256::from(i) > U256::from(data_length) {
        return None;
    }
    let raw_length = U256::from_str(&chunk).unwrap().as_usize();

    // TODO: Excluding single element arrays of static content
    if raw_length == 1 && !is_dynamic {
        // println!("Excluding single element arrays of static content - {:}", i);
        return None;
    }

    let remaining_data_length = std::cmp::min(data_length, *first_tuple - 1) - i;
    // println!("remaining_data_length: {:?}", remaining_data_length);
    if raw_length > remaining_data_length {
        return None;
    }

    if is_dynamic {
        // TODO: Add check that the first tuple in the dynamic array is after the last index
        // println!("is_dynamic: {:?}", (raw_length, 1));
        return Some((raw_length, 1));
    }

    if raw_length == 1 {
        // println!("raw_length == 1 (length, size): {:?}", (1, remaining_data_length));
        return Some((1, remaining_data_length));
    }

    for j in (0..((remaining_data_length / 2) + 1)).rev() {
        if raw_length * j == remaining_data_length {
            // println!("calculated in for loop (length, size): {:?}", (raw_length, j));
            return Some((raw_length, j));
        }
    }
    // println!(
    //     "Cannot generate static array - raw_length: {:} - remaining_data_length: {:} - data_length: {:}, first_tuple: {:}",
    //     raw_length, remaining_data_length, data_length, first_tuple
    // );
    None
}

pub fn tokenize_argument(argument: &str) -> Token {
    let trimmed_argument = argument.trim_start_matches("0");

    // If word does not start with a 0 then assume it is a left aligned type (bytes)
    if trimmed_argument.len() == argument.len() {
        let mut right_trimmed_argument = argument.trim_end_matches("0").to_owned();
        if right_trimmed_argument.len() % 2 == 1 {
            right_trimmed_argument.push_str("0");
        }
        let bytes_len = right_trimmed_argument.len() / 2;

        // println!("right_trimmed_argument length: {:}", right_trimmed_argument.len());
        // println!("bytes-len: {:}", bytes_len);

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
        return token;
    } else {
        panic!("Could not tokenize argument: {}", argument);
    }
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
        test_same_encoding,
        [
            (
                address_bytes_and_uint256,
                vec![
                    Token::Address(
                        H160::from_str("0x7C07F7aBe10CE8e33DC6C5aD68FE033085256A84").unwrap()
                    ),
                    Token::FixedBytes(
                        hex::decode("7C07F7aBe10CE8e33DC6C5aD68FE033085256A").unwrap()
                    ),
                    Token::Uint(U256::from(100)),
                ]
            ),
            (
                address_and_uint256,
                vec![
                    Token::Address(
                        H160::from_str("0x7C07F7aBe10CE8e33DC6C5aD68FE033085256A84").unwrap()
                    ),
                    Token::Uint(U256::from(100)),
                ]
            ),
            (
                uint256_and_address,
                vec![
                    Token::Uint(U256::from(100)),
                    Token::Address(
                        H160::from_str("0x7C07F7aBe10CE8e33DC6C5aD68FE033085256A84").unwrap()
                    ),
                ]
            ),
            (
                uint256_array,
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
                    Token::Address(
                        H160::from_str("0x7C07F7aBe10CE8e33DC6C5aD68FE033085256A84").unwrap()
                    ),
                    Token::Uint(U256::from(10000000000_u64)),
                    Token::Tuple(vec![
                        Token::Array(vec![
                            Token::Address(
                                H160::from_str("0x7C07F7aBe10CE8e33DC6C5aD68FE033085256A84")
                                    .unwrap()
                            ),
                            Token::Address(
                                H160::from_str("0x7C07F7aBe10CE8e33DC6C5aD68FE033085256A84")
                                    .unwrap()
                            ),
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
        [(
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
            utils::print_parse_tree(&argument, 0);
        }
        println!("");
        println!("");
        let encoded_arguments = add_padding(&hex::encode(ethabi::encode(&arguments)));

        let chunks = chunk_data(&encoded_arguments);
        for (i, chunk) in chunks.iter().enumerate() {
            println!(
                "{}: {} - {}",
                i,
                chunk,
                u64::from_str_radix(chunk.trim_start_matches("0"), 16).unwrap_or(0)
            );
        }
        let tokens = decode_chunks(chunks.clone());
        for token in &tokens {
            utils::print_parse_tree(&token, 0);
        }
        assert_eq!(tokens, arguments);
    }

    fn test_can_reencode_with_added_data_at_the_end(arguments: Vec<Token>) {
        println!("Arguments:");
        for argument in &arguments {
            utils::print_parse_tree(&argument, 0);
        }
        println!("");
        println!("");
        let encoded_arguments = add_padding(&hex::encode(ethabi::encode(&arguments)));

        let mut chunks = chunk_data(&encoded_arguments);
        let extra_data = &"01".repeat(32);
        chunks.push(extra_data);
        for (i, chunk) in chunks.iter().enumerate() {
            println!(
                "{}: {} - {}",
                i,
                chunk,
                u64::from_str_radix(chunk.trim_start_matches("0"), 16).unwrap_or(0)
            );
        }
        let tokens = decode_chunks(chunks.clone());
        for token in &tokens {
            utils::print_parse_tree(&token, 0);
        }
        // assert_eq!(tokens, arguments);
    }

    fn test_different_encoding(arguments_and_expected_tokens: (Vec<Token>, Vec<Token>)) {
        let (arguments, expected_tokens) = arguments_and_expected_tokens;
        for argument in &arguments {
            utils::print_parse_tree(&argument, 0);
        }
        println!("");
        println!("");
        let encoded_arguments = add_padding(&hex::encode(ethabi::encode(&arguments)));

        let chunks = chunk_data(&encoded_arguments);
        for (i, chunk) in chunks.iter().enumerate() {
            println!(
                "{}: {} - {}",
                i,
                chunk,
                u64::from_str_radix(chunk.trim_start_matches("0"), 16).unwrap_or(0)
            );
        }
        let tokens = decode_chunks(chunks.clone());
        for token in &tokens {
            utils::print_parse_tree(&token, 0);
        }
        assert_eq!(tokens, expected_tokens);
    }
}
