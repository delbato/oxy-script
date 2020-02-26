use std::{
    collections::{
        HashSet,
        HashMap
    }
};

use rand::{
    RngCore,
    thread_rng
};

/// Convenience struct for generating unique u64s
pub struct UIDGenerator {
    uid_set: HashSet<u64>,
    functions: HashMap<String, u64>,
}

impl UIDGenerator {
    pub fn new() -> UIDGenerator {
        UIDGenerator {
            uid_set: HashSet::new(),
            functions: HashMap::new()
        }
    }

    pub fn generate(&mut self) -> u64 {
        let mut rng = thread_rng();
        let mut uid = rng.next_u64();
        while self.uid_set.contains(&uid) {
            uid = rng.next_u64();
        }
        self.uid_set.insert(uid);
        uid
    }

    pub fn get_function_uid(&mut self, name: &String) -> u64 {
        if self.functions.contains_key(name) {
            let uid = self.functions.get(name).unwrap();
            return *uid;
        }
        let uid = self.generate();
        self.functions.insert(name.clone(), uid);
        uid
    }
}