#version 330 core

in vec2 v_uv;
in vec3 v_normal;
out vec4 frag_color;

uniform sampler2D u_texture;

void main() {
	vec3 light_dir = normalize(vec3(-0.5, -1.0, -0.5));
	float intensity = max(dot(v_normal, light_dir), 0.0);
	float ambient = 0.4;
	frag_color = texture(u_texture, v_uv);
	frag_color.rgb *= (ambient + intensity * 0.8);
}
