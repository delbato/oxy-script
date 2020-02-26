use crate::{
    lexable::Lexable,
    lexer::Lexer,
    source::Source
};

use regex::Regex;
use derive::Lexable;
use lazy_static::lazy_static;

#[derive(Lexable, Clone, Debug, Hash, PartialEq, Eq)]
enum Token {
    #[token = "fn"]
    #[prio = 1]
    Fn,

    #[token = "int"]
    Int,

    #[token = "float"]
    Float,

    #[token = "bool"]
    Bool,

    #[token = "mod"]
    Mod,

    #[token = "/"]
    Divide,

    #[regex = "[0-9]+"]
    IntLiteral,

    #[regex = r"[0-9]+\.[0-9]*"]
    FloatLiteral,

    #[regex = "[a-zA-Z][a-zA-Z0-9]*"]
    Text,

    #[token = ":"]
    Colon,

    #[token = "::"]
    DoubleColon,

    #[token_start = "//"]
    #[token_end = "\n"]
    #[skip]
    SingleLineComment,

    #[token_start = "#"]
    #[token_end = "\n"]
    #[skip]
    HashLineComment,

    #[token_start = "/*"]
    #[token_end = "*/"]
    #[skip]
    MultiLineComment,

    #[end]
    End,

    #[error]
    Error
}

#[test]
fn test_lexer_basic() {
    let code = "bool float int";
    let mut lexer = Token::lexer(code);

    assert_eq!(lexer.token, Token::Bool);
    assert_eq!(lexer.slice(), "bool");
}

#[test]
fn test_lexer_int_literal() {
    let code = "1231232 123331";
    let mut lexer = Token::lexer(code);

    use regex::Regex;

    let regex = Regex::new(r"[0-9]+$").unwrap();
    assert!(regex.is_match("1234"));
    assert!(!regex.is_match("1234 "));

    assert_eq!(lexer.token, Token::IntLiteral);
    assert_eq!(lexer.slice(), "1231232");

    lexer.advance();
    assert_eq!(lexer.token, Token::IntLiteral);
    assert_eq!(lexer.slice(), "123331");
}

#[test]
fn test_lexer_float_literal() {
    let code = "128.774 12 3.14";
    let mut lexer = Token::lexer(code);

    use regex::Regex;

    let int_regex = Regex::new(r"^[0-9]+$").unwrap();
    assert!(int_regex.is_match("1234"));
    assert!(!int_regex.is_match("1234.1234"));

    assert_eq!(lexer.token, Token::FloatLiteral);
    assert_eq!(lexer.slice(), "128.774");

    lexer.advance();
    assert_eq!(lexer.token, Token::IntLiteral);
    assert_eq!(lexer.slice(), "12");

    lexer.advance();
    assert_eq!(lexer.token, Token::FloatLiteral);
    assert_eq!(lexer.slice(), "3.14");
}

#[test]
fn test_lexer_text() {
    let code = "this is some text float is not";
    let mut lexer = Token::lexer(code);

    assert_eq!(lexer.token, Token::Text);
    assert_eq!(lexer.slice(), "this");

    lexer.advance();
    assert_eq!(lexer.token, Token::Text);
    assert_eq!(lexer.slice(), "is");

    lexer.advance();
    assert_eq!(lexer.token, Token::Text);
    assert_eq!(lexer.slice(), "some");

    lexer.advance();
    assert_eq!(lexer.token, Token::Text);
    assert_eq!(lexer.slice(), "text");

    lexer.advance();
    assert_eq!(lexer.token, Token::Float);
    assert_eq!(lexer.slice(), "float");

    lexer.advance();
    assert_eq!(lexer.token, Token::Text);
    assert_eq!(lexer.slice(), "is");

    lexer.advance();
    assert_eq!(lexer.token, Token::Text);
    assert_eq!(lexer.slice(), "not");
}

#[test]
fn test_lexer_keyword_text() {
    let code = "float int thisisntafloat intisntoneeither";
    let mut lexer = Token::lexer(code);

    assert_eq!(lexer.token, Token::Float);
    assert_eq!(lexer.slice(), "float");
    
    lexer.advance();
    assert_eq!(lexer.token, Token::Int);
    assert_eq!(lexer.slice(), "int");

    lexer.advance();
    assert_eq!(lexer.token, Token::Text);
    assert_eq!(lexer.slice(), "thisisntafloat");

    lexer.advance();
    assert_eq!(lexer.token, Token::Text);
    assert_eq!(lexer.slice(), "intisntoneeither");
}

#[test]
fn test_lexer_import_string() {
    let code = "
        root::some::other::module::function
    ";

    let mut lexer = Token::lexer(code);
    
    assert_eq!(lexer.token, Token::Text);
    assert_eq!(lexer.slice(), "root");

    lexer.advance();
    assert_eq!(lexer.token, Token::DoubleColon);
    assert_eq!(lexer.slice(), "::");

    lexer.advance();
    assert_eq!(lexer.token, Token::Text);
    assert_eq!(lexer.slice(), "some");

    lexer.advance();
    assert_eq!(lexer.token, Token::DoubleColon);
    assert_eq!(lexer.slice(), "::");

    lexer.advance();
    assert_eq!(lexer.token, Token::Text);
    assert_eq!(lexer.slice(), "other");

    lexer.advance();
    assert_eq!(lexer.token, Token::DoubleColon);
    assert_eq!(lexer.slice(), "::");

    lexer.advance();
    assert_eq!(lexer.token, Token::Text);
    assert_eq!(lexer.slice(), "module");

    lexer.advance();
    assert_eq!(lexer.token, Token::DoubleColon);
    assert_eq!(lexer.slice(), "::");

    lexer.advance();
    assert_eq!(lexer.token, Token::Text);
    assert_eq!(lexer.slice(), "function");
}

#[test]
fn test_lexer_comments() {
    let code = "
        # Hash line comment
        // Single line comment
        /* multiline comment */
        float
    ";

    use regex::Regex;
    let regex = Regex::new(r"//.*\n").unwrap();
    assert!(regex.is_match("// This is a single line comment\n"));

    let mut lexer = Token::lexer(code);
    assert_eq!(lexer.token, Token::Float);
}

#[test]
fn test_lexer_fn() {
    let code = "fn: main";

    let mut lexer = Token::lexer(code);
    assert_eq!(lexer.token, Token::Fn);
}