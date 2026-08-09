//! The format registry — dynamic loader/saver selection by extension or MIME type.
//!
//! [`Registry`] is the primary entry-point for applications that want to load
//! or save 3D files without hard-coding a format crate.
//!
//! # Example
//!
//! ```rust,no_run
//! use solid_rs::registry::Registry;
//! use solid_rs::traits::LoadOptions;
//!
//! // Assume `solid_obj` crate is available:
//! // use solid_obj::ObjLoader;
//!
//! let mut registry = Registry::new();
//! // registry.register_loader(ObjLoader::default());
//!
//! // let scene = registry.load_file("model.obj").unwrap();
//! // println!("meshes: {}", scene.meshes.len());
//! ```

use std::path::Path;
use std::sync::Arc;

use crate::error::{Result, SolidError};
use crate::parallel::Parallelism;
use crate::scene::scene::Scene;
use crate::traits::{FormatInfo, LoadOptions, Loader, ReadSeek, SaveOptions, Saver};

/// Dynamic registry of [`Loader`] and [`Saver`] implementations.
///
/// Format crates register themselves at runtime; the registry then selects the
/// correct implementation based on file extension, MIME type, or magic bytes.
#[derive(Default)]
pub struct Registry {
    loaders: Vec<Arc<dyn Loader>>,
    savers: Vec<Arc<dyn Saver>>,
}

impl Registry {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    // ── Registration ─────────────────────────────────────────────────────────

    /// Registers a loader implementation.  Returns `&mut self` for chaining.
    pub fn register_loader(&mut self, loader: impl Loader) -> &mut Self {
        self.loaders.push(Arc::new(loader));
        self
    }

    /// Registers a saver implementation.  Returns `&mut self` for chaining.
    pub fn register_saver(&mut self, saver: impl Saver) -> &mut Self {
        self.savers.push(Arc::new(saver));
        self
    }

    // ── Lookup ───────────────────────────────────────────────────────────────

    /// Finds a registered loader by its short format ID (e.g. `"obj"`).
    pub fn loader_by_id(&self, id: &str) -> Option<&dyn Loader> {
        self.loaders
            .iter()
            .find(|l| l.format_info().id.eq_ignore_ascii_case(id))
            .map(Arc::as_ref)
    }

    /// Finds a registered loader by file extension (without leading dot).
    pub fn loader_for_extension(&self, ext: &str) -> Option<&dyn Loader> {
        self.loaders
            .iter()
            .find(|l| l.format_info().matches_extension(ext))
            .map(Arc::as_ref)
    }

    /// Finds a registered loader by MIME type.
    pub fn loader_for_mime(&self, mime: &str) -> Option<&dyn Loader> {
        self.loaders
            .iter()
            .find(|l| l.format_info().matches_mime(mime))
            .map(Arc::as_ref)
    }

    /// Finds a registered saver by its short format ID.
    pub fn saver_by_id(&self, id: &str) -> Option<&dyn Saver> {
        self.savers
            .iter()
            .find(|s| s.format_info().id.eq_ignore_ascii_case(id))
            .map(Arc::as_ref)
    }

    /// Finds a registered saver by file extension (without leading dot).
    pub fn saver_for_extension(&self, ext: &str) -> Option<&dyn Saver> {
        self.savers
            .iter()
            .find(|s| s.format_info().matches_extension(ext))
            .map(Arc::as_ref)
    }

    // ── Convenience file I/O ─────────────────────────────────────────────────

    /// Loads a scene from the file at `path`, selecting a loader by extension.
    pub fn load_file(&self, path: impl AsRef<Path>) -> Result<Scene> {
        self.load_file_with_options(path, &LoadOptions::default())
    }

    /// Loads a scene from `path` with caller-supplied [`LoadOptions`].
    pub fn load_file_with_options(
        &self,
        path: impl AsRef<Path>,
        options: &LoadOptions,
    ) -> Result<Scene> {
        let path = path.as_ref();
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| SolidError::UnsupportedFormat("no file extension".into()))?;

        let loader = self
            .loader_for_extension(ext)
            .ok_or_else(|| SolidError::UnsupportedFormat(format!("no loader for .{ext}")))?;

