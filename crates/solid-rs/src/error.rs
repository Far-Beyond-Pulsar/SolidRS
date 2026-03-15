//! Error types for SolidRS loaders and savers.

use thiserror::Error;

/// The unified error type returned by all SolidRS loaders and savers.
///
/// Format extension crates should map their internal errors into this type,
/// using [`SolidError::format`] for errors that are specific to one format and
/// do not fit the other variants.
#[derive(Debug, Error)]
pub enum SolidError {
    /// An I/O error that occurred while reading or writing.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// The byte stream is syntactically invalid for the expected format.
    #[error("parse error: {0}")]
    Parse(String),

    /// The format is recognised but a specific feature is not supported by
    /// this implementation (e.g. compressed textures, proprietary extensions).
    #[error("unsupported feature: {0}")]
    UnsupportedFeature(String),

    /// No loader or saver has been registered for the requested format.
    #[error("unsupported format: {0}")]
    UnsupportedFormat(String),

    /// An index reference (node ID, mesh index, material index, …) is out of
    /// bounds or points to a non-existent object.
    #[error("invalid reference: {0}")]
    InvalidReference(String),

    /// The scene data is semantically invalid — for example a cyclic node
    /// hierarchy or a skin that references missing joints.
    #[error("invalid scene: {0}")]
    InvalidScene(String),

    /// A format-specific error emitted by an extension crate.
    ///
    /// Construct via [`SolidError::format`].
    #[error("format error ({format}): {message}")]
    Format {
        /// Short identifier of the originating format, e.g. `"fbx"`.
        format: String,
        /// Human-readable error description.
        message: String,
    },

    /// Any other error that does not fit the above categories.
    #[error("error: {0}")]
    Other(String),
}

impl SolidError {
    /// Creates a [`SolidError::Parse`] with the given message.
    #[inline]
    pub fn parse(msg: impl Into<String>) -> Self {
        Self::Parse(msg.into())
    }

    /// Creates a [`SolidError::UnsupportedFeature`] with the given message.
    #[inline]
    pub fn unsupported(msg: impl Into<String>) -> Self {
        Self::UnsupportedFeature(msg.into())
    }

    /// Creates a [`SolidError::Format`] error intended for use inside
    /// extension crates.
    ///
    /// # Example
    ///
    /// ```rust
    /// use solid_rs::SolidError;
    ///
    /// let err = SolidError::format("obj", "missing 'v' keyword on line 3");
    /// ```
    #[inline]
    pub fn format(format: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Format {
            format: format.into(),
            message: message.into(),
        }
    }

    /// Creates a [`SolidError::InvalidReference`] with the given message.
    #[inline]
    pub fn invalid_ref(msg: impl Into<String>) -> Self {
        Self::InvalidReference(msg.into())
    }

    /// Creates a [`SolidError::Other`] with the given message.
    #[inline]
    pub fn other(msg: impl Into<String>) -> Self {
        Self::Other(msg.into())
    }
}

/// Convenience `Result` alias used throughout SolidRS.
pub type Result<T, E = SolidError> = std::result::Result<T, E>;
