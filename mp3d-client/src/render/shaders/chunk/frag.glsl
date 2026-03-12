#version 330 core

layout(location = 0) out vec4 frag_color;
layout(location = 1) out vec4 frag_normal;

flat in ivec3 v_normal;
in vec2 v_uv;

uniform sampler2D u_texture;

vec2 encode_normal(vec3 normal) {
	normal /= abs(normal.x) + abs(normal.y) + abs(normal.z);

	if (normal.z < 0.0) {
		normal.xy = (1.0 - abs(normal.yx)) * sign(normal.xy);
	}

	return normal.xy * 0.5 + 0.5;
}

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
	frag_normal = vec4(encode_normal(vec3(v_normal)), 0.0, 1.0);
}
