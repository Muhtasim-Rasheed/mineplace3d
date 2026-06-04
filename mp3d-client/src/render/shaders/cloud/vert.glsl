// simple cloud shader thingy

#version 330 core

layout(location = 0) in vec2 a_pos;

out vec2 v_uv;

uniform mat4 u_projection;
uniform mat4 u_view;
uniform vec3 u_camera_pos;
uniform float u_time;
uniform float u_offset;
uniform float u_speed;
uniform uint u_altitude;

void main() {
	vec3 world_pos = vec3(
		a_pos.x * 200.0 + u_camera_pos.x,
		float(u_altitude) - 0.23, // Slight offset to prevent z-fighting with the blocks.
		a_pos.y * 200.0 + u_camera_pos.z
	);

	gl_Position = u_projection * u_view * vec4(world_pos, 1.0);

	v_uv = a_pos * 0.5 + 0.5 + vec2(u_camera_pos.x, u_camera_pos.z) / 200.0;
	v_uv.x += u_time * u_speed + u_offset / 200.0;
}
