use std::{
    collections::{
        HashMap
    }
};

/// Manager struct for static data
#[derive(Clone)]
pub struct Data {
    pub bytes: Vec<u8>,
    strings: HashMap<String, usize>
}

impl Data {
    /// Creates a new Data instance
    pub fn new() -> Data {
        Self {
            bytes: Vec::new(),
            strings: HashMap::new()
        }
    }

    pub fn get_string_slice(&mut self, string: &String) -> (u64, u64) {
        if self.strings.contains_key(string) {
            let byte_len = string.as_bytes().len() as u64;
            let addr = *self.strings.get(string).unwrap() as u64;
            return (byte_len, addr);
        }
        let bytes = string.as_bytes();
        let byte_len = bytes.len() as u64;
        let addr = self.bytes.len();
        self.bytes.extend_from_slice(bytes);
        self.strings.insert(string.clone(), addr);
        (byte_len, addr as u64)
    }
}