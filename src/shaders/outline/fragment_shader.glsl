#version 330 core
out vec4 final_color;

uniform vec3 outline_color;

void main() {
	final_color = vec4(outline_color, 1.0);
}
