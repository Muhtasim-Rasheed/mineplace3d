#version 330 core

in vec2 v_uv;
out vec4 frag_color;

uniform sampler2D u_texture;
uniform sampler2D u_depth;
uniform sampler2D u_ssao;

void main() {
	frag_color = texture(u_texture, v_uv);
	// float ao = texture(u_ssao, v_uv).r;
	// frag_color.rgb *= ao;
	// blur
	vec2 texel = 1.0 / vec2(textureSize(u_ssao, 0));

	float centerDepth = texture(u_depth, v_uv).r;
	float ao = 0.0;
	float weightSum = 0.0;

	for (int x = -3; x <= 3; x++)
		for (int y = -3; y <= 3; y++) {
			vec2 offset = vec2(x,y) * texel;

			float sampleAO = texture(u_ssao, v_uv + offset).r;
			float sampleDepth = texture(u_depth, v_uv + offset).r;

			float depthWeight = exp(-abs(centerDepth - sampleDepth) * 50.0);
			float spatialWeight = exp(-(x*x + y*y) / (2.0 * 3.0 * 3.0));

			float w = depthWeight * spatialWeight;

			ao += sampleAO * w;
			weightSum += w;
		}

	ao /= weightSum;

	frag_color.rgb *= ao;
}
