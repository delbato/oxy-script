extern crate oxs;
use oxs::{
    codegen::{
        compiler::{
            Compiler
        },
        program::{
            Program
        },
        instruction::{
            Instruction
        }
    },
    parser::{
        parser::Parser,
        lexer::Token
    }
};

use oxlex::prelude::Lexable;

/*

#[test]
fn test_compile_stmt_var_decl() {
    let code = String::from("
        fn: main() {
            var x: int = (4 + 4) * 2;
        }
    ");

    let parser = Parser::new(code.clone());
    let mut lexer = Token::lexer(code.as_str());

    let decl_list_res = parser.parse_decl_list(&mut lexer, &[]);
    assert!(decl_list_res.is_ok());

    let decl_list = decl_list_res.unwrap();

    for stmt in decl_list.iter() {
        //println!("{:?}", stmt);
    }

    let mut compiler = Compiler::new();
    let compile_res = compiler.compile_root(&decl_list);
    //println!("{:?}", compile_res);
    assert!(compile_res.is_ok());

    let builder = compiler.get_builder();

    for instr in builder.instructions.iter() {
        //println!("{:?}", instr);
    }
}


#[test]
fn test_compile_if() {
    let code = String::from("
        fn: main() {
            var x: int = (4 + 4) * 2;
            if x < 8 {
                var z: int = 4;
            }
        }
    ");

    let parser = Parser::new(code.clone());
    let mut lexer = Token::lexer(code.as_str());

    let decl_list_res = parser.parse_decl_list(&mut lexer, &[]);
    assert!(decl_list_res.is_ok());

    let decl_list = decl_list_res.unwrap();

    for stmt in decl_list.iter() {
        //println!("{:?}", stmt);
    }

    let mut compiler = Compiler::new();
    let compile_res = compiler.compile_root(&decl_list);
    //println!("{:?}", compile_res);
    assert!(compile_res.is_ok());

    let builder = compiler.get_builder();

    for instr in builder.instructions.iter() {
        //println!("{:?}", instr);
    }
}

#[test]
fn test_compile_var_assign() {
    let code = String::from("
        fn: main() {
            var x: int = 0;
            if x < 1 {
                x += 1;
            }
        }
    ");

    let parser = Parser::new(code.clone());
    let mut lexer = Token::lexer(code.as_str());

    let decl_list_res = parser.parse_decl_list(&mut lexer, &[]);
    assert!(decl_list_res.is_ok());

    let decl_list = decl_list_res.unwrap();

    let mut compiler = Compiler::new();
    let compile_res = compiler.compile_root(&decl_list);
    //println!("{:?}", compile_res);
    assert!(compile_res.is_ok());

    let builder = compiler.get_builder();

    for instr in builder.instructions.iter() {
        //println!("{:?}", instr);
    }
}

#[test]
fn test_compile_auto_var() {
    let code = String::from("
        fn: main() {
            var x = 0;
            if x < 1 {
                x += 1;
            }
        }
    ");

    let parser = Parser::new(code.clone());
    let mut lexer = Token::lexer(code.as_str());

    let decl_list_res = parser.parse_decl_list(&mut lexer, &[]);
    assert!(decl_list_res.is_ok());

    let decl_list = decl_list_res.unwrap();

    let mut compiler = Compiler::new();
    let compile_res = compiler.compile_root(&decl_list);
    //println!("{:?}", compile_res);
    assert!(compile_res.is_ok());

    let builder = compiler.get_builder();

    for instr in builder.instructions.iter() {
        //println!("{:?}", instr);
    }
}

#[test]
fn test_compile_while_stmt() {
    let code = String::from("
        fn: main() {
            var x = 0.0;
            while x < 10.0 {
                if x == 7.0 {
                    break;
                }
                x += 1.0;
            }
        }
    ");

    let parser = Parser::new(code.clone());
    let mut lexer = Token::lexer(code.as_str());

    let decl_list_res = parser.parse_decl_list(&mut lexer, &[]);
    assert!(decl_list_res.is_ok());

    let decl_list = decl_list_res.unwrap();

    let mut compiler = Compiler::new();
    let compile_res = compiler.compile_root(&decl_list);
    //println!("{:?}", compile_res);
    assert!(compile_res.is_ok());

    let builder = compiler.get_builder();

    let mut pos = 0;

    for instr in builder.instructions.iter() {
        //println!("{}:  {:?}", pos, instr);
        pos += instr.get_size();
    }
}

#[test]
fn test_compile_cont_instance() {
    let code = String::from("
        cont: Vector {
            x: int;
            y: int;
        }
        fn: main() {
            var v = Vector {
                x: 12,
                y: 4
            };
            
            var x = v.x;
        }
    ");

    let parser = Parser::new(code.clone());
    let mut lexer = Token::lexer(code.as_str());
    
    let decl_list_res = parser.parse_decl_list(&mut lexer, &[]);
    assert!(decl_list_res.is_ok());

    let decl_list = decl_list_res.unwrap();

    let mut compiler = Compiler::new();
    let compile_res = compiler.compile_root(&decl_list);
    //println!("{:?}", compile_res);
    assert!(compile_res.is_ok());

    let builder = compiler.get_builder();

    let mut pos = 0;

    for instr in builder.instructions.iter() {
        //println!("{}:  {:?}", pos, instr);
        pos += instr.get_size();
    }
}

*/

#[test]
fn test_compile_member_call() {
    let code = String::from("
        cont: Vector {
            x: float;
            y: float;
        }

        impl: Vector {
            fn: get_x(&this) ~ float {
                return this.x;
            }
        }

        fn: main() {
            var vec = Vector {
                x: 2.0,
                y: 1.0
            };

            var x = vec.get_x();
        }
    ");
    //println!("Starting parse");
    let parser = Parser::new(code.clone());
    let mut lexer = Token::lexer(code.as_str());

    let decl_list_res = parser.parse_decl_list(&mut lexer, &[]);
    //println!("Finished parse");
    //println!("{:?}", decl_list_res);
    assert!(decl_list_res.is_ok());

    let decl_list = decl_list_res.unwrap();

    let mut compiler = Compiler::new();
    let compile_res = compiler.compile_root(&decl_list);
    //println!("{:?}", compile_res);
    assert!(compile_res.is_ok());

    let builder = compiler.get_builder();

    let mut pos = 0;

    for instr in builder.instructions.iter() {
        //println!("{}:  {:?}", pos, instr);
        pos += instr.get_size();
    }
}