extern crate oxs;
extern crate oxlex;
use oxs::{
    parser::{
        lexer::Token
    }
};
use oxlex::prelude::Lexable;

#[test]
fn test_lex_comment() {
    let i = 0;
    let lexer = Token::lexer("
        // This is a comment
        /*
            This is a multiline comment
        */
        #! This is a shebang line
        this is normal text
    ");

    assert_eq!(lexer.token, Token::Text);
}

#[test]
fn test_lex_string_literal() {
    let lexer = Token::lexer("\"This is a string literal.\"");

    assert_eq!(lexer.token, Token::StringLiteral);
    assert_eq!(lexer.slice(), "\"This is a string literal.\"");
}

#[test]
fn test_lex_while() {
    let mut lexer = Token::lexer("while nextT <= t2 { }");

    assert_eq!(lexer.token, Token::While);
    lexer.advance();
    assert_eq!(lexer.token, Token::Text);
    lexer.advance();
    assert_eq!(lexer.token, Token::LessThanEquals);
    lexer.advance();
    assert_eq!(lexer.token, Token::Text);
    lexer.advance();
    assert_eq!(lexer.token, Token::OpenBlock);
    lexer.advance();
    assert_eq!(lexer.token, Token::CloseBlock);
    lexer.advance();
}

#[test]
fn test_lex_function_decl() {
    let mut lexer = Token::lexer("fn main() {}");

    assert_eq!(lexer.token, Token::Fn);

    lexer.advance();

    assert_eq!(lexer.token, Token::Text);
    assert_eq!(lexer.slice(), "main");
    
    lexer.advance();

    assert_eq!(lexer.token, Token::OpenParan);

    lexer.advance();

    assert_eq!(lexer.token, Token::CloseParan);

    lexer.advance();

    assert_eq!(lexer.token, Token::OpenBlock);

    lexer.advance();

    assert_eq!(lexer.token, Token::CloseBlock);
}

#[test]
fn test_lex_weird_mod_name() {
    let code = "root::some::other::module::function";

    let mut lexer = Token::lexer(code);

    assert_eq!(lexer.token, Token::Text);
    lexer.advance();
    assert_eq!(lexer.token, Token::DoubleColon);
    lexer.advance();
    assert_eq!(lexer.token, Token::Text);
    lexer.advance();
    assert_eq!(lexer.token, Token::DoubleColon);
    lexer.advance();
    assert_eq!(lexer.token, Token::Text);
    lexer.advance();
    assert_eq!(lexer.token, Token::DoubleColon);
    lexer.advance();
    assert_eq!(lexer.token, Token::Text);
    lexer.advance();
    assert_eq!(lexer.token, Token::DoubleColon);
    lexer.advance();
    assert_eq!(lexer.token, Token::Text);
    lexer.advance();
}