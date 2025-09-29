#version 330 core
layout(location = 0) in vec2 corner;
layout(location = 1) in vec2 uv;

uniform mat4 view;
uniform mat4 projection;

uniform vec3 center;
uniform float size;
uniform bool spherical;

out vec2 frag_uv;

void main() {
	vec3 camera_right = vec3(view[0][0], view[1][0], view[2][0]);
	vec3 camera_up    = vec3(0.0, 1.0, 0.0); // Cylindrical billboarding
	if (spherical) {
		camera_up = vec3(view[0][1], view[1][1], view[2][1]);
	}

	vec3 vertex_position = center + (camera_right * corner.x + camera_up * corner.y) * size;

	gl_Position = projection * view * vec4(vertex_position, 1.0);
	
	frag_uv = uv;
}
