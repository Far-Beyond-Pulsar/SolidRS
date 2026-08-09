//! Light asset: directional, point, spot and area lights.

use glam::Vec3;

use solid_rs::scene::{
    AreaLight, DirectionalLight, Light, LightBase, PointLight, SpotLight,
};

/// A light source with its own name, colour and intensity.
#[derive(Debug, Clone, PartialEq)]
pub enum LightAsset {
    /// Infinitely-distant parallel light (e.g. a sun).
    Directional {
        /// Linear RGB colour.
        color: Vec3,
        /// Intensity in candela.
        intensity: f32,
    },
    /// Omnidirectional point light.
    Point {
        /// Linear RGB colour.
        color: Vec3,
        /// Intensity in candela.
        intensity: f32,
        /// Maximum range in metres; `None` = infinite.
        range: Option<f32>,
    },
    /// Cone-shaped spot light.
    Spot {
        /// Linear RGB colour.
        color: Vec3,
        /// Intensity in candela.
        intensity: f32,
        /// Maximum range in metres; `None` = infinite.
        range: Option<f32>,
        /// Half-angle (radians) of the inner bright cone.
        inner_cone_angle: f32,
        /// Half-angle (radians) of the outer falloff cone.
        outer_cone_angle: f32,
    },
    /// Rectangular area light.
    Area {
        /// Linear RGB colour.
        color: Vec3,
        /// Intensity in candela.
        intensity: f32,
        /// Width of the emitting rectangle in metres.
        width: f32,
        /// Height of the emitting rectangle in metres.
        height: f32,
    },
}

impl LightAsset {
    /// Creates a white, 1 cd directional light.
    pub fn directional() -> Self {
        Self::Directional {
            color: Vec3::ONE,
            intensity: 1.0,
        }
    }

    /// Creates a white, 1 cd point light with infinite range.
    pub fn point() -> Self {
        Self::Point {
            color: Vec3::ONE,
            intensity: 1.0,
            range: None,
        }
    }

    /// Creates a white, 1 cd spot light with 15° inner / 30° outer cones.
    pub fn spot() -> Self {
        Self::Spot {
            color: Vec3::ONE,
            intensity: 1.0,
            range: None,
            inner_cone_angle: 0.261_799_4,
            outer_cone_angle: 0.523_598_8,
        }
    }

    /// Creates a white, 1 cd 1×1 m area light.
    pub fn area() -> Self {
        Self::Area {
            color: Vec3::ONE,
            intensity: 1.0,
            width: 1.0,
            height: 1.0,
        }
    }

    /// Linear RGB colour of the light.
    pub fn color(&self) -> Vec3 {
        match self {
            Self::Directional { color, .. }
            | Self::Point { color, .. }
            | Self::Spot { color, .. }
            | Self::Area { color, .. } => *color,
        }
    }

    /// Intensity in candela.
    pub fn intensity(&self) -> f32 {
        match self {
            Self::Directional { intensity, .. }
            | Self::Point { intensity, .. }
            | Self::Spot { intensity, .. }
            | Self::Area { intensity, .. } => *intensity,
        }
    }

    /// Converts to a `solid-rs` light.
    pub fn to_solid(&self, name: &str) -> Light {
        let base = LightBase {
            name: name.to_owned(),
            color: self.color(),
            intensity: self.intensity(),
        };
        match self {
            Self::Directional { .. } => Light::Directional(DirectionalLight {
                base,
                extensions: solid_rs::extensions::Extensions::new(),
            }),
            Self::Point { range, .. } => Light::Point(PointLight {
                base,
                range: *range,
                extensions: solid_rs::extensions::Extensions::new(),
            }),
            Self::Spot {
                range,
                inner_cone_angle,
                outer_cone_angle,
                ..
            } => Light::Spot(SpotLight {
                base,
                range: *range,
                inner_cone_angle: *inner_cone_angle,
                outer_cone_angle: *outer_cone_angle,
                extensions: solid_rs::extensions::Extensions::new(),
            }),
            Self::Area { width, height, .. } => Light::Area(AreaLight {
                base,
                width: *width,
                height: *height,
                extensions: solid_rs::extensions::Extensions::new(),
            }),
        }
    }

    /// Converts from a `solid-rs` light.
    pub fn from_solid(light: &Light) -> Self {
        let color = light.color();
        let intensity = light.intensity();
        match light {
            Light::Directional(_) => Self::Directional { color, intensity },
            Light::Point(l) => Self::Point {
                color,
                intensity,
                range: l.range,
            },
            Light::Spot(l) => Self::Spot {
                color,
                intensity,
                range: l.range,
                inner_cone_angle: l.inner_cone_angle,
                outer_cone_angle: l.outer_cone_angle,
            },
            Light::Area(l) => Self::Area {
                color,
                intensity,
                width: l.width,
                height: l.height,
            },
        }
    }
}
