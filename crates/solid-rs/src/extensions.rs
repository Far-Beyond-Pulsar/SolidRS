//! Typed extension property bag for attaching format-specific data to any
//! scene object.
//!
//! [`Extensions`] is analogous to a heterogeneous `HashMap<TypeId, Box<dyn Any>>`.
//! Format crates define their own extension structs and insert/retrieve them
//! without any central registration step.
//!
//! # Example
//!
//! ```rust
//! use solid_rs::extensions::Extensions;
//!
//! /// A hypothetical FBX-specific property block.
//! #[derive(Debug)]
//! struct FbxUserProperties {
//!     user_data: String,
//!     revision: u32,
//! }
//!
//! let mut ext = Extensions::new();
//! ext.insert(FbxUserProperties { user_data: "hello".into(), revision: 3 });
//!
//! let props = ext.get::<FbxUserProperties>().unwrap();
//! assert_eq!(props.revision, 3);
//! ```

use std::any::{Any, TypeId};
use std::collections::HashMap;

/// A type-erased, type-safe property bag for attaching format-specific data
/// to any scene object ([`Node`](crate::scene::Node),
/// [`Mesh`](crate::scene::Mesh), [`Material`](crate::scene::Material), …).
///
/// Only one value of each concrete type `T` can be stored at a time.
#[derive(Debug, Default)]
pub struct Extensions {
    data: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl Extensions {
    /// Creates an empty extension set.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts `value`, replacing any previously stored value of the same type.
    pub fn insert<T: Any + Send + Sync + 'static>(&mut self, value: T) {
        self.data.insert(TypeId::of::<T>(), Box::new(value));
    }

    /// Returns a shared reference to the stored value of type `T`, or `None`.
    pub fn get<T: Any + Send + Sync + 'static>(&self) -> Option<&T> {
        self.data.get(&TypeId::of::<T>())?.downcast_ref()
    }

    /// Returns a mutable reference to the stored value of type `T`, or `None`.
    pub fn get_mut<T: Any + Send + Sync + 'static>(&mut self) -> Option<&mut T> {
        self.data.get_mut(&TypeId::of::<T>())?.downcast_mut()
    }

    /// Removes and returns the stored value of type `T`, or `None`.
    pub fn remove<T: Any + Send + Sync + 'static>(&mut self) -> Option<T> {
        self.data
            .remove(&TypeId::of::<T>())
            .and_then(|b| b.downcast().ok())
            .map(|b| *b)
    }

    /// Returns `true` if a value of type `T` is present.
    #[inline]
    pub fn contains<T: Any + Send + Sync + 'static>(&self) -> bool {
        self.data.contains_key(&TypeId::of::<T>())
    }

    /// Returns `true` if no extensions have been inserted.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Returns the number of distinct extension types stored.
    #[inline]
    pub fn len(&self) -> usize {
        self.data.len()
    }
}

impl Clone for Extensions {
    /// Cloning an [`Extensions`] returns an **empty** bag because `Box<dyn Any>`
    /// is not `Clone`.  Wrap extension types in `Arc<T>` if shallow-clone
    /// semantics are required.
    fn clone(&self) -> Self {
        Self::new()
    }
}
