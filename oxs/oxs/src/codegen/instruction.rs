use crate::{
    vm::{
        is::Opcode
    },
    codegen::{
        register::Register
    }
};



use serde::{
    Serialize,
    de::DeserializeOwned
};
use bincode::{
    deserialize,
    serialize
};

#[derive(Clone, Debug)]
pub struct Instruction {
    pub opcode: Opcode,
    pub operands: Vec<u8>,
}

impl Instruction {
    pub fn new(opcode: Opcode) -> Instruction {
        Instruction {
            opcode: opcode,
            operands: Vec::new()
        }
    }

    pub fn new_inc_stack(inc: usize) -> Instruction {
        Instruction::new(Opcode::ADDU_I)
            .with_operand::<u8>(Register::SP.into())
            .with_operand::<u64>(inc as u64)
            .with_operand::<u8>(Register::SP.into())
    }

    pub fn new_dec_stack(dec: usize) -> Instruction {
        Instruction::new(Opcode::SUBU_I)
            .with_operand::<u8>(Register::SP.into())
            .with_operand::<u64>(dec as u64)
            .with_operand::<u8>(Register::SP.into())
    }

    pub fn with_operand<T: Serialize>(mut self, operand: T) -> Instruction {
        let mut data = serialize(&operand).expect("ERROR Serializing operand!");
        self.operands.append(&mut data);
        self
    }

    pub fn append_operand<T: Serialize>(&mut self, operand: T) {
        let mut data = serialize(&operand).expect("ERROR Serializing operand!");
        self.operands.append(&mut data);
    }

    pub fn remove_operand_bytes(&mut self, n: usize) {
        self.operands.truncate(self.operands.len() - n);
    }

    pub fn clear_operands(&mut self) {
        self.operands.clear();
    }

    pub fn get_code(mut self) -> Vec<u8> {
        let mut code = Vec::new();

        // Get binary for opcode
        let opcode: u8 = self.opcode.into();
        code.push(opcode);

        // Append the operands
        code.append(&mut self.operands);
        
        code
    }

    pub fn get_size(&self) -> usize {
        self.operands.len() + 1
    }

    pub fn get_operand<T: DeserializeOwned>(&self, offset: usize, size: usize) -> T {
        let t = deserialize(&self.operands[offset..offset + size]).expect("ERROR Deserializing operand!");
        t
    }
}