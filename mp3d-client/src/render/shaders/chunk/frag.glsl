#version 330 core

flat in ivec3 v_normal;
in vec2 v_uv;
out vec4 frag_color;

uniform sampler2D u_texture;

void main() {
	frag_color = texture(u_texture, v_uv);
	if (frag_color.a < 0.1) {
		discard;
	}
	float intensity = 0.4;
	if (abs(v_normal.x) > 0.0) {
		intensity = 0.6;
	} else if (v_normal.y > 0.0) {
		intensity = 1.0;
	} else if (abs(v_normal.z) > 0.0) {
		intensity = 0.8;
	}
	frag_color.rgb *= intensity;
}
