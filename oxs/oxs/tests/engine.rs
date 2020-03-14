extern crate oxs;
use oxs::{
    codegen::{
        compiler::Compiler,
        register::Register
    },
    parser::{
        parser::Parser,
        ast::Type
    },
    engine::Engine,
    api::{
        module::Module,
        function::Function,
        adapter::Adapter
    }
};
/*
#[test]
fn test_engine_simple_function() {
    let code = String::from("
        fn: main() ~ int {
            return 4;
        }
    ");

    let mut engine = Engine::new(1024);
    let load_res = engine.load_code(&code);
    assert!(load_res.is_ok());

    let builder = engine.compiler.get_builder();
    for instr in builder.instructions.iter() {
        //println!("{:?}", instr);
    }

    let run_res = engine.run_fn(&String::from("root::main"));
    //println!("{:?}", run_res);
    assert!(run_res.is_ok());

    let result_res = engine.get_register_value::<i64>(Register::R0);
    assert!(result_res.is_ok());

    assert_eq!(4, result_res.unwrap());
}

#[test]
fn test_engine_if_else() {
    let code = String::from("
        fn: main() ~ int {
            var x: int = 4;
            if x == 5 {
                x = 2;
            } else {
                x = 1;
            }
            return x;
        }
    ");

    let mut engine = Engine::new(1024);
    let load_res = engine.load_code(&code);
    //println!("{:?}", load_res);
    assert!(load_res.is_ok());

    /*
    let builder = engine.compiler.get_builder();
    for instr in builder.instructions.iter() {
        //println!("{:?}", instr);
    }*/

    let run_res = engine.run_fn("root::main");
    //println!("{:?}", run_res);
    assert!(run_res.is_ok());

    let result_res = engine.get_register_value::<i64>(Register::R0);
    assert!(result_res.is_ok());

    assert_eq!(1, result_res.unwrap());
}

