#version 330 core

in vec2 v_uv;
out vec4 frag_color;

uniform sampler2D u_texture;
uniform sampler2D u_depth;
uniform sampler2D u_ssao;
uniform float u_time;

void main() {
	frag_color = texture(u_texture, v_uv);

	vec2 texel = 1.0 / vec2(textureSize(u_ssao, 0));

	float center_depth = texture(u_depth, v_uv).r;
	float ao = 0.0;
	float weight_sum = 0.0;

	for (int x = -4; x <= 4; x++)
		for (int y = -4; y <= 4; y++) {
			vec2 offset = vec2(x,y) * texel;

			float sample_ao = texture(u_ssao, v_uv + offset).r;
			float sample_depth = texture(u_depth, v_uv + offset).r;

			float depth_weight = exp(-abs(center_depth - sample_depth) * 50.0);
			float spatial_weight = exp(-(x*x + y*y) / (2.0 * 4.0 * 4.0));

			float w = depth_weight * spatial_weight;

			ao += sample_ao * w;
			weight_sum += w;
		}

	ao /= weight_sum;

	frag_color.rgb *= ao;
}
