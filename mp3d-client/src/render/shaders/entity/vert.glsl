#version 330 core

layout(location = 0) in vec3 a_pos;
layout(location = 1) in vec2 a_uv;
layout(location = 2) in vec3 a_normal;

uniform mat4 u_model;
uniform mat4 u_view;
uniform mat4 u_projection;

out vec2 v_uv;
out vec3 v_normal;

void main() {
	v_uv = a_uv;
	gl_Position = u_projection * u_view * u_model * vec4(a_pos, 1.0);
	v_normal = normalize(mat3(transpose(inverse(u_model))) * a_normal);
}
