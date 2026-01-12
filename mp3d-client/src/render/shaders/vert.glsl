#version 330 core

layout(location = 0) in vec3 a_pos;
layout(location = 1) in ivec3 a_normal;
layout(location = 2) in vec3 a_color;

flat out ivec3 v_normal;
out vec3 v_color;

uniform mat4 u_projection;
uniform mat4 u_view;

void main() {
	gl_Position = u_projection * u_view * vec4(a_pos, 1.0);
	v_normal = a_normal;
	v_color = a_color;
}
