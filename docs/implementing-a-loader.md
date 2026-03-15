# Implementing a Loader

This guide walks through writing a complete [`Loader`] for a hypothetical
text-based 3-D format.  The same pattern applies to any real format crate
(`solid-obj`, `solid-fbx`, etc.).

---

## 1. Create the Crate

```toml
# Cargo.toml
[package]
name    = "solid-myfmt"
version = "0.1.0"
edition = "2021"

[dependencies]
solid-rs = "0.1"
```

---

## 2. Define `FormatInfo`

```rust
use solid_rs::traits::FormatInfo;

static MY_FORMAT: FormatInfo = FormatInfo {
    name:         "My Format",
    id:           "myfmt",
    extensions:   &["myfmt", "mf"],
    mime_types:   &["model/x-myfmt"],
    can_load:     true,
    can_save:     false,
    spec_version: Some("1.0"),
};
```

---

## 3. Implement the `Loader` Trait

```rust
use solid_rs::prelude::*;
use solid_rs::traits::{LoadOptions, Loader, ReadSeek};
use std::io::{BufRead, BufReader, Read};

pub struct MyFmtLoader;

impl Loader for MyFmtLoader {
    fn load(
        &self,
        reader: &mut dyn ReadSeek,
        options: &LoadOptions,
    ) -> Result<Scene> {
        let mut builder = SceneBuilder::new();
        parse_stream(reader, options, &mut builder)?;
        Ok(builder.build())
    }

    fn format_info(&self) -> &FormatInfo {
        &MY_FORMAT
    }

    fn detect(&self, reader: &mut dyn Read) -> f32 {
        let mut magic = [0u8; 4];
        if reader.read_exact(&mut magic).is_ok() && &magic == b"MYFT" {
            1.0
        } else {
            0.0
        }
    }
}
```

---

## 4. Parse and Populate the SceneBuilder

```rust
fn parse_stream(
    reader: &mut dyn Read,
    opts: &LoadOptions,
    b: &mut SceneBuilder,
) -> Result<()> {
    let mut mesh    = Mesh::new("default");
    let mut mat_idx = None;

    for (line_no, raw) in BufReader::new(reader).lines().enumerate() {
        let line = raw.map_err(SolidError::Io)?;
        let line = line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let mut parts = line.splitn(2, ' ');
        match parts.next().unwrap_or("") {
            // "v x y z"
            "v" => {
                let coords = parse_vec3(parts.next().unwrap_or(""), line_no)?;
                mesh.vertices.push(Vertex::new(coords));
            }

            // "f i0 i1 i2"
            "f" => {
                let indices = parse_indices(parts.next().unwrap_or(""), line_no)?;
                mesh.primitives.push(Primitive::triangles(indices, mat_idx));
            }

            // "mtl name"
            "mtl" => {
                let name = parts.next().unwrap_or("default").to_owned();
                let mat  = Material::new(name);
                mat_idx  = Some(b.push_material(mat));
            }

            unknown => {
                // Silently skip unknown keywords (be lenient on input).
                let _ = unknown;
            }
        }
    }

    if opts.generate_normals {
        generate_normals(&mut mesh);
    }

    mesh.compute_bounds();
    let mesh_idx = b.push_mesh(mesh);
    let root     = b.add_root_node("Root");
    b.attach_mesh(root, mesh_idx);
    Ok(())
}
```

---

## 5. Error Handling in Parsers

Always return descriptive errors with line numbers:

```rust
fn parse_vec3(s: &str, line_no: usize) -> Result<glam::Vec3> {
    let nums: Vec<f32> = s
        .split_whitespace()
        .map(|tok| {
            tok.parse::<f32>().map_err(|_| {
                SolidError::parse(format!(
                    "expected float on line {}, got {:?}", line_no + 1, tok
                ))
            })
        })
        .collect::<Result<_>>()?;

    if nums.len() < 3 {
        return Err(SolidError::parse(format!(
            "expected 3 floats on line {}, got {}", line_no + 1, nums.len()
        )));
    }
    Ok(glam::Vec3::new(nums[0], nums[1], nums[2]))
}
```

---

## 6. Format-Specific Extensions

Store data that doesn't map to the core model in `extensions`:

```rust
#[derive(Debug)]
pub struct MyFmtMeshMeta {
    pub object_id: u32,
    pub flags:     u16,
}

// In your parser:
mesh.extensions.insert(MyFmtMeshMeta { object_id: 7, flags: 0x03 });

// Application code can retrieve it:
if let Some(meta) = mesh.extensions.get::<MyFmtMeshMeta>() {
    println!("object_id = {}", meta.object_id);
}
```

---

## 7. Register the Loader

```rust
// In the application:
registry.register_loader(solid_myfmt::MyFmtLoader);

let scene = registry.load_file("model.myfmt")?;
```

---

## Checklist

- [ ] `FormatInfo` has correct `extensions` and `mime_types`.
- [ ] `detect` inspects magic bytes without consuming the reader.
- [ ] Parse errors include line/offset information.
- [ ] `LoadOptions::triangulate` and `generate_normals` are honoured if applicable.
- [ ] `mesh.compute_bounds()` is called after populating vertices.
- [ ] Format-specific data not in the core model goes into `extensions`.
- [ ] Public API items have doc comments.
