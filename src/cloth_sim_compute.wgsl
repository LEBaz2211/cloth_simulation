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
    gridWidth: u32,
    gridHeight: u32,
    // Add more uniform parameters as needed
};

struct UniformsVec4 {
    gravity: vec4<f32>,
    sphereCenter: vec4<f32>,
    // Add more uniform parameters as needed
};

struct UniformsSpring {
    structuralStiffness: f32,
    shearStiffness: f32,
    bendStiffness: f32,
    restLengthStructural: f32,
    restLengthShear: f32,
    restLengthBend: f32,
    // Add more parameters as needed
};

@group(0) @binding(0) var<storage, read_write> vertexPositions: array<VertexInput>;
@group(0) @binding(1) var<uniform> uniforms_floats: UniformsFloats;
@group(0) @binding(2) var<uniform> uniforms_vec4: UniformsVec4;
@group(0) @binding(3) var<uniform> uniforms_spring: UniformsSpring;

// Function to calculate spring force between two vertices
fn calculate_spring_force(p1: vec4<f32>, p2: vec4<f32>, rest_length: f32, stiffness: f32) -> vec4<f32> {
    let displacement_vector: vec4<f32> = p1 - p2;
    let displacement_length: f32 = length(displacement_vector);
    let spring_vector: vec4<f32> = normalize(displacement_vector);
    let spring_force: vec4<f32> = -stiffness * (displacement_length - rest_length) * spring_vector;
    return spring_force;
}

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

    // Initialize spring force
    var springForce: vec4<f32> = vec4<f32>(0.0, 0.0, 0.0, 0.0);

    // Calculate forces for structural springs
    // Assuming vertexPositions is a 2D grid flattened into a 1D array, and we know grid width and height
    let width: u32 = uniforms_floats.gridWidth;
    let height: u32 = uniforms_floats.gridHeight;
    let current_row: u32 = idx / width;
    let current_column: u32 = idx % width;
    let restLengthStructural: f32 = uniforms_spring.restLengthStructural;

    // Check neighbors (left, right, up, down) for structural springs
    if (current_column > 0u) {
        // Neighbor to the left
        let left_idx = idx - 1u;
        springForce += calculate_spring_force(vertex.position, vertexPositions[left_idx].position, restLengthStructural, uniforms_spring.structuralStiffness);
    }
    if (current_column < (width - 1u)) {
        // Neighbor to the right
        let right_idx = idx + 1u;
        springForce += calculate_spring_force(vertex.position, vertexPositions[right_idx].position, restLengthStructural, uniforms_spring.structuralStiffness);
    }
    if (current_row > 0u) {
        // Neighbor above
        let up_idx = idx - width;
        springForce += calculate_spring_force(vertex.position, vertexPositions[up_idx].position, restLengthStructural, uniforms_spring.structuralStiffness);
    }
    if (current_row < (height - 1u)) {
        // Neighbor below
        let down_idx = idx + width;
        springForce += calculate_spring_force(vertex.position, vertexPositions[down_idx].position, restLengthStructural, uniforms_spring.structuralStiffness);
    }

    // Shear springs: Diagonal neighbors
    if (current_row > 0u && current_column > 0u) {
        // Upper left diagonal neighbor
        let upper_left_idx = idx - width - 1u;
        springForce += calculate_spring_force(vertex.position, vertexPositions[upper_left_idx].position, uniforms_spring.restLengthShear, uniforms_spring.shearStiffness);
    }
    if (current_row > 0u && current_column < (width - 1u)) {
        // Upper right diagonal neighbor
        let upper_right_idx = idx - width + 1u;
        springForce += calculate_spring_force(vertex.position, vertexPositions[upper_right_idx].position, uniforms_spring.restLengthShear, uniforms_spring.shearStiffness);
    }
    if (current_row < (height - 1u) && current_column > 0u) {
        // Lower left diagonal neighbor
        let lower_left_idx = idx + width - 1u;
        springForce += calculate_spring_force(vertex.position, vertexPositions[lower_left_idx].position, uniforms_spring.restLengthShear, uniforms_spring.shearStiffness);
    }
    if (current_row < (height - 1u) && current_column < (width - 1u)) {
        // Lower right diagonal neighbor
        let lower_right_idx = idx + width + 1u;
        springForce += calculate_spring_force(vertex.position, vertexPositions[lower_right_idx].position, uniforms_spring.restLengthShear, uniforms_spring.shearStiffness);
    }

    // Bend springs: Neighbors two positions away
    if (current_column > 1u) {
        // Two left
        let two_left_idx = idx - 2u;
        springForce += calculate_spring_force(vertex.position, vertexPositions[two_left_idx].position, uniforms_spring.restLengthBend, uniforms_spring.bendStiffness);
    }
    if (current_column < (width - 2u)) {
        // Two right
        let two_right_idx = idx + 2u;
        springForce += calculate_spring_force(vertex.position, vertexPositions[two_right_idx].position, uniforms_spring.restLengthBend, uniforms_spring.bendStiffness);
    }
    if (current_row > 1u) {
        // Two above
        let two_up_idx = idx - 2u * width;
        springForce += calculate_spring_force(vertex.position, vertexPositions[two_up_idx].position, uniforms_spring.restLengthBend, uniforms_spring.bendStiffness);
    }
    if (current_row < (height - 2u)) {
        // Two below
        let two_down_idx = idx + 2u * width;
        springForce += calculate_spring_force(vertex.position, vertexPositions[two_down_idx].position, uniforms_spring.restLengthBend, uniforms_spring.bendStiffness);
    }


    // Integrate the spring force into the total force
    totalForce += springForce;

    
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
