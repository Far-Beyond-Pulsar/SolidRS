# Format Registry

The [`Registry`] is a runtime container for [`Loader`] and [`Saver`]
implementations.  Applications populate it at startup and then call
`load_file` / `save_file` without hard-coding a specific format crate.

---

## Creating and Populating a Registry

```rust
use solid_rs::prelude::*;

let mut registry = Registry::new();
registry
    .register_loader(solid_obj::ObjLoader::default())
    .register_saver(solid_obj::ObjSaver::default());
    // .register_loader(solid_gltf::GltfLoader::default())
    // .register_saver(solid_gltf::GltfSaver::default())
```

---

## Loading by File Path

The registry selects a loader based on the file extension:

```rust
let scene = registry.load_file("model.obj")?;
```

With custom options:

```rust
let opts = LoadOptions {
    triangulate:      true,
    generate_normals: true,
    ..Default::default()
};
let scene = registry.load_file_with_options("model.fbx", &opts)?;
```

---

## Saving by File Path

```rust
registry.save_file(&scene, "output.glb")?;
```

With custom options:

```rust
let opts = SaveOptions {
    embed_textures: true,
    pretty_print:   false,
    ..Default::default()
};
registry.save_file_with_options(&scene, "output.gltf", &opts)?;
```

---

## Loading from a Reader

When you already have an open stream (e.g. from a network request):

```rust
use solid_rs::traits::LoadOptions;

let bytes: Vec<u8> = fetch_from_server("model.obj").await?;
let scene = registry.load_from(
    std::io::Cursor::new(bytes),
    "obj",
    &LoadOptions::default(),
)?;
```

---

## Format Lookup

```rust
// Check what's registered
if registry.can_load_extension("fbx") {
    println!("FBX loading is available");
}

// Direct lookup by format ID
if let Some(loader) = registry.loader_by_id("gltf") {
    println!("glTF loader: {}", loader.format_info().name);
}

// Enumerate everything
for info in registry.loader_infos() {
    println!("  load  .{exts}  ({name})",
        exts = info.extensions.join(", ."),
        name = info.name);
}
```

---

## Magic-Byte Auto-Detection

When a `Loader` implements the optional `detect` method the registry can
identify a format from the file contents rather than the extension.  Call
`loader.detect(&mut reader)` directly — the registry does not yet wire this
automatically (a convenience `detect_format` helper is planned):

```rust
use std::fs::File;

let mut file = File::open("unknown_model")?;

let best = registry
    .loader_infos()
    .zip(/* your loaders */)
    .map(|(_, loader)| (loader.detect(&mut file), loader))
    .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
```

---

## Thread Safety

[`Registry`] itself is not `Send + Sync` (it is owned by the thread that
builds it).  All registered [`Loader`] and [`Saver`] implementations **must**
be `Send + Sync + 'static` — this is enforced at compile time by the trait
bounds.

---

## Global / Lazy-Initialized Registry

For applications that want a process-wide registry:

```rust
use std::sync::OnceLock;
use solid_rs::prelude::*;

static REGISTRY: OnceLock<Registry> = OnceLock::new();

fn registry() -> &'static Registry {
    REGISTRY.get_or_init(|| {
        let mut r = Registry::new();
        // r.register_loader(solid_obj::ObjLoader::default());
        r
    })
}
```
