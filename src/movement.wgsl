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

struct DrawIndexedIndirectArgsStorage {
  index_count: u32,
  instance_count: atomic<u32>,
  first_index: u32,
  base_vertex: i32,
  first_instance: u32,
};


@group(0) @binding(0)
var<storage, read_write> instances: InstanceBuffer;

@group(0) @binding(1)
var<uniform> simulation: SimulationUniform;

@group(0) @binding(2)
var<uniform> camera: CameraUniform;

@group(0) @binding(3)
var<storage, read_write> visible_instances: InstanceBuffer;

@group(0) @binding(4)
var<storage, read_write> indirect_args: DrawIndexedIndirectArgsStorage;

fn is_visible(world_position: vec3<f32>, particle_radius: f32) -> bool {
  let clip_position = camera.view_proj * vec4<f32>(world_position, 1.0);

  if (clip_position.w <= 0.0) {
    return false;
  }

  let horizontal_limit = clip_position.w + particle_radius;
  let vertical_limit = clip_position.w + particle_radius;
  let far_limit = clip_position.w + particle_radius;

  if (clip_position.x < -horizontal_limit || clip_position.x > horizontal_limit) {
    return false;
  }

  if (clip_position.y < -vertical_limit || clip_position.y > vertical_limit) {
    return false;
  }

  if (clip_position.z < -particle_radius || clip_position.z > far_limit) {
    return false;
  }

  return true;
}

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

  var instance = instances.values[index];
  let translation = instance.translation.xyz;

  let wave =
      sin(translation.x * simulation.frequency + simulation.time * simulation.speed) +
      sin(translation.z * simulation.frequency + simulation.time * simulation.speed);

  instance.translation.y = wave * simulation.amplitude;
  instances.values[index] = instance;
  
  let particle_radius = 0.02;

  if (is_visible(instance.translation.xyz, particle_radius)) {
    let visible_index = atomicAdd(&indirect_args.instance_count, 1u);
    visible_instances.values[visible_index] = instance;
  }
}
