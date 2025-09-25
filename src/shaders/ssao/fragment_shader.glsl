#version 330 core
in vec2 frag_uv;
out vec4 occlusion;

uniform sampler2D depth_texture;
uniform sampler2D noise_texture;
uniform vec3 samples[64];
uniform mat4 projection;
uniform vec2 screen_size;

const int kernel_size = 64;
const float radius = 5.0;

vec3 getViewPos(vec2 uv, float depth) {
	vec4 clip = vec4(uv * 2.0 - 1.0, depth * 2.0 - 1.0, 1.0);
	vec4 view = inverse(projection) * clip;
	return view.xyz / view.w;
}

vec3 reconstructNormal(vec2 uv, sampler2D depthTex) {
	float depth = texture(depthTex, uv).r;
	vec3 p = getViewPos(uv, depth);

	float dx = 1.0 / screen_size.x;
	float dy = 1.0 / screen_size.y;

	vec3 px = getViewPos(uv + vec2(dx, 0.0), texture(depthTex, uv + vec2(dx, 0.0)).r);
	vec3 py = getViewPos(uv + vec2(0.0, dy), texture(depthTex, uv + vec2(0.0, dy)).r);

	vec3 dxVec = px - p;
	vec3 dyVec = py - p;

	return normalize(cross(dxVec, dyVec));
}

void main() {
	float depth = texture(depth_texture, frag_uv).r;
	if (depth >= 0.99999) {
		occlusion = vec4(1.0);
		return;
	}
	vec3 frag_pos = getViewPos(frag_uv, depth);
	if (!all(equal(frag_pos, frag_pos))) {
		occlusion = vec4(1.0);
		return;
	}
	vec3 normal = reconstructNormal(frag_uv, depth_texture);
	if (!all(equal(normal, normal))) {
		occlusion = vec4(1.0);
		return;
	}

	vec2 noise_scale = screen_size / 4.0;
	vec3 random_vec = texture(noise_texture, frag_uv * noise_scale).xyz * 2.0 - 1.0;
	vec3 tangent = normalize(random_vec - normal * dot(random_vec, normal));
	vec3 bitangent = cross(normal, tangent);
	mat3 TBN = mat3(tangent, bitangent, normal);

	float occ = 0.0;
	for (int i = 0; i < kernel_size; ++i) {
		vec3 sample_vec = TBN * samples[i];
		sample_vec = frag_pos + sample_vec * radius;

		vec4 offset = projection * vec4(sample_vec, 1.0);
		if (offset.w <= 0.0) continue;
		offset.xyz /= offset.w;
		if (!all(equal(offset.xyz, offset.xyz))) continue;
		offset.xyz = offset.xyz * 0.5 + 0.5;
		if (any(lessThan(offset.xy, vec2(0.0))) || any(greaterThan(offset.xy, vec2(1.0))))
			continue;

		float sample_depth = texture(depth_texture, offset.xy).r;
		vec3 sample_view_pos = getViewPos(offset.xy, sample_depth);
		float denom = abs(frag_pos.z - sample_view_pos.z) + 1e-5;
		float range_check = smoothstep(0.0, 1.0, radius / denom);
		occ += (sample_view_pos.z >= sample_vec.z ? 1.0 : 0.0) * range_check;
	}

	occlusion = vec4(1.0 - (occ / float(kernel_size)));
}
