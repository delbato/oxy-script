use std::{
    collections::{
        HashMap
    }
};

use crate::{
    api::{
        function::{
            Function
        }
    }
};

pub struct Module {
    pub name: String,
    pub functions: HashMap<String, Function>,
    pub modules: HashMap<String, Module>
}

impl Module {
    pub fn new<T>(name: T) -> Module
    where String: From<T> {
        let name = String::from(name);
        Module {
            name: name,
            functions: HashMap::new(),
            modules: HashMap::new()
        }
    }

    pub fn with_function(mut self, function: Function) -> Module {
        self.functions.insert(function.name.clone(), function);
        self
    }

    pub fn with_module(mut self, module: Module) -> Module {
        self.modules.insert(module.name.clone(), module);
        self
    }
}