#version 330 core
layout(location = 0) in uint pos;
layout(location = 1) in uint normal;
layout(location = 2) in uint uv;
layout(location = 3) in uint block_type;
layout(location = 4) in uint foliage;

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

vec2 unpack_uv(uint uv) {
	uint u = (uv >> 16) & 0xFFFFu;
	uint v = uv & 0xFFFFu;
	return vec2(float(u) / 65535.0, float(v) / 65535.0);
}

ivec3 unpack_pos(uint pos) {
	uint x = (pos >> 10) & 0x1Fu;
	uint y = (pos >> 5) & 0x1Fu;
	uint z = pos & 0x1Fu;
	return ivec3(int(x), int(y), int(z)) + chunk_pos * 16;
}

vec3 unpack_color(uint color) {
	uint r = (color >> 16) & 0xFFu;
	uint g = (color >> 8) & 0xFFu;
	uint b = color & 0xFFu;
	return vec3(float(r) / 255.0, float(g) / 255.0, float(b) / 255.0);
}

void main() {
	vec3 world_pos = vec3(unpack_pos(pos));
	gl_Position = vec4(world_pos, 1.0);
	if (block_type == 6u) {
		float waveX = sin(time * 2.0 + world_pos.x * 0.5) * 0.0225;
		float waveZ = cos(time * 2.5 + world_pos.z * 0.6) * 0.0225;

		gl_Position.x += waveX;
		gl_Position.z += waveZ;
	}
	gl_Position = projection * view * gl_Position;
	frag_normal = normalize(normals[normal]);
	frag_uv = unpack_uv(uv);
	frag_block_type = block_type;
	frag_foliage = unpack_color(foliage);
	frag_camera_pos = (view * vec4(world_pos, 1.0)).xyz;
	frag_pos = gl_Position;
}
