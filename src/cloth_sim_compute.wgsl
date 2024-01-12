// // cloth_sim_compute.wgsl

// struct Vertex {
//     position: vec3<f32>,
//     color: vec3<f32>,
//     normal: vec3<f32>,
//     texCoord: vec2<f32>,
//     velocity: vec3<f32>,
//     mass: f32,
// };

// // Buffer for vertices data
// @group(0) @binding(1) var<storage, read_write> inVertices: array<Vertex>;
// @group(0) @binding(2) var<storage, read_write> outVertices: array<Vertex>;

// @compute
// @workgroup_size(8, 8) // Adjust workgroup size based on your requirements
// fn main(@builtin(global_invocation_id) global_id : vec3<u32>) {
//     let id = global_id.x;
//     // Simple physics simulation
//     // Update position and velocity of each vertex
//     var vertex = outVertices[id];
//     vertex.position.y += 0.1;
//     // if (id < arrayLength(vertices)) {
//     //     var vertex = vertices[id];
        
//     //     vertex.position = vec3<f32>(1.0, 2.0, 3.0);
//     //     vertices[id] = vertex;
//     // }

// }
