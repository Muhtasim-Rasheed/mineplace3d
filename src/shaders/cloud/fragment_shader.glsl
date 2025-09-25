#version 330 core
in vec2 frag_uv;
in vec2 world_xz;
out vec4 final_color;

uniform sampler2D cloud_texture;
uniform float time;

void main() {
	vec2 uv = ((world_xz * 0.001) + vec2(time * 0.0025, 0.0)) + 0.5;
	vec4 cloud = texture(cloud_texture, uv);
	final_color = vec4(cloud.rgb, cloud.a * 0.5);
}
