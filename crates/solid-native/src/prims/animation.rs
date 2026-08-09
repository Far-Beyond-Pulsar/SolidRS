//! Animation asset: keyframe channels bound to a skeleton or mesh prim.

use solid_rs::scene::Interpolation;

/// The bone property animated by a channel targeting a skeleton bone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BoneProperty {
    /// Translation (3 values per keyframe).
    Translation,
    /// Rotation quaternion (4 values per keyframe).
    Rotation,
    /// Scale (3 values per keyframe).
    Scale,
}

impl BoneProperty {
    /// Number of values per keyframe for this property.
    pub fn value_count(self) -> usize {
        match self {
            Self::Translation | Self::Scale => 3,
            Self::Rotation => 4,
        }
    }
}

/// The scene-graph property a channel animates.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AnimTargetAsset {
    /// A bone in the referenced skeleton prim, indexed by bone index.
    Bone { bone: usize, property: BoneProperty },
    /// A morph-target weight on the referenced mesh prim.
    MorphWeight { target_index: usize },
    /// An arbitrary engine-defined animated property.
    Custom(String),
}

/// One animated property with its keyframe data.
#[derive(Debug, Clone)]
pub struct AnimChannelAsset {
    /// The property and target being animated.
    pub target: AnimTargetAsset,
    /// Interpolation mode.
    pub interpolation: Interpolation,
    /// Keyframe timestamps in seconds (monotonically increasing).
    pub times: Vec<f32>,
    /// Flat buffer of keyframe values (stride depends on target /
    /// interpolation — see [`solid_rs::scene::AnimationChannel`]).
    pub values: Vec<f32>,
}

impl AnimChannelAsset {
    /// Number of keyframes.
    pub fn keyframe_count(&self) -> usize {
        self.times.len()
    }

    /// Duration of the channel in seconds.
    pub fn duration(&self) -> f32 {
        self.times.last().copied().unwrap_or(0.0)
    }
}

/// A named animation clip composed of channels.
#[derive(Debug, Clone, Default)]
pub struct AnimationAsset {
    /// Prim ID of the [`SkeletonAsset`](crate::prims::SkeletonAsset) bound by
    /// [`AnimTargetAsset::Bone`] targets.
    pub skeleton: Option<String>,
    /// Prim ID of the mesh / skeletal mesh bound by
    /// [`AnimTargetAsset::MorphWeight`] targets.
    pub mesh: Option<String>,
    /// Optional explicit clip duration; `None` derives from the channels.
    pub duration: Option<f32>,
    /// All animated channels belonging to this clip.
    pub channels: Vec<AnimChannelAsset>,
}

impl AnimationAsset {
    /// Creates an empty animation clip.
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends a channel.
    pub fn push_channel(&mut self, channel: AnimChannelAsset) -> &mut Self {
        self.channels.push(channel);
        self
    }

    /// Total duration of the clip in seconds.
    pub fn duration(&self) -> f32 {
        self.duration
            .unwrap_or_else(|| self.channels.iter().map(|c| c.duration()).fold(0.0, f32::max))
    }
}
