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

struct InstanceRaw {
  color: vec4<f32>,
  translation: vec4<f32>,
  rotation: vec4<f32>,
};

struct InstanceBuffer {
  values: array<InstanceRaw>,
};

@group(0) @binding(0)
var<storage, read_write> instances: InstanceBuffer;

@group(0) @binding(1)
var<storage, read_write> voxel_values: array<atomic<u32>>;

@group(0) @binding(2)
var<uniform> volume_grid: VolumeGridUniform;

@group(0) @binding(3)
var volume_texture: texture_storage_3d<r32float, write>;

fn voxel_index(x: u32, y: u32, z: u32) -> u32 {
  return z * volume_grid.width * volume_grid.height + y * volume_grid.width + x;
}

fn is_in_bounds(x: i32, y: i32, z: i32) -> bool {
  return x >= 0
    && y >= 0
    && z >= 0
    && u32(x) < volume_grid.width
    && u32(y) < volume_grid.height
    && u32(z) < volume_grid.depth;
}

fn world_to_grid(position_world: vec3<f32>) -> vec3<u32> {
  let world_minimum = volume_grid.world_min.xyz;
  let world_maximum = volume_grid.world_max.xyz;
  let world_extent = max(world_maximum - world_minimum, vec3<f32>(0.00001));

  let normalized_position = clamp(
    (position_world - world_minimum) / world_extent,
    vec3<f32>(0.0),
    vec3<f32>(0.999999)
  );

  let scaled_position = normalized_position * vec3<f32>(
    f32(volume_grid.width),
    f32(volume_grid.height),
    f32(volume_grid.depth)
  );

  return vec3<u32>(scaled_position);
}


@compute
@workgroup_size(8, 8, 8)
fn decay_volume(
  @builtin(global_invocation_id) global_id: vec3<u32>,
) {
  if (global_id.x >= volume_grid.width
    || global_id.y >= volume_grid.height
    || global_id.z >= volume_grid.depth) {
    return;
  }

  let index = voxel_index(global_id.x, global_id.y, global_id.z);
  let current_value_fixed = atomicLoad(&voxel_values[index]);
  let decayed_value_fixed = u32(f32(current_value_fixed) * 0.25);
  atomicStore(&voxel_values[index], decayed_value_fixed);
}

@compute @workgroup_size(256)
fn splat_points(@builtin(global_invocation_id) global_id: vec3<u32>) {
  let point_index = global_id.x;

  if (point_index >= volume_grid.point_count) {
    return;
  }

  let point_position_world = instances.values[point_index].translation.xyz;
  let center_voxel = world_to_grid(point_position_world);

  let center_deposit_fixed =
    u32(volume_grid.deposit_value * volume_grid.fixed_point_scale);

  let neighbor_deposit_fixed =
    u32((volume_grid.deposit_value * 0.5) * volume_grid.fixed_point_scale);

  for (var z_offset: i32 = -1; z_offset <= 1; z_offset = z_offset + 1) {
    for (var y_offset: i32 = -1; y_offset <= 1; y_offset = y_offset + 1) {
      for (var x_offset: i32 = -1; x_offset <= 1; x_offset = x_offset + 1) {
        let voxel_x = i32(center_voxel.x) + x_offset;
        let voxel_y = i32(center_voxel.y) + y_offset;
        let voxel_z = i32(center_voxel.z) + z_offset;

        if (!is_in_bounds(voxel_x, voxel_y, voxel_z)) {
          continue;
        }

        let is_center =
          x_offset == 0 &&
          y_offset == 0 &&
          z_offset == 0;

        let deposit_fixed = select(
          neighbor_deposit_fixed,
          center_deposit_fixed,
          is_center
        );

        let index = voxel_index(
          u32(voxel_x),
          u32(voxel_y),
          u32(voxel_z)
        );

        atomicAdd(&voxel_values[index], deposit_fixed);
      }
    }
  }
}

@compute @workgroup_size(8, 8, 8)
fn write_volume_texture(@builtin(global_invocation_id) global_id: vec3<u32>) {
  if (global_id.x >= volume_grid.width
    || global_id.y >= volume_grid.height
    || global_id.z >= volume_grid.depth) {
    return;
  }

  let index = voxel_index(global_id.x, global_id.y, global_id.z);
  let stored_value_fixed = atomicLoad(&voxel_values[index]);
  let value = f32(stored_value_fixed) / volume_grid.fixed_point_scale;

  textureStore(
    volume_texture,
    vec3<i32>(i32(global_id.x), i32(global_id.y), i32(global_id.z)),
    vec4<f32>(value, 0.0, 0.0, 0.0)
  );
}
