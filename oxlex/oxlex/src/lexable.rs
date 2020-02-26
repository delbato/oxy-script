use crate::{
    lexer::Lexer,
    source::Source
};

use std::{
    hash::{
        Hash
    },
    collections::{
        HashSet
    },
    fmt::Debug
};

pub trait Lexable: Sized + Clone + Eq + Hash + Debug {
    fn lexer<'source, S: Source<'source>>(source: S) -> Lexer<Self, S>;
    fn match_token(slice: &str) -> Vec<Self>;
    fn get_end_variant() -> Self;
    fn get_error_variant() -> Self;
    fn should_skip(&self) -> bool;
    fn is_inclusive(&self) -> bool;
    fn prio(&self) -> i8;
}