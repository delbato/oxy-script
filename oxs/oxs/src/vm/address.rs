use std::{
    convert::{
        From,
        Into
    }
};

#[derive(Debug)]
pub struct Address {
    pub raw_address: u64,
    pub real_address: u64,
    pub address_type: AddressType
}

#[derive(PartialEq, Debug)]
pub enum AddressType {
    Program,
    Stack,
    Heap,
    Foreign,
    Swap
}

impl Address {
    pub fn new(real_address: u64, address_type: AddressType) -> Address {
        let mut type_raw: u64 = match address_type {
            AddressType::Program => 0,
            AddressType::Stack => 1,
            AddressType::Heap => 2,
            AddressType::Swap => 3,
            AddressType::Foreign => 4
        };
        // Shift type to the 3 left most bits
        type_raw = type_raw << 61;
        // Mask these bits over the address
        let raw_address = real_address + type_raw;

        Address {
            real_address: real_address,
            raw_address: raw_address,
            address_type: address_type
        }
    }

    pub fn with_offset(mut self, offset: i16) -> Address {
        if offset < 0 {
            self.real_address -= offset.abs() as u64;
        } else {
            self.real_address += offset as u64;
        }
        self
    } 
}

impl From<u64> for Address {
    fn from(raw: u64) -> Address {
        let type_raw = raw >> 61;
        let address_type = match type_raw {
            0 => AddressType::Program,
            1 => AddressType::Stack,
            2 => AddressType::Heap,
            3 => AddressType::Swap,
            4 => AddressType::Foreign,
            _ => panic!("Address is not formatted correctly!")
        };
        // Remove 2 left most bits, which are the type
        let mut real_address = raw << 3;
        real_address = real_address >> 3;

        Address {
            raw_address: raw,
            real_address: real_address,
            address_type: address_type
        }
    }
}

impl Into<u64> for Address {
    fn into(self) -> u64 {
        self.raw_address
    }
}