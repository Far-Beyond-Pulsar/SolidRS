# SolidRS

A **generic 3D model loading and saving library** for Rust — designed on the
same split as `serde` / `serde_json`:

| | Serialisation | 3D models |
|---|---|---|
| **Core** | `serde` | `solid-rs` |
| **Format** | `serde_json`, `serde_yaml` … | `solid-fbx`, `solid-obj` … |

`solid-rs` defines the scene IR, the `Loader`/`Saver` traits, and the format
`Registry`. Format crates implement those traits for a specific file format and
are pulled in à-la-carte.

---

## Crate Ecosystem

| Crate | Status | Description |
|---|---|---|
| [`solid-rs`](crates/solid-rs) | ✅ stable | Core scene types, traits, registry |
| [`solid-fbx`](crates/solid-fbx) | ✅ stable | Autodesk FBX binary + ASCII loader; ASCII 7.4 saver; cameras, lights, vertex colours |
| [`solid-obj`](crates/solid-obj) | ✅ stable | Wavefront OBJ / MTL loader + saver |
| [`solid-gltf`](crates/solid-gltf) | ✅ stable | glTF 2.0 JSON loader + saver; GLB binary load + save |
| [`solid-stl`](crates/solid-stl) | ✅ stable | STL binary + ASCII loader; binary saver (ASCII helper) |
| [`solid-ply`](crates/solid-ply) | ✅ stable | Stanford PLY ASCII + binary LE/BE loader; ASCII + binary LE saver |
| `solid-usd` | 🔜 planned | OpenUSD / USDA / USDC loader + saver |

---

## Quick Start

Add the core crate plus whichever format crates you need:

```toml
[dependencies]
solid-rs   = "0.1"
solid-fbx  = "0.1"   # Autodesk FBX
solid-obj  = "0.1"   # Wavefront OBJ
solid-gltf = "0.1"   # glTF 2.0 / GLB
solid-stl  = "0.1"   # Stereolithography STL
solid-ply  = "0.1"   # Stanford PLY
```

### Load a file

```rust
use solid_rs::prelude::*;
use solid_rs::registry::Registry;
use solid_fbx::FbxLoader;

fn main() -> solid_rs::Result<()> {
    let mut registry = Registry::new();
    registry.register_loader(FbxLoader);

    let scene = registry.load_file("model.fbx")?;
    println!("Loaded {} mesh(es), {} material(s)",
        scene.meshes.len(),
        scene.materials.len());
    Ok(())
}
```

### Save a file

```rust
use solid_rs::prelude::*;
use solid_rs::registry::Registry;
use solid_obj::{ObjLoader, ObjSaver};

fn main() -> solid_rs::Result<()> {
    let mut registry = Registry::new();
    registry.register_loader(ObjLoader);
    registry.register_saver(ObjSaver);

    let scene = registry.load_file("input.obj")?;
    registry.save_file(&scene, "output.obj")?;
    Ok(())
}
```

### Convert between formats

```rust
use solid_rs::prelude::*;
use solid_rs::registry::Registry;
use solid_fbx::FbxLoader;
use solid_obj::ObjSaver;

fn main() -> solid_rs::Result<()> {
    let mut registry = Registry::new();
    registry.register_loader(FbxLoader);
    registry.register_saver(ObjSaver);

    let opts = LoadOptions { triangulate: true, ..Default::default() };
    let scene = registry.load_file_with_options("scene.fbx", &opts)?;
    registry.save_file(&scene, "scene.obj")?;
    Ok(())
}
```

A ready-to-run conversion example lives in
[`examples/fbx-to-obj`](examples/fbx-to-obj/README.md):

```bash
cargo run -p fbx-to-obj -- input.fbx output.obj
```

---

## Architecture

```
┌────────────────────────────────────────────────────┐
│                     solid-rs                       │
│                                                    │
│  Scene ── Node ── Mesh ── Material ── Texture      │
│  Camera ── Light ── Animation ── Skin              │
│                                                    │
│  trait Loader   trait Saver   Registry             │
│  SceneBuilder   LoadOptions   SaveOptions          │
└──────────────────────┬─────────────────────────────┘
                       │  implements
       ┌───────────────┼───────────────────┐
       ▼               ▼                   ▼
   solid-fbx       solid-obj          solid-gltf
   solid-stl       solid-ply          solid-usd …
```

### Scene IR

The intermediate representation is a **flat, index-based scene graph** —
similar in spirit to glTF's document model:

```
Scene
 ├── nodes:     Vec<Node>          (parent refs by NodeId index)
 ├── meshes:    Vec<Mesh>          (Mesh.primitives: Vec<Primitive>)
 ├── materials: Vec<Material>      (PBR metallic-roughness)
 ├── textures:  Vec<Texture>       (URI or embedded bytes)
 ├── images:    Vec<Image>
 ├── cameras:   Vec<Camera>
 ├── lights:    Vec<Light>
 ├── animations:Vec<Animation>
 └── skins:     Vec<Skin>
```

Every cross-reference is an integer index, keeping `Scene` fully `Clone`-able
without `Arc` or reference cycles.

### Traits

```rust
// dyn-compatible — usable as Arc<dyn Loader>
pub trait Loader: Send + Sync {
    fn format_info(&self) -> &'static FormatInfo;
    fn load(&self, reader: &mut dyn ReadSeek, options: &LoadOptions)
        -> Result<Scene>;
}

pub trait Saver: Send + Sync {
    fn format_info(&self) -> &'static FormatInfo;
    fn save(&self, scene: &Scene, writer: &mut dyn Write, options: &SaveOptions)
        -> Result<()>;
}
```

Both traits are **dyn-compatible** — format drivers can be stored in the
registry as boxed trait objects and dispatched at runtime by file extension or
MIME type.

---

## Building

```bash
# Build everything
cargo build --workspace

# Run the full test suite (~350 tests)
cargo test --workspace

# Run the FBX → OBJ converter example
cargo run -p fbx-to-obj -- input.fbx output.obj
```

### Format support matrix

| Format | Load | Save | Notes |
|---|---|---|---|
| FBX (binary) | ✅ | — | 6.1–7.4, 32 + 64-bit offsets |
| FBX (ASCII) | ✅ | ✅ | 7.4; cameras, lights, vertex colours |
| OBJ / MTL | ✅ | ✅ | N-gon fan triangulation; separate MTL save |
| glTF 2.0 JSON | ✅ | ✅ | Embedded base64 buffers; skinning |
| GLB | ✅ | ✅ | Binary glTF; `GltfSaver::save_glb()` |
| STL binary | ✅ | ✅ | Vertex deduplication on load |
| STL ASCII | ✅ | ✅ | `StlSaver::save_ascii()` helper |
| PLY ASCII | ✅ | ✅ | N-gon fan triangulation; point clouds |
| PLY binary LE | ✅ | ✅ | `PlySaver::save_binary_le()` helper |
| PLY binary BE | ✅ | — | Read-only |

---

## Implementing a Format Crate

See the step-by-step guides in [`docs/`](docs/):

| Document | Topic |
|---|---|
| [`getting-started.md`](docs/getting-started.md) | Workspace setup, first load |
| [`implementing-a-loader.md`](docs/implementing-a-loader.md) | Writing a `Loader` |
| [`implementing-a-saver.md`](docs/implementing-a-saver.md) | Writing a `Saver` |
| [`traits-reference.md`](docs/traits-reference.md) | Full trait API reference |
| [`scene-graph.md`](docs/scene-graph.md) | Scene IR deep-dive |
| [`geometry.md`](docs/geometry.md) | Vertex, Primitive, Mesh |
| [`format-registry.md`](docs/format-registry.md) | Registry & dispatch |
| [`error-handling.md`](docs/error-handling.md) | `SolidError` & `Result` |
| [`architecture.md`](docs/architecture.md) | Design decisions |

Minimal loader skeleton:

```rust
use solid_rs::prelude::*;
use solid_rs::traits::{Loader, ReadSeek};

pub static MY_FORMAT: FormatInfo = FormatInfo {
    name:         "My Format",
    id:           "my-format",
    extensions:   &["mfmt"],
    mime_types:   &["model/x-my-format"],
    can_load:     true,
    can_save:     false,
    spec_version: None,
};

pub struct MyLoader;

impl Loader for MyLoader {
    fn format_info(&self) -> &'static FormatInfo { &MY_FORMAT }

    fn load(&self, reader: &mut dyn ReadSeek, _opts: &LoadOptions)
        -> Result<Scene>
    {
        let mut builder = solid_rs::builder::SceneBuilder::new();
        // … parse reader, call builder methods …
        Ok(builder.build())
    }
}
```

---

## License

MIT — see [`LICENSE`](LICENSE).