        let file = std::fs::File::open(path).map_err(SolidError::Io)?;
        let mut reader = std::io::BufReader::new(file);
        loader.load(&mut reader, options)
    }

    /// Saves `scene` to the file at `path`, selecting a saver by extension.
    pub fn save_file(&self, scene: &Scene, path: impl AsRef<Path>) -> Result<()> {
        self.save_file_with_options(scene, path, &SaveOptions::default())
    }

    /// Saves `scene` to `path` with caller-supplied [`SaveOptions`].
    pub fn save_file_with_options(
        &self,
        scene: &Scene,
        path: impl AsRef<Path>,
        options: &SaveOptions,
    ) -> Result<()> {
        let path = path.as_ref();
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| SolidError::UnsupportedFormat("no file extension".into()))?;

        let saver = self
            .saver_for_extension(ext)
            .ok_or_else(|| SolidError::UnsupportedFormat(format!("no saver for .{ext}")))?;

        let file = std::fs::File::create(path).map_err(SolidError::Io)?;
        let mut writer = std::io::BufWriter::new(file);
        saver.save(scene, &mut writer, options)
    }

    // ── Batch file I/O ────────────────────────────────────────────────────────

    /// Loads every file in `paths` into a [`Scene`], preserving input order.
    ///
    /// Equivalent to calling [`load_file_with_options`](Self::load_file_with_options)
    /// once per path. With the [`parallel`](crate::parallel) feature enabled and
    /// `options.num_threads` not forcing serial (`Some(1)`), files are decoded
    /// concurrently and the result vector stays in the same order as `paths`.
    ///
    /// On failure, the first error is returned wrapped in
    /// [`SolidError::Batch`] with the offending path.
    pub fn load_files<P: AsRef<Path> + Send + Sync>(
        &self,
        paths: &[P],
        options: &LoadOptions,
    ) -> Result<Vec<Scene>> {
        let plan = Parallelism::from_num_threads(options.num_threads);
        let results: Vec<Result<Scene>> =
            plan.map(paths, |p| self.load_file_with_options(p, options));
        let mut scenes = Vec::with_capacity(results.len());
        for (path, result) in paths.iter().zip(results) {
            scenes.push(result.map_err(|e| SolidError::batch(path.as_ref(), e))?);
        }
        Ok(scenes)
    }

    /// Saves each scene to its matching path (zip), with the last file's
    /// errors reported with their path.
    ///
    /// `paths` and `scenes` must have the same length. With the
    /// [`parallel`](crate::parallel) feature enabled and `options.num_threads`
    /// not forcing serial (`Some(1)`), saves run concurrently.
    pub fn save_files<'a, P: AsRef<Path> + Send + Sync>(
        &self,
        scenes: &[&'a Scene],
        paths: &[P],
        options: &SaveOptions,
    ) -> Result<()> {
        if scenes.len() != paths.len() {
            return Err(SolidError::other(format!(
                "save_files: {} scenes but {} paths",
                scenes.len(),
                paths.len()
            )));
        }
        let plan = Parallelism::from_num_threads(options.num_threads);
        let pairs: Vec<(&Scene, &P)> = scenes.iter().copied().zip(paths.iter()).collect();
        let results: Vec<Result<()>> = plan.map(&pairs, |(scene, path)| {
            self.save_file_with_options(*scene, (*path).as_ref(), options)
        });
        for result in results {
            result?;
        }
        Ok(())
    }

    /// Loads every file in `paths` using configurator [`OptionValues`].
    ///
    /// The global [`configurator::keys::THREADS`](crate::configurator::keys::THREADS)
    /// / [`configurator::keys::PARALLEL`](crate::configurator::keys::PARALLEL)
    /// values are honoured automatically and drive concurrent batch decoding.
    #[cfg(feature = "configurator")]
    pub fn load_files_configured<P: AsRef<Path> + Send + Sync>(
        &self,
        paths: &[P],
        values: &crate::configurator::OptionValues,
    ) -> Result<Vec<Scene>> {
        self.load_files(paths, &values.to_load_options())
    }

    /// Loads a scene from an already-open reader using the loader for `format_id`.
    pub fn load_from<R: ReadSeek>(
        &self,
        mut reader: R,
        format_id: &str,
        options: &LoadOptions,
    ) -> Result<Scene> {
        let loader = self
            .loader_by_id(format_id)
            .ok_or_else(|| SolidError::UnsupportedFormat(format!("no loader for '{format_id}'")))?;
        loader.load(&mut reader, options)
    }

    /// Returns the import-options schema advertised by the loader for `ext`
    /// (without leading dot), or `None` if no loader handles that extension.
    ///
    /// The global parallelism options ([`configurator::keys::THREADS`](crate::configurator::keys::THREADS),
    /// [`configurator::keys::PARALLEL`](crate::configurator::keys::PARALLEL)) are
    /// appended automatically, so format crates never need to define them.
    #[cfg(feature = "configurator")]
    pub fn options_schema_for_extension(
        &self,
        ext: &str,
    ) -> Option<crate::configurator::OptionsSchema> {
        self.loader_for_extension(ext)
            .map(|l| l.options_schema().with_global_fields())
    }

    /// Loads a scene from `path` using configurator [`OptionValues`], selecting a
    /// loader by extension.
    #[cfg(feature = "configurator")]
    pub fn load_file_configured(
        &self,
        path: impl AsRef<Path>,
        values: &crate::configurator::OptionValues,
    ) -> Result<Scene> {
        let path = path.as_ref();
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| SolidError::UnsupportedFormat("no file extension".into()))?;

        let loader = self
            .loader_for_extension(ext)
            .ok_or_else(|| SolidError::UnsupportedFormat(format!("no loader for .{ext}")))?;

        let file = std::fs::File::open(path).map_err(SolidError::Io)?;
        let mut reader = std::io::BufReader::new(file);
        loader.load_configured(&mut reader, values)
    }

    /// Loads a scene from an already-open reader using configurator
    /// [`OptionValues`] and the loader for `format_id`.
    #[cfg(feature = "configurator")]
    pub fn load_from_configured<R: ReadSeek>(
        &self,
        mut reader: R,
        format_id: &str,
        values: &crate::configurator::OptionValues,
    ) -> Result<Scene> {
        let loader = self
            .loader_by_id(format_id)
            .ok_or_else(|| SolidError::UnsupportedFormat(format!("no loader for '{format_id}'")))?;
        loader.load_configured(&mut reader, values)
    }

    // ── Introspection ────────────────────────────────────────────────────────

    /// Returns an iterator over the [`FormatInfo`] of every registered loader.
    pub fn loader_infos(&self) -> impl Iterator<Item = &FormatInfo> {
        self.loaders.iter().map(|l| l.format_info())
    }

    /// Returns an iterator over the [`FormatInfo`] of every registered saver.
    pub fn saver_infos(&self) -> impl Iterator<Item = &FormatInfo> {
        self.savers.iter().map(|s| s.format_info())
    }

    /// Returns `true` if at least one loader is registered for `ext`.
    pub fn can_load_extension(&self, ext: &str) -> bool {
        self.loader_for_extension(ext).is_some()
    }

    /// Returns `true` if at least one saver is registered for `ext`.
    pub fn can_save_extension(&self, ext: &str) -> bool {
        self.saver_for_extension(ext).is_some()
    }
}
