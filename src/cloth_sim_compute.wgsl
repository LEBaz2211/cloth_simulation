struct VertexInput {
    position: vec3<f32>,
    color: vec3<f32>,
};

@group(0) @binding(0) var<storage, read_write> vertexPositions: array<VertexInput>;

@compute 
@workgroup_size(128)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    // Perform simulation operations on vertexPositions[idx]
    // For example, applying simple gravity:
    vertexPositions[idx].position.y -= 0.01; // Gravity effect
}

