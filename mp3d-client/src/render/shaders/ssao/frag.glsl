// SSAO!

#version 330 core

layout(location = 0) out vec4 frag_occlusion;

in vec2 v_uv;

uniform sampler2D u_depth;
uniform sampler2D u_normal;
uniform sampler2D u_noise;
uniform vec2 u_noise_scale;
uniform vec3 u_samples[32];
uniform mat4 u_projection;
uniform mat4 u_inv_projection;

vec3 get_position(vec2 uv) {
	float depth = texture(u_depth, uv).r;

	vec4 clip = vec4(uv * 2.0 - 1.0, depth * 2.0 - 1.0, 1.0);
	vec4 view = u_inv_projection * clip;

	return view.xyz / view.w;
}

vec3 decode_normal(vec2 encoded) {
	vec3 normal;
	normal.xy = encoded * 2.0 - 1.0;
	normal.z = 1.0 - abs(normal.x) - abs(normal.y);
	if (normal.z < 0.0) {
		normal.xy = (1.0 - abs(normal.yx)) * sign(normal.xy);
	}
	return normalize(normal);
}

vec3 get_normal(vec2 uv) {
	vec2 encoded = texture(u_normal, uv).xy;
	return decode_normal(encoded);
}

void main() {
	vec3 pos = get_position(v_uv);

	if (pos.z >= 0.99999) {
		frag_occlusion = vec4(1.0);
		return;
	}

	if (!all(equal(pos, pos))) {
		frag_occlusion = vec4(1.0);
		return;
	}

	vec3 normal = get_normal(v_uv);

	if (!all(equal(normal, normal))) {
		frag_occlusion = vec4(1.0);
		return;
	}

	vec3 random = texture(u_noise, v_uv * u_noise_scale).xyz;

	vec3 tangent = normalize(random - normal * dot(random, normal));
	vec3 bitangent = cross(normal, tangent);
	mat3 TBN = mat3(tangent, bitangent, normal);

	float occlusion = 0.0;
	float radius = 2.0;
	float bias = 0.025;

	for (int i = 0; i < 32; i++) {
		vec3 sample_pos = pos + TBN * u_samples[i] * radius;

		vec4 offset = u_projection * vec4(sample_pos, 1.0);

		if (offset.w <= 0.0) {
			continue;
		}

		offset.xyz /= offset.w;
		offset.xyz = offset.xyz * 0.5 + 0.5;

		if (any(lessThan(offset.xy, vec2(0.0))) || any(greaterThan(offset.xy, vec2(1.0))))
			continue;

		vec3 sample_view_pos = get_position(offset.xy);
		float sample_depth = sample_view_pos.z;

		float range_check = smoothstep(0.0, 1.0, radius / abs(pos.z - sample_depth + 0.0001));

		if (sample_depth >= sample_pos.z + bias) {
			occlusion += range_check;
		}
	}

	occlusion = 1.0 - occlusion / 32.0;
	frag_occlusion = vec4(vec3(pow(occlusion, 0.5)), 1.0);
}
