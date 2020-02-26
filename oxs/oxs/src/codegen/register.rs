use crate::{
    codegen::{
        compiler::{
            CompilerResult,
            CompilerError
        }
    }
};

use std::{
    collections::{
        VecDeque,
        HashMap,
        HashSet
    },
    convert::{
        From,
        Into
    }
};

use epd::*;
use num_traits::FromPrimitive;

#[derive(Clone, PartialEq, Eq, Hash, Primitive, Debug)]
pub enum Register {
    R0 = 0,
    R1 = 1,
    R2 = 2,
    R3 = 3,
    R4 = 4,
    R5 = 5,
    R6 = 6,
    R7 = 7,
    R8 = 8,
    R9 = 9,
    R10 = 10,
    R11 = 11,
    R12 = 12,
    R13 = 13,
    R14 = 14,
    R15 = 15,
    SP = 16,
    IP = 17
}

impl From<u8> for Register {
    fn from(val: u8) -> Self {
        Self::from_u8(val).unwrap()
    }
}

impl Into<u8> for Register {
    fn into(self) -> u8 {
        self as u8
    }
}

#[derive(PartialEq, Debug)]
pub struct RegisterAllocator {
    register_queue: VecDeque<Register>,
    blocked_registers: HashSet<Register>,
    forced_temp: Option<Register>
}

impl RegisterAllocator {
    /// Creates a new RegisterAllocator instance
    pub fn new() -> RegisterAllocator {
        let mut register_queue = VecDeque::new();
        for i in 0..15 {
            register_queue.push_back(Register::from(i));
        }
        let mut reg_alloc = RegisterAllocator {
            register_queue: register_queue,
            blocked_registers: HashSet::new(),
            forced_temp: None
        };
        // Block the R0 register, as it is used for function return values
        reg_alloc.block_register(Register::R0).unwrap();
        reg_alloc
    }

    /// Gets the next temporary register, and puts it to the end of the queue
    pub fn get_temp_register(&mut self) -> CompilerResult<Register> {
        self.forced_temp = None;
        let ret = self.register_queue.pop_front()
            .ok_or(CompilerError::RegisterMapping)?;
        self.register_queue.push_back(ret.clone());
        Ok(ret)
    }

    /// Gets the last temporary register
    pub fn get_last_temp_register(&self) -> CompilerResult<Register> {
        if self.forced_temp.is_some() {
            return Ok(self.forced_temp.as_ref().cloned().unwrap());
        }
        self.register_queue.get(self.register_queue.len() - 1)
            .cloned()
            .ok_or(CompilerError::RegisterMapping)
    }

    /// Blocks a register from use for temporary calculations
    pub fn block_register(&mut self, reg: Register) -> CompilerResult<()> {
        let queue_index = self.register_queue.iter().position(|r| *r == reg)
            .ok_or(CompilerError::RegisterMapping)?;
        self.register_queue.remove(queue_index)
            .ok_or(CompilerError::RegisterMapping)?;
        self.blocked_registers.insert(reg);
        Ok(())
    }

    /// Unblocks a register for use for temporary calculations
    pub fn unblock_register(&mut self, reg: Register) -> CompilerResult<()> {
        let removed = self.blocked_registers.remove(&reg);
        if !removed {
            return Err(CompilerError::RegisterMapping);
        }
        self.register_queue.push_back(reg);
        Ok(())
    }

    /// Forces a certain register to be returned from get_last_temp_register()
    pub fn force_temp_register(&mut self, reg: Register) {
        self.forced_temp = Some(reg);
    }
}