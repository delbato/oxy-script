use crate::{
    api::{
        module::{
            Module
        },
        adapter::{
            Adapter
        }
    },
    parser::{
        ast::{
            Type
        }
    }
};

use std::{
    collections::{
        HashMap
    },
    ops::{
        FnMut,
        DerefMut
    },
    cmp::{
        PartialEq
    },
    fmt::{
        Formatter,
        Result as FmtResult,
        Debug
    },
    clone::{
        Clone
    },
    sync::{
        Arc,
        Mutex
    }
};

/// Represents a foreign function
#[derive(Clone)]
pub struct Function {
    /// Name of this function
    pub name: String,
    /// Type signature
    pub arg_types: Vec<Type>,
    arg_offsets: HashMap<usize, i64>,
    arg_sizes: HashMap<usize, usize>,
    /// Return type
    pub return_type: Type,
    closure: Option<Arc<Mutex<FunctionClosureType>>>
}

impl Debug for Function {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "Function ({:?},{:?},{:?},{:?})", self.name, self.arg_types, self.arg_offsets, self.return_type)
    }
}

impl PartialEq for Function {
    fn eq(&self, rhs: &Function) -> bool {
        self.name == rhs.name
    }
}
/*
impl Clone for Function {
    fn clone(&self) -> Function {
        Function {
            name: self.name.clone(),
            arg_types: self.arg_types.clone(),
            arg_offsets: self.arg_offsets.clone(),
            arg_sizes: self.arg_sizes.clone(),
            return_type: self.return_type.clone(),
            closure: None
        }
    }
}*/

pub type FunctionClosureType = dyn FnMut(&mut Adapter) -> ();

impl Function {
    /// Creates a new function
    pub fn new<T>(name: T) -> Function
    where String: From<T> {
        let name = String::from(name);
        Function {
            name: name,
            arg_types: Vec::new(),
            arg_offsets: HashMap::new(),
            arg_sizes: HashMap::new(),
            return_type: Type::Void,
            closure: None
        }
    }

    /// Sets the return type
    pub fn with_ret_type(mut self, ret_type: Type) -> Function {
        self.return_type = ret_type;
        self
    }

    /// Sets the next argument type
    pub fn with_arg(mut self, arg_type: Type) -> Function {
        self.arg_types.push(arg_type);
        self
    }

    /// INTERNAL: Sets the correct argument offsets
    pub fn set_arg_offsets(&mut self, arg_offsets: Vec<i64>) {
        for i in 0..arg_offsets.len() {
            self.arg_offsets.insert(i, arg_offsets[i]);
        }
    }

    /// INTERNAL: Sets the correct argument sizes
    pub fn set_arg_sizes(&mut self, arg_sizes: Vec<usize>) {
        for i in 0..arg_sizes.len() {
            self.arg_sizes.insert(i, arg_sizes[i]);
        }
    }

    /// Gets the byte offset of an argument
    pub fn get_arg_offset(&self, arg_index: usize) -> i64 {
        *self.arg_offsets.get(&arg_index).unwrap()
    }

    /// Runs the internal closure
    pub fn run(&self, adapter: &mut Adapter) {
        let closure_arc = self.closure.as_ref().unwrap();
        let mut closure_lock = closure_arc.lock().unwrap();
        let closure = closure_lock.deref_mut();
        closure(adapter);
    }
    
    /// Sets the closure to be executes
    pub fn with_closure(mut self, closure: Box<FunctionClosureType>) -> Function {
        let closure_arc = Arc::new(Mutex::new(closure));
        self.closure = Some(closure_arc);
        self
    }
}