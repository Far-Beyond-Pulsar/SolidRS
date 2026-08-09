//! The top-level document model: [`SolidDocument`], [`Prim`] and [`PrimData`].

pub mod prim;

pub use prim::{Prim, PrimData, PrimKind};

use std::collections::HashMap;

use solid_rs::value::Value;

/// Arbitrary key/value properties attached to a document or a prim.
///
/// Values are dynamically typed via [`Value`] (Bool, Int, Float, String,
/// Vec2/3/4, Bytes, Array, Map).
pub type Props = HashMap<String, Value>;

/// A Solid Native document — a named collection of [`Prim`]s with an arbitrary
/// property table.
///
/// This is the top-level object serialised to `.slda` (ASCII) or `.sldb`
/// (binary) by [`crate::save_ascii`] / [`crate::save_binary`].
#[derive(Debug, Clone, Default)]
pub struct SolidDocument {
    /// Document name (may be empty).
    pub name: String,
    /// Arbitrary document-level key/value properties.
    pub props: Props,
    /// The prims stored in this document, in insertion order.
    pub prims: Vec<Prim>,
}

impl SolidDocument {
    /// Creates an empty, unnamed document.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an empty document with the given name.
    pub fn named(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    /// Appends a prim.
    pub fn push(&mut self, prim: Prim) -> &mut Self {
        self.prims.push(prim);
        self
    }

    /// Shared slice of all prims.
    pub fn prims(&self) -> &[Prim] {
        &self.prims
    }

    /// Mutable slice of all prims.
    pub fn prims_mut(&mut self) -> &mut Vec<Prim> {
        &mut self.prims
    }

    /// Finds a prim by ID.
    pub fn find(&self, id: &str) -> Option<&Prim> {
        self.prims.iter().find(|p| p.id == id)
    }

    /// Finds a prim by ID (mutable).
    pub fn find_mut(&mut self, id: &str) -> Option<&mut Prim> {
        self.prims.iter_mut().find(|p| p.id == id)
    }

    /// All prim IDs in document order.
    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.prims.iter().map(|p| p.id.as_str())
    }

    /// Number of prims.
    pub fn len(&self) -> usize {
        self.prims.len()
    }

    /// Returns `true` if the document holds no prims.
    pub fn is_empty(&self) -> bool {
        self.prims.is_empty()
    }

    /// Validates the document: prim IDs must be unique and every cross-prim
    /// reference (material → texture, skeletal mesh → skeleton, animation →
    /// skeleton/mesh, primitive → material) must resolve to an existing prim
    /// of the right kind.
    ///
    /// Called automatically by [`crate::save_ascii`] and [`crate::save_binary`]
    /// so invalid documents fail at write time.
    pub fn validate(&self) -> crate::Result<()> {
        let mut seen = HashMap::with_capacity(self.prims.len());
        for prim in &self.prims {
            if seen.insert(prim.id.clone(), ()).is_some() {
                return Err(solid_rs::SolidError::invalid_ref(format!(
                    "duplicate prim id '{}'",
                    prim.id
                )));
            }
        }

        for prim in &self.prims {
            prim.validate(&seen)?;
        }
        Ok(())
    }
}
