#version 330 core
in vec3 frag_normal;
in vec3 frag_foliage;
in vec2 frag_uv;
flat in uint frag_block_type;
in vec3 frag_camera_pos;
in vec4 frag_pos;
out vec4 final_color;

uniform sampler2D texture_sampler;
uniform uint textures_per_row;
uniform uint texture_row_count;

void main() {
	vec4 base_color = texture(texture_sampler, frag_uv);

	if (base_color.a < 0.1) {
		discard;
	}

	if (frag_block_type == 1u) base_color *= vec4(frag_foliage, 1.0);
	if (frag_block_type == 6u) base_color *= vec4(frag_foliage * vec3(0.9, 1.3, 0.9), 1.0);

	vec3 light_color = vec3(1.0, 1.0, 1.0);
	vec3 light_dir = normalize(vec3(1.2, 1.0, 0.7));
	float ambient = 0.44;
	vec3 diffuse = max(dot(frag_normal, light_dir), 0.0) * light_color;
	vec3 lighting = ambient + diffuse;
	float y_factor = clamp(frag_normal.y, 0.0, 1.0);
	lighting *= mix(0.7, 1.0, y_factor);

	final_color = vec4(lighting, 1.0) * base_color;
}
