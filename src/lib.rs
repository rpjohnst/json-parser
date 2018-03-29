#![feature(slice_patterns)]

pub use parse::Parse;

mod parse;
pub(crate) mod lex;
pub mod json;
