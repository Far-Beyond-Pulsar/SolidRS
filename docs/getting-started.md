# Getting Started with SolidRS

SolidRS is a **generic** 3-D model loading and saving library for Rust.  It is
deliberately format-agnostic: the core crate (`solid-rs`) defines the scene
data model and extension traits; format-specific logic lives in separate crates
(`solid-obj`, `solid-gltf`, `solid-fbx`, …) that you opt into.

---

## Installation

Add the core crate and at least one format crate to your `Cargo.toml`:

```toml
[dependencies]
solid-rs   = "0.1"
solid-obj  = "0.1"    # Wavefront OBJ
# solid-gltf = "0.1"  # glTF 2.0 / GLB
# solid-fbx  = "0.1"  # Autodesk FBX
```

---

## Loading a File

```rust
use solid_rs::prelude::*;

fn main() -> Result<()> {
    let mut registry = Registry::new();
    registry.register_loader(solid_obj::ObjLoader::default());

    let scene = registry.load_file("model.obj")?;

    println!("Loaded '{}': {} meshes, {} materials",
        scene.name, scene.meshes.len(), scene.materials.len());

    Ok(())
}
```

---

## Saving a File

```rust
use solid_rs::prelude::*;

fn main() -> Result<()> {
    let mut registry = Registry::new();
    registry.register_saver(solid_obj::ObjSaver::default());

    let scene = build_scene();   // see the build_scene example
    registry.save_file(&scene, "output.obj")?;

    Ok(())
}
```

---

## Inspecting the Scene

A [`Scene`] is a collection of flat arrays (meshes, materials, textures, …)
plus a node hierarchy:

```rust
for mesh in &scene.meshes {
    println!("  Mesh \"{}\" — {} verts", mesh.name, mesh.vertex_count());
    for prim in &mesh.primitives {
        println!("    {} triangles", prim.element_count());
    }
}

for &root_id in &scene.roots {
    scene.walk_from(root_id, &mut |node| {
        println!("  Node \"{}\"", node.name);
    });
}
```

---

## Using Custom Load Options

```rust
use solid_rs::prelude::*;

let opts = LoadOptions {
    triangulate:      true,
    generate_normals: true,
    merge_vertices:   true,
    ..Default::default()
};

let scene = registry.load_file_with_options("model.obj", &opts)?;
```

---

## Next Steps

- [Architecture](architecture.md) — understand the serde-inspired design.
- [Scene Graph](scene-graph.md) — nodes, meshes, materials, lights, cameras, …
- [Implementing a Loader](implementing-a-loader.md) — write your own format crate.
- [Implementing a Saver](implementing-a-saver.md) — export to a custom format.
- [Format Registry](format-registry.md) — dynamic format selection.
