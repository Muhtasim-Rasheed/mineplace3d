#version 330 core

layout(location = 0) in vec3 a_pos;

out vec2 v_uv;

void main() {
	gl_Position = vec4(a_pos, 1.0);
	v_uv = a_pos.xy * 0.5 + 0.5;
}
