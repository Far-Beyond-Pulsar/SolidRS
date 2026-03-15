# Geometry Primitives

The `solid_rs::geometry` module provides the low-level building blocks used by
[`Mesh`](../scene-graph.md#mesh): vertices, indexed primitives, transforms,
and bounding boxes.

---

## Vertex

[`Vertex`] is the fundamental unit in a mesh vertex buffer.

```rust
pub struct Vertex {
    pub position:    Vec3,
    pub normal:      Option<Vec3>,
    pub tangent:     Option<Vec4>,       // w = handedness (±1)
    pub colors:      [Option<Vec4>; 4],  // up to 4 RGBA channels
    pub uvs:         [Option<Vec2>; 8],  // up to 8 UV sets
    pub skin_weights: Option<SkinWeights>,
}
```

### Construction

```rust
use solid_rs::geometry::Vertex;
use glam::{Vec2, Vec3, Vec4};

let v = Vertex::new(Vec3::new(1.0, 0.0, 0.0))
    .with_normal(Vec3::Y)
    .with_uv(Vec2::new(0.5, 0.5))
    .with_color(Vec4::new(1.0, 0.0, 0.0, 1.0));
```

### Skinning

```rust
use solid_rs::geometry::SkinWeights;

let weights = SkinWeights {
    joints:  [0, 1, 2, 3],
    weights: [0.5, 0.3, 0.15, 0.05],
};
let v = Vertex::new(pos).with_skin_weights(weights);
```

---

## Primitive

[`Primitive`] is a draw call within a mesh: a [`Topology`] plus index data
plus an optional material.

```rust
pub struct Primitive {
    pub topology:       Topology,
    pub indices:        Vec<u32>,
    pub material_index: Option<usize>,
}
```

### Topology

| Variant          | Index stride | Notes                    |
|:---------------- |:------------ |:------------------------ |
| `TriangleList`   | 3            | Most common              |
| `TriangleStrip`  | 1 (after 3)  | —                        |
| `LineList`       | 2            | —                        |
| `LineStrip`      | 1 (after 2)  | —                        |
| `PointList`      | 1            | Point clouds             |
| `QuadList`       | 4            | Triangulate for GPU use  |
| `Polygon`        | varies       | N-gons; use extension for loop counts |

### Convenience constructors

```rust
Primitive::triangles(indices, Some(mat_idx));
Primitive::lines(indices, None);
Primitive::points(indices, None);
```

---

## Transform

[`Transform`] stores a node's local pose as decomposed **Translation ·
Rotation (quaternion) · Scale (TRS)**:

```rust
pub struct Transform {
    pub translation: Vec3,
    pub rotation:    Quat,
    pub scale:       Vec3,
}
```

```rust
use solid_rs::geometry::Transform;
use glam::{Vec3, Quat};

let t = Transform::IDENTITY
    .with_translation(Vec3::new(0.0, 2.0, 0.0))
    .with_rotation(Quat::from_rotation_y(std::f32::consts::PI))
    .with_scale(Vec3::splat(2.0));

let mat = t.to_matrix();
let t2  = Transform::from_matrix(mat);  // round-trip (loses shear)
```

---

## Aabb

[`Aabb`] is an axis-aligned bounding box used for culling, picking, and
physics proxies.

```rust
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}
```

```rust
use solid_rs::geometry::Aabb;
use glam::Vec3;

let aabb = Aabb::from_points(vertices.iter().map(|v| v.position))
    .expect("mesh has no vertices");

println!("center: {:?}", aabb.center());
println!("size:   {:?}", aabb.size());
println!("volume: {:.3}", aabb.volume());

// Merge two boxes
let world_aabb = aabb_a.union(&aabb_b);

// Test containment
if aabb.contains(Vec3::ZERO) { /* … */ }
```

Loaders should call `mesh.compute_bounds()` after populating the vertex
buffer.  The bounds are cached in `Mesh::bounds` and are `None` until
computed.

---

## Constants

| Constant             | Value | Description                       |
|:-------------------- |:----- |:--------------------------------- |
| `MAX_UV_CHANNELS`    | 8     | Maximum UV channel count per vertex |
| `MAX_COLOR_CHANNELS` | 4     | Maximum colour channel count per vertex |
