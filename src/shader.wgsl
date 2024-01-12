struct CameraUniform {
    view_proj: mat4x4<f32>,
};
@group(0) @binding(0) var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) texCoord: vec2<f32>,
    @location(4) velocity: vec3<f32>,
    @location(5) mass: f32,
};
@group(0) @binding(1) var<storage> model: VertexInput;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.color = model.color;
    // extract model position from vertex input
    var zoom_factor = 0.5;
    var x = model.position[0] * zoom_factor;
    var y = (model.position[1] - 0.8) * zoom_factor;
    var z = model.position[2] * zoom_factor;

    // transform model position to clip space
    out.clip_position = camera.view_proj * vec4<f32>(x, y, z, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
