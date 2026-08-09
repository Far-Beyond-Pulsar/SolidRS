//! Texture asset: an image (URI or embedded bytes) plus sampler state.

use solid_rs::scene::{Image, ImageSource, Sampler, Texture};

/// A single 2-D texture with its own sampler state.
#[derive(Debug, Clone)]
pub struct TextureAsset {
    /// Human-readable name (also used as the image name).
    pub name: String,
    /// Source image data.
    pub image: Image,
    /// Sampling state.
    pub sampler: Sampler,
    /// Source image width in pixels, when known.
    pub width: Option<u32>,
    /// Source image height in pixels, when known.
    pub height: Option<u32>,
}

impl TextureAsset {
    /// Creates a texture that references an external URI.
    pub fn from_uri(name: impl Into<String>, uri: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            image: Image::from_uri("", uri),
            sampler: Sampler::default(),
            width: None,
            height: None,
        }
    }

    /// Creates a texture backed by embedded bytes.
    pub fn embedded(
        name: impl Into<String>,
        mime_type: impl Into<String>,
        data: Vec<u8>,
    ) -> Self {
        Self {
            name: name.into(),
            image: Image::embedded("", mime_type, data),
            sampler: Sampler::default(),
            width: None,
            height: None,
        }
    }

    /// Returns the image MIME type for embedded sources, or `None`.
    pub fn mime(&self) -> Option<&str> {
        match &self.image.source {
            ImageSource::Embedded { mime_type, .. } => Some(mime_type.as_str()),
            ImageSource::Uri(_) => None,
        }
    }

    /// Converts to a `solid-rs` texture referencing `image_index`.
    pub fn to_solid(&self, image_index: usize) -> Texture {
        Texture {
            name: self.name.clone(),
            image_index,
            sampler: self.sampler.clone(),
            extensions: solid_rs::extensions::Extensions::new(),
        }
    }

    /// Converts from a `solid-rs` texture, bundling its image.
    pub fn from_solid(tex: &Texture, image: &Image) -> Self {
        Self {
            name: tex.name.clone(),
            image: image.clone(),
            sampler: tex.sampler.clone(),
            width: None,
            height: None,
        }
    }
}
