#version 330 core
layout(location = 0) in uint hi;
layout(location = 1) in uint lo;
layout(location = 2) in vec3 position;

out vec3 frag_normal;
out vec2 frag_uv;
flat out uint frag_block_type;
out vec3 frag_foliage;
out vec3 frag_camera_pos;
out vec4 frag_pos;

uniform ivec3 chunk_pos;
uniform mat4 view;
uniform mat4 projection;
uniform float chunk_side_length;
uniform float time;

const vec3 normals[6] = vec3[](
	vec3(0.0, 0.0, -1.0), // North
	vec3(0.0, 0.0, 1.0),  // South
	vec3(1.0, 0.0, 0.0),	// East
	vec3(-1.0, 0.0, 0.0), // West
	vec3(0.0, 1.0, 0.0),  // Up
	vec3(0.0, -1.0, 0.0)  // Down
);

uint get_block_type(uint hi, uint lo) {
	uint lower = (lo >> 28) & 0xFu;
	uint upper = hi & 0xFFFu;
	return (upper << 4) | lower;
}

uint get_normal_idx(uint hi, uint lo) {
		return (lo >> 15) & 0x7u;
}

uint hash_uvec3(uvec3 v) {
	v = v * 1664525u + 1013904223u;
	v.x += v.y * v.z;
	v.y += v.z * v.x;
	v.z += v.x * v.y;
	v ^= v >> 16;
	return v.x ^ v.y ^ v.z;
}

uint flip_uvs(uint block_type, uvec3 position) {
	if (block_type == 0x0004u || block_type == 0x0006u || block_type == 0x000Cu) {
		uint h = hash_uvec3(position);
		return h % 4u;
	}
	return 0u;
}

vec2 unpack_uv(uint hi, uint lo, uvec3 position) {
	uint uv = (lo >> 18) & 0x3FFu;
	uint block_type = get_block_type(hi, lo);

	uint texel_u = (uv >> 5) & 0x1Fu;
	uint texel_v = uv & 0x1Fu;

	uint tile_index = block_type;
	uint tile_x = tile_index % 12u;
	uint tile_y = tile_index / 12u;

	uint tile_u = texel_u - 16u * tile_x;
	uint tile_v = texel_v - 16u * tile_y;

	uint flip = flip_uvs(block_type, uvec3(position));
	if ((flip & 1u) == 1u) {
		tile_u = 16u - texel_u - 16u * tile_x;
	}
	if ((flip & 2u) == 2u) {
		tile_v = 16u - texel_v - 16u * tile_y;
	}

	texel_u = tile_u + 16u * tile_x;
	texel_v = tile_v + 16u * tile_y;

	float uv_unit = 1.0 / 12.0;
	return vec2(float(texel_u), float(texel_v)) / 192.0 + vec2(float(tile_x), float(tile_y)) * uv_unit;
}

vec3 get_pos(vec3 position) {
	return position + vec3(chunk_pos * chunk_side_length);
}

uvec3 get_pos_grid(uint hi, uint lo) {
	uint all_components = (lo >> 3) & 0xFFFFFFu;
	uint x = all_components & 0xFu;
	uint y = (all_components >> 4) & 0xFu;
	uint z = (all_components >> 8) & 0xFu;
	return uvec3(x, y, z);
}

vec3 unpack_color(uint color) {
	uint r = (color >> 14) & 0x3Fu;
	uint g = (color >> 7) & 0x7Fu;
	uint b = color & 0x7Fu;
	return vec3(float(r) / 63.0, float(g) / 127.0, float(b) / 127.0);
}

vec3 unpack_foliage(uint hi, uint lo) {
	uint foliage = (hi >> 12) & 0xFFFFFu;
	return unpack_color(foliage);
}

void main() {
	vec3 world_pos = get_pos(position);
	uint block_type = get_block_type(hi, lo);
	gl_Position = vec4(world_pos, 1.0);
	if (block_type == 6u) {
		float waveX = sin(time * 2.0 + world_pos.x * 0.5) * 0.0225;
		float waveZ = cos(time * 2.5 + world_pos.z * 0.6) * 0.0225;

		gl_Position.x += waveX;
		gl_Position.z += waveZ;
	}
	gl_Position = projection * view * gl_Position;
	frag_normal = normalize(normals[get_normal_idx(hi, lo)]);
	frag_uv = unpack_uv(hi, lo, get_pos_grid(hi, lo));
	frag_block_type = get_block_type(hi, lo);
	frag_foliage = unpack_foliage(hi, lo);
	frag_camera_pos = (view * vec4(world_pos, 1.0)).xyz;
	frag_pos = gl_Position;
}
