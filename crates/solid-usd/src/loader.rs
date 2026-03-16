//! USDA file loader.

use std::io::Read;

use solid_rs::{
    SolidError,
    scene::scene::Scene,
    traits::{FormatInfo, LoadOptions, Loader, ReadSeek},
};

use crate::{USD_FORMAT, convert, parser};

pub struct UsdLoader;

impl Loader for UsdLoader {
    fn format_info(&self) -> &'static FormatInfo {
        &USD_FORMAT
    }

    fn detect(&self, reader: &mut dyn Read) -> f32 {
        let mut buf = [0u8; 10];
        let n = reader.read(&mut buf).unwrap_or(0);
        let s = &buf[..n];
        // USDA starts with `#usda `
        if s.starts_with(b"#usda ") { return 0.95; }
        // USDC binary magic
        if s.starts_with(b"PXR-USDC") { return 0.0; } // we can't read binary USD
        // USDZ (zip)
        if s.starts_with(b"PK\x03\x04") { return 0.0; }
        0.0
    }

    fn load(&self, reader: &mut dyn ReadSeek, _options: &LoadOptions) -> Result<Scene, SolidError> {
        let mut data = Vec::new();
        reader.read_to_end(&mut data).map_err(SolidError::Io)?;

        let src = std::str::from_utf8(&data)
            .map_err(|e| SolidError::parse(format!("USDA is not valid UTF-8: {e}")))?;

        // Reject non-USDA (binary USDC / USDZ) early
        if src.trim_start().starts_with("PXR-USDC") {
            return Err(SolidError::unsupported(
                "USDC (binary USD) is not supported; convert to USDA first",
            ));
        }

        let doc = parser::parse(src)?;
        convert::doc_to_scene(&doc)
    }
}
