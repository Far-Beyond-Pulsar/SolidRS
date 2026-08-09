# solid-native

Solid Native document format for [SolidRS](../solid-rs): a self-describing,
lossless container for 3D assets with two on-disk encodings:

| Extension | Encoding  | Description                                    |
|-----------|-----------|------------------------------------------------|
| `.slda`   | ASCII     | Human-readable, diff-friendly, git-friendly    |
| `.sldb`   | Binary    | Compact, fast to write and read                |

Both encodings carry the **exact same schema** — a `SolidDocument` that holds
any number of *prims* (individual 3D assets) plus a top-level key/value table
of arbitrary properties.

## Prims

A document holds any combination of:

| Prim            | Description                                              |
|-----------------|----------------------------------------------------------|
| `mesh`          | Vertex buffer + indexed primitives + morph targets        |
| `skeleton`      | Bone hierarchy (local transforms) + inverse bind matrices |
| `skeletal_mesh` | Mesh skinned to a `skeleton` prim by bone name            |
| `material`      | PBR material, texture slots bound by prim ID              |
| `texture`       | Image (URI or embedded bytes) + sampler state             |
| `animation`     | Keyframe channels bound to a skeleton / mesh prim         |
| `camera`        | Perspective or orthographic projection                    |
| `light`         | Directional / point / spot / area                         |
| `scene`         | A complete `solid-rs` scene graph                         |

Prims reference each other by stable string IDs, so assets can be stored,
displayed and loaded individually while still binding together in a game
engine (e.g. a `skeletal_mesh` prim references its `skeleton` prim, a
`material` prim references its `texture` prims, an `animation` prim
references the prim it animates).

Every prim and every document also carries its own `props` table — arbitrary
key/value data (`Value` from `solid-rs`): Bool, Int, Float, String, Vec2/3/4,
Bytes, Arrays and nested Maps.

## Quick start

```rust
use solid_native::{SolidDocument, Prim, PrimData, save_ascii, load_ascii,
                   save_binary, load_binary};
use solid_native::prims::{MeshAsset, PrimitiveAsset};
use solid_rs::geometry::{Vertex, Primitive, Topology};

let mut mesh = MeshAsset::new();
mesh.vertices = vec![
    Vertex::new(glam::vec3(0.0, 1.0, 0.0)),
    Vertex::new(glam::vec3(-1.0, -1.0, 0.0)),
    Vertex::new(glam::vec3(1.0, -1.0, 0.0)),
];
mesh.primitives = vec![PrimitiveAsset::triangles(vec![0, 1, 2], None)];

let mut doc = SolidDocument::named("Triangle");
doc.props.insert("author".into(), "SolidRS".into());
doc.push(Prim::mesh("tri", "Triangle", mesh));

let mut ascii_out = Vec::new();
save_ascii(&doc, &mut ascii_out)?;          // .slda

let mut binary_out = Vec::new();
save_binary(&doc, &mut binary_out)?;        // .sldb

let back = load_ascii(&mut ascii_out.as_slice())?;
assert_eq!(back.prims.len(), 1);
# Ok::<(), solid_rs::SolidError>(())
```

## Using the registry

`solid-native` integrates with the `solid-rs` registry like any other format
crate:

```rust
use solid_rs::registry::Registry;
use solid_native::{SldaLoader, SldaSaver, SldbLoader, SldbSaver};

let mut registry = Registry::new();
registry.register_loader(SldaLoader);
registry.register_saver(SldaSaver);
registry.register_loader(SldbLoader);
registry.register_saver(SldbSaver);

// registry.load_file("model.slda")? / registry.save_file(&scene, "model.sldb")?
```

The registry path converts through `solid-rs::Scene`; the document API above
is the full-fidelity entry point for assets.

## Example `.slda`

```
SLDA 1

"name" "Level1"
"props" {
    "author" "Jane"
    "revision" 3
}
"prims" [
    {
        "kind" "mesh"
        "id" "cube"
        "name" "Cube"
        "props" {
            "source" "Blender"
        }
        "vertices" [
            { "p" v3(-1 -1 -1) "n" v3(0 0 1) "uvs" [ v2(0 0) ] }
            { "p" v3(1 -1 -1) "n" v3(0 0 1) "uvs" [ v2(1 0) ] }
            { "p" v3(1 1 -1) "n" v3(0 0 1) "uvs" [ v2(1 1) ] }
        ]
        "primitives" [
            { "topology" "triangle_list" "indices" [ 0 1 2 ] "material" "cube_mat" }
        ]
        "morphs" [ ]
        "morph_weights" [ ]
        "bounds" { "min" v3(-1 -1 -1) "max" v3(1 1 1) }
    }
]
```

## Design

```
                ┌───────────────────────────┐
                │       SolidDocument       │
                │  name · props · prims[]   │
                └────────────┬──────────────┘
                             │  encode / decode (shared schema)
                             ▼
                  ┌─────────────────────┐
                  │       DocNode       │   format-neutral tree
                  └────────┬────────────┘
                ┌──────────┴───────────┐
                ▼                      ▼
         ┌──────────────┐      ┌──────────────┐
         │  ascii/      │      │  binary/     │
         │  .slda       │      │  .sldb       │
         │  writer lexer│      │  writer      │
         │  parser      │      │  reader      │
         └──────────────┘      └──────────────┘
```

The document schema is defined once (`tree/`) and serialised by both
encoders, so ASCII and binary files are guaranteed to carry identical data.

## Module layout

```
src/
├── doc/        SolidDocument, Prim, PrimData, Props
├── prims/      mesh, skeleton, skeletal_mesh, material, texture,
│               animation, camera, light asset types
├── tree/       DocNode + document <-> tree encode/decode
├── ascii/      .slda writer, lexer, parser
├── binary/     .sldb writer, reader
├── convert.rs  solid-rs Scene <-> SolidDocument
├── loader.rs   SldaLoader / SldbLoader
└── saver.rs    SldaSaver / SldbSaver
```
