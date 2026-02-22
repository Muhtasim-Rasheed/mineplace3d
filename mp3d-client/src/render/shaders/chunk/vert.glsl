#version 330 core

layout(location = 0) in vec3 a_pos;
layout(location = 1) in ivec3 a_normal;
layout(location = 2) in vec2 a_uv;

flat out ivec3 v_normal;
out vec2 v_uv;

uniform mat4 u_projection;
uniform mat4 u_view;

void main() {
	gl_Position = u_projection * u_view * vec4(a_pos, 1.0);
	v_normal = a_normal;
	v_uv = a_uv;
}
