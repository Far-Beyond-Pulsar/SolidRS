//! Core extension traits and supporting types.
//!
//! Format crates implement [`Loader`] and/or [`Saver`] and provide a
//! [`FormatInfo`] descriptor.  The optional [`SceneVisitor`] trait enables
//! savers to walk a [`Scene`](crate::scene::Scene) without cloning it.

pub mod format;
pub mod loader;
pub mod saver;
pub mod visitor;

pub use format::FormatInfo;
pub use loader::{LoadOptions, Loader, ReadSeek};
pub use saver::{SaveOptions, Saver};
pub use visitor::SceneVisitor;
