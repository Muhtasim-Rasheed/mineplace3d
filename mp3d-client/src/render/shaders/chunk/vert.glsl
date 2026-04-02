#version 330 core

layout(location = 0) in vec3 a_pos;
layout(location = 1) in ivec3 a_normal;
layout(location = 2) in vec2 a_uv;
layout(location = 3) in uint a_ao;

flat out ivec3 v_normal;
out float v_ao;
out vec2 v_uv;

uniform mat4 u_projection;
uniform mat4 u_view;

void main() {
	gl_Position = u_projection * u_view * vec4(a_pos, 1.0);
	v_normal = a_normal;
	v_ao = mix(0.4, 1.0, float(a_ao) / 3.0);
	v_uv = a_uv;
}
