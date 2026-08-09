//! Camera asset: perspective or orthographic projection.

use solid_rs::scene::{Camera, OrthographicCamera, Projection};

/// A camera projection definition.
#[derive(Debug, Clone, PartialEq)]
pub struct CameraAsset {
    /// Projection type and parameters.
    pub projection: Projection,
}

impl CameraAsset {
    /// Creates a perspective camera with default settings.
    pub fn perspective() -> Self {
        Self {
            projection: Projection::default(),
        }
    }

    /// Creates an orthographic camera with default settings.
    pub fn orthographic() -> Self {
        Self {
            projection: Projection::Orthographic(OrthographicCamera::default()),
        }
    }

    /// Converts to a `solid-rs` camera.
    pub fn to_solid(&self) -> Camera {
        Camera {
            name: String::new(),
            projection: self.projection.clone(),
            extensions: solid_rs::extensions::Extensions::new(),
        }
    }

    /// Converts from a `solid-rs` camera.
    pub fn from_solid(cam: &Camera) -> Self {
        Self {
            projection: cam.projection.clone(),
        }
    }
}
