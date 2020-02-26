use crate::{
    api::{
        function::{
            Function
        }
    },
    codegen::{
        register::{
            Register
        }
    },
    vm::{
        core::{
            Core
        },
        register::{
            Register as RegisterUnion,
            RegisterAccess
        }
    }
};

use std::{
    sync::{
        Arc,
        Mutex
    }
};

use serde::{
    de::DeserializeOwned
};

pub struct Adapter<'c> {
    pub function: Function,
    pub core: &'c mut Core
}

impl<'c> Adapter<'c> {
    pub fn new(func: &Function, core: &'c mut Core) -> Adapter<'c> {
        Adapter {
            function: func.clone(),
            core: core
        }
    }

    pub fn get_arg<T>(&mut self, arg_index: usize) -> T
    where T: FromArg {
        T::get(self, arg_index)
    }

    pub fn return_value<T>(&mut self, value: T)
    where RegisterUnion: RegisterAccess<T> {
        self.core.reg(Register::R0.into()).unwrap().set::<T>(value);
    }

    // Retrieves a foreign pointer and returns the correct
    /// Arc<Mutex<T>> if found.
    pub fn get_foreign_ptr<T>(&self, ptr: u64) -> Arc<Mutex<T>> {
        self.core.get_foreign_ptr(ptr).unwrap()
    }

    /// Inserts a foreign pointer
    pub fn insert_foreign_ptr<T>(&mut self, item: Arc<Mutex<T>>) -> u64 {
        self.core.insert_foreign_ptr(item).unwrap()
    }

    /// Removes a foreign pointer
    pub fn remove_foreign_ptr<T>(&mut self, ptr: u64) -> Arc<Mutex<T>> {
        self.core.remove_foreign_ptr(ptr).unwrap()
    }
}

pub trait FromArg: DeserializeOwned {
    fn get(adapter: &mut Adapter, arg_index: usize) -> Self;
}

impl FromArg for String {
    fn get(adapter: &mut Adapter, arg_index: usize) -> String {
        let arg_offset = adapter.function.get_arg_offset(arg_index).abs() as u64;
        //println!("Arg offset of Arg #{}: -{}B", arg_index, arg_offset);
        let mut stack_addr = adapter.core.reg(16).unwrap().get::<u64>();
        stack_addr -= arg_offset;
        let string_res = adapter.core.mem_get_string(stack_addr);
        //println!("{:?}", string_res);
        string_res.unwrap()
    }
}

impl FromArg for i64 {
    fn get(adapter: &mut Adapter, arg_index: usize) -> i64 {
        let arg_offset = adapter.function.get_arg_offset(arg_index) as i16;
        let addr = adapter.core.reg(16).unwrap().get::<u64>();
        adapter.core.mem_get((addr, arg_offset)).unwrap()
    }
}

impl FromArg for f32 {
    fn get(adapter: &mut Adapter, arg_index: usize) -> f32 {
        let arg_offset = adapter.function.get_arg_offset(arg_index) as i16;
        let addr = adapter.core.reg(16).unwrap().get::<u64>();
        adapter.core.mem_get((addr, arg_offset)).unwrap()
    }
}

impl FromArg for u64 {
    fn get(adapter: &mut Adapter, arg_index: usize) -> u64 {
        let arg_offset = adapter.function.get_arg_offset(arg_index) as i16;
        let addr = adapter.core.reg(16).unwrap().get::<u64>();
        adapter.core.mem_get((addr, arg_offset)).unwrap()
    }
}