#version 330 core

layout(location = 0) in vec3 a_pos;

uniform mat4 u_view;
uniform mat4 u_projection;
uniform vec3 u_offset;
uniform float u_scale;

void main() {
	vec3 pos = a_pos * u_scale + u_offset;
	gl_Position = u_projection * u_view * vec4(pos, 1.0);
}
