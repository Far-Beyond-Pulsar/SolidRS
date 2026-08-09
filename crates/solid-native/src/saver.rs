//! [`Saver`](solid_rs::traits::Saver) implementations for the `.slda` and
//! `.sldb` encodings.  Both save a [`Scene`](solid_rs::scene::Scene) by
//! wrapping it in a document with a single `scene` prim.

use std::io::Write;

use solid_rs::error::Result;
use solid_rs::scene::Scene;
use solid_rs::traits::{FormatInfo, SaveOptions, Saver};

use crate::convert;

static FMT_SLDA: FormatInfo = FormatInfo {
    name: "Solid Native ASCII",
    id: "slda",
    extensions: &["slda"],
    mime_types: &["text/x-slda"],
    can_load: false,
    can_save: true,
    spec_version: Some("1"),
};

static FMT_SLDB: FormatInfo = FormatInfo {
    name: "Solid Native Binary",
    id: "sldb",
    extensions: &["sldb"],
    mime_types: &["application/x-sldb"],
    can_load: false,
    can_save: true,
    spec_version: Some("1"),
};

/// Saves `.slda` files.
#[derive(Debug, Clone, Copy, Default)]
pub struct SldaSaver;

impl Saver for SldaSaver {
    fn save(&self, scene: &Scene, writer: &mut dyn Write, _options: &SaveOptions) -> Result<()> {
        let doc = convert::scene_to_document(scene);
        let node = crate::tree::encode::document_to_tree(&doc);
        crate::ascii::write(&node, writer)
    }

    fn format_info(&self) -> &FormatInfo {
        &FMT_SLDA
    }
}

/// Saves `.sldb` files.
#[derive(Debug, Clone, Copy, Default)]
pub struct SldbSaver;

impl Saver for SldbSaver {
    fn save(&self, scene: &Scene, writer: &mut dyn Write, _options: &SaveOptions) -> Result<()> {
        let doc = convert::scene_to_document(scene);
        let node = crate::tree::encode::document_to_tree(&doc);
        crate::binary::write(&node, writer)
    }

    fn format_info(&self) -> &FormatInfo {
        &FMT_SLDB
    }
}
