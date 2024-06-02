#![doc = include_str!(env!("README_PATH"))]
#![deny(missing_debug_implementations)]

mod element;
mod parser;
mod tokenizer;

pub use element::*;
pub use nxml_rs_macros::*;
pub use parser::*;
