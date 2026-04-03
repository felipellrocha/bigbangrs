struct DebugVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

struct CameraUniform {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    right: vec4<f32>,
    up: vec4<f32>,
    forward: vec4<f32>,
    eye: vec4<f32>,
    znear: f32,
    zfar: f32,
    aspect: f32,
    fovy_radians: f32,
};

@group(0) @binding(0)
var gravity_texture: texture_3d<f32>;

@group(1) @binding(0)
var<uniform> camera: CameraUniform;

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

fn intersect_aabb(ray_origin: vec3<f32>, ray_direction: vec3<f32>, box_min: vec3<f32>, box_max: vec3<f32>) -> vec2<f32> {
    let t_min = (box_min - ray_origin) / ray_direction;
    let t_max = (box_max - ray_origin) / ray_direction;

    let t1 = min(t_min, t_max);
    let t2 = max(t_min, t_max);

    let near_distance = max(max(t1.x, t1.y), t1.z);
    let far_distance = min(min(t2.x, t2.y), t2.z);

    return vec2<f32>(near_distance, far_distance);
}

const VOLUME_MIN: vec3<f32> = vec3<f32>(-100.0, -100.0, -100.0); // CHANGED: replace if your grid bounds differ
const VOLUME_MAX: vec3<f32> = vec3<f32>( 100.0,  100.0,  100.0); // CHANGED: replace if your grid bounds differ

fn world_to_texture_uv(world_position: vec3<f32>) -> vec3<f32> {
    return clamp((world_position - VOLUME_MIN) / (VOLUME_MAX - VOLUME_MIN), vec3<f32>(0.0), vec3<f32>(0.9999));
}

/*
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
*/

fn sample_projection(uv: vec2<f32>) -> f32 {
    let dimensions = textureDimensions(gravity_texture);
    let ray_origin = camera.eye.xyz;

    let ndc_x = (uv.x * 2.0) - 1.0;
    let ndc_y = 1.0 - (uv.y * 2.0);

    let tan_half_fovy = tan(camera.fovy_radians * 0.5);

    let ray_direction = normalize(
        camera.forward.xyz + // CHANGED: was normalize(-camera.view[2].xyz)
        camera.right.xyz * ndc_x * camera.aspect * tan_half_fovy +
        camera.up.xyz * ndc_y * tan_half_fovy
    );

    let hit = intersect_aabb(ray_origin, ray_direction, VOLUME_MIN, VOLUME_MAX);

    if hit.x > hit.y || hit.y < 0.0 {
        return 0.0;
    }

    let start_distance = max(hit.x, 0.0);
    let end_distance = hit.y;

    let step_count = 64;
    let step_size = (end_distance - start_distance) / f32(step_count);

    var accumulated = 0.0;

    for (var step_index = 0; step_index < step_count; step_index = step_index + 1) {
        let distance_along_ray = start_distance + (f32(step_index) + 0.5) * step_size;
        let world_position = ray_origin + ray_direction * distance_along_ray;
        let texture_uv = world_to_texture_uv(world_position);

        let value = textureLoad(
            gravity_texture,
            vec3<i32>(
                i32(texture_uv.x * f32(dimensions.x)),
                i32(texture_uv.y * f32(dimensions.y)),
                i32(texture_uv.z * f32(dimensions.z))
            ),
            0
        ).r;

        accumulated = accumulated + value;
    }

    return accumulated / f32(step_count);
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
