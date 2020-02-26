use crate::{
    api::{
        function::Function
    },
};

use std::{
    collections::{
        BTreeMap,
        HashMap
    },
    ops::Range
};

#[derive(PartialEq, Debug)]
pub struct Program {
    pub code: Vec<u8>,
    pub functions: HashMap<u64, usize>,
    pub foreign_functions: HashMap<u64, Function>,
    pub static_pointers: BTreeMap<usize, Range<usize>> 
}

impl Program {
    pub fn new() -> Program {
        Program {
            code: Vec::new(),
            functions: HashMap::new(),
            foreign_functions: HashMap::new(),
            static_pointers: BTreeMap::new() 
        }
    }

    pub fn with_code(mut self, code: Vec<u8>) -> Program {
        self.code = code;
        self
    }

    pub fn with_functions(mut self, functions: HashMap<u64, usize>) -> Program {
        self.functions = functions;
        self
    }

    pub fn with_foreign_functions(mut self, functions: HashMap<u64, Function>) -> Program {
        self.foreign_functions = functions;
        self
    }

    pub fn with_static_pointers(mut self, static_pointers: BTreeMap<usize, Range<usize>>) -> Program {
        self.static_pointers =static_pointers;
        self
    }

    pub fn get_size(&self) -> usize {
        self.code.len()
    }
}