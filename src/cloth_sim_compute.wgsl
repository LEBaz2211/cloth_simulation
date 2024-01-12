struct ClothVertexState {
    position: vec3<f32>,
    velocity: vec3<f32>,
};

struct ClothRange {
    start: u32,
    end: u32,
};


@group(0) @binding(1) var<uniform> clothRange: ClothRange;
@group(0) @binding(2) var<storage, read> clothStatePrev: array<ClothVertexState>;
@group(0) @binding(3) var<storage, read_write> clothStateNext: array<ClothVertexState>;

@compute
@workgroup_size(WORKGROUP_SIZE, WORKGROUP_SIZE)
fn computeMain(@builtin(global_invocation_id) global_id: vec3u) {
    let index: u32 = global_id.x;
    if (index >= clothRange.start && index < clothRange.end) {
        if (index < arrayLength(&clothStatePrev)) {
            var prevState = clothStatePrev[index];
            var nextState = prevState;
            
            nextState.position += prevState.velocity;

            clothStateNext[index] = nextState;
        }
    }
}
