struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
    @location(2) velocity: vec4<f32>,
    @location(3) mass: vec4<f32>,
};

struct UniformsFloats {
    damping: f32,
    timeStep: f32,
    sphereRadius: f32,
    // Add more uniform parameters as needed
};

struct UniformsVec4 {
    gravity: vec4<f32>,
    sphereCenter: vec4<f32>,
    // Add more uniform parameters as needed
};

@group(0) @binding(0) var<storage, read_write> vertexPositions: array<VertexInput>;
@group(0) @binding(1) var<uniform> uniforms_floats: UniformsFloats;
@group(0) @binding(2) var<uniform> uniforms_vec4: UniformsVec4;

@compute
@workgroup_size(128)
fn main(@builtin(global_invocation_id) global_id: vec3u) {
    let idx = global_id.x;
    
    // Retrieve the current vertex position and velocity
    var vertex = vertexPositions[idx];
    
    // Calculate gravitational force
    let gravityForce = vec4<f32>(uniforms_vec4.gravity.x, uniforms_vec4.gravity.y, uniforms_vec4.gravity.z, 0.0) * vertex.mass.x;


    // Initialize total force with gravity and damping
    var totalForce = gravityForce - uniforms_floats.damping * vertex.velocity;

    // Add spring forces, collision detection, etc. here...
    // ...
    
    // Collision detection with the sphere
    // let toSphere = vertex.position.xyz - uniforms.sphereCenter;
    // if (length(toSphere) < uniforms.sphereRadius) {
    //     // Collision response
    //     // ...
    // }
    

    // Euler integration to update velocity and position
    let deltaVelocity = totalForce * uniforms_floats.timeStep / vertex.mass.x; // Assuming w holds mass
    vertex.velocity.x += deltaVelocity.x;
    vertex.velocity.y += deltaVelocity.y;
    vertex.velocity.z += deltaVelocity.z;

    let deltaPosition = vertex.velocity.xyz * uniforms_floats.timeStep;
    vertex.position.x += deltaPosition.x;
    vertex.position.y += deltaPosition.y;
    vertex.position.z += deltaPosition.z;

    // Write the updated vertex back to the buffer
    vertexPositions[idx] = vertex;

}
