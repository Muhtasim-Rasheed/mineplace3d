#version 330 core
in vec2 frag_uv;
out vec4 final_color;

uniform sampler2D texture_sampler;
uniform vec4 ui_color;

void main() {
	vec4 color = texture(texture_sampler, frag_uv);
	if (color.a < 0.1) {
		discard;
	}
	final_color = color * ui_color;
}
