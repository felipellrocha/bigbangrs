@group(0) @binding(0) var<storage, read> input: array<u32>;
@group(0) @binding(1) var<storage, read_write> output: array<u32>;

@compute
@workgroup_size(64, 1, 1)
fn main(
  @builtin(workgroup_id) workgroup_id: vec3<u32>,
  @builtin(local_invocation_id) local_invocation_id: vec3<u32>,
  @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
) {
  let index = global_invocation_id.x;
  let total = arrayLength(&input);

  if (index >= total) { return; }

  output[global_invocation_id.x] = input[global_invocation_id.x];
}
