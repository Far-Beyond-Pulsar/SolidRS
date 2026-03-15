# Traits Reference

SolidRS defines four core traits in `solid_rs::traits`:

| Trait           | Who implements it | Purpose                             |
|:--------------- |:----------------- |:----------------------------------- |
| `Loader`        | format crates     | Parse a byte stream into a `Scene`  |
| `Saver`         | format crates     | Serialise a `Scene` to a writer     |
| `SceneVisitor`  | format crates     | Walk a scene without cloning it     |
| *(not a trait)* | core crate        | `FormatInfo` — format metadata      |

---

## `Loader`

```rust
pub trait Loader: Send + Sync + 'static {
    fn load(
        &self,
        reader: &mut dyn ReadSeek,
        options: &LoadOptions,
    ) -> Result<Scene>;

    fn format_info(&self) -> &FormatInfo;

    // Optional — for magic-byte detection:
    fn detect(&self, reader: &mut dyn Read) -> f32 { 0.0 }
}
```

### `LoadOptions`

| Field               | Type               | Default   | Description                          |
|:------------------- |:------------------ |:--------- |:------------------------------------ |
| `generate_normals`  | `bool`             | `false`   | Synthesise normals if absent          |
| `triangulate`       | `bool`             | `false`   | Convert quads / n-gons to triangles   |
| `merge_vertices`    | `bool`             | `false`   | Weld duplicate vertices               |
| `flip_uv_v`         | `bool`             | `false`   | Flip V coordinate (`v' = 1 − v`)      |
| `max_texture_size`  | `Option<u32>`      | `None`    | Max texture dimension                 |
| `base_dir`          | `Option<PathBuf>`  | `None`    | Base path for relative texture URIs   |

---

## `Saver`

```rust
pub trait Saver: Send + Sync + 'static {
    fn save(
        &self,
        scene: &Scene,
        writer: &mut dyn Write,
        options: &SaveOptions,
    ) -> Result<()>;

    fn format_info(&self) -> &FormatInfo;
}
```

### `SaveOptions`

| Field             | Type            | Default   | Description                             |
|:----------------- |:--------------- |:--------- |:--------------------------------------- |
| `embed_textures`  | `bool`          | `false`   | Inline textures as binary blobs          |
| `pretty_print`    | `bool`          | `false`   | Pretty-print JSON/text output            |
| `copyright`       | `Option<String>`| `None`    | Override file-header copyright string    |
| `generator`       | `Option<String>`| `None`    | Override generator tag                   |
| `flip_uv_v`       | `bool`          | `false`   | Flip V coordinate when writing           |

---

## `SceneVisitor`

```rust
pub trait SceneVisitor {
    fn visit_node     (&mut self, node:      &Node,      )           -> Result<()> { Ok(()) }
    fn visit_mesh     (&mut self, mesh:      &Mesh,      index: usize) -> Result<()> { Ok(()) }
    fn visit_material (&mut self, material:  &Material,  index: usize) -> Result<()> { Ok(()) }
    fn visit_texture  (&mut self, texture:   &Texture,   index: usize) -> Result<()> { Ok(()) }
    fn visit_image    (&mut self, image:     &Image,     index: usize) -> Result<()> { Ok(()) }
    fn visit_camera   (&mut self, camera:    &Camera,    index: usize) -> Result<()> { Ok(()) }
    fn visit_light    (&mut self, light:     &Light,     index: usize) -> Result<()> { Ok(()) }
    fn visit_skin     (&mut self, skin:      &Skin,      index: usize) -> Result<()> { Ok(()) }
    fn visit_animation(&mut self, animation: &Animation, index: usize) -> Result<()> { Ok(()) }
}
```

All methods default to no-ops so you only implement what you need.

### Usage in a Saver

```rust
// The visitor holds a reference to the concrete writer.
struct ObjWriter<'a> {
    writer: &'a mut dyn Write,
}

impl SceneVisitor for ObjWriter<'_> {
    fn visit_mesh(&mut self, mesh: &Mesh, _index: usize) -> Result<()> {
        for v in &mesh.vertices {
            writeln!(self.writer, "v {} {} {}", v.position.x, v.position.y, v.position.z)
                .map_err(SolidError::Io)?;
        }
        Ok(())
    }
}

pub struct ObjSaver;

impl Saver for ObjSaver {
    fn save(&self, scene: &Scene, writer: &mut dyn Write, _opts: &SaveOptions) -> Result<()> {
        let mut visitor = ObjWriter { writer };
        scene.visit(&mut visitor)
    }
    fn format_info(&self) -> &FormatInfo { &OBJ_INFO }
}
```

---

## `FormatInfo`

```rust
pub struct FormatInfo {
    pub name:         &'static str,
    pub id:           &'static str,
    pub extensions:   &'static [&'static str],
    pub mime_types:   &'static [&'static str],
    pub can_load:     bool,
    pub can_save:     bool,
    pub spec_version: Option<&'static str>,
}
```

Typically stored as a `static` constant in the format crate:

```rust
static OBJ_INFO: FormatInfo = FormatInfo {
    name:         "Wavefront OBJ",
    id:           "obj",
    extensions:   &["obj"],
    mime_types:   &["model/obj", "text/plain"],
    can_load:     true,
    can_save:     true,
    spec_version: None,
};
```
