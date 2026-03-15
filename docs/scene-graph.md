# Scene Graph

The SolidRS scene graph is a **directed acyclic graph (DAG)** of [`Node`]s,
each carrying a local transform and optional references to attached objects
(meshes, cameras, lights, skins).

---

## Scene

[`Scene`] is the top-level container.  It owns flat `Vec`s of every
scene-object type and a list of root node IDs.

```rust
pub struct Scene {
    pub name:       String,
    pub roots:      Vec<NodeId>,
    pub nodes:      Vec<Node>,
    pub meshes:     Vec<Mesh>,
    pub materials:  Vec<Material>,
    pub textures:   Vec<Texture>,
    pub images:     Vec<Image>,
    pub cameras:    Vec<Camera>,
    pub lights:     Vec<Light>,
    pub skins:      Vec<Skin>,
    pub animations: Vec<Animation>,
    pub metadata:   Metadata,
    pub extensions: Extensions,
}
```

### Traversal

```rust
// Depth-first walk from a single root
scene.walk_from(root_id, &mut |node| {
    println!("{}", node.name);
});

// Walk all nodes in insertion order
scene.walk_all(&mut |node| { /* … */ });

// Visitor pattern (for savers)
scene.visit(&mut my_exporter)?;
```

---

## Node

A [`Node`] represents one entry in the transform hierarchy.

```rust
pub struct Node {
    pub id:         NodeId,
    pub name:       String,
    pub transform:  Transform,   // TRS, relative to parent
    pub children:   Vec<NodeId>,
    pub mesh:       Option<usize>,
    pub camera:     Option<usize>,
    pub light:      Option<usize>,
    pub skin:       Option<usize>,
    pub extensions: Extensions,
}
```

`NodeId` is an opaque `u32` wrapper.  IDs are assigned by [`SceneBuilder`]
and remain stable for the lifetime of the scene.

---

## Mesh

[`Mesh`] owns a flat vertex buffer plus one or more [`Primitive`] draw calls.

```rust
pub struct Mesh {
    pub name:          String,
    pub vertices:      Vec<Vertex>,
    pub primitives:    Vec<Primitive>,
    pub morph_targets: Vec<MorphTarget>,
    pub bounds:        Option<Aabb>,
    pub extensions:    Extensions,
}
```

A **Primitive** pairs a [`Topology`] with an index list and an optional
material index.  Most meshes have one primitive; OBJ files often produce one
per material group.

```rust
pub struct Primitive {
    pub topology:       Topology,      // TriangleList, LineList, …
    pub indices:        Vec<u32>,
    pub material_index: Option<usize>,
}
```

---

## Material

[`Material`] follows the **glTF 2.0 PBR metallic-roughness** model.

```rust
pub struct Material {
    pub name:                    String,
    pub base_color_factor:       Vec4,
    pub base_color_texture:      Option<TextureRef>,
    pub metallic_factor:         f32,
    pub roughness_factor:        f32,
    pub metallic_roughness_texture: Option<TextureRef>,
    pub normal_texture:          Option<TextureRef>,
    pub occlusion_texture:       Option<TextureRef>,
    pub emissive_factor:         Vec3,
    pub emissive_texture:        Option<TextureRef>,
    pub alpha_mode:              AlphaMode,
    pub alpha_cutoff:            f32,
    pub double_sided:            bool,
    pub extensions:              Extensions,
}
```

Format crates that use a different material model (Phong, Lambert, …) should
map to PBR fields where possible and store the remainder in `extensions`.

---

## Texture and Image

Textures and images are separated to allow multiple textures to share a
single image with different sampler settings:

```
Scene::textures[i]  →  image_index  →  Scene::images[j]
                    ↘  sampler (WrapMode, FilterMode)
```

---

## Camera

A camera is attached to a node and looks down the **−Z** axis in local space:

```rust
pub enum Projection {
    Perspective(PerspectiveCamera),
    Orthographic(OrthographicCamera),
}
```

---

## Light

```rust
pub enum Light {
    Directional(DirectionalLight),
    Point(PointLight),
    Spot(SpotLight),
    Area(AreaLight),
}
```

All variants share a `LightBase` carrying `name`, `color` (linear RGB), and
`intensity` (candela).

---

## Skin & Skeletal Animation

[`Skin`] maps joints (nodes in the scene graph) to vertex blend weights stored
in [`Vertex::skin_weights`]:

```rust
pub struct Skin {
    pub name:                    String,
    pub skeleton_root:           Option<NodeId>,
    pub joints:                  Vec<NodeId>,
    pub inverse_bind_matrices:   Vec<Mat4>,
}
```

---

## Animation

An [`Animation`] is a clip composed of [`AnimationChannel`]s.  Each channel
targets a specific node property (`Translation`, `Rotation`, `Scale`, or
`MorphWeight`) and stores flat arrays of times + values:

```rust
pub struct AnimationChannel {
    pub target:        AnimationTarget,
    pub interpolation: Interpolation,  // Linear | Step | CubicSpline
    pub times:         Vec<f32>,
    pub values:        Vec<f32>,
}
```

---

## Extensions

Every scene object has an `extensions: Extensions` field.  Format crates use
it to attach their own typed structs without any central registration:

```rust
// In solid-fbx:
pub struct FbxNodeProps { pub uid: i64, pub user_data: String }

node.extensions.insert(FbxNodeProps { uid: 12345, user_data: "hello".into() });

// In application code:
if let Some(props) = node.extensions.get::<FbxNodeProps>() {
    println!("FBX UID = {}", props.uid);
}
```
