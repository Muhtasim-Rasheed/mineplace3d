#version 330 core

flat in ivec3 v_normal;
in vec3 v_color;
out vec4 frag_color;

void main() {
	frag_color = vec4(v_color, 1.0);
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
