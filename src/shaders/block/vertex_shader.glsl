#version 330 core
layout(location = 0) in ivec3 pos;
layout(location = 1) in vec3 normal;
layout(location = 2) in vec2 uv;
layout(location = 3) in uint block_type;
layout(location = 4) in vec3 foliage;

out vec3 frag_normal;
out vec2 frag_uv;
flat out uint frag_block_type;
out vec3 frag_foliage;
out vec3 frag_camera_pos;
out vec4 frag_pos;

uniform mat4 view;
uniform mat4 projection;
uniform float time;

void main() {
	gl_Position = vec4(pos, 1.0);
	if (block_type == 6u) {
		float waveX = sin(time * 2.0 + pos.x * 0.5) * 0.0225;
		float waveZ = cos(time * 2.5 + pos.z * 0.6) * 0.0225;

		gl_Position.x += waveX;
		gl_Position.z += waveZ;
	}
	gl_Position = projection * view * gl_Position;
	frag_normal = normalize(normal);
	frag_uv = uv;
	frag_block_type = block_type;
	frag_foliage = foliage;
	frag_camera_pos = (view * vec4(pos, 1.0)).xyz;
	frag_pos = gl_Position;
}
