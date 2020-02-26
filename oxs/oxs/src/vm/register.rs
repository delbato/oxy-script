use std::{
    marker::Copy,
    fmt::{
        Result as FmtResult,
        Formatter,
        Debug
    }
};

#[derive(Clone)]
pub union Register {
    pub uint64: u64,
    pub int64: i64,
    pub float: f32,
    pub boolean: bool
}

impl Debug for Register {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", unsafe { self.uint64 })
    }
}


impl Register {
    pub fn new() -> Register {
        Register {
            uint64: 0
        }
    }

    pub fn get<T>(&self) -> T
        where Self: RegisterAccess<T> {
        self.get_val()
    }

    pub fn set<T>(&mut self, item: T)
        where Self: RegisterAccess<T> {
        self.set_val(item);
    }

    pub fn inc<T>(&mut self, item: T)
        where Self: RegisterAccess<T> {
        self.inc_val(item);
    }

    pub fn dec<T>(&mut self, item: T)
        where Self: RegisterAccess<T> {
        self.dec_val(item);
    }
}

impl Copy for Register {}

pub trait RegisterAccess<T> {
    fn get_val(&self) -> T;
    fn set_val(&mut self, item: T);
    fn inc_val(&mut self, item: T);
    fn dec_val(&mut self, item: T);
}

impl RegisterAccess<i64> for Register {
    fn get_val(&self) -> i64 {
        unsafe {
            self.int64
        }
    }
    fn set_val(&mut self, item: i64) {
        self.int64 = item;
    }
    fn inc_val(&mut self, item: i64) {
        unsafe {
            self.int64 += item;
        }
    }
    fn dec_val(&mut self, item: i64) {
        unsafe {
            self.int64 -= item;
        }
    }
}

impl RegisterAccess<u64> for Register {
    fn get_val(&self) -> u64 {
        unsafe {
            self.uint64
        }
    }
    fn set_val(&mut self, item: u64) {
        self.uint64 = item;
    }
    fn inc_val(&mut self, item: u64) {
        unsafe {
            self.uint64 += item;
        }
    }
    fn dec_val(&mut self, item: u64) {
        unsafe {
            self.uint64 -= item;
        }
    }
}

impl RegisterAccess<f32> for Register {
    fn get_val(&self) -> f32 {
        unsafe {
            self.float
        }
    }
    fn set_val(&mut self, item: f32) {
        self.float = item;
    }
    fn inc_val(&mut self, item: f32) {
        unsafe {
            self.float += item;
        }
    }
    fn dec_val(&mut self, item: f32) {
        unsafe {
            self.float -= item;
        }
    }
}

impl RegisterAccess<bool> for Register {
    fn get_val(&self) -> bool {
        unsafe {
            self.boolean
        }
    }
    fn set_val(&mut self, item: bool) {
        self.boolean = item;
    }
    fn inc_val(&mut self, item: bool) {
    }
    fn dec_val(&mut self, item: bool) {
    }
}

impl RegisterAccess<usize> for Register {
    fn get_val(&self) -> usize {
        unsafe {
            self.uint64 as usize
        }
    }
    fn set_val(&mut self, item: usize) {
        self.uint64 = item as u64;
    }
    fn inc_val(&mut self, item: usize) {
        unsafe {
            self.uint64 += item as u64;
        }
    }
    fn dec_val(&mut self, item: usize) {
        unsafe {
            self.uint64 -= item as u64;
        }
    }
}
