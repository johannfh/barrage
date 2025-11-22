use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};

pub fn create_polygon_mesh(sides: usize, radius: f32) -> Mesh {
    let mut vertices = Vec::with_capacity(sides + 1);
    let mut indices = Vec::with_capacity(sides * 3);

    // Center vertex
    vertices.push([0.0, 0.0, 0.0]);

    // Outer vertices
    for i in 0..sides {
        let angle = i as f32 / sides as f32 * std::f32::consts::TAU;
        let x = radius * angle.cos();
        let y = radius * angle.sin();
        vertices.push([x, y, 0.0]);
    }

    // Indices for triangles
    for i in 0..sides {
        indices.push(0);
        indices.push((i + 1) as u32);
        indices.push(((i + 1) % sides + 1) as u32);
    }

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
    .with_inserted_indices(Indices::U32(indices))
}
