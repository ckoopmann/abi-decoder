use ethereum_types::U256;
use std::collections::HashMap;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct Location {
    pub start: usize,
    pub end: usize,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarkerType {
    Array,
    Tuple,
    DynamicArray,
    DynamicBytes,
}

pub fn add_disallowed_marker(
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

pub fn get_index(marker: &ParseMarker) -> usize {
    match marker {
        ParseMarker::Word(location) => *location,
        ParseMarker::Tuple(location) => location.start - 1,
        ParseMarker::DynamicBytes(_, location) => location.start - 1,
        ParseMarker::StaticArray(_element_size, location) => location.start - 1,
        ParseMarker::DynamicOffset(i, _) => *i,
        ParseMarker::DynamicArray(i, _) => *i,
        ParseMarker::TopLevel => {
            panic!("TopLevel marker should not be used");
        }
    }
}

fn get_dynamic_offset_marker(
    parse_markers: &[ParseMarker],
    i: usize,
    chunks: &[&str],
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

    Some((tuple_offset, tuple_location, parse_marker))
}

pub fn generate_parse_markers(
    parent_marker: &ParseMarker,
    disallowed_markers: HashMap<usize, MarkerType>,
    chunks: &[&str],
    in_dynamic_offset: bool,
) -> Vec<ParseMarker> {
    match parent_marker {
        ParseMarker::DynamicArray(_, locations) => locations
            .iter()
            .enumerate()
            .map(|e| ParseMarker::DynamicOffset(e.0, e.1.clone()))
            .collect(),
        _ => {
            let mut parse_markers: Vec<ParseMarker> = Vec::new();
            let mut most_recent_tuple_offset: usize = 0;
            let mut most_recent_tuple_location: Option<usize> = None;
            let data_length = chunks.len() - 1;
            let mut first_tuple = <usize>::max_value();
            let mut i = 0;

            while i <= data_length && i < first_tuple {
                if let Some((tuple_offset, tuple_location, parse_marker)) =
                    get_dynamic_offset_marker(
                        &parse_markers,
                        i,
                        chunks,
                        most_recent_tuple_offset,
                        data_length,
                        &disallowed_markers,
                    )
                {
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
                    chunks,
                    data_length,
                    &mut first_tuple,
                    in_dynamic_offset && i == 0,
                ) {
                    if let ParseMarker::DynamicBytes(ref _padding, ref location) = bytes_marker {
                        i = location.end;
                        parse_markers.push(bytes_marker);
                    } else {
                        panic!("Invalid bytes marker");
                    }
                } else if let Some(array_marker) = get_array_marker(
                    &parse_markers,
                    i,
                    chunks,
                    data_length,
                    &mut most_recent_tuple_offset,
                    &mut most_recent_tuple_location,
                    &mut first_tuple,
                    in_dynamic_offset && i == 0,
                ) {
                    if let ParseMarker::StaticArray(_element_size, ref location) = array_marker {
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
            parse_markers
        }
    }
}

fn update_tuple_variables(
    parse_markers: &mut [ParseMarker],
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
    parse_markers: &mut [ParseMarker],
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
    if U256::from_str(chunk).unwrap() > U256::from(data_length) * U256::from(32) {
        return None;
    }

    let decoded_num = U256::from_str(chunk).unwrap().as_usize();

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
    Some(offset)
}

#[allow(clippy::too_many_arguments)]
fn get_array_marker(
    parse_markers: &Vec<ParseMarker>,
    i: usize,
    chunks: &[&str],
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
        Some(marker)
    } else {
        get_array_marker_static(i, chunks, data_length, first_tuple)
    }
}

fn get_dynamic_bytes_marker(
    i: usize,
    chunks: &[&str],
    data_length: usize,
    first_tuple: &mut usize,
    first_element_in_dynamic_offset: bool,
) -> Option<ParseMarker> {
    if !first_element_in_dynamic_offset {
        return None;
    }
    let remaining_data_length = std::cmp::min(data_length, *first_tuple - 1) - i;
    let raw_length = U256::from_str(chunks[i]).ok()?;
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

    if length_words + i != remaining_data_length {
        return None;
    }

    let last_word = chunks[i + length_words];
    let padding_bytes = &last_word[64 - padding * 2..];
    if padding_bytes != "0".repeat(padding * 2) {
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
    chunks: &[&str],
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
    Some(marker)
}

fn get_array_marker_dynamic(
    parse_markers: &Vec<ParseMarker>,
    i: usize,
    chunks: &[&str],
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
    let mut most_recent_tuple_offset_copy = *most_recent_tuple_offset;
    let mut most_recent_tuple_location_copy = *most_recent_tuple_location;
    let mut first_tuple_copy = *first_tuple;
    let mut parse_markers_copy = (*parse_markers).clone();
    let limited_chunks = &chunks[i + 1..data_length].to_vec();
    if limited_chunks.is_empty() {
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
                    return None;
                }
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
                    if let ParseMarker::DynamicOffset(_j, ref location) = marker {
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
    parse_marker
}

fn get_array_length(
    i: usize,
    chunk: &str,
    data_length: usize,
    is_dynamic: bool,
    first_tuple: &mut usize,
) -> Option<(usize, usize)> {
    if U256::from_str(chunk).unwrap() + U256::from(i) > U256::from(data_length) {
        return None;
    }
    let raw_length = U256::from_str(chunk).unwrap().as_usize();

    // TODO: Excluding single element arrays of static content
    if raw_length == 1 && !is_dynamic {
        return None;
    }

    let remaining_data_length = std::cmp::min(data_length, *first_tuple - 1) - i;
    if raw_length > remaining_data_length {
        return None;
    }

    if is_dynamic {
        // TODO: Add check that the first tuple in the dynamic array is after the last index
        return Some((raw_length, 1));
    }

    if raw_length == 1 {
        return Some((1, remaining_data_length));
    }

    for j in (0..((remaining_data_length / 2) + 1)).rev() {
        if raw_length * j == remaining_data_length {
            return Some((raw_length, j));
        }
    }
    None
}
