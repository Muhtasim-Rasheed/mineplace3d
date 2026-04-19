#version 330 core

layout(location = 0) in vec2 a_vertex;

layout(location = 1) in vec3 i_position;
layout(location = 2) in float i_size;
layout(location = 3) in vec2 i_uv_min;
layout(location = 4) in vec2 i_uv_max;
layout(location = 5) in int i_sprite_type;

out vec2 v_uv;
flat out int v_sprite_type;

uniform mat4 u_view;
uniform mat4 u_proj;

void main() {
	vec3 right = vec3(u_view[0][0], u_view[1][0], u_view[2][0]);
	vec3 up    = vec3(u_view[0][1], u_view[1][1], u_view[2][1]);
	vec3 world_pos = i_position
		+ right * a_vertex.x * i_size
		+ up    * a_vertex.y * i_size;
	gl_Position = u_proj * u_view * vec4(world_pos, 1.0);
	vec2 local_uv = a_vertex + vec2(0.5);
	v_uv = mix(i_uv_min, i_uv_max, local_uv);
	v_sprite_type = i_sprite_type;
}
