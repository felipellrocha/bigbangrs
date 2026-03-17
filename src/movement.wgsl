//@group(0) @binding(0) var<storage, read> input: array<u32>;
//@group(0) @binding(1) var<storage, read_write> output: array<u32>;

struct InstanceRaw {
  color: vec4<f32>,
  translation: vec4<f32>,
  rotation: vec4<f32>,
};

struct InstanceBuffer {
  values: array<InstanceRaw>,
};

struct TimeUniform {
  time: f32,
  amplitude: f32,
  frequency: f32,
  speed: f32,
};

@group(0) @binding(0)
var<storage, read_write> instances: InstanceBuffer;

@group(0) @binding(1)
var<uniform> simulation: TimeUniform;

@compute
@workgroup_size(64, 1, 1)
fn main(
  @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
) {
  let index = global_invocation_id.x;
  let total = arrayLength(&instances.values);

  if (index >= total) { return; }

  let translation = instances.values[index].translation.xyz;

  let wave =
      sin(translation.x * simulation.frequency + simulation.time * simulation.speed) +
      sin(translation.z * simulation.frequency + simulation.time * simulation.speed);

  instances.values[index].translation.y = wave * simulation.amplitude;
}
