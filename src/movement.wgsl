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

struct SimulationUniform {
  time: f32,
  amplitude: f32,
  frequency: f32,
  speed: f32,
  particle_count: u32,
  workgroups_per_row: u32,
  _padding0: vec2<u32>,
};

@group(0) @binding(0)
var<storage, read_write> instances: InstanceBuffer;

@group(0) @binding(1)
var<uniform> simulation: SimulationUniform;

@compute
@workgroup_size(64)
fn main(
  @builtin(workgroup_id) workgroup_id: vec3<u32>,
  @builtin(local_invocation_id) local_invocation_id: vec3<u32>,
) {
  let linear_group_index =
    workgroup_id.y * simulation.workgroups_per_row + workgroup_id.x;
  let index: u32 = linear_group_index * 64u + local_invocation_id.x;

  if (index >= simulation.particle_count) {
    return;
  }

  let translation = instances.values[index].translation.xyz;

  let wave =
      sin(translation.x * simulation.frequency + simulation.time * simulation.speed) +
      sin(translation.z * simulation.frequency + simulation.time * simulation.speed);

  instances.values[index].translation.y = wave * simulation.amplitude;
}
