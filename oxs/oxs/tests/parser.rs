extern crate oxs;
extern crate oxlex;
use oxs::{
    parser::{
        parser::*,
        ast::*,
        lexer::*
    }
};

use oxlex::prelude::Lexable;

#[test]
fn test_parse_import_decl() {
    let code = String::from("
        import root::lol::get_fucked = GetFucked;
    ");

    let mut lexer = Token::lexer(code.as_str());
    let parser = Parser::new(code.clone());

    let decl_res = parser.parse_import_decl(&mut lexer);
    assert!(decl_res.is_ok());

    let decl_list = decl_res.unwrap();

    if let Declaration::Import(import_string, import_name) = &decl_list[0] {
        assert_eq!(*import_string, String::from("root::lol::get_fucked"));
        assert_eq!(*import_name, String::from("GetFucked"));
    }
}

#[test]
fn test_parse_multi_import() {
    let code = String::from("
        import std::{
            printi,
            //println
        };
    ");

    let mut lexer = Token::lexer(code.as_str());
    let parser = Parser::new(code.clone());

    let decl_res = parser.parse_import_decl(&mut lexer);
    assert!(decl_res.is_ok());

    let decl_list = decl_res.unwrap();

    //println!("Current token: {:?}", lexer.token);

    assert_eq!(decl_list.len(), 2);

    assert_eq!(decl_list[0], Declaration::Import(String::from("std::printi"), String::from("printi")));
    assert_eq!(decl_list[1], Declaration::Import(String::from("std::println"), String::from("println")));
}

#[test]
fn test_parse_nested_multi_import() {
    let code = String::from("
        import std::{
            printi,
            //println,
            ext::{
                malloc = alloc,
                dealloc,
                inner::*
            }
        };
    ");

    let mut lexer = Token::lexer(code.as_str());
    let parser = Parser::new(code.clone());

    let decl_res = parser.parse_import_decl(&mut lexer);
    assert!(decl_res.is_ok());

    let decl_list = decl_res.unwrap();

    for decl in decl_list {
        //println!("{:?}", decl);
    }
}

#[test]
fn test_neg_parse_struct_decl() {
    let code = String::from("
        cont: Integer {
            inner: int;
            inner: int;
        }
    ");

    let mut lexer = Token::lexer(code.as_str());
    let parser = Parser::new(code.clone());

    let decl_res = parser.parse_container_decl(&mut lexer);
    assert!(decl_res.is_err());
}

#[test]
fn test_parse_container_decl() {
    let code = String::from("
        cont: Integer {
            inner: int;
        }
    ");

    let mut lexer = Token::lexer(code.as_str());
    let parser = Parser::new(code.clone());

    let decl_res = parser.parse_container_decl(&mut lexer);
    assert!(decl_res.is_ok());
}

#[test]
fn test_parse_empty_fn_decl() {
    let code = String::from("fn: main(arg: int) ~ int;");
    let mut lexer = Token::lexer(code.as_str());
    let parser = Parser::new(code.clone());
    let decl_res = parser.parse_fn_decl(&mut lexer);

    assert!(decl_res.is_ok());

    if let Declaration::Function(fn_decl) = decl_res.unwrap() {
        assert_eq!(fn_decl.name, String::from("main"));
    assert_eq!(fn_decl.arguments.len(), 1);
    assert!(fn_decl.code_block.is_none());
    }
}

#[test]
fn test_parse_full_fn_decl() {
    let code = String::from("fn: main(arg: int) ~ int {}");
    let mut lexer = Token::lexer(code.as_str());
    let parser = Parser::new(code.clone());
    let decl_res = parser.parse_fn_decl(&mut lexer);

    assert!(decl_res.is_ok());

    if let Declaration::Function(fn_decl) = decl_res.unwrap() {
        assert_eq!(fn_decl.name, String::from("main"));
        assert_eq!(fn_decl.arguments.len(), 1);
        assert!(fn_decl.code_block.is_some());
    }
}

#[test]
fn test_parse_fn_mul_args() {
    let code = String::from("fn: main21(arg: int, noarg: int) ~ int {}");
    let mut lexer = Token::lexer(code.as_str());
    let parser = Parser::new(code.clone());
    let decl_res = parser.parse_fn_decl(&mut lexer);

    assert!(decl_res.is_ok());

    if let Declaration::Function(fn_decl) = decl_res.unwrap() {
        assert_eq!(fn_decl.name, String::from("main21"));
        assert_eq!(fn_decl.arguments.len(), 2);
        assert!(fn_decl.code_block.is_some());
    }
}

#[test]
fn test_parse_decl_list() {
    let code = String::from("
        fn: main1(argc: int) ~ int;
        fn: test2(noint: float) ~ float {}
    ");
    let parser = Parser::new(code);

    let decl_list_res = parser.parse_root_decl_list();

    assert!(decl_list_res.is_ok());

    let decl_list = decl_list_res.unwrap();

    assert_eq!(decl_list.len(), 2);
}

#[test]
fn test_parse_stmt_list() {
    let code = String::from("
        var x: int = 4;
        var y: int = 6;
    ");

    let mut lexer = Token::lexer(code.as_str());
    let parser = Parser::new(code.clone());
    let stmt_list_res = parser.parse_statement_list(&mut lexer);

    assert!(stmt_list_res.is_ok());
    let stmt_list = stmt_list_res.unwrap();

    assert_eq!(stmt_list.len(), 2);
}

#[test]
fn test_parse_stmt_addition() {
    let code = String::from("
        var x: int = 4;
        y = 1 + 2 * 3 + x;
    ");

    let mut lexer = Token::lexer(code.as_str());
    let parser = Parser::new(code.clone());
    let stmt_list_res = parser.parse_statement_list(&mut lexer);

    assert!(stmt_list_res.is_ok());
    let stmt_list = stmt_list_res.unwrap();

    assert_eq!(stmt_list.len(), 2);

    //println!("{:?}", stmt_list);
}

#[test]
fn test_parse_stmt_call() {
    let code = String::from("
        std::alloc::allocate();
    ");

    let mut lexer = Token::lexer(code.as_str());
    let parser = Parser::new(code.clone());
    let stmt_list_res = parser.parse_statement_list(&mut lexer);

    assert!(stmt_list_res.is_ok());
    let stmt_list = stmt_list_res.unwrap();

    assert_eq!(stmt_list.len(), 1);

    //println!("{:?}", stmt_list);
}

#[test]
fn test_parse_float_expr() {
    let code = String::from("
        (2.0 * 2.0) * 3.14;
    ");
    let mut lexer = Token::lexer(code.as_str());
    let parser = Parser::new(code.clone());

    let expr_res = parser.parse_expr(&mut lexer, &[Token::Semicolon]);
    assert!(expr_res.is_ok());
    let expr = expr_res.unwrap();
    expr.print(0);
}

#[test]
fn test_parse_raw_expr() {
    let code = String::from("
        (1 + 2 + 3) * 7 - 8 + 3;
    ");
    let mut lexer = Token::lexer(code.as_str());
    let parser = Parser::new(code.clone());

    let expr_res = parser.parse_expr(&mut lexer, &[Token::Semicolon]);
    assert!(expr_res.is_ok());
    let expr = expr_res.unwrap();
    expr.print(0);
}

#[test]
fn test_parse_raw_var_expr() {
    let code = String::from("
        (1 + z + 3) * x - 8 + y;
    ");
    let mut lexer = Token::lexer(code.as_str());
    let parser = Parser::new(code.clone());
    let expr_res = parser.parse_expr(&mut lexer, &[Token::Semicolon]);
    assert!(expr_res.is_ok());
    let expr = expr_res.unwrap();
    //expr.print(0);
}

#[test]
fn test_parse_full_fn() {
    let code = String::from("
        fn: main(argc: int) ~ int {
            var x: int = 4;
            var y: int = 6;
            return x + y;
        }
    ");

    let parser = Parser::new(code.clone());
    let decl_list_res = parser.parse_root_decl_list();
    assert!(decl_list_res.is_ok());
}

#[test]
fn test_parse_expr_paran_delim() {
    use oxs::{
        parser::ast::*  
    };

    let code = String::from("
        (1 + 2) + 2)
    ");
    let mut lexer = Token::lexer(code.as_str());
    let parser = Parser::new(code.clone());
    let expr_res = parser.parse_expr(&mut lexer, &[
        Token::CloseParan
    ]);
    assert!(expr_res.is_ok());
    let expr = expr_res.unwrap();
    match expr {
        Expression::Addition(lhs, rhs) => {
            match *lhs {
                Expression::Addition(lhs, rhs) => {
                    match *lhs {
                        Expression::IntLiteral(_) => {},
                        _ => {
                            panic!("Incorrect expression! Should be IntLiteral.");
                        }
                    };
                    match *rhs {
                        Expression::IntLiteral(_) => {},
                        _ => {
                            panic!("Incorrect expression! Should be IntLiteral.");
                        }
                    };
                },
                _ => {
                    panic!("Incorrect expression! Should be Addition.");
                }
            };
            match *rhs {
                Expression::IntLiteral(_) => {},
                _ => {
                    panic!("Incorrect expression! Should be IntLiteral.");
                }
            };
        },
        _ => {
            panic!("Incorrect expression! Should be Addition.");
        }
    }
}

#[test]
fn test_parse_call_stmt() {
    use oxs::{
        parser::ast::*  
    };

    let code = String::from("
        add(5, 5);
    ");
    let mut lexer = Token::lexer(code.as_str());
    let parser = Parser::new(code.clone());
    let stmt_res = parser.parse_fn_call_stmt(&mut lexer);
    assert!(stmt_res.is_ok());
    if let Statement::Call(name, args) = stmt_res.unwrap() {
        assert_eq!(name, String::from("add"));
        assert_eq!(args.len(), 2);
        assert_eq!(args, vec![
            Expression::IntLiteral(5),
            Expression::IntLiteral(5)
        ]);
    }
}

#[test]
fn test_parse_call_expr() {
    use oxs::parser::ast::Expression;

    let code = String::from("
        add(5, 5);
    ");
    let mut lexer = Token::lexer(code.as_str());
    let parser = Parser::new(code.clone());
    let delims = [
        Token::Semicolon
    ];

    let expr_res = parser.parse_expr(&mut lexer, &delims);
    assert!(expr_res.is_ok());
    if let Expression::Call(name, args) = expr_res.unwrap() {
        assert_eq!(name, String::from("add"));
        assert_eq!(args.len(), 2);
        assert_eq!(args, vec![
            Expression::IntLiteral(5),
            Expression::IntLiteral(5)
        ]);
    }
}

#[test]
fn test_parse_complex_call_expr() {
    use oxs::parser::ast::Expression;

    let code = String::from("
        add(5, 5) + 5;
    ");
    let mut lexer = Token::lexer(code.as_str());
    let parser = Parser::new(code.clone());
    let expr_res = parser.parse_expr(&mut lexer, &[
        Token::Semicolon
    ]);
    assert!(expr_res.is_ok());
    let expr = expr_res.unwrap();
    match expr {
        Expression::Addition(lhs, rhs) => {
            match *lhs {
                Expression::Call(fn_name, args) => {
                    assert_eq!(fn_name, String::from("add"));
                    assert_eq!(args.len(), 2);
                },
                _ => {
                    panic!("Wrong expression! Should be Call.");
                }
            };
            match *rhs {
                Expression::IntLiteral(int) => {
                    assert_eq!(int, 5);
                },
                _ => {
                    panic!("Wrong expression! Should be IntLiteral.");
                }
            };
        },
        _ => {
            panic!("Wrong expression! Should be Addition.");
        }
    }
}

#[test]
fn test_parse_mod_decl() {
    let code = String::from("
        mod: somemodule {
            mod: nestedmodule {
                fn: five() ~ int {
                    return 5
                }
            }
        }
        mod: othermodule {
            fn: multiply(lhs: int, rhs: int) ~ int {
                return lhs * rhs;
            }
        }

        fn: main() ~ int {
            var five: int = somemodule::nestedmodule::five();
            var fifty: int = othermodule::multiply(five, 10);
            return fifty;
        }
    ");

    let parser = Parser::new(code);
    let decl_list_res = parser.parse_root_decl_list();
    assert!(decl_list_res.is_ok());
}

#[test]
fn test_parse_while() {
    let code = String::from("
        while true {
            var x: int = 0;
        }
    ");

    let parser = Parser::new(code.clone());
    let mut lexer = Token::lexer(code.as_str());
    let stmt_res = parser.parse_while(&mut lexer);
    assert!(stmt_res.is_ok());

    if let Statement::While(expr_box, stmt_list) = stmt_res.unwrap() {
        //println!("while expr: {:?}", *expr_box);
        //println!("while stmt list: {:?}", stmt_list);
    }
}

#[test]
fn test_parse_loop() {
    let code = String::from("
        loop {
            var x: int = 0;
        }
    ");

    let parser = Parser::new(code.clone());
    let mut lexer = Token::lexer(code.as_str());
    let stmt_res = parser.parse_loop(&mut lexer);
    assert!(stmt_res.is_ok());

    if let Statement::Loop(stmt_list) = stmt_res.unwrap() {
        //println!("loop stmt list: {:?}", stmt_list);
    }
}

#[test]
fn test_parse_if() {
    let code = String::from("
        if false {
            var x: int = 0;
            x += 2;
        }
    ");

    let parser = Parser::new(code.clone());
    let mut lexer = Token::lexer(code.as_str());
    let stmt_res = parser.parse_if(&mut lexer);
    assert!(stmt_res.is_ok());

    //println!("{:?}", stmt_res.unwrap());
}

#[test]
fn test_parse_if_else() {
    let code = String::from("
        if false {
            var x: int = 0;
            x += 2;
        } else {
            var x: int = 0;
            x += 1;
        }
    ");

    let parser = Parser::new(code.clone());
    let mut lexer = Token::lexer(code.as_str());
    let stmt_res = parser.parse_if(&mut lexer);
    assert!(stmt_res.is_ok());

    //println!("{:?}", stmt_res.unwrap());
}

#[test]
fn test_parse_if_else_if() {
    let code = String::from("
        if false {
            var x: int = 0;
            x += 2;
        } else if true {
            var x: int = 0;
            x += 1;
        }
    ");

    let parser = Parser::new(code.clone());
    let mut lexer = Token::lexer(code.as_str());
    let stmt_res = parser.parse_if(&mut lexer);
    assert!(stmt_res.is_ok());

    //println!("{:?}", stmt_res.unwrap());
}

#[test]
fn test_parse_if_else_if_else() {
    let code = String::from("
        if false {
            var x: int = 0;
            x += 2;
        } else if true {
            var x: int = 0;
            x += 1;
        } else {
            var x: int = 0;
            x += 3;
        }
    ");

    let parser = Parser::new(code.clone());
    let mut lexer = Token::lexer(code.as_str());
    let stmt_res = parser.parse_if(&mut lexer);
    assert!(stmt_res.is_ok());

    //println!("{:?}", stmt_res.unwrap());
}

#[test]
fn test_parse_member() {
    let code = String::from("
        engine.blub.get() * engine.foo;
    ");

    let parser = Parser::new(code.clone());
    let mut lexer = Token::lexer(code.as_str());

    let expr_res = parser.parse_expr(&mut lexer, &[ Token::Semicolon ]);
    assert!(expr_res.is_ok());

    expr_res.unwrap().print(0);
}

#[test]
fn test_parse_add_assign() {
    let code = String::from("
        engine.blub += 12.7;
    ");

    let parser = Parser::new(code.clone());
    let mut lexer = Token::lexer(code.as_str());

    let expr_res = parser.parse_expr(&mut lexer, &[ Token::Semicolon ]);
    assert!(expr_res.is_ok());

    expr_res.unwrap().print(0);
}

#[test]
fn test_parse_stmt_expr() {
    let code = String::from("
            engine.blub += 12.7;
        }
    ");

    let parser = Parser::new(code.clone());
    let mut lexer = Token::lexer(code.as_str());

    let stmt_list_res = parser.parse_statement_list(&mut lexer);
    assert!(stmt_list_res.is_ok());
}

#[test]
fn test_parse_cont_impl() {
    let code = String::from("
        cont: Vector {
            x: float;
            y: float;
        }
        impl: Vector {
            fn: len(&this) ~ float {
                return 1.0;
            }
        }
    ");

    let parser = Parser::new(code.clone());
    let mut lexer = Token::lexer(code.as_str());

    let decl_list_res = parser.parse_decl_list(&mut lexer, &[Token::CloseBlock]);
    assert!(decl_list_res.is_ok());

    for decl in decl_list_res.unwrap() {
        //println!("{:?}", decl);
    }
}

#[test]
fn test_parse_cont_instance() {
    let code = String::from("
        cont: Test {
            x: string;
            y: string;
        }
        
        fn: main() {
            var test = Test {
                x: \"Hello,\",
                y: \" world!\"
            };
        }
    ");

    let parser = Parser::new(code.clone());

    let decl_list_res = parser.parse_root_decl_list();
    //println!("{:?}", decl_list_res);
    assert!(decl_list_res.is_ok());

    for decl in decl_list_res.unwrap() {
        if let Declaration::Function(fn_decl_args) = decl {
            for stmt in fn_decl_args.code_block.iter() {
                //println!("{:?}", stmt);
            }
        }
    }
}