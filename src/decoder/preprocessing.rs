// Adds padding to the end of the data if it contains trailing bytes
pub fn add_padding(encoded_data: &str) -> String {
    let mut encoded_data = encoded_data.to_string();
    if encoded_data.len() % 64 != 0 {
        let padding = 64 - (encoded_data.len() % 64);
        encoded_data.push_str(&"0".repeat(padding));
    }
    encoded_data
}

pub fn chunk_data(encoded_data: &str) -> Vec<&str> {
    let mut encoded_data = encoded_data;
    if encoded_data.len() % 64 != 0 {
        panic!("Invalid data length");
    }
    let mut chunks = Vec::new();
    while !encoded_data.is_empty() {
        let (word, rest) = encoded_data.split_at(64);
        chunks.push(word);
        encoded_data = rest;
    }
    chunks
}
