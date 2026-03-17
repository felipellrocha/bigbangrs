struct CameraUniform {
  view_proj: mat4x4<f32>,
  view: mat4x4<f32>,
  right: vec4<f32>,
  up: vec4<f32>,
  eye: vec4<f32>,
  znear: f32,
  zfar: f32,
  _padding: vec2<f32>,
};
@group(0) @binding(0) // 1.
var<uniform> camera: CameraUniform;

struct VertexInput {
  @location(0) position: vec3<f32>,
}

struct InstanceInput {
    @location(2) color: vec4<f32>,
    @location(3) translation: vec4<f32>,
    @location(4) rotation: vec4<f32>,
};

struct VertexOutput {
  @builtin(position) position: vec4<f32>,
  @location(0) brightness: f32,
}

@vertex
fn clip_space(
  model: VertexInput,
  instance: InstanceInput,
) -> VertexOutput {
  var out: VertexOutput;
  /*
  out.color = instance.color.rgb;
  let model_matrix = mat4x4<f32>(
    instance.model_matrix_0,
    instance.model_matrix_1,
    instance.model_matrix_2,
    instance.model_matrix_3,
  );
  out.position = camera.view_proj * model_matrix * model.position;
  */

  let instance_view_position = camera.view * vec4<f32>(instance.translation.xyz, 1.0);
  let depth = -instance_view_position.z;

  let normalized_depth = clamp(
    (depth - camera.znear) / (camera.zfar - camera.znear),
    0.0,
    1.0
  );

  let brightness = 1.0 - normalized_depth;
  out.brightness = brightness * brightness;

  let depth_curve = normalized_depth * normalized_depth;
  let particle_scale = 0.01 + (0.04 - 0.01) * depth_curve;

  let scaled_position = model.position.xy * particle_scale;
  let billboard =
    camera.right.xyz * scaled_position.x +
    camera.up.xyz * scaled_position.y;

  let world_position = instance.translation.xyz + billboard;

  out.position = camera.view_proj * vec4<f32>(world_position, 1.0);
  return out;
}

@fragment
fn paint(in: VertexOutput) -> @location(0) vec4<f32> {
  return vec4<f32>(
    in.brightness,
    in.brightness,
    in.brightness,
    1.0,
  );
}
