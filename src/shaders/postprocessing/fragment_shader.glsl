#version 330 core
in vec2 frag_uv;
out vec4 final_color;

uniform sampler2D texture_sampler;
uniform sampler2D ssao_texture;

void main() {
	float ao = texture(ssao_texture, frag_uv).r;
	vec4 color = texture(texture_sampler, frag_uv);
	if (color.a < 0.1) {
		discard;
	}
	color = vec4(color.rgb * ao, color.a);

	final_color = color;
}
