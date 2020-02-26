#[cfg(feature = "derive")]
extern crate oxlex_derive as derive;
extern crate regex;
extern crate lazy_static;

pub mod lexer;

pub mod source;

pub mod lexable;

#[cfg(test)]
mod test;

pub mod prelude {
    pub use crate::lexer::Lexer;
    pub use crate::lexable::Lexable;
    pub use crate::source::Source;
    #[cfg(feature = "derive")]
    pub use crate::derive::Lexable;
    pub use crate::regex::Regex;
    pub use crate::lazy_static::lazy_static;
}