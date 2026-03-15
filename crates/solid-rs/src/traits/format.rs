//! Format descriptor type.

/// Static metadata describing a 3D file format implementation.
///
/// Every [`Loader`](crate::traits::Loader) and [`Saver`](crate::traits::Saver)
/// returns a reference to one of these from `format_info()`.
#[derive(Debug, Clone)]
pub struct FormatInfo {
    /// Full human-readable name, e.g. `"Wavefront OBJ"`.
    pub name: &'static str,

    /// Short lowercase identifier used for programmatic lookup,
    /// e.g. `"obj"`, `"fbx"`, `"gltf"`.
    pub id: &'static str,

    /// File extensions (without leading dot) handled by this implementation,
    /// e.g. `&["obj"]` or `&["gltf", "glb"]`.
    pub extensions: &'static [&'static str],

    /// MIME types associated with this format,
    /// e.g. `&["model/obj"]` or `&["model/gltf+json", "model/gltf-binary"]`.
    pub mime_types: &'static [&'static str],

    /// Whether this implementation can **load** files of this format.
    pub can_load: bool,

    /// Whether this implementation can **save** files of this format.
    pub can_save: bool,

    /// Version of the format specification targeted by this implementation.
    /// `None` if not applicable.
    pub spec_version: Option<&'static str>,
}

impl FormatInfo {
    /// Returns `true` if `ext` (without leading dot) matches any registered
    /// extension, compared case-insensitively.
    pub fn matches_extension(&self, ext: &str) -> bool {
        self.extensions.iter().any(|e| e.eq_ignore_ascii_case(ext))
    }

    /// Returns `true` if `mime` matches any registered MIME type,
    /// compared case-insensitively.
    pub fn matches_mime(&self, mime: &str) -> bool {
        self.mime_types.iter().any(|m| m.eq_ignore_ascii_case(mime))
    }
}
