struct InstanceRaw {
  force: vec4<f32>,
  translation: vec4<f32>,
  rotation: vec4<f32>,
};

struct InstanceBuffer {
  values: array<InstanceRaw>,
};

struct SimulationUniform {
  time: f32,
  delta_time: f32,
  gravity_strength: f32,
  particle_count: u32,
  workgroups_per_row: u32,
  _padding0: u32,
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

struct VolumeGridUniform {
  width: u32,
  height: u32,
  depth: u32,
  point_count: u32,

  world_min: vec4<f32>,
  world_max: vec4<f32>,

  deposit_value: f32,
  fixed_point_scale: f32,
  _padding: vec2<f32>,
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

@group(0) @binding(5)
var<storage, read> voxel_values_in: array<u32>;

@group(0) @binding(6)
var<uniform> volume_grid: VolumeGridUniform;

fn flatten_voxel_index(x: u32, y: u32, z: u32) -> u32 {
  return z * volume_grid.width * volume_grid.height + y * volume_grid.width + x;
}

fn world_to_voxel_position(world_position: vec3<f32>) -> vec3<f32> {
  let world_minimum = volume_grid.world_min.xyz;
  let world_maximum = volume_grid.world_max.xyz;
  let world_size = world_maximum - world_minimum;

  let normalized_position = clamp(
    (world_position - world_minimum) / world_size,
    vec3<f32>(0.0),
    vec3<f32>(1.0),
  );

  return vec3<f32>(
    normalized_position.x * f32(volume_grid.width - 1u),
    normalized_position.y * f32(volume_grid.height - 1u),
    normalized_position.z * f32(volume_grid.depth - 1u),
  );
}

fn wrap_world_position(world_position: vec3<f32>) -> vec3<f32> {
  let world_minimum = volume_grid.world_min.xyz;
  let world_maximum = volume_grid.world_max.xyz;
  let world_size = world_maximum - world_minimum;

  var wrapped_position = world_position;

  if (wrapped_position.x < world_minimum.x) {
    wrapped_position.x = wrapped_position.x + world_size.x;
  }
  if (wrapped_position.x > world_maximum.x) {
    wrapped_position.x = wrapped_position.x - world_size.x;
  }

  if (wrapped_position.y < world_minimum.y) {
    wrapped_position.y = wrapped_position.y + world_size.y;
  }
  if (wrapped_position.y > world_maximum.y) {
    wrapped_position.y = wrapped_position.y - world_size.y;
  }

  if (wrapped_position.z < world_minimum.z) {
    wrapped_position.z = wrapped_position.z + world_size.z;
  }
  if (wrapped_position.z > world_maximum.z) {
    wrapped_position.z = wrapped_position.z - world_size.z;
  }

  return wrapped_position;
}

fn read_voxel_scalar(x: u32, y: u32, z: u32) -> f32 {
  let voxel_index = flatten_voxel_index(x, y, z);
  return f32(voxel_values_in[voxel_index]) / volume_grid.fixed_point_scale;
}

fn sample_field_nearest(world_position: vec3<f32>) -> f32 {
  let voxel_position = world_to_voxel_position(world_position);

  let voxel_x = u32(round(voxel_position.x));
  let voxel_y = u32(round(voxel_position.y));
  let voxel_z = u32(round(voxel_position.z));

  return read_voxel_scalar(voxel_x, voxel_y, voxel_z);
}

/*
fn sample_field_gradient(world_position: vec3<f32>) -> vec3<f32> {
  let voxel_position = world_to_voxel_position(world_position);

  let center_x = u32(round(voxel_position.x));
  let center_y = u32(round(voxel_position.y));
  let center_z = u32(round(voxel_position.z));

  let min_x = max(center_x, 1u) - 1u;
  let max_x = min(center_x + 1u, volume_grid.width - 1u);

  let min_y = max(center_y, 1u) - 1u;
  let max_y = min(center_y + 1u, volume_grid.height - 1u);

  let min_z = max(center_z, 1u) - 1u;
  let max_z = min(center_z + 1u, volume_grid.depth - 1u);

  let sample_x_negative = read_voxel_scalar(min_x, center_y, center_z);
  let sample_x_positive = read_voxel_scalar(max_x, center_y, center_z);

  let sample_y_negative = read_voxel_scalar(center_x, min_y, center_z);
  let sample_y_positive = read_voxel_scalar(center_x, max_y, center_z);

  let sample_z_negative = read_voxel_scalar(center_x, center_y, min_z);
  let sample_z_positive = read_voxel_scalar(center_x, center_y, max_z);

  return vec3<f32>(
    sample_x_positive - sample_x_negative,
    sample_y_positive - sample_y_negative,
    sample_z_positive - sample_z_negative,
  );
}
*/

fn sample_field_gradient(world_position: vec3<f32>) -> vec3<f32> {
  let voxel_position = world_to_voxel_position(world_position);

  let center_x = u32(round(voxel_position.x));
  let center_y = u32(round(voxel_position.y));
  let center_z = u32(round(voxel_position.z));

  let min_x = max(center_x, 1u) - 1u;
  let max_x = min(center_x + 1u, volume_grid.width - 1u);

  let min_y = max(center_y, 1u) - 1u;
  let max_y = min(center_y + 1u, volume_grid.height - 1u);

  let min_z = max(center_z, 1u) - 1u;
  let max_z = min(center_z + 1u, volume_grid.depth - 1u);

  let sample_x_negative = read_voxel_scalar(min_x, center_y, center_z);
  let sample_x_positive = read_voxel_scalar(max_x, center_y, center_z);

  let sample_y_negative = read_voxel_scalar(center_x, min_y, center_z);
  let sample_y_positive = read_voxel_scalar(center_x, max_y, center_z);

  let sample_z_negative = read_voxel_scalar(center_x, center_y, min_z);
  let sample_z_positive = read_voxel_scalar(center_x, center_y, max_z);

  // CHANGED: compute voxel cell size in world-space units.
  let world_minimum = volume_grid.world_min.xyz;
  let world_maximum = volume_grid.world_max.xyz;
  let world_size = world_maximum - world_minimum;

  let cell_size = vec3<f32>(
    world_size.x / f32(volume_grid.width),
    world_size.y / f32(volume_grid.height),
    world_size.z / f32(volume_grid.depth),
  );

  // CHANGED: central difference divided by physical spacing.
  return vec3<f32>(
    (sample_x_positive - sample_x_negative) / max(cell_size.x * 2.0, 0.000001),
    (sample_y_positive - sample_y_negative) / max(cell_size.y * 2.0, 0.000001),
    (sample_z_positive - sample_z_negative) / max(cell_size.z * 2.0, 0.000001),
  );
}

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
@workgroup_size(256)
fn main(
  @builtin(workgroup_id) workgroup_id: vec3<u32>,
  @builtin(local_invocation_id) local_invocation_id: vec3<u32>,
) {
  let linear_group_index =
    workgroup_id.y * simulation.workgroups_per_row + workgroup_id.x;
  let index: u32 = linear_group_index * 256u + local_invocation_id.x;

  if (index >= simulation.particle_count) {
    return;
  }

  var instance = instances.values[index];

  var particle_force = instance.force.xyz;
  var particle_position = instance.translation.xyz;

  /*
  let field_value = sample_field_nearest(particle_position);
  let field_gradient = sample_field_gradient(particle_position);
  var gravity = vec3<f32>(0.0);
  let gradient_length_squared = dot(field_gradient, field_gradient);
  if (gradient_length_squared > 0.000001) {
    let downhill_direction = -normalize(field_gradient);

    // CHANGED: use field magnitude * gradient direction as acceleration.
    gravity = downhill_direction * field_value * simulation.gravity_strength;
  }
  */

  let field_gradient = sample_field_gradient(particle_position);
  let gravity = field_gradient * simulation.gravity_strength;

  particle_force = particle_force + gravity * simulation.delta_time;
  particle_position = particle_position + particle_force * simulation.delta_time;
  let world_minimum = volume_grid.world_min.xyz;
  let world_maximum = volume_grid.world_max.xyz;
  particle_position = wrap_world_position(particle_position);

  //let translation = instance.translation.xyz;


  //instance.translation.y = wave * simulation.amplitude;
  instance.translation = vec4<f32>(particle_position, 1.0);
  instance.force = vec4<f32>(particle_force, 1.0);

  instances.values[index] = instance;
  
  let particle_radius = 0.02;

  if (is_visible(instance.translation.xyz, particle_radius)) {
    let visible_index = atomicAdd(&indirect_args.instance_count, 1u);
    visible_instances.values[visible_index] = instance;
  }
}
