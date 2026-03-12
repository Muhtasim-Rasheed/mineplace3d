#version 330 core

in vec2 v_uv;
out vec4 frag_color;

uniform sampler2D u_texture;
uniform sampler2D u_ssao;

void main() {
	frag_color = texture(u_texture, v_uv);
	float ao = texture(u_ssao, v_uv).r;
	frag_color.rgb *= ao;
}