#[test]
fn test_engine_if_else_if() {
    let code = String::from("
        fn: main() ~ int {
            var x: int = 4;
            if x == 5 {
                x = 2;
            } else if x == 3 {
                x = 7;
            } else if x == 4 {
                x = 3;
            } else {
                x = 1;
            }
            return x;
        }
    ");

    let mut engine = Engine::new(1024);
    let load_res = engine.load_code(&code);
    //println!("{:?}", load_res);
    assert!(load_res.is_ok());

    let mut offset = 0;
    let builder = engine.compiler.get_builder();
    for instr in builder.instructions.iter() {
        //println!("{}: {:?}", offset, instr);
        offset += instr.get_size();
    }

    let run_res = engine.run_fn(&String::from("root::main"));
    //println!("{:?}", run_res);
    assert!(run_res.is_ok());

    let result_res = engine.get_register_value::<i64>(Register::R0);
    assert!(result_res.is_ok());

    assert_eq!(3, result_res.unwrap());
    assert_eq!(0, engine.get_stack_size());
}

#[test]
fn test_engine_while() {
    let code = String::from("
        fn: main() ~ float {
            var x = 0.0;
            while x < 10.0 {
                if x == 9.0 {
                    break;
                } else if x == 7.0 {
                    break;
                }
                x += 1.0;
            }
            return x;
        }
    ");

    let mut engine = Engine::new(1024);
    let load_res = engine.load_code(&code);
    //println!("{:?}", load_res);
    assert!(load_res.is_ok());

    let run_res = engine.run_fn("root::main");
    //println!("{:?}", run_res);
    assert!(run_res.is_ok());

    let reg_val_res = engine.get_register_value::<f32>(Register::R0);
    //println!("{:?}", reg_val_res);
    assert_eq!(7.0, reg_val_res.unwrap());
    assert_eq!(0, engine.get_stack_size());
}

#[test]
fn test_engine_foreign_module() {
    let code = String::from("
        import std::{
            printi,
            //println,
            print
        };

        fn: main() ~ int {
            var x = 0;
            while x < 10 {
                if x == 3 || x == 5 {
                    x += 1;
                    continue;
                } else if x == 7 {
                    break;
                }
                print(\"Value of x: \");
                printi(x);
                print(\"\n\");
                x += 1;
            }
            return x;
        }
    ");

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
    let //println_function = Function::new("println")
        .with_arg(Type::String)
        .with_ret_type(Type::Void)
        .with_closure(Box::new(|adapter: &mut Adapter| {
            //println!("Calling //println!");
            let arg: String = adapter.get_arg(0);
            //println!("{}", arg);
        }));
    
    let module = Module::new("std")
        .with_function(printi_function)
        .with_function(print_function)
        .with_function(println_function);

    let mut engine = Engine::new(1024);

    let reg_res = engine.register_module(module);
    //println!("{:?}", reg_res);
    assert!(reg_res.is_ok());

    let load_res = engine.load_code(&code);
    //println!("{:?}", load_res);
    assert!(load_res.is_ok());

    let mut offset = 0;
    for instr in engine.compiler.get_builder().instructions.iter() {
        //println!("{}: {:?}", offset, instr);
        offset += instr.get_size();
    }

    let run_res = engine.run_fn("root::main");
    //println!("{:?}", run_res);
    assert!(run_res.is_ok());

    let reg_val_res = engine.get_register_value::<i64>(Register::R0);
    assert!(reg_val_res.is_ok());
    assert_eq!(0, engine.get_stack_size());
    assert_eq!(7, reg_val_res.unwrap());
}

#[test]
fn test_engine_cont_simple() {
    let code = String::from("
        mod: inner {
            cont: Vector {
                x: float;
                y: float;
            }
            /*
            impl: Vector {
                fn: length(&this) ~ float {
                    return this.x * this.y;
                }
            }
            */
        }

        fn: main() {
            var vec = inner::Vector {
                x: 10.0,
                y: 5.0
            };

            vec.x *= 0.0-1.0;
            vec.y /= 2.0;

            std::print(\"x:\");
            std::printf(vec.x);
            std::print(\", y:\");
            std::printf(vec.y);
            std::println(\" \");
        }
    ");
    let printf_function = Function::new("printf")
        .with_arg(Type::Float)
        .with_ret_type(Type::Void)
        .with_closure(Box::new(|adapter| {
            let arg: f32 = adapter.get_arg(0);
            print!("{}", arg);
        }));
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
    let //println_function = Function::new("println")
        .with_arg(Type::String)
        .with_ret_type(Type::Void)
        .with_closure(Box::new(|adapter: &mut Adapter| {
            //println!("Calling //println!");
            let arg: String = adapter.get_arg(0);
            //println!("{}", arg);
        }));
    let std_module = Module::new("std")
        .with_function(printi_function)
        .with_function(println_function)
        .with_function(print_function)
        .with_function(printf_function);
    
    let mut engine = Engine::new(1024);

    let reg_res = engine.register_module(std_module);
    //println!("{:?}", reg_res);
    assert!(reg_res.is_ok());

    let load_res = engine.load_code(&code);
    //println!("{:?}", load_res);
    assert!(load_res.is_ok());

    let mut offset = 0;
    for instr in engine.compiler.get_builder().instructions.iter() {
        //println!("{}: {:?}", offset, instr);
        offset += instr.get_size();
    }

    let run_res = engine.run_fn("root::main");
    //println!("{:?}", run_res);
    assert!(run_res.is_ok());
}
*/
#[test]
fn test_engine_member_call() {
    let code = String::from("
        import std::{
            print,
            //println,
            printf
        };
        
        cont: Vector {
            x: float;
            y: float;
        }
        
        impl: Vector {
            fn: get_x(&this) ~ float {
                return this.x;
            }
            fn: get_x_ptr(vec: &Vector) ~ &float {
                return &vec.x;
            }
        }
        

        fn: get_ten() ~ float {
            return 10.0;
        }

        fn: main() {
            var vec = Vector {
                x: 2.0,
                y: 1.0
            };
            //var x = &vec.x;
            var x = Vector::get_x(&vec);
            vec.x += 3.14159;
            print(\"Value of x: \");
            printf(x);
            //println(\" \");
        }
    ");

    let printf_function = Function::new("printf")
        .with_arg(Type::Float)
        .with_ret_type(Type::Void)
        .with_closure(Box::new(|adapter| {
            let arg: f32 = adapter.get_arg(0);
            print!("{}", arg);
        }));
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
    let println_function = Function::new("println")
        .with_arg(Type::String)
        .with_ret_type(Type::Void)
        .with_closure(Box::new(|adapter: &mut Adapter| {
            //println!("Calling //println!");
            let arg: String = adapter.get_arg(0);
            //println!("{}", arg);
        }));
    let std_module = Module::new("std")
        .with_function(printi_function)
        .with_function(println_function)
        .with_function(print_function)
        .with_function(printf_function);
    
    let mut engine = Engine::new(1024);

    let reg_res = engine.register_module(std_module);
    //println!("{:?}", reg_res);
    assert!(reg_res.is_ok());

    let load_res = engine.load_code(&code);
    //println!("{:?}", load_res);
    assert!(load_res.is_ok());

    assert_eq!(engine.get_stack_size(), 0);

    /*
    let mut offset = 0;
    for instr in engine.compiler.get_builder().instructions.iter() {
        //println!("{}: {:?}", offset, instr);
        offset += instr.get_size();
    }
    */

    let run_res = engine.run_fn("root::main");

    assert_eq!(engine.get_stack_size(), 0);
    //println!("{:?}", run_res);
    assert!(run_res.is_ok());
}