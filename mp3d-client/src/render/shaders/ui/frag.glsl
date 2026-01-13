#version 330 core

in vec2 v_uv;
out vec4 frag_color;

uniform sampler2D u_tex;
uniform vec4 u_color;
uniform bool u_solid;

void main() {
	if (u_solid) {
		frag_color = u_color;
	} else {
		frag_color = texture(u_tex, v_uv) * u_color;
	}
}
