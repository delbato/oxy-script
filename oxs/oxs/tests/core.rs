extern crate oxs;
use oxs::{
    vm::{
        core::*,
        is::Opcode
    },
    codegen::{
        program::Program,
        builder::Builder,
        instruction::Instruction
    }
};

use bincode::serialize;
#[test]
fn test_core_addi() {
    let mut builder = Builder::new();

    let ldi_instr0 = Instruction::new(Opcode::LDI) // LDI 58, r0
        .with_operand(58i64)
        .with_operand(0u8);
    let ldi_instr1 = Instruction::new(Opcode::LDI) // LDI 42, r1
        .with_operand(42i64)
        .with_operand(1u8);
    let lda_instr = Instruction::new(Opcode::LDA) // LDI 8, r2
        .with_operand(8u64)
        .with_operand(2u8);
    let addi_instr = Instruction::new(Opcode::ADDI) // ADDI r0, r1, r0
        .with_operand(0u8)
        .with_operand(1u8)
        .with_operand(0u8);
    let add_sp_instr = Instruction::new(Opcode::ADDU) // ADDU sp, r2, sp
        .with_operand(16u8)
        .with_operand(2u8)
        .with_operand(16u8);
    let mov_instr = Instruction::new(Opcode::MOVI_RA) // MOVI r0, [sp-8]
        .with_operand(0u8)
        .with_operand(16u8)
        .with_operand::<i16>(-8);
    
    builder.push_instr(ldi_instr0);
    builder.push_instr(ldi_instr1);
    builder.push_instr(lda_instr);
    builder.push_instr(addi_instr);
    builder.push_instr(add_sp_instr);
    builder.push_instr(mov_instr);

    let program = Program::new().with_code(builder.build());

    let mut core = Core::new(1024);
    core.load_program(program);
    let run_res = core.run();
    assert!(run_res.is_ok());
    let stack_res = core.pop_stack::<i64>();
    assert!(stack_res.is_ok());
    assert_eq!(stack_res.unwrap(), 100);
}

#[test]
fn test_push_pop_stack() {
    let mut code: Vec<u8> = Vec::new();
    let x: i64 = 4;
    let y: i64 = 6;
    let mut x_bytes = serialize(&x).unwrap();
    let mut y_bytes = serialize(&y).unwrap();
    code.append(&mut x_bytes);

    let program = Program::new().with_code(code);

    let mut core = Core::new(1024);
    core.load_program(program);
    let run_res = core.run();
    assert!(run_res.is_ok());
    let stack_res = core.pop_stack::<i64>();
    assert!(stack_res.is_ok());
    assert_eq!(stack_res.unwrap(), 4);
}

#[test]
fn test_core_foreign_ptr() {
    use std::{
        sync::{
            Arc,
            Mutex
        },
        ops::Drop
    };

    #[derive(Debug)]
    struct Int(i32);

    impl Drop for Int {
        fn drop(&mut self) {
            //println!("Int with val {} dropped!", self.0);
        }
    }

    let mut int_arc = Arc::new(Mutex::new(Int(0)));

    let mut core = Core::new(1024);
    let ptr_res = core.insert_foreign_ptr(int_arc);
    //println!("{:?}", ptr_res);
    assert!(ptr_res.is_ok());
    let ptr = ptr_res.unwrap();

    {
        let get_res = core.get_foreign_ptr(ptr);
        //println!("{:?}", get_res);
        assert!(get_res.is_ok());

        int_arc = get_res.unwrap();
    }

    {
        let mut int = int_arc.lock().unwrap();
        int.0 += 10;
    }

    let remove_res = core.remove_foreign_ptr(ptr);
    //println!("{:?}", remove_res);
    assert!(remove_res.is_ok());

    int_arc = remove_res.unwrap();

    {
        let int = int_arc.lock().unwrap();
        assert_eq!(int.0, 10);
    }
}