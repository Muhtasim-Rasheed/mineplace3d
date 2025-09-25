#version 330 core
layout(location = 0) in vec2 pos;
layout(location = 1) in vec2 uv;

out vec2 frag_uv;
out vec2 world_xz;

uniform mat4 view;
uniform mat4 projection;

void main() {
	vec3 camera_pos = -transpose(mat3(view)) * vec3(view[3][0], view[3][1], view[3][2]);
	world_xz = pos * 500.0 + camera_pos.xz;
	gl_Position = projection * view * vec4(world_xz.x, 96.0, world_xz.y, 1.0);
	frag_uv = uv;
}
