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
    
    // Collision detection and response with the sphere
    let toSphere = vertex.position.xyz - uniforms_vec4.sphereCenter.xyz;
    let distanceToSphere = length(toSphere);
    if (distanceToSphere < uniforms_floats.sphereRadius) {
        // Collision detected, reposition vertex on the surface of the sphere
        let normal = normalize(toSphere);
        let penetrationDepth = uniforms_floats.sphereRadius - distanceToSphere;
        vertex.position.x += normal.x * penetrationDepth;
        vertex.position.y += normal.y * penetrationDepth;
        vertex.position.z += normal.z * penetrationDepth;

        // Adjust velocity after collision
        // Reflect the velocity vector around the normal (basic response)
        let velocityDotNormal = dot(vertex.velocity.xyz, normal);
        vertex.velocity.x -= 2.0 * velocityDotNormal * normal.x;
        vertex.velocity.y -= 2.0 * velocityDotNormal * normal.y;
        vertex.velocity.z -= 2.0 * velocityDotNormal * normal.z;

        // Optionally apply some restitution coefficient if you want the cloth to bounce off
        // Restitution is the bounciness of the material, 0 for no bounce and 1 for a perfect bounce
        let restitution: f32 = 0.5; // Example restitution value
        vertex.velocity.x *= restitution;
        vertex.velocity.y *= restitution;
        vertex.velocity.z *= restitution;

        // Apply friction if needed
        let frictionCoeff: f32 = 0.2; // Example friction coefficient
        let tangentialVelocity = vertex.velocity.xyz - velocityDotNormal * normal;
        vertex.velocity.x -= frictionCoeff * tangentialVelocity.x;
        vertex.velocity.y -= frictionCoeff * tangentialVelocity.y;
        vertex.velocity.z -= frictionCoeff * tangentialVelocity.z;
    }
    

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
