#version 330 core

in vec2 v_uv;
in vec3 v_normal;
out vec4 frag_color;

uniform sampler2D u_tex;
uniform vec4 u_color;
uniform bool u_solid;
uniform bool u_shade;

void main() {
	if (u_solid) {
		frag_color = u_color;
	} else {
		frag_color = texture(u_tex, v_uv) * u_color;
	}
	if (frag_color.a < 0.1) {
		discard;
	}
	if (u_shade) {
		float shade = max(dot(normalize(v_normal), normalize(vec3(-0.05, 0.15, 0.6))), 0.0) * 1.5;
		frag_color.rgb *= shade;
	}
}
