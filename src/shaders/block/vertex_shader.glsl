#version 330 core
layout(location = 0) in uint hi;
layout(location = 1) in uint lo;

out vec3 frag_normal;
out vec2 frag_uv;
flat out uint frag_block_type;
out vec3 frag_foliage;
out vec3 frag_camera_pos;
out vec4 frag_pos;

uniform ivec3 chunk_pos;
uniform mat4 view;
uniform mat4 projection;
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

vec2 unpack_uv(uint hi, uint lo) {
	uint uv = (lo >> 18) & 0x3FFu;
	uint block_type = get_block_type(hi, lo);

	uint local_u = (uv >> 5) & 0x1Fu;
	uint local_v = uv & 0x1Fu;

	uint tile_index = block_type;
	uint tile_x = tile_index % 12u;
	uint tile_y = tile_index / 12u;
	float uv_unit = 1.0 / 12.0;
	return vec2(float(local_u), float(local_v)) / 192.0 + vec2(float(tile_x), float(tile_y)) * uv_unit;
}

ivec3 unpack_pos(uint hi, uint lo) {
	uint pos = lo & 0x7FFFu;

	uint x = (pos >> 10) & 0x1Fu;
	uint y = (pos >> 5) & 0x1Fu;
	uint z = pos & 0x1Fu;
	return ivec3(int(x), int(y), int(z)) + chunk_pos * 16;
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
	vec3 world_pos = vec3(unpack_pos(hi, lo));
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
	frag_uv = unpack_uv(hi, lo);
	frag_block_type = get_block_type(hi, lo);
	frag_foliage = unpack_foliage(hi, lo);
	frag_camera_pos = (view * vec4(world_pos, 1.0)).xyz;
	frag_pos = gl_Position;
}
