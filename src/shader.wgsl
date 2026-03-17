struct CameraUniform {
    view_proj: mat4x4<f32>,
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
  @location(0) color: vec3<f32>,
}

@vertex
fn clip_space(
  model: VertexInput,
  instance: InstanceInput,
) -> VertexOutput {
  var out: VertexOutput;
  out.color = instance.color.rgb;
  /*
  let model_matrix = mat4x4<f32>(
    instance.model_matrix_0,
    instance.model_matrix_1,
    instance.model_matrix_2,
    instance.model_matrix_3,
  );
  out.position = camera.view_proj * model_matrix * model.position;
  */
  out.position = camera.view_proj * vec4<f32>(instance.translation.xyz + model.position.xyz, 1.0);
  return out;
}

@fragment
fn paint(in: VertexOutput) -> @location(0) vec4<f32> {
  return vec4<f32>(1.0, 1.0, 1.0, 1.0);
}
