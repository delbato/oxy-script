extern crate oxs;

use oxs::{
    engine::{
        Engine,
        EngineResult
    },
    api::{
        function::{
            Function
        },
        adapter::Adapter,
        module::{
            Module
        }
    },
    parser::{
        ast::{
            Type
        }
    },
    vm::{
        core::Core
    }
};

fn register_std_print(engine: &mut Engine) -> EngineResult<()> {
    let printi_function = Function::new("printi")
        .with_arg(Type::Int)
        .with_ret_type(Type::Void)
        .with_closure(Box::new(|adapter: &mut Adapter| {
            //println!("Calling printi!");
            let arg: i64 = adapter.get_arg(0);
            print!("{}", arg);
        }));
    let print_function = Function::new("print")
        .with_arg(Type::String)
        .with_ret_type(Type::Void)
        .with_closure(Box::new(|adapter: &mut Adapter| {
            //println!("Calling print!");
            let arg: String = adapter.get_arg(0);
            print!("{}", arg);
        }));
    let printf_function = Function::new("printf")
        .with_arg(Type::Float)
        .with_ret_type(Type::Void)
        .with_closure(Box::new(|adapter| {
            let arg: f32 = adapter.get_arg(0);
            print!("{}", arg);
        }));
    let println_function = Function::new("println")
        .with_arg(Type::String)
        .with_ret_type(Type::Void)
        .with_closure(Box::new(|adapter: &mut Adapter| {
            //println!("Calling println!");
            let arg: String = adapter.get_arg(0);
            println!("{}", arg);
        }));
    
    let module = Module::new("std")
        .with_function(printi_function)
        .with_function(print_function)
        .with_function(println_function)
        .with_function(printf_function);
    
    engine.register_module(module)
}

#[no_mangle]
pub extern fn register_extension(engine: &mut Engine) -> EngineResult<()> {
    register_std_print(engine)?;
    Ok(())
}