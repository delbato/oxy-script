use super::{
    instruction::{
        Instruction
    }
};
use crate::{
    vm::{
        is::Opcode
    }
};

use std::{
    collections::{
        HashMap
    },
    ops::DerefMut
};

use serde::{
    Serialize
};
use bincode::serialize;

#[derive(Clone)]
pub struct Builder {
    data: Vec<u8>,
    pub instructions: Vec<Instruction>,
    pub jmp_instructions: Vec<usize>,
    pub labels: HashMap<String, usize>,
    pub tags: HashMap<u64, Vec<usize>>
}

impl Builder {
    pub fn new() -> Builder {
        Builder {
            data: Vec::new(),
            instructions: Vec::new(),
            labels: HashMap::new(),
            tags: HashMap::new(),
            jmp_instructions: Vec::new()
        }
    }

    pub fn push_label(&mut self, label: String) {
        self.labels.insert(label, self.instructions.len());
    }

    pub fn tag(&mut self, tag: u64) {
        let pos = self.instructions.len();
        if let Some(tag_list) = self.tags.get_mut(&tag) {
            if !tag_list.contains(&pos) {
                tag_list.push(pos);
            }
        } else {
            let mut tag_list = Vec::new();
            tag_list.push(pos);
            self.tags.insert(tag, tag_list);
        }
    }

    pub fn get_tag(&mut self, tag: &u64) -> Option<Vec<usize>> {
        let pos_list = self.tags.get(tag)
            .cloned()
            .or(None)?;
        Some(pos_list)
    }

    pub fn get_instr(&mut self, offset: &usize) -> Option<&mut Instruction> {
        self.instructions.get_mut(*offset)
    }

    pub fn push_instr(&mut self, instruction: Instruction) {
        if instruction.opcode == Opcode::JMP ||
            instruction.opcode == Opcode::JMPT ||
            instruction.opcode == Opcode::JMPF {
            self.jmp_instructions.push(self.instructions.len());
        }
        self.instructions.push(instruction);
    }

    pub fn append_instr(&mut self, mut instructions: Vec<Instruction>) {
        self.instructions.append(&mut instructions);
    }

    pub fn push_data<T: Serialize>(&mut self, data: T) {
        let mut data = serialize(&data).expect("Could not serialize builder data!");
        self.data.append(&mut data);
    }

    pub fn build(mut self) -> Vec<u8> {
        let mut code = Vec::new();

        code.append(&mut self.data);

        for instruction in self.instructions {
            let mut instr_code = instruction.get_code();
            code.append(&mut instr_code);
        }

        code
    }

    pub fn get_label_offset(&mut self, label: &String) -> Option<usize> {
        let mut code_before_size = 0;
        let label_instr_offset = self.labels.get(label)
            .or(None)?;
        
        for i in 0..*label_instr_offset {
            code_before_size += self.instructions[i].get_size();
        }

        Some(code_before_size)
    }
    pub fn get_current_offset(&self) -> usize {
        let mut offset = 0;
        for instr in self.instructions.iter() {
            offset += instr.get_size();
        }
        offset
    }
}