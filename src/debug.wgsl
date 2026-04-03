struct DebugVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0)
var gravity_texture: texture_3d<f32>;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> DebugVertexOutput {
    var positions = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0,  1.0),
    );

    var output: DebugVertexOutput;
    let position = positions[vertex_index];
    output.clip_position = vec4<f32>(position, 0.0, 1.0);
    output.uv = position * 0.5 + vec2<f32>(0.5, 0.5);
    return output;
}

fn sample_projection(uv: vec2<f32>) -> f32 {
    let dimensions = textureDimensions(gravity_texture);

    var sum_value = 0.0;

    for (var slice_index: i32 = 0; slice_index < i32(dimensions.z); slice_index = slice_index + 1) {
        let value = textureLoad(
            gravity_texture,
            vec3<i32>(
                i32(clamp(uv.x, 0.0, 0.9999) * f32(dimensions.x)),
                i32(clamp(uv.y, 0.0, 0.9999) * f32(dimensions.y)),
                slice_index
            ),
            0
        ).r;
        sum_value = sum_value + value;
    }

    return sum_value / f32(dimensions.z); // CHANGED: average instead of max
}

@fragment
fn fs_main(input: DebugVertexOutput) -> @location(0) vec4<f32> {
    let value = sample_projection(input.uv);

    // CHANGED: compress dynamic range a bit so faint fields remain visible.
    //let intensity = clamp(log2(1.0 + value * 8.0) / 4.0, 0.0, 1.0);
    let intensity = clamp(value * 0.0001, 0.0, 1.0);

    let color = vec3<f32>(
        intensity,
        intensity * intensity,
        0.25 + intensity * 0.75
    );

    return vec4<f32>(color, 0.9);
    //return vec4<f32>(1.0, 0.0, 0.0, 1.0);
}
