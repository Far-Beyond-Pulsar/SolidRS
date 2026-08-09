//! The ASCII (`.slda`) encoding.
//!
//! A `.slda` file is a UTF-8 text document:
//!
//! ```text
//! SLDA 1
//! "name" "Level1"
//! "props" { "author" "Jane" }
//! "prims" [ ... ]
//! ```
//!
//! The grammar is whitespace-insensitive.  Values are:
//!
//! | Syntax            | Node                                  |
//! |-------------------|---------------------------------------|
//! | `null`            | [`DocNode::Null`]                     |
//! | `true` / `false`  | [`DocNode::Bool`]                     |
//! | `-12`             | [`DocNode::Int`]                      |
//! | `0.5` / `1e-3`    | [`DocNode::Float`]                    |
//! | `"text"`          | [`DocNode::String`]                   |
//! | `b64"..."`        | [`DocNode::Bytes`]                    |
//! | `v2(x y)`         | [`DocNode::Vec2`]                     |
//! | `v3(x y z)`       | [`DocNode::Vec3`]                     |
//! | `v4(x y z w)`     | [`DocNode::Vec4`]                     |
//! | `[ a b c ]`       | [`DocNode::Array`]                    |
//! | `{ "k" v ... }`   | [`DocNode::Map`]                      |
//!
//! Lines beginning with `#` are ignored as comments.  A `#` to the end of
//! the line is also treated as a comment wherever it appears.

mod parser;
mod writer;

pub(crate) use parser::parse;
pub(crate) use writer::write;
