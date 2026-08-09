//! [`Loader`](solid_rs::traits::Loader) implementations for the `.slda` and
//! `.sldb` encodings.  Both load through the document model and then convert
//! to a [`Scene`](solid_rs::scene::Scene).

use std::io::Read;

use solid_rs::error::Result;
use solid_rs::scene::Scene;
use solid_rs::traits::{FormatInfo, LoadOptions, Loader, ReadSeek};

use crate::convert;

static FMT_SLDA: FormatInfo = FormatInfo {
    name: "Solid Native ASCII",
    id: "slda",
    extensions: &["slda"],
    mime_types: &["text/x-slda"],
    can_load: true,
    can_save: false,
    spec_version: Some("1"),
};

static FMT_SLDB: FormatInfo = FormatInfo {
    name: "Solid Native Binary",
    id: "sldb",
    extensions: &["sldb"],
    mime_types: &["application/x-sldb"],
    can_load: true,
    can_save: false,
    spec_version: Some("1"),
};

/// Loads `.slda` files.
#[derive(Debug, Clone, Copy, Default)]
pub struct SldaLoader;

impl Loader for SldaLoader {
    fn load(&self, reader: &mut dyn ReadSeek, _options: &LoadOptions) -> Result<Scene> {
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf)?;
        let node = crate::ascii::parse(&buf)?;
        let doc = crate::tree::decode::tree_to_document(&node)?;
        convert::document_to_scene(&doc)
    }

    fn format_info(&self) -> &FormatInfo {
        &FMT_SLDA
    }

    fn detect(&self, reader: &mut dyn Read) -> f32 {
        let mut buf = [0u8; 5];
        let n = reader.read(&mut buf).unwrap_or(0);
        if n >= 5 && &buf[..5] == b"SLDA " {
            1.0
        } else {
            0.0
        }
    }
}

/// Loads `.sldb` files.
#[derive(Debug, Clone, Copy, Default)]
pub struct SldbLoader;

impl Loader for SldbLoader {
    fn load(&self, reader: &mut dyn ReadSeek, _options: &LoadOptions) -> Result<Scene> {
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf)?;
        let node = crate::binary::read(&buf)?;
        let doc = crate::tree::decode::tree_to_document(&node)?;
        convert::document_to_scene(&doc)
    }

    fn format_info(&self) -> &FormatInfo {
        &FMT_SLDB
    }

    fn detect(&self, reader: &mut dyn Read) -> f32 {
        let mut buf = [0u8; 4];
        let n = reader.read(&mut buf).unwrap_or(0);
        if n >= 4 && &buf[..4] == b"SLDB" {
            1.0
        } else {
            0.0
        }
    }
}
