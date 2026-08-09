//! Solid Native document format for SolidRS.
//!
//! A self-describing, lossless container for 3D assets with two on-disk
//! encodings:
//!
//! | Extension | Encoding | Description                              |
//! |-----------|----------|------------------------------------------|
//! | `.slda`   | ASCII    | Human-readable, diff-friendly, git-friendly |
//! | `.sldb`   | Binary   | Compact, fast to write and read          |
//!
//! Both encodings carry the exact same schema — a [`SolidDocument`] that
//! holds any number of *prims* (individual 3D assets) plus a top-level
//! key/value table of arbitrary properties.
//!
//! ```
//! use solid_native::{SolidDocument, Prim, save_ascii, load_ascii};
//! use solid_native::prims::{MeshAsset, PrimitiveAsset};
//! use solid_rs::geometry::{Vertex, Topology};
//!
//! let mut mesh = MeshAsset::new();
//! mesh.vertices = vec![
//!     Vertex::new(glam::vec3(0.0, 1.0, 0.0)),
//!     Vertex::new(glam::vec3(-1.0, -1.0, 0.0)),
//!     Vertex::new(glam::vec3(1.0, -1.0, 0.0)),
//! ];
//! mesh.primitives = vec![PrimitiveAsset::triangles(vec![0, 1, 2], None)];
//!
//! let mut doc = SolidDocument::named("Triangle");
//! doc.props.insert("author".into(), "SolidRS".into());
//! doc.push(Prim::mesh("tri", "Triangle", mesh));
//!
//! let mut ascii_out = Vec::new();
//! save_ascii(&doc, &mut ascii_out).unwrap();   // .slda
//!
//! let back = load_ascii(&mut ascii_out.as_slice()).unwrap();
//! assert_eq!(back.prims.len(), 1);
//! ```
//!
//! The [`Registry`](solid_rs::registry::Registry) integration converts
//! through [`Scene`](solid_rs::scene::Scene); see [`SldaLoader`],
//! [`SldaSaver`], [`SldbLoader`] and [`SldbSaver`].

pub mod doc;
pub mod prims;

mod ascii;
mod binary;
mod convert;
mod loader;
mod saver;
mod tree;

pub use doc::{Prim, PrimData, Props, SolidDocument};
pub use loader::{SldaLoader, SldbLoader};
pub use saver::{SldaSaver, SldbSaver};
pub use solid_rs::error::Result;

use std::io::{Read, Write};

/// Writes `doc` in the ASCII (`.slda`) encoding.
pub fn save_ascii<W: Write>(doc: &SolidDocument, writer: &mut W) -> Result<()> {
    let node = tree::encode::document_to_tree(doc)?;
    ascii::write(&node, writer)
}

/// Reads a document from the ASCII (`.slda`) encoding.
pub fn load_ascii<R: Read>(reader: &mut R) -> Result<SolidDocument> {
    let mut buf = Vec::new();
    reader.read_to_end(&mut buf)?;
    let node = ascii::parse(&buf)?;
    tree::decode::tree_to_document(&node)
}

/// Writes `doc` in the binary (`.sldb`) encoding.
pub fn save_binary<W: Write>(doc: &SolidDocument, writer: &mut W) -> Result<()> {
    let node = tree::encode::document_to_tree(doc)?;
    binary::write(&node, writer)
}

/// Reads a document from the binary (`.sldb`) encoding.
pub fn load_binary<R: Read>(reader: &mut R) -> Result<SolidDocument> {
    let mut buf = Vec::new();
    reader.read_to_end(&mut buf)?;
    let node = binary::read(&buf)?;
    tree::decode::tree_to_document(&node)
}
